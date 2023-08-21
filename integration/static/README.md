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
    model::Message,
    Bot, ProvidedBot,
    message_handler,
};

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    let token = std::env::var("DISCORD_TOKEN").unwrap();
    let bot = ProvidedBot::new(token);
    bot.listen_to_messages().await;
}

#[message_handler]
async fn handle(msg: Message) {
    let token = std::env::var("DISCORD_TOKEN").unwrap();
    let bot = ProvidedBot::new(token);
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
```

[get_client()] is a `Discord` constructor that represents a bot.

[listen_to_messages()] is responsible for registering a listener for the bot
represented by the `token`. When a new `Message` coming, the fn `handle`
is called with received `Message`.


## Using the default Bot
If you don't want to create your own Discord Bot, we have created a public Bot which can be used by all users.

```rust
use discord_flows::{
    model::{
        application::interaction::InteractionResponseType,
        prelude::application::interaction::application_command::ApplicationCommandInteraction,
    },
    Bot, DefaultBot,
    application_command_handler,
};
use std::time::Duration;

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn run() {
    let channel_id = 1104392662985220296; // Your channel id
    let bot = DefaultBot {};
    bot.listen_to_application_commands_from_channel(channel_id).await;
}

#[application_command_handler]
async fn handle(ac: ApplicationCommandInteraction) {
    let bot = DefaultBot {};
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
```

To invite this public Bot, please `Connect` to your Discord server on top of this page.
