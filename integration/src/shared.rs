use std::{collections::HashMap, sync::Arc};

use once_cell::sync::OnceCell;
use reqwest::Client;
use serenity::client::bridge::gateway::ShardManager;
use tokio::sync::Mutex;

type ShardMap = Mutex<HashMap<String, Arc<Mutex<ShardManager>>>>;

pub fn shard_map() -> &'static ShardMap {
    static INSTANCE: OnceCell<ShardMap> = OnceCell::new();
    INSTANCE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn get_client() -> &'static Client {
    static INS: OnceCell<Client> = OnceCell::new();
    INS.get_or_init(Client::new)
}
