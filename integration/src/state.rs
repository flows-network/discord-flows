use std::future::Future;

use crate::{common::shard_map, handler::Handler, GatewayIntents};
use itertools::Itertools;
use serenity::Client;
use upstash_redis_rs::{Command, ReResponse, Redis};

#[derive(Clone)]
pub struct AppState {
    pub redis: Redis,
}

impl AppState {
    pub async fn start_client<F, Fut>(&self, token: &str, cb: F) -> serenity::Result<()>
    where
        F: FnOnce(bool) -> Fut + std::marker::Send + 'static,
        Fut: Future<Output = ()> + std::marker::Send,
    {
        let mut guard = shard_map().lock().await;
        let shard = guard.get(token);
        if shard.is_some() {
            return Ok(());
        }

        let intents = GatewayIntents::all();
        let mut client = Client::builder(token.clone(), intents)
            .event_handler(Handler {
                token: token.to_string(),
                redis: self.redis.clone(),
            })
            .await
            .unwrap();

        let shard_manager = client.shard_manager.clone();

        tokio::spawn(async move { cb(client.start().await.is_ok()).await });

        guard.insert(token.to_string(), shard_manager);
        drop(guard);

        Ok(())
    }

    pub async fn listen_ws(&self) {
        let tokens = self
            .redis
            .hgetall("discord:listen")
            .unwrap()
            .send()
            .await
            .unwrap();

        if let ReResponse::Success { result } = tokens {
            for token in result.iter().skip(1).step_by(2).unique() {
                _ = self.start_client(token, |_| async {}).await;
            }
        }
    }
}
