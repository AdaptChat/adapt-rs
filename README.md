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
use adapt::models::Message;
use adapt::prelude::*;

// Define an event handler
struct Handler;

impl EventHandler for Handler {
    // This method is called when the bot is ready to receive events
    async fn on_ready(&self, context: Context) {
        // The context includes the user that the bot is logged in as
        println!("Ready as {}", context.user().username);
    }

    // This method is called when a message is sent
    async fn on_message(&self, message: WithCtx<Message>) {
        // Check if the message content is "!ping"
        if message.content == "!ping" {
            // If so, reply to the message with "pong"
            if let Err(e) = message.reply("pong").await {
                // If an error occurs, print the error
                eprintln!("Error replying to message: {e:?}")
            }
        }
    }
}

#[tokio::main]
async fn main() -> adapt::Result<()> {
    // Get the bot token from environment variables
    let token = std::env::var("ADAPT_TOKEN").expect("No token found");

    // Create a new client with the token, register the event handler, and start the client
    Client::from_token(&token).with_consumer(Handler).start().await?;

    Ok(())
}
```
