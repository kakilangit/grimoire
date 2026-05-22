# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-05-22

### Added

- **grimoire-sdk** — Rust SDK for building Summoner plugins
  - `Plugin` trait with webhook, event, tools, hooks, provider, and theme handlers
  - HTTP server (axum) with health check, capability endpoints, and callback client
  - Typed request/response structs matching `plugin_contract.yaml`
  - Config injection per-request from Summoner
  - `invoke_agent` and `invoke_agent_async` callback helpers

- **grimoire-slack** — Bidirectional Slack integration
  - `/summon` slash command invokes agents from Slack
  - `@mention` triggers agent invocation (strips bot mention prefix)
  - Webhook signature verification with Slack signing secret
  - Event subscription for `invocation.completed` and `invocation.failed` to post responses back to Slack
  - Channel-to-agent routing via `channel_agent_map` config
  - Default agent fallback for unrouted channels
  - Tools: `slack_send_message`, `slack_send_reply`, `slack_add_reaction`, `slack_list_channels`
