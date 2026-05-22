use grimoire_sdk::{Context, PluginError, WebhookResponse};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::collections::HashMap;

type HmacSha256 = Hmac<Sha256>;

/// Handle inbound Slack webhooks.
///
/// # Errors
///
/// Returns `PluginError` if signature verification fails or payload parsing fails.
pub async fn handle(
    ctx: &Context,
    route: &str,
    headers: &HashMap<String, String>,
    body: &str,
) -> Result<WebhookResponse, PluginError> {
    let signing_secret = ctx.config_required("signing_secret")?;
    verify_signature(signing_secret, headers, body)?;

    match route {
        "events" => {
            let parsed: serde_json::Value = serde_json::from_str(body)
                .map_err(|e| PluginError::InvalidParams(e.to_string()))?;
            handle_events(ctx, &parsed).await
        }
        "commands" => {
            let params = parse_form_urlencoded(body);
            handle_commands(ctx, &params).await
        }
        "interactions" => {
            let params = parse_form_urlencoded(body);
            let parsed = if let Some(payload) = params.get("payload") {
                serde_json::from_str(payload)
                    .map_err(|e| PluginError::InvalidParams(e.to_string()))?
            } else {
                serde_json::json!({})
            };
            Ok(handle_interactions(&parsed))
        }
        _ => Ok(WebhookResponse::ack()),
    }
}

/// Verify Slack request signature (HMAC-SHA256).
fn verify_signature(
    signing_secret: &str,
    headers: &HashMap<String, String>,
    body: &str,
) -> Result<(), PluginError> {
    let timestamp = headers
        .get("x-slack-request-timestamp")
        .ok_or(PluginError::SignatureInvalid)?;
    let signature = headers
        .get("x-slack-signature")
        .ok_or(PluginError::SignatureInvalid)?;

    let basestring = format!("v0:{timestamp}:{body}");

    let mut mac = HmacSha256::new_from_slice(signing_secret.as_bytes())
        .map_err(|e| PluginError::Internal(e.to_string()))?;
    mac.update(basestring.as_bytes());
    let expected = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

    if expected != *signature {
        return Err(PluginError::SignatureInvalid);
    }

    Ok(())
}

/// Handle Slack Events API payloads.
async fn handle_events(
    ctx: &Context,
    body: &serde_json::Value,
) -> Result<WebhookResponse, PluginError> {
    // URL verification challenge
    if let Some(challenge) = body.get("challenge").and_then(|c| c.as_str()) {
        tracing::info!("URL verification challenge received");
        return Ok(WebhookResponse::ok(serde_json::json!({
            "challenge": challenge
        })));
    }

    let event = body
        .get("event")
        .ok_or_else(|| PluginError::InvalidParams("missing 'event' field".into()))?;

    let event_type = event.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match event_type {
        "app_mention" => handle_message_event(ctx, event).await,
        "message" => {
            tracing::debug!("ignoring plain message event (only app_mention triggers invocation)");
            Ok(WebhookResponse::ack())
        }
        _ => {
            tracing::debug!("ignoring event type: {event_type}");
            Ok(WebhookResponse::ack())
        }
    }
}

/// Handle a message or `app_mention` event.
///
/// Strips the bot mention prefix from the text, then invokes the configured
/// agent asynchronously using the channel ID as `external_ref`.
async fn handle_message_event(
    ctx: &Context,
    event: &serde_json::Value,
) -> Result<WebhookResponse, PluginError> {
    if event.get("bot_id").is_some() || event.get("subtype").is_some() {
        return Ok(WebhookResponse::ack());
    }

    let channel = event.get("channel").and_then(|c| c.as_str()).unwrap_or("");
    let raw_text = event.get("text").and_then(|t| t.as_str()).unwrap_or("");

    // Strip the leading `<@UXXXXX> ` mention prefix to get the actual message.
    let text = strip_mention_prefix(raw_text);

    if text.is_empty() {
        return Ok(WebhookResponse::ack());
    }

    let agent_callname = ctx.config("default_agent").unwrap_or("default");
    let external_ref = channel;

    tracing::debug!("app_mention from channel {channel}");

    match ctx
        .invoke_agent_async(agent_callname, text, Some(external_ref))
        .await
    {
        Ok(()) => {
            tracing::info!("mention invocation accepted");
            Ok(WebhookResponse::ack())
        }
        Err(e) => {
            tracing::error!("failed to invoke agent for mention: {e}");
            Ok(WebhookResponse::ack())
        }
    }
}

/// Strip a leading `<@UXXXXX>` mention prefix from Slack message text.
fn strip_mention_prefix(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(rest) = trimmed.strip_prefix("<@")
        && let Some(after_mention) = rest.find('>')
    {
        return rest[after_mention + 1..].trim_start();
    }
    trimmed
}

/// Handle Slack slash commands.
///
/// Acks immediately (Slack requires response within 3s), then invokes
/// the configured agent asynchronously. The result arrives via domain
/// event `invocation.completed` → `events::handle` posts to Slack.
async fn handle_commands(
    ctx: &Context,
    params: &HashMap<String, String>,
) -> Result<WebhookResponse, PluginError> {
    let command = params.get("command").map_or("", String::as_str);
    let text = params.get("text").map_or("", String::as_str);
    let channel_id = params.get("channel_id").map_or("", String::as_str);
    let user_id = params.get("user_id").map_or("", String::as_str);

    tracing::info!("slash command received: {command} {text} from {user_id} in {channel_id}");

    if text.is_empty() {
        return Ok(WebhookResponse::ok(serde_json::json!({
            "response_type": "ephemeral",
            "text": "Usage: /summon <your message>"
        })));
    }

    let agent_callname = ctx.config("default_agent").unwrap_or("default");

    // Use channel_id as external_ref so the event handler knows where to post.
    // The event handler will use chat.postMessage to the channel.
    let external_ref = channel_id;

    match ctx
        .invoke_agent_async(agent_callname, text, Some(external_ref))
        .await
    {
        Ok(()) => {
            tracing::info!("invocation accepted");
            Ok(WebhookResponse::ok(serde_json::json!({
                "response_type": "in_channel",
                "text": format!(":hourglass_flowing_sand: Processing: _{text}_")
            })))
        }
        Err(e) => {
            tracing::error!("failed to invoke agent: {e}");
            Ok(WebhookResponse::ok(serde_json::json!({
                "response_type": "ephemeral",
                "text": format!(":warning: Failed to invoke agent: {e}")
            })))
        }
    }
}

/// Handle Slack Block Kit interactions.
fn handle_interactions(body: &serde_json::Value) -> WebhookResponse {
    let payload = body
        .get("payload")
        .and_then(|p| p.as_str())
        .and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok());

    let effective = payload.as_ref().unwrap_or(body);

    let action_type = effective.get("type").and_then(|t| t.as_str()).unwrap_or("");

    if action_type == "block_actions" {
        tracing::info!("block_actions interaction received");
    } else {
        tracing::debug!("ignoring interaction type: {action_type}");
    }

    WebhookResponse::ack()
}

/// Parse `application/x-www-form-urlencoded` body into a `HashMap`.
fn parse_form_urlencoded(body: &str) -> HashMap<String, String> {
    form_urlencoded::parse(body.as_bytes())
        .into_owned()
        .collect()
}
