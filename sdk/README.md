# grimoire-sdk

[![crates.io](https://img.shields.io/crates/v/grimoire-sdk.svg)](https://crates.io/crates/grimoire-sdk)
[![docs.rs](https://docs.rs/grimoire-sdk/badge.svg)](https://docs.rs/grimoire-sdk)
[![license](https://img.shields.io/crates/l/grimoire-sdk.svg)](../LICENSE)

SDK for building [Summoner](https://github.com/kakilangit/summoner) Grimoire plugins in Rust.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
grimoire-sdk = "0.1"
```

## Quick Start

Implement the `Plugin` trait and call `grimoire_sdk::run`:

```rust
use grimoire_sdk::{Plugin, PluginError, Context, run};

struct MyPlugin;

impl Plugin for MyPlugin {
    fn name(&self) -> &'static str { "my-plugin" }
}

#[tokio::main]
async fn main() {
    run(MyPlugin).await;
}
```

Override trait methods to add capabilities (tools, webhooks, hooks, events, provider). See the [docs.rs documentation](https://docs.rs/grimoire-sdk) and `plugins/` directory for examples.

## Features

- Async runtime (tokio + axum)
- OpenAI-compatible streaming via SSE
- Bidirectional HTTP communication with Summoner
- Structured logging (JSON via tracing)

## License

MIT
