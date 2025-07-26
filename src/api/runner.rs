//! Fluent runners for executing single prompts or complex chains.

use llm::{
    chain::{MultiChainStepBuilder, MultiChainStepMode, MultiPromptChain},
    LLMProvider,
};
use regex::Regex;
use std::collections::{HashMap, HashSet};

use super::{
    error::{RunError, StoreError},
    llm_bridge::LLMBackendRef,
    store::PromptStore,
    RunOutput,
};

/// A fluent builder to configure and execute a single stored prompt.
pub struct PromptRunner<'a> {
    store: &'a PromptStore,
    id_or_title: &'a str,
    vars: HashMap<String, String>,
    backend: Option<&'a dyn LLMProvider>,
}

impl<'a> PromptRunner<'a> {
    /// Creates a new `PromptRunner`.
    pub(crate) fn new(store: &'a PromptStore, id_or_title: &'a str) -> Self {
        Self {
            store,
            id_or_title,
            vars: HashMap::new(),
            backend: None,
        }
    }

    /// Sets the variables for template substitution in the prompt.
    pub fn vars(
        mut self,
        vars: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.vars = vars
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }

    /// Sets the LLM backend to execute the prompt with.
    /// If not set, `run()` will only perform template substitution and return the result.
    pub fn backend(mut self, llm: &'a dyn LLMProvider) -> Self {
        self.backend = Some(llm);
        self
    }

    /// Finds, decrypts, renders, and executes the prompt.
    pub async fn run(self) -> Result<RunOutput, RunError> {
        let pd = self.store.find_prompt(self.id_or_title)?;
        let rendered = render_template(&pd.content, &self.vars);

        let result = if let Some(llm) = self.backend {
            use llm::chat::ChatMessage;
            let req = ChatMessage::user().content(&rendered).build();
            let resp = llm.chat(&[req]).await?;
            resp.text().unwrap_or_default()
        } else {
            rendered // No backend, just return the rendered prompt
        };

        Ok(RunOutput::Prompt(result))
    }
}

// --- ChainRunner for multi-step chains ---

#[derive(Clone)]
pub(crate) struct ChainStepDefinition {
    pub output_key: String,
    pub prompt_id_or_title: String,
    pub provider_id: Option<String>,
    pub mode: MultiChainStepMode,
}

/// A fluent builder to define and execute a multi-step prompt chain.
pub struct ChainRunner<'a> {
    store: &'a PromptStore,
    backend: LLMBackendRef<'a>,
    steps: Vec<ChainStepDefinition>,
    vars: HashMap<String, String>,
}

impl<'a> ChainRunner<'a> {
    /// Creates a new `ChainRunner`.
    pub(crate) fn new(store: &'a PromptStore, backend: LLMBackendRef<'a>) -> Self {
        Self {
            store,
            backend,
            steps: Vec::new(),
            vars: HashMap::new(),
        }
    }

    /// Adds a new step to the chain.
    ///
    /// # Arguments
    ///
    /// * `output_key` - The name of the variable where this step's output will be stored.
    ///   This key can be used in subsequent prompt templates (e.g., `{{output_key}}`).
    /// * `prompt` - The ID or title of the prompt to load from the store for this step.
    pub fn step(mut self, output_key: &str, prompt: &str) -> Self {
        self.steps.push(ChainStepDefinition {
            output_key: output_key.to_string(),
            prompt_id_or_title: prompt.to_string(),
            provider_id: None,
            mode: MultiChainStepMode::Completion, // Default mode
        });
        self
    }

    /// Specifies the provider from the `LLMRegistry` to use for the *last added step*.
    pub fn with_provider(mut self, provider_id: &str) -> Self {
        if let Some(last_step) = self.steps.last_mut() {
            last_step.provider_id = Some(provider_id.to_string());
        }
        self
    }

    /// Sets the execution mode for the *last added step*.
    pub fn with_mode(mut self, mode: MultiChainStepMode) -> Self {
        if let Some(last_step) = self.steps.last_mut() {
            last_step.mode = mode;
        }
        self
    }

    /// Sets the initial variables for the chain. These are available to all steps.
    pub fn vars(
        mut self,
        vars: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.vars = vars
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }

    /// Executes the entire chain sequentially.
    ///
    /// The output of each step becomes available as a variable for all subsequent steps.
    pub async fn run(self) -> Result<RunOutput, RunError> {
        let reg = match self.backend {
            LLMBackendRef::Registry(reg) => reg,
            LLMBackendRef::Provider(_) => {
                return Err(StoreError::Configuration(
                    "ChainRunner requires an LLMRegistry, not a single Provider.".to_string(),
                )
                .into())
            }
        };

        let mut multi_chain = MultiPromptChain::new(reg);
        let step_ids: HashSet<_> = self.steps.iter().map(|s| s.output_key.clone()).collect();
        let re = Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap();

        for step_def in self.steps {
            let provider_id = step_def.provider_id.ok_or_else(|| {
                StoreError::Configuration(format!(
                    "Step '{}' is missing a provider ID.",
                    step_def.output_key
                ))
            })?;
            let prompt_data = self.store.find_prompt(&step_def.prompt_id_or_title)?;

            // Pre-render initial variables, but leave placeholders for step outputs.
            let rendered_template = re
                .replace_all(&prompt_data.content, |caps: &regex::Captures| {
                    let key = &caps[1];
                    if self.vars.contains_key(key) && !step_ids.contains(key) {
                        self.vars.get(key).unwrap().clone()
                    } else {
                        caps[0].to_string()
                    }
                })
                .into_owned();

            let multi_step = MultiChainStepBuilder::new(step_def.mode)
                .provider_id(&provider_id)
                .id(&step_def.output_key)
                .template(rendered_template)
                .build()?;

            multi_chain = multi_chain.step(multi_step);
        }

        let result_map = multi_chain.run().await?;
        Ok(RunOutput::Chain(result_map))
    }
}


/// Renders a template string with the given variables.
fn render_template(template: &str, vars: &HashMap<String, String>) -> String {
    let re = Regex::new(r"\{\{\s*(\w+)\s*\}\}").unwrap();
    re.replace_all(template, |caps: &regex::Captures| {
        let key = &caps[1];
        vars.get(key).map(|s| s.as_str()).unwrap_or("").to_string()
    })
    .into_owned()
}