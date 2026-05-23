# Changelog

All notable changes to this project will be documented in this file.

## [0.1.1] - 2026-05-23

### Added

- **grimoire-sdk** — Streaming helpers for provider plugins
  - `SseReader` — buffers raw bytes and extracts SSE `data:` payloads
  - `read_oai_stream` — full pipeline from `reqwest::Response` to `ChatChunk` channel, with reasoning merge and `[DONE]` handling
  - Provider trait: `is_provider()`, `list_models()`, `handle_chat()`, `handle_chat_stream()`
  - Provider types: `Model`, `ChatRequest`, `ChatResponse`, `ChatChunk`, `ChatDelta`, `ToolCall`, `Usage`

- **grimoire-ollama** — Ollama inference provider plugin
  - Proxies `GET /models` via Ollama's `/api/tags`
  - Proxies `POST /chat` via Ollama's `/v1/chat/completions` (streaming and non-streaming)
  - Merges `reasoning` field into content for models like Qwen
  - Config: `ollama_url` (e.g. `http://host.docker.internal:11434`)

### Changed

- Workspace deps declare version only; crates drive their own features
- `Plugin` trait: removed `theme` capability methods

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
