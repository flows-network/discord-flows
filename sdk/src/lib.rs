#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

use std::{fmt::Display, future::Future};

pub mod http;

pub mod model;

use flowsnet_platform_sdk::write_error_log;
use http::{Http, HttpBuilder};
use http_req::request;
use model::Message;

const API_PREFIX: &str = match std::option_env!("DISCORD_API_PREFIX") {
    Some(v) => v,
    None => "https://discord.flows.network",
};

const DEFAULT_BOT_PLACEHOLDER: &str = "DEFAULT_BOT";

extern "C" {
    // Flag if current running is for listening(1) or message receving(0)
    fn is_listening() -> i32;

    // Return the user id of the flows platform
    fn get_flows_user(p: *mut u8) -> i32;

    // Return the flow id
    fn get_flow_id(p: *mut u8) -> i32;

    fn get_event_body_length() -> i32;
    fn get_event_body(p: *mut u8) -> i32;
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

/// Revoke previous registered listener of current flow.
///
/// Most of the time you do not need to call this function. As inside
/// the [listen_to_event()] it will revoke previous registered
/// listener, so the only circumstance you need this function is when
/// you want to change the listener from Discord to others.
pub fn revoke_listeners<S>(bot: S)
where
    S: Into<Bot>,
{
    unsafe {
        let flows_user = _get_flows_user();
        let flow_id = _get_flow_id();

        let mut writer = Vec::new();
        let res = request::post(
            format!(
                "{}/{}/{}/revoke?bot_token={}",
                API_PREFIX,
                flows_user,
                flow_id,
                bot.into(),
            ),
            &[],
            &mut writer,
        )
        .unwrap();

        match res.status_code().is_success() {
            true => (),
            false => {
                write_error_log!(String::from_utf8_lossy(&writer));
                set_error_code(format!("{}", res.status_code()).parse::<i16>().unwrap_or(0));
            }
        }
    }
}

pub enum Bot {
    Default,
    Provided(String),
}

impl Bot {
    pub fn new<S: Into<String>>(token: S) -> Self {
        Self::Provided(token.into())
    }
}

impl Display for Bot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bot::Default => f.write_str(DEFAULT_BOT_PLACEHOLDER),
            Bot::Provided(token) => f.write_str(&token),
        }
    }
}

impl Default for Bot {
    fn default() -> Self {
        Bot::Default
    }
}

impl From<String> for Bot {
    fn from(value: String) -> Self {
        Bot::new(value)
    }
}

impl From<&str> for Bot {
    fn from(value: &str) -> Self {
        Bot::new(value)
    }
}

impl From<Option<String>> for Bot {
    fn from(value: Option<String>) -> Self {
        match value {
            Some(v) => Bot::new(v),
            None => Bot::default(),
        }
    }
}

impl From<Option<&str>> for Bot {
    fn from(value: Option<&str>) -> Self {
        match value {
            Some(v) => Bot::new(v),
            None => Bot::default(),
        }
    }
}

/// Create a listener for Discord bot represented by `bot_token`
///
/// Before creating the listener, this function will revoke previous
/// registered listener of current flow so you don't need to do it manually.
///
/// `callback` is a callback function which will be called when new `Message` is received.
///
/// # Example
///
/// ## Provide token
/// ```rust
/// #[tokio::main]
/// pub async run() {
///     listen_to_event("YOUR BOT TOKEN", |msg| async {
///         todo!()
///     }).await;
/// }
/// ```
///
/// ## Use the token provided by flows.network
/// ```rust
/// #[tokio::main]
/// pub async run() {
///     listen_to_event(Bot::default(), |msg| async {
///         todo!()
///     }).await;
/// }
/// ```
pub async fn listen_to_event<S, F, Fut>(bot: S, callback: F)
where
    S: Into<Bot>,
    F: FnOnce(Message) -> Fut,
    Fut: Future<Output = ()>,
{
    unsafe {
        match is_listening() {
            // Calling register
            1 => {
                let flows_user = _get_flows_user();
                let flow_id = _get_flow_id();

                let mut writer = Vec::new();
                let res = request::post(
                    format!(
                        "{}/{}/{}/listen?bot_token={}",
                        API_PREFIX,
                        flows_user,
                        flow_id,
                        bot.into(),
                    ),
                    &[],
                    &mut writer,
                )
                .unwrap();

                if !res.status_code().is_success() {
                    write_error_log!(String::from_utf8_lossy(&writer));
                    set_error_code(format!("{}", res.status_code()).parse::<i16>().unwrap_or(0));
                }
            }
            _ => {
                if let Some(event) = event_from_subcription() {
                    callback(event).await;
                }
            }
        }
    }
}

/// Get a Discord Client as a bot represented by `bot_token`
#[inline]
pub fn get_client<S>(bot: S) -> Http
where
    S: Into<Bot>,
{
    match bot.into() {
        Bot::Default => HttpBuilder::new(DEFAULT_BOT_PLACEHOLDER),
        Bot::Provided(token) => HttpBuilder::new(token),
    }
    .build()
}

fn event_from_subcription() -> Option<Message> {
    unsafe {
        let l = get_event_body_length();
        let mut event_body = Vec::<u8>::with_capacity(l as usize);
        let c = get_event_body(event_body.as_mut_ptr());
        assert!(c == l);
        event_body.set_len(c as usize);
        match serde_json::from_slice(&event_body) {
            Ok(e) => Some(e),
            Err(_) => None,
        }
    }
}
