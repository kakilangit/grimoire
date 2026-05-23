//! Summoner Plugin SDK
//!
//! Provides the HTTP server, callback client, types, and helpers for building
//! Summoner Grimoire plugins in Rust.
//!
//! # Usage
//!
//! ```rust,no_run
//! use grimoire_sdk::{Plugin, PluginError, Context, run};
//! use std::collections::HashMap;
//!
//! struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn name(&self) -> &'static str { "my-plugin" }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     run(MyPlugin).await;
//! }
//! ```

mod callback;
pub mod contract;
pub mod provider;
mod server;
pub mod streaming;
mod types;

pub use contract::{EventData, InvocationResult};
pub use provider::{
    ChatChunk, ChatDelta, ChatMessage, ChatRequest, ChatResponse, Model, ModelCapability,
    ModelsResponse, ToolCall, ToolCallDelta, ToolCallFunction, ToolCallFunctionDelta, Usage,
};
pub use server::run;
pub use streaming::{SseReader, read_oai_stream};
pub use types::*;

use std::collections::HashMap;

/// The trait every Summoner plugin implements.
///
/// Default implementations return "not supported" errors for all methods.
/// Override only the methods your plugin's capabilities require.
///
/// All handler methods receive a [`Context`] with workspace-scoped config
/// and access to the Summoner callback API.
pub trait Plugin: Send + Sync + 'static {
    /// Plugin name. Must match the `name` field in `grimoire.json`.
    fn name(&self) -> &'static str;

    /// List tools this plugin provides.
    /// Required for the `tools` capability.
    fn list_tools(&self) -> Vec<ToolDefinition> {
        vec![]
    }

    /// Handle a tool call.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::MethodNotFound` by default. Implementations
    /// should return `PluginError` variants appropriate to the failure.
    fn handle_tool_call(
        &self,
        _ctx: &Context,
        _name: &str,
        _arguments: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, PluginError>> + Send {
        async { Err(PluginError::MethodNotFound("tool".into())) }
    }

    /// Handle an inbound webhook forwarded by Summoner.
    ///
    /// # Errors
    ///
    /// Returns `PluginError::MethodNotFound` by default. Implementations
    /// should return `PluginError` variants appropriate to the failure.
    fn handle_webhook(
        &self,
        _ctx: &Context,
        _route: &str,
        _headers: HashMap<String, String>,
        _body: &str,
    ) -> impl std::future::Future<Output = Result<WebhookResponse, PluginError>> + Send {
        async { Err(PluginError::MethodNotFound("webhook".into())) }
    }

    /// Handle a lifecycle hook.
    ///
    /// # Errors
    ///
    /// Returns `HookResponse::Proceed` by default. Implementations may
    /// return `PluginError` on failure.
    fn handle_hook(
        &self,
        _ctx: &Context,
        _point: HookPoint,
        _data: serde_json::Value,
    ) -> impl std::future::Future<Output = Result<HookResponse, PluginError>> + Send {
        async { Ok(HookResponse::Proceed) }
    }

    /// Handle a domain event forwarded by Summoner.
    ///
    /// # Errors
    ///
    /// Returns `Ok(())` by default. Implementations may return
    /// `PluginError` on failure.
    fn handle_event(
        &self,
        _ctx: &Context,
        _event: &EventData,
        _external_ref: Option<&str>,
    ) -> impl std::future::Future<Output = Result<(), PluginError>> + Send {
        async { Ok(()) }
    }

    // -----------------------------------------------------------------
    // Provider capability (optional)
    // -----------------------------------------------------------------

    /// Whether this plugin implements the provider capability.
    /// Override to return `true` if your plugin serves as an inference provider.
    fn is_provider(&self) -> bool {
        false
    }

    /// List models available from this provider.
    /// Required when `is_provider()` returns `true`.
    fn list_models(
        &self,
        _ctx: &Context,
    ) -> impl std::future::Future<Output = Result<Vec<Model>, PluginError>> + Send {
        async { Err(PluginError::MethodNotFound("list_models".into())) }
    }

    /// Handle a non-streaming chat completion request.
    /// Required when `is_provider()` returns `true`.
    fn handle_chat(
        &self,
        _ctx: &Context,
        _request: &ChatRequest,
    ) -> impl std::future::Future<Output = Result<ChatResponse, PluginError>> + Send {
        async { Err(PluginError::MethodNotFound("chat".into())) }
    }

    /// Handle a streaming chat completion request.
    /// Send chunks via the provided sender. The SDK handles SSE framing.
    /// Required when `is_provider()` returns `true` and `stream: true`.
    fn handle_chat_stream(
        &self,
        _ctx: &Context,
        _request: &ChatRequest,
        _tx: tokio::sync::mpsc::Sender<ChatChunk>,
    ) -> impl std::future::Future<Output = Result<(), PluginError>> + Send {
        async { Err(PluginError::MethodNotFound("chat_stream".into())) }
    }
}
