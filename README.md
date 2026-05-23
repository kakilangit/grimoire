# Grimoires

Official plugin repository for [Summoner](https://github.com/kakilangit/summoner). Each plugin is a Rust binary that communicates with Summoner over bidirectional HTTP, packaged as an OCI container image.

## Structure

```
grimoire/
├── sdk/              # grimoire-sdk crate (async, tokio + axum + reqwest)
├── plugins/
│   ├── ollama/       # grimoire-ollama — Ollama inference provider
│   └── slack/        # grimoire-slack — Slack integration
├── Cargo.toml        # workspace
└── Makefile
```

## Development

```sh
# Format, lint, test
make ci

# Build a plugin container locally
make build-slack
make build-ollama
```

## Creating a Plugin

1. Create `plugins/<name>/` with `Cargo.toml`, `src/main.rs`, `grimoire.json`, `Dockerfile`, `VERSION`
2. Implement `grimoire_sdk::Plugin` trait
3. Add the plugin to the workspace in root `Cargo.toml`

## Releasing

### Plugins

Bump the version in `plugins/<name>/VERSION` and merge to `main`. The release workflow builds and pushes the OCI image to GHCR:

```
ghcr.io/kakilangit/grimoire-<name>:<version>
```

### SDK

Tag with `sdk-v<version>` (e.g. `sdk-v0.1.0`). The publish workflow verifies the tag matches `sdk/Cargo.toml` version and publishes to crates.io:

```sh
git tag sdk-v0.1.0
git push --tags
```

## Available Plugins

| Plugin | Description | Status |
|--------|-------------|--------|
| `grimoire-ollama` | Ollama inference provider | In Development |
| `grimoire-slack` | Bidirectional Slack integration | In Development |
