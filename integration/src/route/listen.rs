use axum::extract::{Path, Query, State};
use reqwest::StatusCode;

use crate::{
    model::{Bot, Flow, ListenerQuery},
    state::AppState,
    utils::{
        database::{del_raw_by_token, safe_shutdown},
        http::check_token,
    },
};

pub async fn listen(
    Path(Flow {
        flows_user,
        flow_id,
    }): Path<Flow>,
    State(state): State<AppState>,
    Query(ListenerQuery { bot_token }): Query<ListenerQuery>,
) -> Result<StatusCode, String> {
    if !check_token(&bot_token).await {
        return Err("Unauthorized token".to_string());
    }

    // select old token
    let select = "
        SELECT bot_token
        FROM listener
        WHERE flow_id = $1 AND flows_user = $2
    ";
    let bot: Option<Bot> = sqlx::query_as(select)
        .bind(&flow_id)
        .bind(&flows_user)
        .bind(bot_token.clone())
        .fetch_optional(&*state.pool)
        .await
        .map_err(|e| e.to_string())?;

    if let Some(bt) = bot {
        if bt.token == bot_token {
            return Ok(StatusCode::OK);
        }

        safe_shutdown(&bt.token, &state.pool).await;
    }

    let insert = "
        INSERT INTO listener(flow_id, flows_user, bot_token)
        VALUES ($1, $2, $3)
        ON CONFLICT (flow_id, flows_user)
        DO UPDATE SET bot_token = excluded.bot_token
    ";
    _ = sqlx::query(insert)
        .bind(&flow_id)
        .bind(&flows_user)
        .bind(bot_token.clone())
        .execute(&*state.pool)
        .await
        .map_err(|e| e.to_string())?;

    tokio::spawn(async move {
        let cloned = state.pool.clone();
        _ = state
            .start_client(bot_token.clone(), |start| async move {
                if !start {
                    _ = del_raw_by_token(&flow_id, &flows_user, &bot_token, &cloned).await;
                    safe_shutdown(&bot_token, &cloned).await;
                }
            })
            .await;
    });

    Ok(StatusCode::OK)
}
