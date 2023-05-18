use std::sync::Arc;

use crate::{
    common::{clients_map, Bot},
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
    pub async fn start_client(&self, token: String) -> serenity::Result<()> {
        let intents = GatewayIntents::all();
        let mut client = Client::builder(token.clone(), intents)
            .event_handler(Handler {
                token: token.clone(),
                pool: self.pool.clone(),
            })
            .await
            .unwrap();

        // TODO:
        client.start().await?;

        let mut guard = clients_map().lock().await;
        guard.insert(token, client);

        Ok(())
    }

    pub async fn listen_ws(&self) {
        let sql = "SELECT bot_token FROM listener";
        let bots: Vec<Bot> = sqlx::query_as(sql)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| e.to_string())
            .unwrap();
        for Bot { token } in bots {
            _ = self.start_client(token).await;
        }
    }
}
