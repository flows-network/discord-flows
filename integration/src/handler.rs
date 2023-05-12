use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::{Context, EventHandler};
use upstash_redis_rs::{Command, ReResponse, Redis};

use crate::common::{get_cache, get_client, Cache};

impl Handler {
    async fn _query_uuid(&self, cache: &mut Cache) -> Option<Vec<String>> {
        let uuid = self
            .redis
            .smembers(format!("discord:{}:handle", self.token))
            .unwrap()
            .send()
            .await
            .unwrap();

        if let ReResponse::Success { result: uuids } = uuid {
            cache.put(self.token.clone(), uuids)
        } else {
            None
        }
    }

    async fn query_uuid(&self) -> Option<Vec<String>> {
        let mut cache = get_cache().lock().await;
        let v = cache.get(&self.token);
        match v {
            Some(s) => Some(s.to_vec()),
            None => self._query_uuid(&mut *cache).await,
        }
    }
}

pub struct Handler {
    pub token: String,
    pub redis: Redis,
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        let hook_url = "https://code.flows.network/hook/discord/message";
        let uuids = self.query_uuid().await;

        if let Some(uuids) = uuids {
            for uuid in uuids {
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
}
