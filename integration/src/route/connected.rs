use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::Value;

use crate::{model::GuildAuthor, state::AppState};

pub async fn connected(
    Path(flows_user): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, String> {
    let mut results = Vec::new();

    let sql = "SELECT * FROM guild_author WHERE flows_user = $1";
    let bots = sqlx::query_as(sql)
        .bind(flows_user)
        .fetch_all(&*state.pool)
        .await
        .map_err(|e| e.to_string())?;

    for GuildAuthor {
        flows_user: _,
        discord_guild_name,
    } in bots
    {
        results.push(serde_json::json!({
            "name": discord_guild_name,
        }));
    }

    Ok(Json(serde_json::json!({
        "title": "Connected Servers",
        "list": results,
    })))
}
