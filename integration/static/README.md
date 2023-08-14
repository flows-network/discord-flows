<div align="center">
  <h1><code>Discord Flows</code></h1>
  <a href="https://docs.rs/discord-flows/">
    <img src="https://docs.rs/discord-flows/badge.svg">
  </a>
  <a href="https://crates.io/crates/discord-flows">
    <img src="https://img.shields.io/crates/v/discord-flows.svg">
  </a>

  Discord Integration for [Flows.network](https://flows.network)
</div>

## Quick Start

This is a plain text echo bot. Pass your own Discord Bot's token then act
as your bot. For how to create your own Bot please refer to our [blog](https://flows.network/blog/discord-chat-bot-guide).

```rust
use discord_flows::{
    model::application::interaction::InteractionResponseType, Bot, EventModel, ProvidedBot,
};
use std::time::Duration;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn run() {
    let token = std::env::var("DISCORD_TOKEN").unwrap();
    let bot = ProvidedBot::new(token);
    bot.listen(|em| handle(&bot, em)).await;
}

async fn handle<B: Bot>(bot: &B, em: EventModel) {
    match em {
        // Slash command received
        EventModel::ApplicationCommand(ac) => {
            let client = bot.get_client();

            _ = client
                .create_interaction_response(
                    ac.id.into(),
                    &ac.token,
                    &serde_json::json!({
                        "type": InteractionResponseType::DeferredChannelMessageWithSource as u8,
                    }),
                )
                .await;
            tokio::time::sleep(Duration::from_secs(3)).await;
            client.set_application_id(ac.application_id.into());
            _ = client
                .edit_original_interaction_response(
                    &ac.token,
                    &serde_json::json!({
                        "content": "Pong"
                    }),
                )
                .await;

            if let Ok(m) = client
                .create_followup_message(
                    &ac.token,
                    &serde_json::json!({
                        "content": "PongPong"
                    }),
                )
                .await
            {
                _ = client
                    .edit_followup_message(
                        &ac.token,
                        m.id.into(),
                        &serde_json::json!({
                            "content": "PongPongPong"
                        }),
                    )
                    .await;
            }
        }
        // Normal message received
        EventModel::Message(msg) => {
            let client = bot.get_client();
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
    }
}
```

[get_client()] is a `Discord` constructor that represents a bot.

[listen_to_event()] is responsible for registering a listener for the bot
represented by the `token`. When a new `Message` coming, the callback
is called with received `Message`.


## Using the default Bot
If you don't want to create your own Discord Bot, we have created a public Bot which can be used by all users.

```rust
use discord_flows::{
    model::application::interaction::InteractionResponseType, Bot, EventModel, DefaultBot,
};
use std::time::Duration;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn run() {
    let channel_id = 1104392662985220296; // Your channel id
    let bot = DefaultBot {};
    bot.listen_to_channel(channel_id, |em| handle(&bot, em)).await;
}

async fn handle<B: Bot>(bot: &B, em: EventModel) {
    match em {
        // Slash command received
        EventModel::ApplicationCommand(ac) => {
            let client = bot.get_client();

            _ = client
                .create_interaction_response(
                    ac.id.into(),
                    &ac.token,
                    &serde_json::json!({
                        "type": InteractionResponseType::DeferredChannelMessageWithSource as u8,
                    }),
                )
                .await;
            tokio::time::sleep(Duration::from_secs(3)).await;
            client.set_application_id(ac.application_id.into());
            _ = client
                .edit_original_interaction_response(
                    &ac.token,
                    &serde_json::json!({
                        "content": "Pong"
                    }),
                )
                .await;

            if let Ok(m) = client
                .create_followup_message(
                    &ac.token,
                    &serde_json::json!({
                        "content": "PongPong"
                    }),
                )
                .await
            {
                _ = client
                    .edit_followup_message(
                        &ac.token,
                        m.id.into(),
                        &serde_json::json!({
                            "content": "PongPongPong"
                        }),
                    )
                    .await;
            }
        }
        // Normal message received
        EventModel::Message(msg) => {
            let client = bot.get_client();
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
    }
}
```

To invite this public Bot, please `Connect` to your Discord server on top of this page.
