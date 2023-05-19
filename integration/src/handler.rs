use std::sync::Arc;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::{Context, EventHandler};
use sqlx::PgPool;

use crate::common::get_client;

pub struct Handler {
    pub token: String,
    pub pool: Arc<PgPool>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        let hook_url = "https://code.flows.network/hook/discord/message";
        let client = get_client();
        _ = client
            .post(hook_url)
            .json(&msg)
            .header("X-Discord-token", &self.token)
            .send()
            .await;
    }
}
