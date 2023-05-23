use axum::extract::{Path, Query, State};
use reqwest::StatusCode;

use crate::{
    model::{Flow, ListenerQuery},
    state::AppState,
    utils::database::{del_listener_by_token, safe_shutdown},
};

pub async fn revoke(
    Path(Flow {
        flows_user,
        flow_id,
    }): Path<Flow>,
    State(state): State<AppState>,
    Query(ListenerQuery { bot_token }): Query<ListenerQuery>,
) -> Result<StatusCode, String> {
    if bot_token == "DEFAULT_BOT" {
        return del_listener_by_token(&flow_id, &flows_user, &bot_token, &state.pool).await;
    }

    safe_shutdown(&bot_token, &state.pool).await;
    del_listener_by_token(&flow_id, &flows_user, &bot_token, &state.pool).await
}
