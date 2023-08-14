use axum::extract::{Path, Query, State};
use reqwest::StatusCode;
use sqlx::{PgPool, Postgres};

use crate::{
    model::{DiscordChannel, GuildAuthor, ListenPath, ListenerQuery},
    shared::get_client,
    state::AppState,
    utils::{
        database::{del_listener_by_token, safe_shutdown},
        http::check_token,
    },
    DEFAULT_BOT_PLACEHOLDER, DEFAULT_TOKEN,
};

const NONE_CHANNEL_ID: &'static str = "0";

pub async fn listen(
    Path(ListenPath {
        flows_user,
        flow_id,
        channel_id,
    }): Path<ListenPath>,
    State(state): State<AppState>,
    Query(ListenerQuery { bot_token }): Query<ListenerQuery>,
) -> Result<StatusCode, (StatusCode, String)> {
    let pool = &state.pool;

    if bot_token == DEFAULT_BOT_PLACEHOLDER {
        match channel_id != NONE_CHANNEL_ID {
            true => match authorized_channel(&flows_user, &channel_id, pool).await? {
                true => {
                    listener::insert_listener(
                        &flow_id,
                        &flows_user,
                        &channel_id,
                        DEFAULT_BOT_PLACEHOLDER,
                        pool,
                    )
                    .await?;
                    return Ok(StatusCode::OK);
                }
                false => {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        String::from("Not authorized channel"),
                    ));
                }
            },
            false => {
                return Err((StatusCode::BAD_REQUEST, String::from("Bad request")));
            }
        }
    }

    let channel_id = match channel_id == NONE_CHANNEL_ID {
        true => String::new(),
        false => channel_id,
    };

    if !check_token(&bot_token).await {
        return Err((StatusCode::FORBIDDEN, "Unauthorized token".to_string()));
    }

    let old = listener::select_old(&flow_id, &flows_user, &channel_id, pool).await;
    if old.is_some() {
        if old.as_ref().unwrap().token == bot_token {
            return Ok(StatusCode::OK);
        }
    }

    listener::insert_listener(&flow_id, &flows_user, &channel_id, &bot_token, pool).await?;

    if old.is_some() {
        safe_shutdown(&old.as_ref().unwrap().token, pool).await;
    }

    tokio::spawn(async move {
        let cloned = state.pool.clone();
        _ = state
            .start_client(bot_token.clone(), |start| async move {
                if !start {
                    _ = del_listener_by_token(&flow_id, &flows_user, "", &bot_token, &cloned).await;
                    safe_shutdown(&bot_token, &cloned).await;
                }
            })
            .await;
    });

    Ok(StatusCode::OK)
}

async fn authorized_channel(
    flows_user: &str,
    channel_id: &str,
    pool: &PgPool,
) -> Result<bool, (StatusCode, String)> {
    let channel = get_channel(&channel_id).await?;
    let (sql, id) = match channel.guild_id {
        Some(gid) => (
            "SELECT * FROM guild_author
            WHERE flows_user = $1 AND discord_guild_id = $2
            ",
            gid,
        ),
        None => match channel.owner_id {
            Some(oid) => (
                "SELECT * FROM guild_author
                WHERE flows_user = $1 AND discord_user_id = $2
                ",
                oid,
            ),
            None => {
                return Ok(false);
            }
        },
    };

    Ok(sqlx::query_as::<Postgres, GuildAuthor>(sql)
        .bind(flows_user)
        .bind(id)
        .fetch_optional(pool)
        .await
        .unwrap_or_default()
        .is_some())
}

async fn get_channel(channel_id: &str) -> Result<DiscordChannel, (StatusCode, String)> {
    let url = format!("https://discord.com/api/channels/{}", channel_id);

    let client = get_client();
    let resp = client
        .get(url)
        .header("Authorization", &format!("Bot {}", &*DEFAULT_TOKEN))
        .send()
        .await;

    match resp {
        Ok(r) => match r.status().is_success() {
            true => r
                .json::<DiscordChannel>()
                .await
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string())),
            false => Err((r.status(), r.text().await.unwrap_or_else(|e| e.to_string()))),
        },
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

mod listener {
    use reqwest::StatusCode;
    use sqlx::PgPool;

    use crate::model::Bot;

    pub async fn insert_listener(
        flow_id: &str,
        flows_user: &str,
        channel_id: &str,
        bot_token: &str,
        pool: &PgPool,
    ) -> Result<(), (StatusCode, String)> {
        let insert = "
            INSERT INTO listener(flow_id, flows_user, channel_id, bot_token)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (flow_id, flows_user)
            DO UPDATE SET bot_token = excluded.bot_token, channel_id = excluded.channel_id
        ";
        _ = sqlx::query(insert)
            .bind(flow_id)
            .bind(flows_user)
            .bind(channel_id)
            .bind(bot_token)
            .execute(pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(())
    }

    pub async fn select_old(
        flow_id: &str,
        flows_user: &str,
        channel_id: &str,
        pool: &PgPool,
    ) -> Option<Bot> {
        // select old token
        let select = "
        SELECT bot_token
        FROM listener
        WHERE flow_id = $1 AND flows_user = $2 AND channel_id = $3
    ";
        sqlx::query_as(select)
            .bind(flow_id)
            .bind(flows_user)
            .bind(channel_id)
            .fetch_optional(pool)
            .await
            .ok()?
    }
}
