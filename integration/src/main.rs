use std::sync::Arc;

use axum::{
    body::{self, Empty, Full},
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use common::{check_token, shard_map, Bot, Flow, ListenerQuery};
use include_dir::{include_dir, Dir};
use reqwest::header;
use serde_json::Value;
use serenity::model::gateway::GatewayIntents;
use sqlx::{Executor, PgPool};
use state::AppState;
use uuid::Uuid;

mod common;
mod handler;
mod state;

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

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

    let mut guard = shard_map().lock().await;
    let v = guard.remove(&bot_token);

    if let Some(shard_manager) = v {
        shard_manager.lock().await.shutdown_all().await;
    }
    drop(guard);

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

    // TODO: replace token with bot name
    let display = token.drain(..7).collect::<String>() + "...";
    results.push(serde_json::json!({
        "name": display,
    }));

    Ok(Json(serde_json::json!({
        "title": "Connected Bots",
        "list": results,
    })))
}

async fn static_path(Path(path): Path<String>) -> impl IntoResponse {
    let path = path.trim_start_matches('/');
    let mime_type = mime_guess::from_path(path).first_or_text_plain();

    match STATIC_DIR.get_file(path) {
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body::boxed(Empty::new()))
            .unwrap(),
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref()).unwrap(),
            )
            .body(body::boxed(Full::from(file.contents())))
            .unwrap(),
    }
}

#[tokio::main]
async fn main() {
    #[cfg(feature = "debug")]
    env_logger::init();

    let db_url = env!("DATABASE_URL");
    let pool = Arc::new(PgPool::connect(db_url).await.unwrap());

    _ = pool.execute(include_str!("../schema.sql")).await.unwrap();

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
        .route("/static/*path", get(static_path))
        .with_state(state);

    axum::Server::bind(&"0.0.0.0:6870".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
