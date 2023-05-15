use lru::LruCache;
use once_cell::sync::OnceCell;
use reqwest::Client;
use serenity::client::bridge::gateway::ShardManager;
use std::{collections::HashMap, num::NonZeroUsize, sync::Arc};
use tokio::sync::Mutex;
use upstash_redis_rs::{Command, ReResponse, Redis};

pub fn shard_map() -> &'static Mutex<HashMap<String, Arc<Mutex<ShardManager>>>> {
    static INSTANCE: OnceCell<Mutex<HashMap<String, Arc<Mutex<ShardManager>>>>> = OnceCell::new();
    INSTANCE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn get_client() -> &'static Client {
    static INS: OnceCell<Client> = OnceCell::new();
    INS.get_or_init(Client::new)
}

pub type Cache = LruCache<String, Vec<String>>;

pub fn get_cache() -> &'static Mutex<Cache> {
    static INS: OnceCell<Mutex<Cache>> = OnceCell::new();
    INS.get_or_init(|| {
        let cache = LruCache::new(NonZeroUsize::new(30).unwrap());
        Mutex::new(cache)
    })
}

pub async fn check_token(token: &str) -> bool {
    let url = "https://discord.com/api/users/@me";

    let client = get_client();
    let resp = client
        .get(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bot {token}"))
        .send()
        .await;

    if let Ok(r) = resp {
        if r.status().is_success() {
            return true;
        }
    }

    false
}

pub async fn batch_del(
    redis: &Redis,
    token: &str,
    flow_id: &str,
    flows_user: &str,
) -> Result<(), String> {
    let uuid = redis
        .smembers(format!("discord:{}:handle", token))
        .unwrap()
        .send()
        .await
        .unwrap();
    match uuid {
        ReResponse::Success { result } => {
            for uuid in result {
                redis
                    .hdel(format!("discord:{uuid}:event"), flow_id)
                    .unwrap()
                    .send()
                    .await
                    .unwrap();
            }
        }
        ReResponse::Error { error } => return Err(error),
    }
    redis
        .hdel("discord:listen", format!("{flows_user}:{flow_id}"))
        .unwrap()
        .send()
        .await
        .unwrap();

    Ok(())
}
