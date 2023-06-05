use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::Value;

use crate::{model::Flow, state::AppState, DEFAULT_BOT_PLACEHOLDER};

pub async fn event(
    Path(token): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, String> {
    let mut flows = Vec::new();

    let sql = if token == DEFAULT_BOT_PLACEHOLDER {
        "
        WITH filtering AS (
            SELECT
                flow_id
            FROM
                filter
        )

        SELECT
            listener.flows_user,
            listener.flow_id
        FROM
            listener
        INNER JOIN filtering
        ON listener.flow_id = filtering.flow_id
        WHERE bot_token = $1
        "
    } else {
        "
        SELECT flows_user, flow_id
        FROM listener
        WHERE bot_token = $1
        "
    };

    let fs: Vec<Flow> = sqlx::query_as(sql)
        .bind(token)
        .fetch_all(&*state.pool)
        .await
        .map_err(|e| e.to_string())?;

    for Flow {
        flows_user,
        flow_id,
    } in fs
    {
        flows.push(serde_json::json!({
            "flows_user": flows_user,
            "flow_id": flow_id,
        }));
    }

    Ok(Json(flows))
}
