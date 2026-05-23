use grimoire_sdk::{ChatChunk, ChatRequest, ChatResponse, Context, Model, Plugin, PluginError};

mod ollama;

pub struct OllamaPlugin;

impl Plugin for OllamaPlugin {
    fn name(&self) -> &'static str {
        "grimoire-ollama"
    }

    fn is_provider(&self) -> bool {
        true
    }

    async fn list_models(&self, ctx: &Context) -> Result<Vec<Model>, PluginError> {
        let base_url = ollama_url(ctx)?;
        let models = ollama::list_models(&base_url).await?;
        Ok(models)
    }

    async fn handle_chat(
        &self,
        ctx: &Context,
        request: &ChatRequest,
    ) -> Result<ChatResponse, PluginError> {
        let base_url = ollama_url(ctx)?;
        let response = ollama::chat(&base_url, request).await?;
        Ok(response)
    }

    async fn handle_chat_stream(
        &self,
        ctx: &Context,
        request: &ChatRequest,
        tx: tokio::sync::mpsc::Sender<ChatChunk>,
    ) -> Result<(), PluginError> {
        let base_url = ollama_url(ctx)?;
        ollama::chat_stream(&base_url, request, tx).await?;
        Ok(())
    }
}

fn ollama_url(ctx: &Context) -> Result<String, PluginError> {
    ctx.config
        .get("ollama_url")
        .map(|s| s.trim_end_matches('/').to_string())
        .ok_or_else(|| PluginError::InvalidParams("ollama_url not configured".into()))
}

#[tokio::main]
async fn main() {
    grimoire_sdk::run(OllamaPlugin).await;
}
