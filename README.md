# adapt-rs
High-level wrapper around Adapt's API for Rust.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
adapt = { git = "https://github.com/adaptchat/adapt-rs" }
```

### Cargo Features

| Feature | Default | Description                                              |
|---------|---------|----------------------------------------------------------|
| `ws`    | Yes     | Enables receiving events over Harmony, Adapt's gateway.  |
| `simd`  | No      | Enables SIMD speedups for JSON parsing via `simd-json`.  |

## Requirements

- Rust 1.80 **nightly** or later

## Example

```rust
use adapt::prelude::*;
use adapt::models::{Message, ReadyEvent};

struct Handler;

impl EventHandler for Handler {
    type Error = adapt::Error;

    async fn ready(&self, event: &ReadyEvent) -> Result<()> {
        info!("Logged in as {}", ?event.user);

        Ok(())
    }

    async fn message_create(&self, message: &Message) -> Result<()> {
        if message.content.starts_with("!ping") {
            message.reply("Pong").await?;
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Get the bot token from environment variables
    let token = std::env::var("ADAPT_TOKEN").expect("No token provided");
    
    // Create a new client with the token, register an event handler, and start it
    Client::new(&token).with_event_handler(Handler).start().await
}
```
