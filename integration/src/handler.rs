use std::sync::Arc;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::{Context, EventHandler};
use sqlx::PgPool;

use crate::shared::get_client;
use crate::HOOK_URL;

pub struct Handler {
    pub token: String,
    pub pool: Arc<PgPool>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        let client = get_client();
        _ = client
            .post(HOOK_URL.as_str())
            .json(&msg)
            .header("X-Discord-token", &self.token)
            .send()
            .await;
    }
}
