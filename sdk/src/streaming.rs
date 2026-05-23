//! SSE and OpenAI-compatible streaming helpers for provider plugins.
//!
//! Most LLM backends expose an OpenAI-compatible `/v1/chat/completions`
//! endpoint with SSE streaming. This module provides:
//!
//! - [`SseReader`] — extracts `data:` payloads from any SSE byte stream
//! - [`read_oai_stream`] — full pipeline: HTTP response → parsed `ChatChunk`s
//!   sent through an `mpsc` channel, ready for the SDK's SSE framing layer.

use futures_util::StreamExt;
use serde::Deserialize;

use crate::PluginError;
use crate::provider::{ChatChunk, ChatDelta, ToolCallDelta, ToolCallFunctionDelta, Usage};

// ─── SSE line reader ─────────────────────────────────────────────────────────

/// Buffers raw bytes and extracts `data:` payloads from an SSE stream.
///
/// Handles partial lines across TCP chunk boundaries, skips SSE comments
/// (lines starting with `:`), and recognises the `[DONE]` sentinel.
pub struct SseReader {
    buffer: String,
}

impl SseReader {
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Feed raw bytes into the internal buffer.
    pub fn push(&mut self, bytes: &[u8]) {
        self.buffer.push_str(&String::from_utf8_lossy(bytes));
    }

    /// Extract the next complete `data:` payload.
    ///
    /// Returns:
    /// - `Some(Some(payload))` — a data line (owned, trimmed)
    /// - `Some(None)` — the `[DONE]` sentinel
    /// - `None` — no complete line buffered yet
    pub fn next_payload(&mut self) -> Option<Option<String>> {
        loop {
            let line_end = self.buffer.find('\n')?;
            let line = self.buffer[..line_end].trim().to_string();
            self.buffer = self.buffer[line_end + 1..].to_string();

            // Skip empty lines and SSE comments
            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            // Only process `data:` fields
            let data = match line.strip_prefix("data: ") {
                Some(d) => d.trim().to_string(),
                None => continue,
            };

            if data == "[DONE]" {
                return Some(None);
            }

            return Some(Some(data));
        }
    }
}

impl Default for SseReader {
    fn default() -> Self {
        Self::new()
    }
}

// ─── OpenAI-compatible stream types (private) ───────────────────────────────

#[derive(Deserialize)]
struct OaiStreamResponse {
    choices: Vec<OaiStreamChoice>,
    #[serde(default)]
    usage: Option<OaiStreamUsage>,
}

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
    /// Models like Qwen send reasoning tokens in a separate field.
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

#[allow(clippy::struct_field_names)]
#[derive(Deserialize)]
struct OaiStreamUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    total_tokens: u64,
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Read an OpenAI-compatible SSE stream from a [`reqwest::Response`] and send
/// parsed [`ChatChunk`]s through the provided channel.
///
/// Handles:
/// - SSE framing (buffering across TCP chunks, `data:` extraction)
/// - `[DONE]` sentinel
/// - `reasoning` field merged into `content` (for models like Qwen)
/// - Graceful stop when the receiver is dropped
///
/// # Errors
///
/// Returns [`PluginError::Internal`] on stream read errors.
pub async fn read_oai_stream(
    resp: reqwest::Response,
    tx: &tokio::sync::mpsc::Sender<ChatChunk>,
) -> Result<(), PluginError> {
    let mut reader = SseReader::new();
    let mut bytes_stream = resp.bytes_stream();

    while let Some(chunk_result) = bytes_stream.next().await {
        let chunk_bytes =
            chunk_result.map_err(|e| PluginError::Internal(format!("stream read error: {e}")))?;
        reader.push(&chunk_bytes);

        while let Some(maybe_payload) = reader.next_payload() {
            let Some(data) = maybe_payload else {
                return Ok(()); // [DONE]
            };

            let parsed: OaiStreamResponse = match serde_json::from_str(&data) {
                Ok(v) => v,
                Err(_) => continue,
            };

            for choice in parsed.choices {
                if let Some(chunk) = oai_choice_to_chunk(choice, parsed.usage.as_ref())
                    && tx.send(chunk).await.is_err()
                {
                    return Ok(()); // receiver dropped
                }
            }
        }
    }

    Ok(())
}

/// Convert a single OAI stream choice into a [`ChatChunk`].
///
/// Returns `None` for chunks with no meaningful payload (empty content
/// during reasoning, no tool calls, no finish reason).
fn oai_choice_to_chunk(
    choice: OaiStreamChoice,
    usage: Option<&OaiStreamUsage>,
) -> Option<ChatChunk> {
    // Merge reasoning into content — the grimoire protocol has no separate
    // reasoning field. Skip chunks with no meaningful payload.
    let text = match (&choice.delta.content, &choice.delta.reasoning) {
        (Some(c), _) if !c.is_empty() => Some(c.clone()),
        (_, Some(r)) if !r.is_empty() => Some(r.clone()),
        _ if choice.finish_reason.is_some() => None,
        _ if choice.delta.tool_calls.is_some() => None,
        _ => return None,
    };

    Some(ChatChunk {
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
        usage: usage.map(|u| Usage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }),
    })
}
