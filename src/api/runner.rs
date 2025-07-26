//! Fluent runners for executing single prompts or complex chains.

use futures::future;
use llm::{chain::MultiChainStepMode, LLMProvider};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{
    error::{RunError, StoreError},
    llm_bridge::LLMBackendRef,
    store::PromptStore,
    RunOutput,
};

/// Represents the source of a prompt for a chain step.
#[derive(Clone)]
enum PromptSource {
    /// Load the prompt from the store using its ID or title.
    Stored(String),
    /// Use a raw, in-memory string as the prompt template.
    Raw(String),
}

// --- PromptRunner for single prompts ---

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
            rendered
        };

        Ok(RunOutput::Prompt(result))
    }
}

// --- ChainRunner for multi-step chains ---

/// Defines a single step in a chain.
struct ChainStepDefinition<'a> {
    pub output_key: String,
    pub source: PromptSource,
    pub provider_id: Option<String>,
    pub mode: MultiChainStepMode,
    pub condition: Option<Box<dyn Fn(&HashMap<String, String>) -> bool + Send + Sync + 'a>>,
    pub fallback_source: Option<PromptSource>,
}

/// Represents a node in the execution graph of a chain.
enum ExecutionNode<'a> {
    /// A single, sequential step.
    Step(ChainStepDefinition<'a>),
    /// A group of steps to be executed in parallel.
    Parallel(Vec<ChainStepDefinition<'a>>),
}

/// A builder for defining a group of parallel steps.
pub struct ParallelGroupBuilder<'a> {
    steps: Vec<ChainStepDefinition<'a>>,
}

impl<'a> ParallelGroupBuilder<'a> {
    fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Adds a step from the store to the parallel group.
    pub fn step(mut self, output_key: &str, prompt_id_or_title: &str) -> Self {
        self.steps.push(ChainStepDefinition {
            output_key: output_key.to_string(),
            source: PromptSource::Stored(prompt_id_or_title.to_string()),
            provider_id: None,
            mode: MultiChainStepMode::Completion,
            condition: None,
            fallback_source: None,
        });
        self
    }

    /// Adds a raw prompt step to the parallel group.
    pub fn step_raw(mut self, output_key: &str, prompt_content: &str) -> Self {
        self.steps.push(ChainStepDefinition {
            output_key: output_key.to_string(),
            source: PromptSource::Raw(prompt_content.to_string()),
            provider_id: None,
            mode: MultiChainStepMode::Completion,
            condition: None,
            fallback_source: None,
        });
        self
    }

    /// Sets the provider for the last added step in the parallel group.
    pub fn with_provider(mut self, provider_id: &str) -> Self {
        if let Some(last_step) = self.steps.last_mut() {
            last_step.provider_id = Some(provider_id.to_string());
        }
        self
    }
}

/// A fluent builder to define and execute a multi-step prompt chain.
pub struct ChainRunner<'a> {
    store: &'a PromptStore,
    backend: LLMBackendRef<'a>,
    nodes: Vec<ExecutionNode<'a>>,
    vars: HashMap<String, String>,
}

impl<'a> ChainRunner<'a> {
    /// Creates a new `ChainRunner`.
    pub(crate) fn new(store: &'a PromptStore, backend: LLMBackendRef<'a>) -> Self {
        Self {
            store,
            backend,
            nodes: Vec::new(),
            vars: HashMap::new(),
        }
    }

    /// Adds a sequential step from the store.
    pub fn step(mut self, output_key: &str, prompt_id_or_title: &str) -> Self {
        self.nodes.push(ExecutionNode::Step(ChainStepDefinition {
            output_key: output_key.to_string(),
            source: PromptSource::Stored(prompt_id_or_title.to_string()),
            provider_id: None,
            mode: MultiChainStepMode::Completion,
            condition: None,
            fallback_source: None,
        }));
        self
    }

    /// Adds a sequential step with a raw prompt.
    pub fn step_raw(mut self, output_key: &str, prompt_content: &str) -> Self {
        self.nodes.push(ExecutionNode::Step(ChainStepDefinition {
            output_key: output_key.to_string(),
            source: PromptSource::Raw(prompt_content.to_string()),
            provider_id: None,
            mode: MultiChainStepMode::Completion,
            condition: None,
            fallback_source: None,
        }));
        self
    }

    /// Adds a conditional step from the store. It runs only if the condition is met.
    pub fn step_if<F>(mut self, output_key: &str, prompt_id_or_title: &str, condition: F) -> Self
    where
        F: Fn(&HashMap<String, String>) -> bool + Send + Sync + 'a,
    {
        self.nodes.push(ExecutionNode::Step(ChainStepDefinition {
            output_key: output_key.to_string(),
            source: PromptSource::Stored(prompt_id_or_title.to_string()),
            provider_id: None,
            mode: MultiChainStepMode::Completion,
            condition: Some(Box::new(condition)),
            fallback_source: None,
        }));
        self
    }

    /// Adds a group of steps that will be executed in parallel.
    pub fn parallel<F>(mut self, build_group: F) -> Self
    where
        F: for<'b> FnOnce(ParallelGroupBuilder<'b>) -> ParallelGroupBuilder<'b>,
    {
        let group_builder = ParallelGroupBuilder::new();
        let finished_group = build_group(group_builder);
        self.nodes
            .push(ExecutionNode::Parallel(finished_group.steps));
        self
    }

    /// Sets a fallback prompt from the store for the last added step.
    /// This is executed if the primary prompt execution fails.
    pub fn on_error_stored(mut self, fallback_id_or_title: &str) -> Self {
        if let Some(node) = self.nodes.last_mut() {
            if let ExecutionNode::Step(step_def) = node {
                step_def.fallback_source =
                    Some(PromptSource::Stored(fallback_id_or_title.to_string()));
            }
        }
        self
    }

    /// Sets a raw fallback prompt for the last added step.
    pub fn on_error_raw(mut self, fallback_content: &str) -> Self {
        if let Some(node) = self.nodes.last_mut() {
            if let ExecutionNode::Step(step_def) = node {
                step_def.fallback_source = Some(PromptSource::Raw(fallback_content.to_string()));
            }
        }
        self
    }

    /// Specifies the provider for the last added step or all steps in the last parallel group.
    pub fn with_provider(mut self, provider_id: &str) -> Self {
        if let Some(node) = self.nodes.last_mut() {
            match node {
                ExecutionNode::Step(step) => {
                    step.provider_id = Some(provider_id.to_string());
                }
                ExecutionNode::Parallel(steps) => {
                    for step in steps {
                        if step.provider_id.is_none() {
                            step.provider_id = Some(provider_id.to_string());
                        }
                    }
                }
            }
        }
        self
    }

    /// Sets the execution mode for the last added step.
    pub fn with_mode(mut self, mode: MultiChainStepMode) -> Self {
        if let Some(ExecutionNode::Step(step)) = self.nodes.last_mut() {
            step.mode = mode;
        }
        self
    }

    /// Sets initial variables for the chain.
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

    /// Executes the chain.
    pub async fn run(self) -> Result<RunOutput, RunError> {
        let reg = match self.backend {
            LLMBackendRef::Registry(reg) => reg,
            _ => {
                return Err(StoreError::Configuration(
                    "ChainRunner requires a LLMRegistry".to_string(),
                )
                .into())
            }
        };

        let context = Arc::new(Mutex::new(self.vars.clone()));

        for node in &self.nodes {
            match node {
                ExecutionNode::Step(step_def) => {
                    self.execute_step(step_def, Arc::clone(&context), reg)
                        .await?;
                }
                ExecutionNode::Parallel(steps) => {
                    let tasks = steps
                        .iter()
                        .map(|step| {
                            let context_clone = Arc::clone(&context);
                            self.execute_step(step, context_clone, reg)
                        })
                        .collect::<Vec<_>>();

                    future::try_join_all(tasks).await?;
                }
            }
        }

        let final_context = Arc::try_unwrap(context).ok().unwrap().into_inner().unwrap();
        Ok(RunOutput::Chain(final_context))
    }

    async fn execute_step(
        &self,
        step_def: &ChainStepDefinition<'a>,
        context: Arc<Mutex<HashMap<String, String>>>,
        reg: &'a llm::chain::LLMRegistry,
    ) -> Result<(), RunError> {
        let should_run = {
            let ctx = context.lock().unwrap();
            step_def.condition.as_ref().map_or(true, |cond| cond(&ctx))
        };
        if !should_run {
            return Ok(());
        }

        let result = self
            .try_execute_source(&step_def.source, &context, step_def, reg)
            .await;

        let final_output = match (result, &step_def.fallback_source) {
            (Ok(output), _) => Ok(output),
            (Err(_), Some(fallback)) => {
                self.try_execute_source(fallback, &context, step_def, reg)
                    .await
            }
            (Err(e), None) => Err(e),
        }?;

        let mut ctx = context.lock().unwrap();
        ctx.insert(step_def.output_key.clone(), final_output);
        Ok(())
    }

    async fn try_execute_source(
        &self,
        source: &PromptSource,
        context: &Arc<Mutex<HashMap<String, String>>>,
        step_def: &ChainStepDefinition<'a>,
        reg: &'a llm::chain::LLMRegistry,
    ) -> Result<String, RunError> {
        let provider_id = step_def.provider_id.as_deref().ok_or_else(|| {
            StoreError::Configuration(format!(
                "Step '{}' is missing a provider ID.",
                step_def.output_key
            ))
        })?;
        let provider = reg.get(provider_id).ok_or_else(|| {
            StoreError::Configuration(format!("Provider '{}' not found in registry", provider_id))
        })?;

        let prompt_content = match source {
            PromptSource::Stored(id) => self.store.find_prompt(id)?.content,
            PromptSource::Raw(content) => content.clone(),
        };

        let rendered = {
            let ctx = context.lock().unwrap();
            render_template(&prompt_content, &ctx)
        };

        use llm::chat::ChatMessage;
        let req = ChatMessage::user().content(&rendered).build();
        let resp = provider.chat(&[req]).await?;
        Ok(resp.text().unwrap_or_default())
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
