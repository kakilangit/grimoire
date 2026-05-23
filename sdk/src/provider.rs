//! Provider capability types.
//!
//! Plugins with the `provider` capability expose LLM inference through
//! `GET /models` and `POST /chat` endpoints.

use serde::{Deserialize, Serialize};

/// A model available from this provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// Model identifier used in chat requests.
    pub id: String,
    /// Human-readable model name.
    pub name: String,
    /// Maximum context window in tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_length: Option<u64>,
    /// What the model supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<ModelCapability>>,
}

/// Model capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    Chat,
    Tools,
    Vision,
}

/// Chat completion request from Summoner.
#[derive(Debug, Clone, Deserialize)]
pub struct ChatRequest {
    /// Plugin context (`workspace_id`, `plugin_id`, config).
    pub context: serde_json::Value,
    /// Model ID from `/models` response.
    pub model: String,
    /// Conversation messages.
    pub messages: Vec<ChatMessage>,
    /// Available tools (`OpenAI` format).
    #[serde(default)]
    pub tools: Option<Vec<serde_json::Value>>,
    /// Sampling temperature.
    #[serde(default)]
    pub temperature: Option<f64>,
    /// Maximum tokens to generate.
    #[serde(default)]
    pub max_tokens: Option<u64>,
    /// Whether to stream the response.
    #[serde(default)]
    pub stream: bool,
}

/// A single chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// A tool call from the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

/// Function details within a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    /// JSON-encoded arguments.
    pub arguments: String,
}

/// Non-streaming chat response.
#[derive(Debug, Clone, Serialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    pub finish_reason: String,
}

/// A single SSE chunk for streaming responses.
#[derive(Debug, Clone, Serialize)]
pub struct ChatChunk {
    pub delta: ChatDelta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// Delta content in a streaming chunk.
#[derive(Debug, Clone, Serialize)]
pub struct ChatDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

/// Partial tool call in a streaming chunk.
#[derive(Debug, Clone, Serialize)]
pub struct ToolCallDelta {
    pub index: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<ToolCallFunctionDelta>,
}

/// Partial function in a tool call delta.
#[derive(Debug, Clone, Serialize)]
pub struct ToolCallFunctionDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
}

/// Models list response.
#[derive(Debug, Clone, Serialize)]
pub struct ModelsResponse {
    pub models: Vec<Model>,
}
