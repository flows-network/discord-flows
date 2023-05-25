use axum::{
    body::Body,
    extract::Path,
    http::{response::Builder, HeaderValue, Request, Response, Uri},
};
use reqwest::Request as RRequest;
use serde::Deserialize;

use crate::{shared::get_client, DEFAULT_BOT_PLACEHOLDER, DEFAULT_TOKEN};

#[derive(Deserialize)]
pub struct PF {
    pub api: DiscordApi,
    pub path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscordApi {
    Api,
    Status,
}

pub async fn proxy(Path(PF { api, path }): Path<PF>, mut req: Request<Body>) -> Response<Body> {
    let hds = req.headers_mut();
    let x_token = hds.remove("Authorization").unwrap();
    let token = x_token.to_str().unwrap().strip_prefix("Bot ").unwrap();

    let token = if token == DEFAULT_BOT_PLACEHOLDER {
        &*DEFAULT_TOKEN
    } else {
        token
    };

    let (host, api_ver) = match api {
        DiscordApi::Api => ("discord.com:443", "api/v10"),
        DiscordApi::Status => ("status.discord.com:443", "api/v2"),
    };
    let mut api = format!("https://{host}/{api_ver}/{path}");

    if let Some(query) = req.uri().query() {
        api.push('?');
        api.push_str(query);
    }

    *req.uri_mut() = Uri::try_from(&api).unwrap();

    let hds = req.headers_mut();
    hds.insert("Host", HeaderValue::from_static(host));
    hds.insert("Accept", HeaderValue::from_static("*/*"));
    hds.insert("User-Agent", HeaderValue::from_static("flows.network"));
    hds.insert("Authorization", format!("Bot {token}").parse().expect("?"));
    hds.remove("Accept-Encoding");

    let new_req = RRequest::try_from(req).unwrap();

    let client = get_client();
    let resp = client.execute(new_req).await;

    match resp {
        Ok(res) => {
            let mut builder = Builder::new()
                .version(res.version())
                .status(res.status().as_u16());

            let headers = builder.headers_mut().unwrap();
            for (key, value) in res.headers() {
                headers.insert(key, value.clone());
            }

            let text = res.text().await.unwrap();
            headers.remove("Transfer-Encoding");
            let body = Body::from(text);

            builder.body(body).unwrap()
        }
        Err(_) => Response::new(Body::empty()),
    }
}
