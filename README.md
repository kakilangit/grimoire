# Grimoires

Official plugin repository for [Summoner](https://github.com/kakilangit/summoner). Each plugin is a Rust binary that communicates with Summoner over bidirectional HTTP, packaged as an OCI container image.

## Structure

```
grimoire/
├── sdk/              # grimoire-sdk crate
├── plugins/
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
```

## Creating a Plugin

1. Create `plugins/<name>/` with `Cargo.toml`, `src/main.rs`, `grimoire.json`, `Dockerfile`, `VERSION`
2. Implement `grimoire_sdk::Plugin` trait
3. Add the plugin to the workspace in root `Cargo.toml`

## Releasing

Bump the version in `plugins/<name>/VERSION` and merge to `main`. The release workflow automatically builds and pushes the OCI image to GHCR:

```
ghcr.io/kakilangit/grimoire-<name>:<version>
```

## Available Plugins

| Plugin | Description | Status |
|--------|-------------|--------|
| `grimoire-slack` | Bidirectional Slack integration | In Development |
