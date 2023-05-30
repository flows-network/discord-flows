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
    if bot_token.starts_with(DEFAULT_BOT_PLACEHOLDER) {
        if let Some(gid) = bot_token.strip_prefix(DEFAULT_BOT_PLACEHOLDER) {
            filter::delete_gid(gid, &state.pool).await;
            Ok(StatusCode::OK)
        } else {
            Ok(StatusCode::FORBIDDEN)
        }
    } else {
        safe_shutdown(&bot_token, &state.pool).await;
        del_listener_by_token(&flow_id, &flows_user, &bot_token, &state.pool).await
    }
}

mod filter {
    use sqlx::PgPool;

    pub async fn delete_gid(gid: &str, pool: &PgPool) {
        let delete = "
            DELETE FROM filter
            WHERE guild_id = $1
        ";
        _ = sqlx::query(delete).bind(gid).execute(pool).await;
    }
}
