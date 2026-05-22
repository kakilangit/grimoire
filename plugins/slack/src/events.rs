use std::sync::OnceLock;

use reqwest::Client;
use grimoire_sdk::{Context, EventData, PluginError};

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn client() -> &'static Client {
    HTTP_CLIENT.get_or_init(Client::new)
}

/// Handle domain events forwarded by Summoner.
/// Posts results back to Slack via the Web API.
///
/// # Errors
///
/// Returns `PluginError` if posting to Slack fails.
pub async fn handle(
    ctx: &Context,
    event: &EventData,
    external_ref: Option<&str>,
) -> Result<(), PluginError> {
    let Some(ext_ref) = external_ref else {
        tracing::debug!("event has no external_ref, skipping");
        return Ok(());
    };

    let bot_token = ctx.config_required("bot_token")?;

    // external_ref format: "channel_id" (from slash commands) or "channel:thread_ts" (from messages)
    let (channel, thread_ts) = if let Some((ch, ts)) = ext_ref.split_once(':') {
        (ch, Some(ts))
    } else {
        (ext_ref, None)
    };

    match event {
        EventData::InvocationCompleted { response, .. } => {
            let text = response.as_deref().unwrap_or("(no output)");
            post_message(bot_token, channel, thread_ts, text).await?;
        }
        EventData::InvocationFailed { error, .. } => {
            let err = error.as_deref().unwrap_or("Unknown error");
            let msg = format!(":warning: Invocation failed: {err}");
            post_message(bot_token, channel, thread_ts, &msg).await?;
        }
        _ => {
            tracing::debug!("ignoring event");
        }
    }

    Ok(())
}

/// Post a text message to Slack.
async fn post_message(
    bot_token: &str,
    channel: &str,
    thread_ts: Option<&str>,
    text: &str,
) -> Result<(), PluginError> {
    let mut body = serde_json::json!({
        "channel": channel,
        "text": text,
    });

    if let Some(ts) = thread_ts {
        body["thread_ts"] = serde_json::Value::String(ts.to_string());
    }

    slack_post(bot_token, "chat.postMessage", &body).await?;
    Ok(())
}

/// Make an async POST request to the Slack Web API.
async fn slack_post(
    bot_token: &str,
    method: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, PluginError> {
    let url = format!("https://slack.com/api/{method}");

    let resp: serde_json::Value = client()
        .post(&url)
        .header("Authorization", format!("Bearer {bot_token}"))
        .json(body)
        .send()
        .await
        .map_err(|e| PluginError::Http(e.to_string()))?
        .json()
        .await
        .map_err(|e| PluginError::Http(e.to_string()))?;

    if resp.get("ok") == Some(&serde_json::Value::Bool(true)) {
        Ok(resp)
    } else {
        let error = resp
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("unknown_error");
        Err(PluginError::Internal(format!("Slack API error: {error}")))
    }
}
