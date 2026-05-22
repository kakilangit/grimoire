//! Typed contract structs matching Summoner's plugin contract.
//!
//! These types are the Rust-side mirror of the schemas defined in
//! `summoner/priv/openapi/plugin_contract.yaml`. Summoner is the
//! source of truth — when the contract changes, update this module
//! to match.

use serde::{Deserialize, Serialize};

// ── Event data ────────────────────────────────────────────────────

/// Discriminated union of all domain event payloads sent by Summoner.
///
/// The `event_type` field acts as the discriminator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum EventData {
    /// An invocation has started running.
    #[serde(rename = "invocation.started")]
    InvocationStarted {
        workspace_id: String,
        agent_id: String,
        invocation_id: String,
        #[serde(default)]
        conversation_id: Option<String>,
    },

    /// An invocation completed successfully.
    #[serde(rename = "invocation.completed")]
    InvocationCompleted {
        workspace_id: String,
        agent_id: String,
        invocation_id: String,
        #[serde(default)]
        conversation_id: Option<String>,
        /// The agent's text response (may be absent).
        #[serde(default)]
        response: Option<String>,
    },

    /// An invocation failed.
    #[serde(rename = "invocation.failed")]
    InvocationFailed {
        workspace_id: String,
        agent_id: String,
        invocation_id: String,
        #[serde(default)]
        conversation_id: Option<String>,
        /// Error message (may be absent).
        #[serde(default)]
        error: Option<String>,
    },

    /// A pipeline run status changed.
    #[serde(rename = "pipeline.started")]
    PipelineStarted {
        workspace_id: String,
        pipeline_id: String,
        run_id: String,
        status: String,
    },

    /// A pipeline stage status changed.
    #[serde(rename = "pipeline.completed")]
    PipelineCompleted {
        workspace_id: String,
        pipeline_id: String,
        run_id: String,
        position: i64,
        status: String,
    },

    /// A swarm turn was dispatched to an agent.
    #[serde(rename = "swarm.turn")]
    SwarmTurn {
        workspace_id: String,
        swarm_id: String,
        conversation_id: String,
        agent_id: String,
    },

    /// A swarm completed.
    #[serde(rename = "swarm.done")]
    SwarmDone {
        workspace_id: String,
        swarm_id: String,
        conversation_id: String,
        summary: String,
    },

    /// An agent timed out during a swarm turn.
    #[serde(rename = "swarm.timeout")]
    SwarmTimeout {
        workspace_id: String,
        swarm_id: String,
        conversation_id: String,
        agent_id: String,
    },

    /// A webhook was successfully triggered.
    #[serde(rename = "webhook.triggered")]
    WebhookTriggered {
        workspace_id: String,
        webhook_id: String,
        response_mode: String,
        invocation_id: String,
        #[serde(default)]
        timestamp: Option<String>,
    },

    /// A webhook trigger failed.
    #[serde(rename = "webhook.failed")]
    WebhookFailed {
        workspace_id: String,
        webhook_id: String,
        error_reason: String,
        #[serde(default)]
        timestamp: Option<String>,
    },

    /// An agent failed over to its backup.
    #[serde(rename = "failover")]
    Failover {
        #[serde(default)]
        workspace_id: Option<String>,
        invocation_id: String,
        from_agent_id: String,
        to_agent_id: String,
        reason: String,
        depth: i64,
    },

    /// Media generation started.
    #[serde(rename = "media.started")]
    MediaStarted {
        workspace_id: String,
        conversation_id: String,
        attachment_id: String,
        #[serde(rename = "type")]
        media_type: String,
        prompt: String,
    },

    /// Media generation completed.
    #[serde(rename = "media.completed")]
    MediaCompleted {
        workspace_id: String,
        conversation_id: String,
        attachment_id: String,
        url: String,
    },

    /// Media generation failed.
    #[serde(rename = "media.failed")]
    MediaFailed {
        workspace_id: String,
        conversation_id: String,
        attachment_id: String,
        error: String,
    },

    /// An agent's tool/MCP configuration changed.
    #[serde(rename = "agent.config_changed")]
    AgentConfigChanged { agent_id: String },
}

// ── Callback request params ───────────────────────────────────────

/// Parameters for `invoke_agent` / `invoke_agent_async` callback actions.
#[derive(Debug, Clone, Serialize)]
pub struct InvokeAgentParams<'a> {
    pub agent: &'a str,
    pub message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<&'a str>,
}

/// Parameters for `emit_event` callback action.
#[derive(Debug, Clone, Serialize)]
pub struct EmitEventParams<'a> {
    pub event: &'a str,
    pub data: serde_json::Value,
}

/// Parameters for `log` callback action.
#[derive(Debug, Clone, Serialize)]
pub struct LogParams<'a> {
    pub level: &'a str,
    pub message: &'a str,
}

/// Parameters for `get_state` / `delete_state` callback actions.
#[derive(Debug, Clone, Serialize)]
pub struct KeyParams<'a> {
    pub key: &'a str,
}

/// Parameters for `set_state` callback action.
#[derive(Debug, Clone, Serialize)]
pub struct SetStateParams<'a> {
    pub key: &'a str,
    pub value: serde_json::Value,
}

// ── Callback response types ──────────────────────────────────────

/// Typed callback response envelope.
#[derive(Debug, Clone, Deserialize)]
pub struct CallbackResponse {
    pub ok: bool,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
}

/// Result of a synchronous `invoke_agent` callback.
#[derive(Debug, Clone, Deserialize)]
pub struct InvocationResult {
    pub invocation_id: String,
    pub output: String,
}

/// Result of a `get_state` callback.
#[derive(Debug, Clone, Deserialize)]
pub struct StateResult {
    pub key: String,
    pub value: Option<serde_json::Value>,
}
