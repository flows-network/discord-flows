use serde::Deserialize;

#[derive(Deserialize, sqlx::FromRow)]
pub struct Flow {
    pub flows_user: String,
    pub flow_id: String,
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

#[derive(Debug, sqlx::FromRow)]
pub struct Fid {
    pub flow_id: String,
}
