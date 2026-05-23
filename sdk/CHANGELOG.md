# Changelog

## 0.1.1

### Added

- Provider capability: `list_models`, `handle_chat`, `handle_chat_stream`
- OpenAI-compatible SSE streaming (`read_oai_stream`)
- `SseReader` for parsing server-sent events
- SDK README with crates.io, docs.rs, and license badges
- Root README badges for crates.io, docs.rs, and license

### Changed

- `Context.callback_url` and `Context.callback_token` now default to empty string via `#[serde(default)]`
- Streaming logic extracted from plugins into SDK

## 0.1.0

### Added

- Initial release
- `Plugin` trait with tools, webhooks, hooks, and events capabilities
- HTTP server (`run`) with health, manifest, and capability endpoints
- `CallbackClient` for bidirectional communication with Summoner
- Structured JSON logging via `tracing`
