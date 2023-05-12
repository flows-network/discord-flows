use crate::{common::shard_map, handler::Handler, GatewayIntents};
use serenity::Client;
use upstash_redis_rs::{Command, ReResponse, Redis};

#[derive(Clone)]
pub struct AppState {
    pub redis: Redis,
}

impl AppState {
    pub async fn start_client(&self, token: String) -> serenity::Result<()> {
        let intents = GatewayIntents::all();
        let mut client = Client::builder(token.clone(), intents)
            .event_handler(Handler {
                token: token.clone(),
                redis: self.redis.clone(),
            })
            .await
            .unwrap();

        let shard_manager = client.shard_manager.clone();

        let mut guard = shard_map().lock().await;
        guard.insert(token, shard_manager);
        drop(guard);

        tokio::spawn(async move { client.start().await });

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
            for token in result.iter().skip(1).step_by(2) {
                _ = self.start_client(token.to_string()).await;
            }
        }
    }
}
