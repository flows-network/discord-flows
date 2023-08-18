use std::result::Result as StdResult;

use serde::de::Deserializer;
use serde::Deserialize;
use serde_json::Value;
use serenity::json::ToNumber;
use serenity::model::prelude::ReactionType;
use url::Url;

use crate::http::error::DiscordJsonSingleError;

#[allow(clippy::missing_errors_doc)]
pub fn deserialize_errors<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> StdResult<Vec<DiscordJsonSingleError>, D::Error> {
    let map: Value = Value::deserialize(deserializer)?;

    if !map.is_object() {
        return Ok(vec![]);
    }

    let mut errors = Vec::new();

    loop_errors(&map, &mut errors, &[]);

    Ok(errors)
}

fn loop_errors(value: &Value, errors: &mut Vec<DiscordJsonSingleError>, path: &[String]) {
    for (key, looped) in value.as_object().expect("expected object").iter() {
        let object = looped.as_object().expect("expected object");
        if object.contains_key("_errors") {
            let found_errors = object
                .get("_errors")
                .expect("expected _errors")
                .as_array()
                .expect("expected array")
                .clone();

            for error in found_errors {
                let error_object = error.as_object().expect("expected object");
                let mut object_path = path.to_owned();

                object_path.push(key.to_string());

                errors.push(DiscordJsonSingleError {
                    code: error_object
                        .get("code")
                        .expect("expected code")
                        .as_str()
                        .expect("expected string")
                        .to_owned(),
                    message: error_object
                        .get("message")
                        .expect("expected message")
                        .as_str()
                        .expect("expected string")
                        .to_owned(),
                    path: object_path.join("."),
                });
            }
            continue;
        }

        let mut new_path = path.to_owned();
        new_path.push(key.to_string());

        loop_errors(looped, errors, &new_path);
    }
}

#[must_use]
pub fn parse_webhook(url: &Url) -> Option<(u64, &str)> {
    let (webhook_id, token) = url.path().strip_prefix("/api/webhooks/")?.split_once('/')?;
    if !["http", "https"].contains(&url.scheme())
        || !["discord.com", "discordapp.com"].contains(&url.domain()?)
        || !(17..=20).contains(&webhook_id.len())
        || !(60..=68).contains(&token.len())
    {
        return None;
    }
    Some((webhook_id.parse().ok()?, token))
}

// pub(crate) fn to_string<T>(v: &T) -> Result<String>
// where
//     T: Serialize,
// {
//     Ok(serde_json::to_string(v)?)
// }

pub(crate) fn from_number(n: impl ToNumber) -> Value {
    n.to_number()
}

macro_rules! api {
    ($($rest:tt)*) => {
        {
            let mut api = String::from(crate::API_PREFIX);
            api.push_str("/proxy/api");
            _ = write!(api, $($rest)*);
            api
        }
    };
}
pub(crate) use api;

macro_rules! status {
    ($e:expr) => {
        const_format::concatcp!(crate::API_PREFIX, "/proxy/status", $e)
    };
}
pub(crate) use status;

#[inline]
pub fn as_data(reaction_type: &ReactionType) -> String {
    match reaction_type {
        ReactionType::Custom { id, ref name, .. } => {
            format!("{}:{}", name.as_ref().map_or("", String::as_str), id)
        }
        ReactionType::Unicode(ref unicode) => unicode.clone(),
        _ => panic!("ops"),
    }
}
