use prompt_store::Builder;
use llm::builder::{LLMBuilder, LLMBackend};

#[tokio::main]
async fn main() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let openai_llm = LLMBuilder::new()
        .backend(LLMBackend::OpenAI)
        .api_key(api_key)
        .model("gpt-4o")
        .build()
        .unwrap();

    let result = Builder::new()
        .prompt("welcome")
        .vars([("name", "Alice")])
        .backend(openai_llm.as_ref())
        .run()
        .await
        .expect("Prompt execution failed");

}
