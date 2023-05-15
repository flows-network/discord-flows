use discord_flows::model::Message;
use http_req::request;

const API_PREFIX: &str = "https://discord.flows.network";

extern "C" {
    fn get_event_body_length() -> i32;
    fn get_event_body(p: *mut u8) -> i32;
    fn get_event_headers_length() -> i32;
    fn get_event_headers(p: *mut u8) -> i32;
    fn set_flows(p: *const u8, len: i32);
}

#[no_mangle]
pub unsafe fn message() {
    if let Some(msg) = message_from_channel() {
        // TODO: maybe pass bot_id from headers instead of uuid
        if msg.author.bot || msg.author.name == "bot_name" {}

        let headers = headers_from_subcription().unwrap_or_default();
        let uuid = headers
            .into_iter()
            .find(|(k, _)| k.to_lowercase() == "x-discord-uuid")
            .unwrap_or((String::new(), String::new()))
            .1;

        let mut writer = Vec::new();
        let res = request::get(format!("{}/event/{}", API_PREFIX, uuid), &mut writer).unwrap();

        if res.status_code().is_success() {
            if let Ok(flows) = String::from_utf8(writer) {
                set_flows(flows.as_ptr(), flows.len() as i32);
            }
        }
    }
}

fn message_from_channel() -> Option<Message> {
    unsafe {
        let l = get_event_body_length();
        let mut event_body = Vec::<u8>::with_capacity(l as usize);
        let c = get_event_body(event_body.as_mut_ptr());
        assert!(c == l);
        event_body.set_len(c as usize);
        serde_json::from_slice(&event_body).ok()
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
