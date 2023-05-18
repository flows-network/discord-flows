use std::sync::Arc;

use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::{Context, EventHandler};
use sqlx::PgPool;

use crate::common::{get_cache, get_client};

#[derive(sqlx::FromRow)]
struct Uuid {
    uuid: String,
}

impl Handler {
    async fn _query_uuid(&self) -> Option<String> {
        let sql = "
            SELECT uuid FROM listener WHERE bot_token = $1
        ";
        let Uuid { uuid } = sqlx::query_as(sql)
            .bind(self.token.clone())
            .fetch_one(&*self.pool)
            .await
            .ok()?;

        Some(uuid)
    }

    async fn query_uuid(&self) -> Option<String> {
        let mut cache = get_cache().lock().await;
        let v = cache.get(&self.token);
        match v {
            Some(s) => Some(s.to_string()),
            None => self._query_uuid().await,
        }
    }
}

pub struct Handler {
    pub token: String,
    pub pool: Arc<PgPool>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        let hook_url = "https://code.flows.network/hook/discord/message";
        let uuid = self.query_uuid().await;

        if let Some(uuid) = uuid {
            let client = get_client();
            _ = client
                .post(hook_url)
                .json(&msg)
                .header("X-Discord-uuid", uuid)
                .send()
                .await;
        }
    }
}
