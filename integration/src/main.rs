use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use common::{check_token, clients_map};
use handler::Handler;
use serde::Deserialize;
use serde_json::Value;
use serenity::{model::gateway::GatewayIntents, Client};
use shuttle_runtime::CustomError;
use sqlx::{Executor, PgPool};
use uuid::Uuid;

mod common;
mod handler;

#[derive(Deserialize, sqlx::FromRow)]
struct Flow {
    flows_user: String,
    flow_id: String,
}

#[derive(sqlx::FromRow)]
struct Bot {
    #[sqlx(rename = "bot_token")]
    token: String,
}

#[derive(Deserialize)]
struct ListenerQuery {
    bot_token: String,
}

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

#[derive(Clone)]
struct AppState {
    pool: Arc<PgPool>,
}

impl AppState {
    async fn start_client(&self, token: String) -> serenity::Result<()> {
        let intents = GatewayIntents::all();
        let mut client = Client::builder(token.clone(), intents)
            .event_handler(Handler {
                token: token.clone(),
                pool: self.pool.clone(),
            })
            .await
            .unwrap();

        client.start().await?;

        let mut guard = clients_map().lock().await;
        guard.insert(token, client);

        Ok(())
    }

    async fn listen_ws(&self) {
        let sql = "SELECT bot_token FROM listener";
        let bots: Vec<Bot> = sqlx::query_as(sql)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| e.to_string())
            .unwrap();
        for Bot { token } in bots {
            _ = self.start_client(token).await;
        }
    }
}

#[shuttle_runtime::main]
async fn axum(#[shuttle_shared_db::Postgres] pool: PgPool) -> shuttle_axum::ShuttleAxum {
    let pool = Arc::new(pool);

    pool.execute(include_str!("../schema.sql"))
        .await
        .map_err(CustomError::new)?;

    let state = AppState { pool };

    let state_cloned = state.clone();
    tokio::spawn(async move {
        state_cloned.listen_ws().await;
    });

    let router = Router::new()
        .route("/:flows_user/:flow_id/listen", post(listen))
        .route("/:flows_user/:flow_id/revoke", post(revoke))
        .route("/event/:uuid", get(event))
        .route("/connected/:flows_user", get(connected))
        .with_state(state);

    Ok(router.into())
}
