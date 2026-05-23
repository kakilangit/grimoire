//! Ollama API client.
//!
//! Ollama exposes an OpenAI-compatible API at `/v1/chat/completions` and
//! a native `/api/tags` endpoint for listing models.

use grimoire_sdk::{
    ChatChunk, ChatMessage, ChatRequest, ChatResponse, Model, ModelCapability, PluginError,
    ToolCall, ToolCallFunction, Usage, read_oai_stream,
};
use serde::{Deserialize, Serialize};

// ─── Shared HTTP helpers ─────────────────────────────────────────────────────

fn client() -> reqwest::Client {
    // Reuse across calls within the same request; for cross-request reuse
    // the SDK would need to inject a shared client.
    reqwest::Client::new()
}

async fn checked_response(
    resp: reqwest::Response,
    endpoint: &str,
) -> Result<reqwest::Response, PluginError> {
    if resp.status().is_success() {
        return Ok(resp);
    }
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    Err(PluginError::Internal(format!(
        "ollama {endpoint} returned {status}: {body}"
    )))
}

// ─── Type conversions ────────────────────────────────────────────────────────

impl From<OaiUsage> for Usage {
    fn from(u: OaiUsage) -> Self {
        Self {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }
    }
}

impl From<OaiToolCall> for ToolCall {
    fn from(tc: OaiToolCall) -> Self {
        Self {
            id: tc.id,
            call_type: tc.call_type,
            function: ToolCallFunction {
                name: tc.function.name,
                arguments: tc.function.arguments,
            },
        }
    }
}

// ─── List Models ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<TagModel>,
}

#[derive(Deserialize)]
struct TagModel {
    name: String,
}

pub async fn list_models(base_url: &str) -> Result<Vec<Model>, PluginError> {
    let url = format!("{base_url}/api/tags");
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| PluginError::Internal(format!("ollama tags request failed: {e}")))?;

    let resp = checked_response(resp, "/api/tags").await?;

    let tags: TagsResponse = resp
        .json()
        .await
        .map_err(|e| PluginError::Internal(format!("failed to parse tags response: {e}")))?;

    let models = tags
        .models
        .into_iter()
        .map(|m| {
            let name = m.name.clone();
            Model {
                id: m.name,
                name,
                context_length: None,
                capabilities: Some(vec![ModelCapability::Chat]),
            }
        })
        .collect();

    Ok(models)
}

// ─── OAI types ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct OaiRequest {
    model: String,
    messages: Vec<OaiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u64>,
    stream: bool,
}

impl OaiRequest {
    fn from_chat(request: &ChatRequest, stream: bool) -> Self {
        Self {
            model: request.model.clone(),
            messages: to_oai_messages(&request.messages),
            tools: request.tools.clone(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct OaiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OaiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
struct OaiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OaiFunction,
}

#[derive(Serialize, Deserialize, Clone)]
struct OaiFunction {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct OaiResponse {
    choices: Vec<OaiChoice>,
    #[serde(default)]
    usage: Option<OaiUsage>,
}

#[derive(Deserialize)]
struct OaiChoice {
    message: OaiMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[allow(clippy::struct_field_names)]
#[derive(Deserialize)]
struct OaiUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    total_tokens: u64,
}

fn to_oai_messages(messages: &[ChatMessage]) -> Vec<OaiMessage> {
    messages
        .iter()
        .map(|m| OaiMessage {
            role: m.role.clone(),
            content: m.content.clone(),
            reasoning: None,
            tool_calls: m.tool_calls.as_ref().map(|tcs| {
                tcs.iter()
                    .map(|tc| OaiToolCall {
                        id: tc.id.clone(),
                        call_type: tc.call_type.clone(),
                        function: OaiFunction {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    })
                    .collect()
            }),
            tool_call_id: m.tool_call_id.clone(),
        })
        .collect()
}

// ─── Chat (non-streaming) ────────────────────────────────────────────────────

pub async fn chat(base_url: &str, request: &ChatRequest) -> Result<ChatResponse, PluginError> {
    let url = format!("{base_url}/v1/chat/completions");

    let resp = client()
        .post(&url)
        .json(&OaiRequest::from_chat(request, false))
        .send()
        .await
        .map_err(|e| PluginError::Internal(format!("ollama chat request failed: {e}")))?;

    let resp = checked_response(resp, "/v1/chat/completions").await?;

    let oai: OaiResponse = resp
        .json()
        .await
        .map_err(|e| PluginError::Internal(format!("failed to parse chat response: {e}")))?;

    let choice = oai
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| PluginError::Internal("no choices in response".into()))?;

    Ok(ChatResponse {
        message: ChatMessage {
            role: choice.message.role,
            content: choice.message.content,
            tool_calls: choice
                .message
                .tool_calls
                .map(|tcs| tcs.into_iter().map(ToolCall::from).collect()),
            tool_call_id: None,
        },
        usage: oai.usage.map(Usage::from),
        finish_reason: choice.finish_reason.unwrap_or_else(|| "stop".into()),
    })
}

// ─── Chat (streaming) ────────────────────────────────────────────────────────

pub async fn chat_stream(
    base_url: &str,
    request: &ChatRequest,
    tx: tokio::sync::mpsc::Sender<ChatChunk>,
) -> Result<(), PluginError> {
    let url = format!("{base_url}/v1/chat/completions");

    let resp = client()
        .post(&url)
        .json(&OaiRequest::from_chat(request, true))
        .send()
        .await
        .map_err(|e| PluginError::Internal(format!("ollama stream request failed: {e}")))?;

    let resp = checked_response(resp, "/v1/chat/completions (stream)").await?;

    read_oai_stream(resp, &tx).await
}
