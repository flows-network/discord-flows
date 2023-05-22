use crate::route::{connected, event, listen, revoke, static_path};

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use include_dir::{include_dir, Dir};
use serenity::model::gateway::GatewayIntents;
use sqlx::{Executor, PgPool};
use state::AppState;

mod handler;
mod model;
mod route;
mod shared;
mod state;
mod utils;

const HOOK_URL: &str = "https://code.flows.network/hook/discord/message";
static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

#[tokio::main]
async fn main() {
    let state = init().await;

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

async fn init() -> AppState {
    #[cfg(feature = "debug")]
    env_logger::init();

    let db_url = std::env::var("DATABASE_URL").unwrap();
    let pool = Arc::new(PgPool::connect(&db_url).await.unwrap());

    _ = pool.execute(include_str!("../schema.sql")).await.unwrap();

    let state = AppState { pool };

    let state_cloned = state.clone();
    tokio::spawn(async move {
        state_cloned.listen_ws().await;
    });

    state
}
