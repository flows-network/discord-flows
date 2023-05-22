use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::Value;

use crate::{model::Bot, state::AppState};

pub async fn connected(
    Path(flows_user): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, String> {
    let mut results = Vec::new();

    let sql = "SELECT bot_token FROM listener WHERE flows_user = $1";
    let bots = sqlx::query_as(sql)
        .bind(flows_user)
        .fetch_all(&*state.pool)
        .await
        .map_err(|e| e.to_string())?;

    for Bot { mut token } in bots {
        // TODO: replace token with bot name
        let display = token.drain(..7).collect::<String>() + "...";
        results.push(serde_json::json!({
            "name": display,
        }));
    }

    Ok(Json(serde_json::json!({
        "title": "Connected Bots",
        "list": results,
    })))
}
