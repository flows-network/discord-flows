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

This is a plain text echo bot.

```rust
use discord_flows::{get_client, listen_to_event, model::Message, Bot};

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
If you don't have a token, please see this [section](#Creating-a-Bot-Account).

[listen_to_event()] is responsible for registering a listener for the bot
represented by the `bot_token`. When a new `Message` coming, the callback
is called with received `Message`.

## Creating a Bot Account

Some of the following are excerpts from
[discord.py docs](https://discordpy.readthedocs.io/en/stable/discord.html)

1. Make sure you’re logged on to the Discord [website](https://discord.com/).
2. Navigate to the [application page](https://discord.com/developers/applications).
3. Click on the “New Application” button.
![new-application](https://res.cloudinary.com/wasm-reactor/image/upload/v1684130272/extension/discord/new-application_dkoadi.png)
4. Give the application a name and click “Create”.
![fill-name](https://res.cloudinary.com/wasm-reactor/image/upload/v1684130273/extension/discord/fill-name_jlxnq9.png)
5. Navigate to the “Bot”.
6. Make sure that Public Bot is ticked if you want others to invite your bot.
And tick all the options of the "Privileged Gateway Intents".
![Privileged Gateway Intents](https://res.cloudinary.com/wasm-reactor/image/upload/v1685068895/extension/discord/intents_sqxirg.png)
7. Click on the "Reset Token" button.
![reset-token](https://res.cloudinary.com/wasm-reactor/image/upload/v1684130273/extension/discord/reset-token_hbgjof.png)
8. Confirm reset by clicking "Yes, do it!" button.
![confirm-reset-token](https://res.cloudinary.com/wasm-reactor/image/upload/v1684130272/extension/discord/confirm-reset-token_xweokd.png)
9. Copy the token using the “Copy” button.
> It should be worth noting that this token is essentially your bot’s password.
> You should never share this with someone else.
> In doing so, someone can log in to your bot and do malicious things,
> such as leaving servers, ban all members inside a server,
> or pinging everyone maliciously.
>
> The possibilities are endless, so **do not share this token**.
>
> If you accidentally leaked your token,
> click the “Regenerate” button as soon as possible.
> This revokes your old token and re-generates a new one.
> Now you need to use the new token to login.
10. Keep your token in a **safe** place, the token will only be shown **once**.

## Inviting Your Bot
So you’ve made a Bot User but it’s not actually in any server.

If you want to invite your bot you must create an invite URL for it.

1. Make sure you’re logged on to the Discord website.
2. Navigate to the application page
3. Click on your bot’s page.
4. Go to the “OAuth2” tab.
  ![discord_oauth2](https://res.cloudinary.com/wasm-reactor/image/upload/v1684130895/extension/discord/discord_oauth2_kh7mwv.webp)
5. Tick the “bot” checkbox under “scopes”.
  ![discord_oauth2_scope](https://res.cloudinary.com/wasm-reactor/image/upload/v1684130895/extension/discord/discord_oauth2_scope_tpxmka.webp)
6. Tick the permissions required for your bot to function under “Bot Permissions”.
  - Please be aware of the consequences of requiring your bot to have the “Administrator” permission.
  - Bot owners must have 2FA enabled for certain actions and permissions when added in servers that have Server-Wide 2FA enabled. Check the 2FA support page for more information.
    ![discord_oauth2_perm](https://res.cloudinary.com/wasm-reactor/image/upload/v1684130895/extension/discord/discord_oauth2_perms_eskbwl.webp)
7. Now the resulting URL can be used to add your bot to a server. Copy and paste the URL into your browser, choose a server to invite the bot to, and click “Authorize”.

> The person adding the bot needs “Manage Server” permissions to do so.


## Using the default Bot
If you don't want to create your own Discord Bot, we have created a pub Bot which can be used by all users.

```rust
use discord_flows::{get_client, listen_to_event, model::Message, Bot};

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn run() {
    listen_to_event(Bot::Default, move |msg| handle(msg)).await;
}

async fn handle(msg: Message) {
    let client = get_client(Bot::Default);
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

To invite this public Bot, you should compose the auth url with the permissions and scope
and replace the `client_id` by `1090851501473271919`.
For example, if the scope is `bot` and permissions is `Send Messages` then the auth url should look like:
https://discord.com/api/oauth2/authorize?client_id=1090851501473271919&permissions=2048&scope=bot
