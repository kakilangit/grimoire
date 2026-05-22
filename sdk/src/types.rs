use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Error type for plugin operations.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Invalid params: {0}")]
    InvalidParams(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Signature verification failed")]
    SignatureInvalid,

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Callback error: {0}")]
    Callback(String),
}

/// MCP tool definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// Lifecycle hook trigger points.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookPoint {
    BeforeInvocation,
    AfterInvocation,
    OnToolCall,
    OnError,
}

/// Response from a lifecycle hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum HookResponse {
    Proceed,
    Modify { context: serde_json::Value },
    Halt { reason: String },
}

/// Response from a webhook handler — represents the HTTP response to send back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookResponse {
    #[serde(default = "default_status")]
    pub status: u16,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: serde_json::Value,
}

fn default_status() -> u16 {
    200
}

impl WebhookResponse {
    /// Create a simple 200 JSON response.
    #[must_use]
    pub fn ok(body: serde_json::Value) -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body,
        }
    }

    /// Create a response with no meaningful body (ack).
    #[must_use]
    pub fn ack() -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: serde_json::json!({}),
        }
    }
}

/// Plugin execution context provided by Summoner on each request.
#[derive(Debug, Clone, Deserialize)]
pub struct Context {
    pub workspace_id: String,
    pub plugin_id: String,
    #[serde(default)]
    pub config: HashMap<String, String>,
    pub callback_url: String,
    pub callback_token: String,
}

/// Server-sent event types for streaming agent responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    Token {
        text: String,
    },
    Done {
        invocation_id: String,
        output: String,
    },
    Error {
        message: String,
    },
}
