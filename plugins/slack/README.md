# Grimoire Slack

Bidirectional Slack integration for Summoner. Receives messages and commands from Slack, invokes agents, and posts results back.

## Capabilities

- **Webhooks**: Slack Events API, slash commands, Block Kit interactions
- **Events**: `invocation.completed`, `invocation.failed`, `approval.pending`
- **Tools**: `slack_send_message`, `slack_send_reply`, `slack_add_reaction`, `slack_list_channels`

## Setup

### 1. Create Slack App

Go to https://api.slack.com/apps → **Create New App** → **From a manifest** → paste:

```json
{
    "display_information": {
        "name": "Summoner",
        "description": "AI agent platform integration",
        "background_color": "#1a1a2e"
    },
    "features": {
        "bot_user": {
            "display_name": "Summoner",
            "always_online": true
        },
        "slash_commands": [
            {
                "command": "/summon",
                "url": "https://<YOUR_DOMAIN>/api/v1/tenants/<TENANT_ID>/workspaces/<WORKSPACE_ID>/plugins/<PLUGIN_REF>/webhooks/commands",
                "description": "Invoke a Summoner agent",
                "usage_hint": "[ask @agent_name] message",
                "should_escape": false
            }
        ]
    },
    "oauth_config": {
        "scopes": {
            "bot": [
                "app_mentions:read",
                "channels:read",
                "chat:write",
                "commands",
                "groups:read",
                "im:history",
                "reactions:write"
            ]
        }
    },
    "settings": {
        "event_subscriptions": {
            "request_url": "https://<YOUR_DOMAIN>/api/v1/tenants/<TENANT_ID>/workspaces/<WORKSPACE_ID>/plugins/<PLUGIN_REF>/webhooks/events",
            "bot_events": [
                "app_mention",
                "message.im"
            ]
        },
        "interactivity": {
            "is_enabled": true,
            "request_url": "https://<YOUR_DOMAIN>/api/v1/tenants/<TENANT_ID>/workspaces/<WORKSPACE_ID>/plugins/<PLUGIN_REF>/webhooks/interactions"
        },
        "org_deploy_enabled": false,
        "socket_mode_enabled": false,
        "is_hosted": false,
        "token_rotation_enabled": false
    }
}
```

Replace:
- `<YOUR_DOMAIN>` — your Summoner instance's public URL (e.g. an ngrok tunnel)
- `<TENANT_ID>` — your tenant (guild) ID
- `<WORKSPACE_ID>` — your workspace (realm) ID
- `<PLUGIN_REF>` — the 12-char plugin ref (shown on the plugin detail page after install). This is a deterministic hash of the image path, so you can pre-compute it from the image name.

### 2. Install App to Workspace

After creating the app, click **Install to Workspace** and authorize.

### 3. Collect Credentials

| Credential | Location |
|---|---|
| Bot Token (`xoxb-...`) | OAuth & Permissions → Bot User OAuth Token |
| Signing Secret | Basic Information → App Credentials → Signing Secret |

### 4. Install Plugin in Summoner

Install the grimoire and configure with:

| Config Key | Value |
|---|---|
| `bot_token` | `xoxb-...` |
| `signing_secret` | Your app's signing secret |
| `default_agent` | Agent callname for unrouted messages |
| `channel_agent_map` | *(optional)* JSON map of channel ID → agent callname |

Example `channel_agent_map`:

```json
{"C01ABC123": "support-agent", "C02DEF456": "ops-agent"}
```

### 5. Verify Webhooks

Slack will send a URL verification challenge to your events endpoint on save. Summoner must be running and the plugin must be enabled before you save the Events API URL in the Slack app settings.

## Docker Image

```
docker.io/kakilangit/grimoire-slack:0.1.3
```

## Usage

### DMs and Mentions

Send a DM to the bot or `@Summoner` in a channel. The plugin routes the message to the configured agent and replies in-thread.

### Slash Command

```
/summon what is the status of project X
/summon ask @ops-agent deploy to staging
```

### Agent Tools

Agents with this plugin's tools can:

- `slack_send_message` — post to any channel
- `slack_send_reply` — reply in a thread
- `slack_add_reaction` — add emoji reactions
- `slack_list_channels` — list accessible channels

## Environment Variables (Container)

The plugin container receives config as `PLUGIN_` prefixed env vars:

| Env Var | Source |
|---|---|
| `PLUGIN_BOT_TOKEN` | `bot_token` config |
| `PLUGIN_SIGNING_SECRET` | `signing_secret` config |
| `PLUGIN_DEFAULT_AGENT` | `default_agent` config |
| `PLUGIN_CHANNEL_AGENT_MAP` | `channel_agent_map` config |
