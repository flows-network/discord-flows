<div align="center">
  <h1><code>Discord Flows</code></h1>
  <a href="https://docs.rs/discord-flows/">
    <img src="https://docs.rs/discord-flows/badge.svg">
  </a>
  <a href="https://crates.io/crates/discord-flows">
    <img src="https://img.shields.io/crates/v/discord-flows.svg">
  </a>

  Discord Integration for [Flows.network](https://test.flows.network)
</div>

## Quick Start

There is a echo bot, but plain text:

```rust
use discord_flows::{get_client, listen_to_event, model::Message};

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn run() {
    let token = std::env::var("DISCORD_TOKEN").unwrap();

    listen_to_event(token.clone(), move |msg| handle(msg, token)).await;
}

async fn handle(msg: Message, token: String) {
    let client = get_client(token);
    let channel_id = msg.channel_id;
    let content = msg.content;

    if msg.author.bot {
        return;
    }

    _ = client
        .send_message(
            channel_id.into(),
            &serde_json::json!({
                "content": content,
            }),
        )
        .await;
}
```

[get_client()] is a `Discord` constructor that represents a bot.

[listen_to_event()] is responsible for registering a listener for the bot
represented by the `bot_token`. When a new `Message` coming, the callback
is called with received `Message`.

