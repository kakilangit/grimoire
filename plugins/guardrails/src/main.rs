use grimoire_sdk::{Context, HookPoint, HookResponse, Plugin, PluginError};
use serde::Deserialize;

pub struct GuardrailsPlugin;

impl Plugin for GuardrailsPlugin {
    fn name(&self) -> &'static str {
        "grimoire-guardrails"
    }

    async fn handle_hook(
        &self,
        ctx: &Context,
        point: HookPoint,
        data: serde_json::Value,
    ) -> Result<HookResponse, PluginError> {
        let cfg = config(ctx);

        match point {
            HookPoint::BeforeInvocation => {
                let text = extract_last_user_message(&data).to_lowercase();
                if let Some(word) = cfg
                    .blocklist
                    .iter()
                    .find(|w| text.contains(&w.to_lowercase()))
                {
                    Ok(HookResponse::Halt {
                        reason: format!("Blocked: {word}"),
                    })
                } else {
                    Ok(HookResponse::Proceed)
                }
            }
            HookPoint::OnToolCall => {
                let tool = data.get("tool_name").and_then(|v| v.as_str()).unwrap_or("");
                if cfg.denied_tools.iter().any(|t| t == tool) {
                    Ok(HookResponse::Halt {
                        reason: format!("Tool denied: {tool}"),
                    })
                } else {
                    Ok(HookResponse::Proceed)
                }
            }
            _ => Ok(HookResponse::Proceed),
        }
    }
}

#[derive(Deserialize, Default)]
struct Config {
    #[serde(default)]
    blocklist: Vec<String>,
    #[serde(default)]
    denied_tools: Vec<String>,
}

fn config(ctx: &Context) -> Config {
    let blocklist = ctx
        .config
        .get("blocklist")
        .and_then(|v| serde_json::from_str(v).ok())
        .unwrap_or_default();

    let denied_tools = ctx
        .config
        .get("denied_tools")
        .and_then(|v| serde_json::from_str(v).ok())
        .unwrap_or_default();

    Config {
        blocklist,
        denied_tools,
    }
}

fn extract_last_user_message(data: &serde_json::Value) -> &str {
    data.get("messages")
        .and_then(|m| m.as_array())
        .map(|a| a.iter().rev())
        .into_iter()
        .flatten()
        .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
        .and_then(|m| m.get("content").and_then(|c| c.as_str()))
        .unwrap_or("")
}

#[tokio::main]
async fn main() {
    grimoire_sdk::run(GuardrailsPlugin).await;
}
