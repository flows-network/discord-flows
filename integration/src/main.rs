use std::sync::Arc;

use axum::{
    body::{self, Empty, Full},
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use common::{check_token, del_and_shutdown, is_token_dangling, Bot, Flow, ListenerQuery};
use include_dir::{include_dir, Dir};
use reqwest::header;
use serde_json::Value;
use serenity::model::gateway::GatewayIntents;
use sqlx::{Executor, PgPool};
use state::AppState;

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

    let sql = "
        INSERT INTO listener(flow_id, flows_user, bot_token)
        VALUES ($1, $2, $3)
        ON CONFLICT (flow_id, flows_user)
        DO UPDATE SET bot_token = excluded.bot_token
    ";
    let result = sqlx::query(sql)
        .bind(&flow_id)
        .bind(&flows_user)
        .bind(bot_token.clone())
        .execute(&*state.pool)
        .await
        .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        // DO NOTHING
        return Ok(StatusCode::OK);
    }

    if is_token_dangling(&bot_token, &state.pool).await? {
        _ = del_and_shutdown(&flow_id, &flows_user, &bot_token, &state.pool).await;
    }

    tokio::spawn(async move {
        let cloned = state.pool.clone();
        _ = state
            .start_client(bot_token.clone(), |start| async move {
                if !start {
                    _ = del_and_shutdown(&flow_id, &flows_user, &bot_token, &cloned).await;
                }
            })
            .await;
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
    del_and_shutdown(&flow_id, &flows_user, &bot_token, &state.pool).await
}

async fn event(
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
        .route("/event/:token", get(event))
        .route("/connected/:flows_user", get(connected))
        .route("/static/*path", get(static_path))
        .with_state(state);

    axum::Server::bind(&"0.0.0.0:6870".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
