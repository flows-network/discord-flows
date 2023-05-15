use crate::{common::shard_map, handler::Handler, GatewayIntents};
use itertools::Itertools;
use serenity::Client;
use upstash_redis_rs::{Command, ReResponse, Redis};

#[derive(Clone)]
pub struct AppState {
    pub redis: Redis,
}

impl AppState {
    pub async fn start_client(&self, token: &str) -> serenity::Result<bool> {
        let mut guard = shard_map().lock().await;
        let shard = guard.get(token);
        if shard.is_some() {
            return Ok(true);
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

        let handle = tokio::spawn(async move { client.start().await.is_ok() });
        let start = handle.await.unwrap_or(false);

        println!("\n\n\nok");

        if !start {
            return Ok(false);
        }

        guard.insert(token.to_string(), shard_manager);
        drop(guard);

        Ok(true)
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
                _ = self.start_client(token).await;
            }
        }
    }
}
