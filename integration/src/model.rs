use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Flow {
    pub flows_user: String,
    pub flow_id: String,
    pub handler_fn: Option<String>,
}

#[derive(Deserialize, sqlx::FromRow)]
pub struct ListenPath {
    pub flows_user: String,
    pub flow_id: String,
    pub channel_id: String,
}

#[derive(sqlx::FromRow)]
pub struct Bot {
    #[sqlx(rename = "bot_token")]
    pub token: String,
}

#[derive(sqlx::FromRow)]
pub struct Count {
    pub count: i64,
}

#[derive(Deserialize)]
pub struct Recipient {
    pub id: String,
}

#[derive(Deserialize)]
pub struct DiscordChannel {
    #[serde(rename = "type")]
    pub ctype: u8,
    pub guild_id: Option<String>,
    pub recipients: Option<Vec<Recipient>>,
}

#[derive(Deserialize)]
pub struct ListenerQuery {
    pub handler_fn: Option<String>,
    pub bot_token: String,
}

#[derive(Deserialize)]
pub struct AuthQuery {
    pub state: String,
    pub code: String,
}

#[derive(Serialize)]
pub struct AuthTokenRequest {
    pub client_id: String,
    pub client_secret: String,
    pub grant_type: String,
    pub code: String,
    pub redirect_uri: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthTokenResponse {
    pub access_token: String,
    pub guild: Guild,
}

#[derive(Debug, Deserialize)]
pub struct Guild {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct AuthStateClaim {
    pub flows_user: String,
    exp: usize,
}

#[derive(Debug, Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct GuildAuthor {
    pub flows_user: String,
    pub discord_guild_name: String,
}
