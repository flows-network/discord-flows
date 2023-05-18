use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use common::{check_token, clients_map, pool, Bot, Flow, ListenerQuery};
use serde_json::Value;
use serenity::model::gateway::GatewayIntents;
use shuttle_runtime::CustomError;
use sqlx::Executor;
use state::AppState;
use uuid::Uuid;

mod common;
mod handler;
mod state;

async fn listen(
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

    let uuid = Uuid::new_v4().simple().to_string();

    let sql = "
        INSERT INTO listener(flow_id, flows_user, bot_token, uuid)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (flow_id, flows_user, bot_token) DO NOTHING
    ";
    sqlx::query(sql)
        .bind(flow_id)
        .bind(flows_user)
        .bind(bot_token.clone())
        .bind(uuid)
        .execute(&*state.pool)
        .await
        .map_err(|e| e.to_string())?;

    tokio::spawn(async move {
        _ = state.start_client(bot_token).await;
    });

    Ok(StatusCode::OK)
}

async fn revoke(
    Path(Flow {
        flows_user,
        flow_id,
    }): Path<Flow>,
    State(state): State<AppState>,
    Query(ListenerQuery { bot_token }): Query<ListenerQuery>,
) -> Result<StatusCode, String> {
    let sql = "
        DELETE FROM listener
        WHERE flow_id = $1 AND flows_user = $2 AND bot_token = $3
    ";
    sqlx::query(sql)
        .bind(flow_id)
        .bind(flows_user)
        .bind(&bot_token)
        .execute(&*state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut guard = clients_map().lock().await;
    let v = guard.remove(&bot_token);

    if let Some(client) = v {
        client.shard_manager.lock().await.shutdown_all().await;
    }

    Ok(StatusCode::OK)
}

async fn event(
    Path(uuid): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<Value>>, String> {
    let mut flows = Vec::new();

    let sql = "
        SELECT flows_user, flow_id FROM listener
        WHERE uuid = $1
    ";
    let fs: Vec<Flow> = sqlx::query_as(sql)
        .bind(uuid)
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

async fn connected(
    Path(flows_user): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, String> {
    let mut results = Vec::new();

    let sql = "SELECT bot_token FROM listener WHERE flows_user = $1";
    let Bot { mut token } = sqlx::query_as(sql)
        .bind(flows_user)
        .fetch_one(&*state.pool)
        .await
        .map_err(|e| e.to_string())?;

    let display = token.drain(..7).collect::<String>() + "...";
    results.push(serde_json::json!({
        "name": display,
    }));

    Ok(Json(serde_json::json!({
        "title": "Connected Bots",
        "list": results,
    })))
}

#[tokio::main]
async fn main() {
    let pool = pool();

    _ = pool
        .execute(include_str!("../schema.sql"))
        .await
        .map_err(CustomError::new);

    let state = AppState { pool };

    let state_cloned = state.clone();
    tokio::spawn(async move {
        state_cloned.listen_ws().await;
    });

    let app = Router::new()
        .route("/:flows_user/:flow_id/listen", post(listen))
        .route("/:flows_user/:flow_id/revoke", post(revoke))
        .route("/event/:uuid", get(event))
        .route("/connected/:flows_user", get(connected))
        .with_state(state);

    axum::Server::bind(&"0.0.0.0:6870".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
