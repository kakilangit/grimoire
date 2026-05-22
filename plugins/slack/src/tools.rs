use std::sync::OnceLock;

use reqwest::Client;
use grimoire_sdk::{Context, PluginError, ToolDefinition};

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn client() -> &'static Client {
    HTTP_CLIENT.get_or_init(Client::new)
}

/// Return tool definitions for the Slack plugin.
pub fn definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "slack_send_message".into(),
            description: "Send a message to a Slack channel".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "channel": {
                        "type": "string",
                        "description": "Slack channel ID (e.g. C01ABCDEF)"
                    },
                    "text": {
                        "type": "string",
                        "description": "Message text (supports Slack mrkdwn)"
                    }
                },
                "required": ["channel", "text"]
            }),
        },
        ToolDefinition {
            name: "slack_send_reply".into(),
            description: "Reply to a message in a Slack thread".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "channel": {
                        "type": "string",
                        "description": "Slack channel ID"
                    },
                    "thread_ts": {
                        "type": "string",
                        "description": "Thread timestamp to reply to"
                    },
                    "text": {
                        "type": "string",
                        "description": "Reply text (supports Slack mrkdwn)"
                    }
                },
                "required": ["channel", "thread_ts", "text"]
            }),
        },
        ToolDefinition {
            name: "slack_add_reaction".into(),
            description: "Add an emoji reaction to a Slack message".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "channel": {
                        "type": "string",
                        "description": "Slack channel ID"
                    },
                    "timestamp": {
                        "type": "string",
                        "description": "Message timestamp to react to"
                    },
                    "emoji": {
                        "type": "string",
                        "description": "Emoji name without colons (e.g. 'thumbsup')"
                    }
                },
                "required": ["channel", "timestamp", "emoji"]
            }),
        },
        ToolDefinition {
            name: "slack_list_channels".into(),
            description: "List Slack channels the bot has access to".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of channels to return (default 100)",
                        "default": 100
                    }
                }
            }),
        },
    ]
}

/// Handle a tool call.
///
/// # Errors
///
/// Returns `PluginError` if the tool call fails or the Slack API returns an error.
pub async fn handle(
    ctx: &Context,
    name: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value, PluginError> {
    match name {
        "slack_send_message" => send_message(ctx, params).await,
        "slack_send_reply" => send_reply(ctx, params).await,
        "slack_add_reaction" => add_reaction(ctx, params).await,
        "slack_list_channels" => list_channels(ctx, params).await,
        _ => Err(PluginError::MethodNotFound(name.into())),
    }
}

async fn send_message(
    ctx: &Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, PluginError> {
    let channel = require_str(params, "channel")?;
    let text = require_str(params, "text")?;

    slack_api_call(
        ctx,
        "chat.postMessage",
        &serde_json::json!({
            "channel": channel,
            "text": text,
        }),
    )
    .await
}

async fn send_reply(
    ctx: &Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, PluginError> {
    let channel = require_str(params, "channel")?;
    let thread_ts = require_str(params, "thread_ts")?;
    let text = require_str(params, "text")?;

    slack_api_call(
        ctx,
        "chat.postMessage",
        &serde_json::json!({
            "channel": channel,
            "thread_ts": thread_ts,
            "text": text,
        }),
    )
    .await
}

async fn add_reaction(
    ctx: &Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, PluginError> {
    let channel = require_str(params, "channel")?;
    let timestamp = require_str(params, "timestamp")?;
    let emoji = require_str(params, "emoji")?;

    slack_api_call(
        ctx,
        "reactions.add",
        &serde_json::json!({
            "channel": channel,
            "timestamp": timestamp,
            "name": emoji,
        }),
    )
    .await
}

async fn list_channels(
    ctx: &Context,
    params: &serde_json::Value,
) -> Result<serde_json::Value, PluginError> {
    let limit = params
        .get("limit")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(100);

    slack_api_call(
        ctx,
        "conversations.list",
        &serde_json::json!({
            "limit": limit,
            "types": "public_channel,private_channel",
        }),
    )
    .await
}

/// Extract a required string field from JSON params.
fn require_str<'a>(params: &'a serde_json::Value, field: &str) -> Result<&'a str, PluginError> {
    params
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| PluginError::InvalidParams(format!("missing '{field}'")))
}

/// Make an async Slack Web API call.
async fn slack_api_call(
    ctx: &Context,
    method: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, PluginError> {
    let bot_token = ctx.config_required("bot_token")?;
    let url = format!("https://slack.com/api/{method}");

    let result: serde_json::Value = client()
        .post(&url)
        .header("Authorization", format!("Bearer {bot_token}"))
        .json(body)
        .send()
        .await
        .map_err(|e| PluginError::Http(e.to_string()))?
        .json()
        .await
        .map_err(|e| PluginError::Http(e.to_string()))?;

    if result.get("ok") == Some(&serde_json::Value::Bool(true)) {
        Ok(result)
    } else {
        let error = result
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("unknown_error");
        Err(PluginError::Internal(format!("Slack API error: {error}")))
    }
}
