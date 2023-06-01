use axum::extract::{Path, Query, State};
use reqwest::StatusCode;

use crate::{
    model::{Flow, ListenerQuery},
    state::AppState,
    utils::{
        database::{del_listener_by_token, safe_shutdown},
        http::check_token,
    },
    DEFAULT_BOT_PLACEHOLDER,
};

pub async fn listen(
    Path(Flow {
        flows_user,
        flow_id,
    }): Path<Flow>,
    State(state): State<AppState>,
    Query(ListenerQuery { bot_token }): Query<ListenerQuery>,
) -> Result<StatusCode, (StatusCode, String)> {
    match bot_token.split_once(DEFAULT_BOT_PLACEHOLDER) {
        Some((gid, cid)) if filter::insert_gcid(gid, cid, &state.pool).await => {
            listener::insert_listener(&flow_id, &flows_user, DEFAULT_BOT_PLACEHOLDER, &state.pool)
                .await?;
            return Ok(StatusCode::OK);
        }
        _ => (),
    }

    if !check_token(&bot_token).await {
        return Err((StatusCode::FORBIDDEN, "Unauthorized token".to_string()));
    }

    if let Some(bt) = listener::select_old(&flow_id, &flows_user, &bot_token, &state.pool).await {
        if bt.token == bot_token {
            return Ok(StatusCode::OK);
        }

        safe_shutdown(&bt.token, &state.pool).await;
    }

    listener::insert_listener(&flow_id, &flows_user, &bot_token, &state.pool).await?;

    tokio::spawn(async move {
        let cloned = state.pool.clone();
        _ = state
            .start_client(bot_token.clone(), |start| async move {
                if !start {
                    _ = del_listener_by_token(&flow_id, &flows_user, &bot_token, &cloned).await;
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
        bot_token: &str,
        pool: &PgPool,
    ) -> Result<(), (StatusCode, String)> {
        let insert = "
        INSERT INTO listener(flow_id, flows_user, bot_token)
        VALUES ($1, $2, $3)
        ON CONFLICT (flow_id, flows_user)
        DO UPDATE SET bot_token = excluded.bot_token
    ";
        _ = sqlx::query(insert)
            .bind(flow_id)
            .bind(flows_user)
            .bind(bot_token)
            .execute(pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(())
    }

    pub async fn select_old(
        flow_id: &str,
        flows_user: &str,
        bot_token: &str,
        pool: &PgPool,
    ) -> Option<Bot> {
        // select old token
        let select = "
        SELECT bot_token
        FROM listener
        WHERE flow_id = $1 AND flows_user = $2
    ";
        sqlx::query_as(select)
            .bind(flow_id)
            .bind(flows_user)
            .bind(bot_token)
            .fetch_optional(pool)
            .await
            .ok()?
    }
}

mod filter {
    use sqlx::PgPool;

    pub async fn insert_gcid(gid: &str, cid: &str, pool: &PgPool) -> bool {
        let insert = "
            INSERT INTO filter
            VALUES ($1, $2);
        ";
        sqlx::query(insert)
            .bind(gid)
            .bind(cid)
            .execute(pool)
            .await
            .is_ok_and(|rq| rq.rows_affected() != 0)
    }
}
