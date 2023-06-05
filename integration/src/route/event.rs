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

    let query = match token.strip_prefix(DEFAULT_BOT_PLACEHOLDER) {
        Some(flow_id) => {
            let sql = "
                SELECT
                    flows_user,
                    flow_id
                FROM
                    listener
                WHERE bot_token = $1 AND flow_id = $2
        ";
            sqlx::query_as(sql)
                .bind(DEFAULT_BOT_PLACEHOLDER)
                .bind(flow_id)
        }
        None => {
            let sql = "
                SELECT flows_user, flow_id
                FROM listener
                WHERE bot_token = $1
            ";
            sqlx::query_as(sql).bind(token)
        }
    };

    let fs: Vec<Flow> = query
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
