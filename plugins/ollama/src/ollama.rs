//! Ollama API client.
//!
//! Ollama exposes an OpenAI-compatible API at `/v1/chat/completions` and
//! a native `/api/tags` endpoint for listing models.

use futures_util::StreamExt;
use grimoire_sdk::{
    ChatChunk, ChatDelta, ChatMessage, ChatRequest, ChatResponse, Model, ModelCapability,
    PluginError, ToolCall, ToolCallDelta, ToolCallFunction, ToolCallFunctionDelta, Usage,
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

#[derive(Deserialize)]
struct OaiStreamChoice {
    delta: OaiStreamDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OaiStreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OaiStreamToolCall>>,
}

#[derive(Deserialize)]
struct OaiStreamToolCall {
    index: u32,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<OaiStreamFunction>,
}

#[derive(Deserialize)]
struct OaiStreamFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct OaiStreamResponse {
    choices: Vec<OaiStreamChoice>,
    #[serde(default)]
    usage: Option<OaiUsage>,
}

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

    let mut buffer = String::new();
    let mut bytes_stream = resp.bytes_stream();
    while let Some(chunk_result) = bytes_stream.next().await {
        let chunk_bytes =
            chunk_result.map_err(|e| PluginError::Internal(format!("stream read error: {e}")))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk_bytes));

        // Process complete SSE lines
        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer = buffer[line_end + 1..].to_string();

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            let data = if let Some(stripped) = line.strip_prefix("data: ") {
                stripped.trim()
            } else {
                continue;
            };

            if data == "[DONE]" {
                return Ok(());
            }

            let parsed: OaiStreamResponse = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => continue,
            };

            for choice in parsed.choices {
                // Merge reasoning into content — grimoire protocol has no
                // separate reasoning field. Skip chunks with no text at all.
                let text = match (&choice.delta.content, &choice.delta.reasoning) {
                    (Some(c), _) if !c.is_empty() => Some(c.clone()),
                    (_, Some(r)) if !r.is_empty() => Some(r.clone()),
                    _ if choice.finish_reason.is_some() => None,
                    _ if choice.delta.tool_calls.is_some() => None,
                    _ => continue,
                };

                let chunk = ChatChunk {
                    delta: ChatDelta {
                        content: text,
                        tool_calls: choice.delta.tool_calls.map(|tcs| {
                            tcs.into_iter()
                                .map(|tc| ToolCallDelta {
                                    index: tc.index,
                                    id: tc.id,
                                    function: tc.function.map(|f| ToolCallFunctionDelta {
                                        name: f.name,
                                        arguments: f.arguments,
                                    }),
                                })
                                .collect()
                        }),
                    },
                    finish_reason: choice.finish_reason,
                    usage: parsed.usage.as_ref().map(|u| Usage {
                        prompt_tokens: u.prompt_tokens,
                        completion_tokens: u.completion_tokens,
                        total_tokens: u.total_tokens,
                    }),
                };

                if tx.send(chunk).await.is_err() {
                    // Receiver dropped — client disconnected
                    return Ok(());
                }
            }
        }
    }

    Ok(())
}
