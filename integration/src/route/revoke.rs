use axum::extract::{Path, Query, State};
use reqwest::StatusCode;

use crate::{
    model::{Flow, ListenerQuery},
    state::AppState,
    utils::database::{del_listener_by_token, safe_shutdown},
    DEFAULT_BOT_PLACEHOLDER,
};

pub async fn revoke(
    Path(Flow {
        flows_user,
        flow_id,
    }): Path<Flow>,
    State(state): State<AppState>,
    Query(ListenerQuery { bot_token }): Query<ListenerQuery>,
) -> Result<StatusCode, String> {
    match bot_token.split_once(DEFAULT_BOT_PLACEHOLDER) {
        Some((gid, cid)) => {
            filter::delete_gcid(&state.pool, gid, cid, &flow_id).await;
            Ok(StatusCode::OK)
        }
        _ => {
            safe_shutdown(&bot_token, &state.pool).await;
            del_listener_by_token(&flow_id, &flows_user, &bot_token, &state.pool).await
        }
    }
}

mod filter {
    use sqlx::PgPool;

    pub async fn delete_gcid(pool: &PgPool, gid: &str, cid: &str, flow_id: &str) -> bool {
        let delete = "
            DELETE FROM filter
            WHERE guild_id = $1 AND channel_id = $2 AND flow_id = $3
        ";
        sqlx::query(delete)
            .bind(gid)
            .bind(cid)
            .bind(flow_id)
            .execute(pool)
            .await
            .is_ok_and(|rq| rq.rows_affected() != 0)
    }
}
