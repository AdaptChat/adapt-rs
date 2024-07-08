# adapt-rs
High-level wrapper around Adapt's API for Rust.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
adapt = { git = "https://github.com/adaptchat/adapt-rs" }
```

### Cargo Features

| Feature  | Default | Description                                                                                 |
|----------|---------|---------------------------------------------------------------------------------------------|
| `ws`     | Yes     | Enables receiving events over Harmony, Adapt's gateway.                                     |
| `simd`   | No      | Enables SIMD speedups for JSON parsing via `simd-json`.                                     |
| `chrono` | No      | Timestamps will be represented using `chrono::DateTime` instead of `std::time::SystemTime`. |

## Requirements

- Rust 1.80 **nightly** or later

## Example

```rust
use adapt::prelude::*;
use adapt::models::Message;

struct Handler;

impl FallibleEventHandler for Handler {
    type Error = adapt::Error;

    async fn on_error(&self, _error: Self::Error) {
        error!("An error occurred: {:?}", error);
    }

    async fn on_ready(&self, ctx: Context) -> Result<()> {
        info!("Logged in as {:?}", context.user());

        Ok(())
    }

    async fn on_message(&self, message: WithCtx<Message>) -> Result<()> {
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
