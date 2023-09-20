#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod http;

pub mod model;

pub use discord_flows_macros::*;

use async_trait::async_trait;
use flowsnet_platform_sdk::write_error_log;
use http::{Http, HttpBuilder};
use http_req::request;
use std::fmt;

const API_PREFIX: &str = match std::option_env!("DISCORD_API_PREFIX") {
    Some(v) => v,
    None => "https://discord.flows.network",
};

const DEFAULT_BOT_PLACEHOLDER: &str = "DEFAULT_BOT";

extern "C" {
    // Return the user id of the flows platform
    fn get_flows_user(p: *mut u8) -> i32;

    // Return the flow id
    fn get_flow_id(p: *mut u8) -> i32;

    fn set_output(p: *const u8, len: i32);
    fn set_error_code(code: i16);
}

pub(crate) unsafe fn _get_flows_user() -> String {
    let mut flows_user = Vec::<u8>::with_capacity(100);
    let c = get_flows_user(flows_user.as_mut_ptr());
    flows_user.set_len(c as usize);
    String::from_utf8(flows_user).unwrap()
}

pub(crate) unsafe fn _get_flow_id() -> String {
    let mut flow_id = Vec::<u8>::with_capacity(100);
    let c = get_flow_id(flow_id.as_mut_ptr());
    if c == 0 {
        panic!("Failed to get flow id");
    }
    flow_id.set_len(c as usize);
    String::from_utf8(flow_id).unwrap()
}

enum TriggerType {
    Message,
    ApplicationCommand,
}

impl fmt::Display for TriggerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Message => "messages",
                Self::ApplicationCommand => "application commands",
            }
        )
    }
}

#[async_trait]
pub trait Bot {
    fn get_token(&self) -> String;

    /// Create a message listener for Discord bot provided by flows.network
    ///
    /// # Example
    ///
    /// ```rust
    /// #[tokio::main]
    /// pub async run() {
    ///     let bot = DefaultBot;
    ///     bot.listen_to_messages_from_channel(123456).await;
    /// }
    /// ```
    async fn listen_to_messages_from_channel(&self, channel_id: u64) {
        listen_to(&self.get_token(), TriggerType::Message, Some(channel_id)).await;
    }

    /// Create a application command listener for Discord bot provided by flows.network
    ///
    /// # Example
    ///
    /// ```rust
    /// #[tokio::main]
    /// pub async run() {
    ///     let bot = DefaultBot;
    ///     bot.listen_to_application_commands_from_channel(123456).await;
    /// }
    /// ```
    async fn listen_to_application_commands_from_channel(&self, channel_id: u64) {
        listen_to(
            &self.get_token(),
            TriggerType::ApplicationCommand,
            Some(channel_id),
        )
        .await;
    }

    /// Get a Discord Client as a bot represented by `bot_token`
    #[inline]
    fn get_client(&self) -> Http {
        HttpBuilder::new(self.get_token()).build()
    }
}

pub struct DefaultBot;

impl Bot for DefaultBot {
    fn get_token(&self) -> String {
        String::from(DEFAULT_BOT_PLACEHOLDER)
    }
}

pub struct ProvidedBot {
    token: String,
}

impl Bot for ProvidedBot {
    fn get_token(&self) -> String {
        self.token.clone()
    }
}

impl ProvidedBot {
    pub fn new<S: Into<String>>(token: S) -> Self {
        Self {
            token: token.into(),
        }
    }
}

/// Create a listener for Discord bot represented by `bot_token`
///
/// # Example
///
/// ```rust
/// #[tokio::main]
/// pub async run() {
///     let bot = ProvidedBot::new("YOUR BOT TOKEN");
///     bot.listen_to_messages().await;
/// }
/// ```
impl ProvidedBot {
    pub async fn listen_to_messages(&self) {
        listen_to(&self.token, TriggerType::Message, None).await;
    }

    pub async fn listen_to_application_commands(&self) {
        listen_to(&self.token, TriggerType::ApplicationCommand, None).await;
    }
}

async fn listen_to(token: &str, trigger_type: TriggerType, channel_id: Option<u64>) {
    let handler_fn = match trigger_type {
        TriggerType::Message => "__discord__on_message_received",
        TriggerType::ApplicationCommand => "__discord__on_application_command_received",
    };

    unsafe {
        let flows_user = _get_flows_user();
        let flow_id = _get_flow_id();

        let mut writer = Vec::new();
        let res = request::post(
            format!(
                "{}/{}/{}/{}/listen?handler_fn={}&bot_token={}",
                API_PREFIX,
                flows_user,
                flow_id,
                channel_id.unwrap_or(0),
                handler_fn,
                token,
            ),
            &[],
            &mut writer,
        )
        .unwrap();

        match res.status_code().is_success() {
            true => {
                let output = match channel_id {
                    Some(c) => format!(
                        "[{}] Listening {} from channel `{}`.",
                        std::env!("CARGO_CRATE_NAME"),
                        trigger_type,
                        c
                    ),
                    None => format!(
                        "[{}] Listening {} from all channels your bot is on.",
                        std::env!("CARGO_CRATE_NAME"),
                        trigger_type,
                    ),
                };
                set_output(output.as_ptr(), output.len() as i32);
            }
            false => {
                write_error_log!(String::from_utf8_lossy(&writer));
                set_error_code(format!("{}", res.status_code()).parse::<i16>().unwrap_or(0));
            }
        }
    }
}
