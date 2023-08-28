pub mod http {
    use crate::shared::get_client;

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
}

pub mod database {
    use reqwest::StatusCode;
    use sqlx::PgPool;

    use crate::{model::Count, shared::shard_map, DEFAULT_BOT_PLACEHOLDER};

    pub async fn del_listener_by_token(
        bot_token: &str,
        pool: &PgPool,
    ) -> Result<StatusCode, String> {
        let delete = "
            DELETE FROM listener
            WHERE bot_token = $1
        ";
        sqlx::query(delete)
            .bind(bot_token)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(StatusCode::OK)
    }

    pub async fn safe_shutdown(bot_token: &str, pool: &PgPool) {
        // Don't shutdown the default Bot
        if bot_token == DEFAULT_BOT_PLACEHOLDER {
            return;
        }
        if is_token_dangling(bot_token, pool).await.unwrap_or(false) {
            shutdown(bot_token).await;
        }
    }

    pub async fn is_token_dangling(bot_token: &str, pool: &PgPool) -> Result<bool, String> {
        let select: Count =
            sqlx::query_as("SELECT COUNT(bot_token) FROM listener WHERE bot_token = $1")
                .bind(bot_token)
                .fetch_one(pool)
                .await
                .map_err(|e| e.to_string())
                .unwrap();

        Ok(select.count == 0)
    }

    pub async fn shutdown(bot_token: &str) {
        let mut guard = shard_map().lock().await;
        let v = guard.remove(bot_token);

        if let Some(shard_manager) = v {
            shard_manager.lock().await.shutdown_all().await;
        }
        drop(guard);
    }
}
