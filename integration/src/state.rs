use std::{future::Future, sync::Arc};

use crate::{
    common::{shard_map, Bot},
    handler::Handler,
    GatewayIntents,
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
        let mut client = Client::builder(token.clone(), intents)
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
        let sql = "SELECT DISTINCT bot_token FROM listener";
        let bots: Vec<Bot> = sqlx::query_as(sql)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| e.to_string())
            .unwrap();
        for Bot { token } in bots {
            _ = self.start_client(token, |_| async {}).await;
        }
    }
}
