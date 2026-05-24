# Grimoires

[![crates.io](https://img.shields.io/crates/v/grimoire-sdk.svg)](https://crates.io/crates/grimoire-sdk)
[![docs.rs](https://docs.rs/grimoire-sdk/badge.svg)](https://docs.rs/grimoire-sdk)
[![license](https://img.shields.io/crates/l/grimoire-sdk.svg)](LICENSE)

Official plugin repository for [Summoner](https://github.com/kakilangit/summoner). Each plugin is a Rust binary that communicates with Summoner over bidirectional HTTP, packaged as an OCI container image.

## Structure

```
grimoire/
├── sdk/              # grimoire-sdk crate (async, tokio + axum + reqwest)
├── plugins/
│   ├── ollama/       # grimoire-ollama — Ollama inference provider
│   ├── slack/        # grimoire-slack — Slack integration
│   └── guardrails/   # grimoire-guardrails — Content safety hooks
├── Cargo.toml        # workspace
└── Makefile
```

## Development

```sh
# Format, lint, test all
make ci

# Build a plugin container locally
make build PLUGIN=ollama
make build PLUGIN=slack
make build PLUGIN=guardrails

# Build all plugins
make build
```

## Creating a Plugin

1. Create `plugins/<name>/` with `Cargo.toml`, `src/main.rs`, `grimoire.json`, `Dockerfile`, `VERSION`
2. Implement `grimoire_sdk::Plugin` trait
3. The workspace auto-discovers plugins via `plugins/*` — no manual registration needed

## Releasing

### Plugins

Bump the version in `plugins/<name>/VERSION` and merge to `main`. The release workflow builds and pushes the OCI image to GHCR:

```
ghcr.io/kakilangit/grimoire:<name>-<version>
```

### SDK

Tag with `sdk-v<version>` (e.g. `sdk-v0.1.1`). The publish workflow verifies the tag matches `sdk/Cargo.toml` version and publishes to crates.io:

```sh
git tag sdk-v0.1.1
git push --tags
```

### Streaming

Plugins can call `ctx.invoke_agent_stream()` to receive tokens as they're generated via SSE. See [`grimoire-slack`](plugins/slack/src/main.rs) for an example.

## Available Plugins

| Plugin | Description | Capabilities |
|--------|-------------|--------------|
| `grimoire-ollama` | Ollama inference provider | provider |
| `grimoire-slack` | Bidirectional Slack integration | webhooks, events, tools |
| `grimoire-guardrails` | Content safety via hooks (blocklist, tool deny list) | hooks |
