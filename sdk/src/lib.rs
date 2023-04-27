use std::future::Future;

pub mod http;

pub mod model;

use flowsnet_platform_sdk::write_error_log;
use http::{Http, HttpBuilder};
use http_req::request;
use model::Message;

const API_PREFIX: &str = "https://discord-flows.shuttleapp.rs";

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

unsafe fn _get_flows_user() -> String {
    let mut flows_user = Vec::<u8>::with_capacity(100);
    let c = get_flows_user(flows_user.as_mut_ptr());
    flows_user.set_len(c as usize);
    String::from_utf8(flows_user).unwrap()
}

unsafe fn _get_flow_id() -> String {
    let mut flow_id = Vec::<u8>::with_capacity(100);
    let c = get_flow_id(flow_id.as_mut_ptr());
    if c == 0 {
        panic!("Failed to get flow id");
    }
    flow_id.set_len(c as usize);
    String::from_utf8(flow_id).unwrap()
}

pub fn revoke_listeners<S>(bot_token: S)
where
    S: AsRef<str>,
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
                bot_token.as_ref(),
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

pub async fn listen_to_event<S, F, Fut>(bot_token: S, callback: F)
where
    S: AsRef<str>,
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
                        bot_token.as_ref(),
                    ),
                    &[],
                    &mut writer,
                )
                .unwrap();

                match res.status_code().is_success() {
                    true => {
                        if let Ok(event) = serde_json::from_slice::<Message>(&writer) {
                            callback(event).await;
                        }
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

#[inline]
pub fn get_client<S>(bot_token: S) -> Http
where
    S: AsRef<str>,
{
    HttpBuilder::new(bot_token).build()
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
