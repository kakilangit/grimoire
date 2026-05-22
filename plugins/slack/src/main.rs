use std::collections::HashMap;
use summoner_plugin_sdk::{
    Context, EventData, HookPoint, HookResponse, Plugin, PluginError, ToolDefinition,
    WebhookResponse,
};

mod events;
mod tools;
mod webhooks;

pub struct SlackPlugin;

impl Plugin for SlackPlugin {
    fn name(&self) -> &'static str {
        "grimoire-slack"
    }

    fn list_tools(&self) -> Vec<ToolDefinition> {
        tools::definitions()
    }

    async fn handle_tool_call(
        &self,
        ctx: &Context,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, PluginError> {
        tools::handle(ctx, name, &arguments).await
    }

    async fn handle_webhook(
        &self,
        ctx: &Context,
        route: &str,
        headers: HashMap<String, String>,
        body: &str,
    ) -> Result<WebhookResponse, PluginError> {
        webhooks::handle(ctx, route, &headers, body).await
    }

    async fn handle_hook(
        &self,
        _ctx: &Context,
        _point: HookPoint,
        _data: serde_json::Value,
    ) -> Result<HookResponse, PluginError> {
        Ok(HookResponse::Proceed)
    }

    async fn handle_event(
        &self,
        ctx: &Context,
        event: &EventData,
        external_ref: Option<&str>,
    ) -> Result<(), PluginError> {
        events::handle(ctx, event, external_ref).await
    }
}

#[tokio::main]
async fn main() {
    summoner_plugin_sdk::run(SlackPlugin).await;
}
