use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use itertools::Itertools;
use serde::Deserialize;
use serde_json::Value;
use serenity::model::gateway::GatewayIntents;
use upstash_redis_rs::{Command, ReResponse, Redis};
use uuid::Uuid;

use common::{check_token, shard_map};
use state::AppState;

mod common;
mod handler;
mod state;

#[derive(Deserialize)]
struct Flow {
    flows_user: String,
    flow_id: String,
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

    let res = state
        .redis
        .hset(
            format!("discord:listen"),
            format!("{flows_user}:{flow_id}"),
            &bot_token,
        )
        .unwrap()
        .send()
        .await
        .unwrap();

    if let ReResponse::Success { result } = res {
        if result == 0 {
            // already listening
            return Ok(StatusCode::OK);
        }
    }

    state
        .redis
        .sadd(format!("discord:{}:connected", flows_user), &bot_token)
        .unwrap()
        .send()
        .await
        .unwrap();
    state
        .redis
        .sadd(format!("discord:{}:handle", bot_token), &uuid)
        .unwrap()
        .send()
        .await
        .unwrap();
    state
        .redis
        .hset(format!("discord:{}:event", &uuid), flow_id, flows_user)
        .unwrap()
        .send()
        .await
        .unwrap();

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
    let uuid = state
        .redis
        .smembers(format!("discord:{}:handle", bot_token))
        .unwrap()
        .send()
        .await
        .unwrap();
    match uuid {
        ReResponse::Success { result } => {
            for uuid in result {
                state
                    .redis
                    .hdel(format!("discord:{uuid}:event"), &flow_id)
                    .unwrap()
                    .send()
                    .await
                    .unwrap();
            }
        }
        ReResponse::Error { error } => return Err(error),
    }
    state
        .redis
        .hdel("discord:listen", format!("{flows_user}:{flow_id}"))
        .unwrap()
        .send()
        .await
        .unwrap();

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

    let fs = state
        .redis
        .hgetall(format!("discord:{uuid}:event"))
        .unwrap()
        .send()
        .await
        .unwrap();

    if let ReResponse::Success { result } = fs {
        for mut flow in &result.into_iter().chunks(2) {
            let flow_id = flow.next().unwrap();
            let flows_user = flow.next().unwrap();

            flows.push(serde_json::json!({
                "flows_user": flows_user,
                "flow_id": flow_id,
            }));
        }
    }

    Ok(Json(flows))
}

async fn connected(
    Path(flows_user): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Value>, String> {
    let mut results = Vec::new();

    let token = state
        .redis
        .smembers(format!("discord:{flows_user}:connected"))
        .unwrap()
        .send()
        .await
        .unwrap();

    if let ReResponse::Success { result } = token {
        for token in result {
            let display = token.clone().drain(..7).collect::<String>() + "...";
            results.push(serde_json::json!({
                "name": display,
            }));
        }
    }

    Ok(Json(serde_json::json!({
        "title": "Connected Bots",
        "list": results,
    })))
}

#[tokio::main]
async fn main() {
    #[cfg(feature = "debug")]
    env_logger::init();

    let url = env!("UPSTASH_REDIS_REST_URL");
    let token = env!("UPSTASH_REDIS_REST_TOKEN");
    let redis = Redis::new(url, token).unwrap();

    let state = AppState { redis };

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

    axum::Server::bind(&"0.0.0.0:6780".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
