use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::Value;

use crate::{model::Flow, state::AppState};

pub async fn event(
    Path(token): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, String> {
    let mut flows = Vec::new();

    let sql = "
        SELECT flows_user, flow_id FROM listener
        WHERE bot_token = $1
    ";
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
