use std::sync::Arc;

use axum::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::{Context, EventHandler};
use sqlx::PgPool;

use crate::model::Flow;
use crate::shared::get_client;
use crate::{DEFAULT_BOT_PLACEHOLDER, HOOK_URL};

pub struct Handler {
    pub token: String,
    pub pool: Arc<PgPool>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        let channel_id = msg.channel_id;
        let flows: Option<Vec<Flow>> = if self.token == DEFAULT_BOT_PLACEHOLDER {
            let select = "
                SELECT flows_user, flow_id
                FROM listener
                WHERE channel_id = $1 and bot_token = $2
            ";
            sqlx::query_as(select)
                .bind(channel_id.as_u64().to_string())
                .bind(DEFAULT_BOT_PLACEHOLDER)
                .fetch_all(&*self.pool)
                .await
                .ok()
        } else {
            let select = "
                SELECT flows_user, flow_id
                FROM listener
                WHERE (channel_id = '' or channel_id = $1) and bot_token = $2
            ";
            sqlx::query_as(select)
                .bind(channel_id.as_u64().to_string())
                .bind(self.token.clone())
                .fetch_all(&*self.pool)
                .await
                .ok()
        };

        let flows = match flows {
            Some(vf) if vf.len() > 0 => serde_json::to_string(&vf).unwrap(),
            _ => return,
        };

        let client = get_client();
        _ = client
            .post(HOOK_URL.as_str())
            .json(&msg)
            .header("X-Discord-flows", flows)
            .send()
            .await;
    }
}
