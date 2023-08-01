#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

use std::future::Future;

pub mod http;

pub mod model;

use async_trait::async_trait;
use flowsnet_platform_sdk::write_error_log;
use http::{Http, HttpBuilder};
use http_req::request;
use model::{
    application::interaction::application_command::ApplicationCommandInteraction, Message,
};

const API_PREFIX: &str = match std::option_env!("DISCORD_API_PREFIX") {
    Some(v) => v,
    None => "https://discord.flows.network",
};

const DEFAULT_BOT_PLACEHOLDER: &str = "DEFAULT_BOT";

pub enum EventModel {
    Message(Message),
    ApplicationCommand(ApplicationCommandInteraction),
}

extern "C" {
    // Flag if current running is for listening(1) or message receving(0)
    fn is_listening() -> i32;

    // Return the user id of the flows platform
    fn get_flows_user(p: *mut u8) -> i32;

    // Return the flow id
    fn get_flow_id(p: *mut u8) -> i32;

    fn get_event_body_length() -> i32;
    fn get_event_body(p: *mut u8) -> i32;
    fn get_event_headers_length() -> i32;
    fn get_event_headers(p: *mut u8) -> i32;
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

#[async_trait]
pub trait Bot {
    fn get_token(&self) -> String;

    /// Create a listener for Discord bot provided by flows.network
    ///
    /// Before creating the listener, this function will revoke previous
    /// registered listener of current flow so you don't need to do it manually.
    ///
    /// `callback` is a callback function which will be called when new `Message` is received.
    ///
    /// # Example
    ///
    /// ```rust
    /// #[tokio::main]
    /// pub async run() {
    ///     let bot = DefaultBot;
    ///     bot.listen_to_channel(123456, |msg| async {
    ///         todo!()
    ///     }).await;
    /// }
    /// ```
    async fn listen_to_channel<F, Fut>(&self, channel_id: u64, callback: F)
    where
        F: FnOnce(EventModel) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
    {
        listen_to_event(&self.get_token(), Some(channel_id), callback).await;
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
/// Before creating the listener, this function will revoke previous
/// registered listener of current flow so you don't need to do it manually.
///
/// `callback` is a callback function which will be called when new `Message` is received.
///
/// # Example
///
/// ```rust
/// #[tokio::main]
/// pub async run() {
///     let bot = ProvidedBot::new("YOUR BOT TOKEN");
///     bot.listen(|msg| async {
///         todo!()
///     }).await;
/// }
/// ```
impl ProvidedBot {
    pub async fn listen<F, Fut>(&self, callback: F)
    where
        F: FnOnce(EventModel) -> Fut + Send,
        Fut: Future<Output = ()> + Send,
    {
        listen_to_event(&self.token, None, callback).await;
    }
}

async fn listen_to_event<F, Fut>(token: &str, channel_id: Option<u64>, callback: F)
where
    F: FnOnce(EventModel) -> Fut + Send,
    Fut: Future<Output = ()> + Send,
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
                        "{}/{}/{}/{}/listen?bot_token={}",
                        API_PREFIX,
                        flows_user,
                        flow_id,
                        channel_id.unwrap_or(0),
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
                                "[{}] Listening to channel `{}`.",
                                std::env!("CARGO_CRATE_NAME"),
                                c
                            ),
                            None => format!(
                                "[{}] Listening to all channels your bot is on.",
                                std::env!("CARGO_CRATE_NAME")
                            ),
                        };
                        set_output(output.as_ptr(), output.len() as i32);
                    }
                    false => {
                        write_error_log!(String::from_utf8_lossy(&writer));
                        set_error_code(
                            format!("{}", res.status_code()).parse::<i16>().unwrap_or(0),
                        );
                    }
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

fn event_from_subcription() -> Option<EventModel> {
    unsafe {
        let headers = headers_from_subcription().unwrap_or_default();
        let event_model = headers
            .into_iter()
            .find(|(k, _)| k.to_lowercase() == "x-discord-event-model")
            .unwrap_or((String::new(), String::new()))
            .1;

        let l = get_event_body_length();
        let mut event_body = Vec::<u8>::with_capacity(l as usize);
        let c = get_event_body(event_body.as_mut_ptr());
        assert!(c == l);
        event_body.set_len(c as usize);

        match event_model.as_str() {
            "ApplicationCommand" => {
                match serde_json::from_slice::<ApplicationCommandInteraction>(&event_body) {
                    Ok(e) => Some(EventModel::ApplicationCommand(e)),
                    Err(_) => None,
                }
            }
            "Message" => match serde_json::from_slice::<Message>(&event_body) {
                Ok(e) => Some(EventModel::Message(e)),
                Err(_) => None,
            },
            _ => None,
        }
    }
}

fn headers_from_subcription() -> Option<Vec<(String, String)>> {
    unsafe {
        let l = get_event_headers_length();
        let mut event_body = Vec::<u8>::with_capacity(l as usize);
        let c = get_event_headers(event_body.as_mut_ptr());
        assert!(c == l);
        event_body.set_len(c as usize);

        serde_json::from_slice(&event_body).ok()
    }
}
