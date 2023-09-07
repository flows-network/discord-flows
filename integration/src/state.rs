use std::{future::Future, sync::Arc};

use crate::{
    handler::Handler, model::Bot, shared::shard_map, GatewayIntents, DEFAULT_BOT_PLACEHOLDER,
    DEFAULT_TOKEN,
};
use serenity::Client;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: Arc<PgPool>,
}

impl AppState {
    pub async fn start_client<F, Fut>(&self, token: String, cb: F) -> serenity::Result<()>
    where
        F: FnOnce(bool) -> Fut + std::marker::Send + 'static,
        Fut: Future<Output = ()> + std::marker::Send,
    {
        let mut guard = shard_map().lock().await;
        let shard = guard.get(&token);
        if shard.is_some() {
            return Ok(());
        }

        let intents = GatewayIntents::all();
        let real_token = match token == DEFAULT_BOT_PLACEHOLDER {
            true => DEFAULT_TOKEN.as_str(),
            false => token.as_str(),
        };
        let mut client = Client::builder(real_token, intents)
            .event_handler(Handler {
                token: token.clone(),
                pool: self.pool.clone(),
            })
            .await
            .unwrap();

        let shard_manager = client.shard_manager.clone();

        guard.insert(token, shard_manager);
        drop(guard);

        tokio::spawn(async move {
            cb(client.start().await.is_ok()).await;
        });

        Ok(())
    }

    pub async fn listen_ws(&self) {
        self.start_default_client().await;
        self.start_user_client().await;
    }

    async fn start_default_client(&self) {
        _ = self
            .start_client(DEFAULT_BOT_PLACEHOLDER.to_string(), |_| async {})
            .await;
    }

    async fn start_user_client(&self) {
        let sql = "SELECT DISTINCT bot_token FROM listener";
        let bots: Vec<Bot> = sqlx::query_as(sql)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| e.to_string())
            .unwrap();

        for Bot { token } in bots {
            if token.ne(DEFAULT_BOT_PLACEHOLDER) {
                _ = self.start_client(token, |_| async {}).await;
            }
        }
    }
}
