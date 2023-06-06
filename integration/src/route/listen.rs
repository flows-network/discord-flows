use axum::extract::{Path, Query, State};
use reqwest::StatusCode;

use crate::{
    model::{ListenPath, ListenerQuery},
    state::AppState,
    utils::{
        database::{del_listener_by_token, safe_shutdown},
        http::check_token,
    },
    DEFAULT_BOT_PLACEHOLDER,
};

const NONE_CHANNEL_ID: &'static str = "0";

pub async fn listen(
    Path(ListenPath {
        flows_user,
        flow_id,
        channel_id,
    }): Path<ListenPath>,
    State(state): State<AppState>,
    Query(ListenerQuery { bot_token }): Query<ListenerQuery>,
) -> Result<StatusCode, (StatusCode, String)> {
    let pool = &state.pool;

    if bot_token == DEFAULT_BOT_PLACEHOLDER {
        match channel_id != NONE_CHANNEL_ID {
            true => {
                listener::insert_listener(
                    &flow_id,
                    &flows_user,
                    &channel_id,
                    DEFAULT_BOT_PLACEHOLDER,
                    pool,
                )
                .await?;
                return Ok(StatusCode::OK);
            }
            false => {
                return Err((StatusCode::BAD_REQUEST, String::from("Bad request")));
            }
        }
    }

    let channel_id = match channel_id == NONE_CHANNEL_ID {
        true => String::new(),
        false => channel_id,
    };

    if !check_token(&bot_token).await {
        return Err((StatusCode::FORBIDDEN, "Unauthorized token".to_string()));
    }

    let old = listener::select_old(&flow_id, &flows_user, &channel_id, pool).await;
    if old.is_some() {
        if old.as_ref().unwrap().token == bot_token {
            return Ok(StatusCode::OK);
        }
    }

    listener::insert_listener(&flow_id, &flows_user, &channel_id, &bot_token, pool).await?;

    if old.is_some() {
        safe_shutdown(&old.as_ref().unwrap().token, pool).await;
    }

    tokio::spawn(async move {
        let cloned = state.pool.clone();
        _ = state
            .start_client(bot_token.clone(), |start| async move {
                if !start {
                    _ = del_listener_by_token(&flow_id, &flows_user, "", &bot_token, &cloned).await;
                    safe_shutdown(&bot_token, &cloned).await;
                }
            })
            .await;
    });

    Ok(StatusCode::OK)
}

mod listener {
    use reqwest::StatusCode;
    use sqlx::PgPool;

    use crate::model::Bot;

    pub async fn insert_listener(
        flow_id: &str,
        flows_user: &str,
        channel_id: &str,
        bot_token: &str,
        pool: &PgPool,
    ) -> Result<(), (StatusCode, String)> {
        let insert = "
            INSERT INTO listener(flow_id, flows_user, channel_id, bot_token)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (flow_id, flows_user)
            DO UPDATE SET bot_token = excluded.bot_token, channel_id = excluded.channel_id
        ";
        _ = sqlx::query(insert)
            .bind(flow_id)
            .bind(flows_user)
            .bind(channel_id)
            .bind(bot_token)
            .execute(pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(())
    }

    pub async fn select_old(
        flow_id: &str,
        flows_user: &str,
        channel_id: &str,
        pool: &PgPool,
    ) -> Option<Bot> {
        // select old token
        let select = "
        SELECT bot_token
        FROM listener
        WHERE flow_id = $1 AND flows_user = $2 AND channel_id = $3
    ";
        sqlx::query_as(select)
            .bind(flow_id)
            .bind(flows_user)
            .bind(channel_id)
            .fetch_optional(pool)
            .await
            .ok()?
    }
}
