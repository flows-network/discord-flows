use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Flow {
    pub flows_user: String,
    pub flow_id: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
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
pub struct ListenerQuery {
    pub bot_token: String,
}
