use axum::{
    extract::{Query, State},
    response::Redirect,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

use crate::{
    model::{AuthQuery, AuthStateClaim, AuthTokenRequest, AuthTokenResponse, UserResponse},
    shared::get_client,
    state::AppState,
};

pub async fn auth(
    State(global_state): State<AppState>,
    Query(AuthQuery { state, code }): Query<AuthQuery>,
) -> Result<Redirect, String> {
    let mut val = Validation::new(Algorithm::RS256);
    // Set the time skew
    val.leeway = 60;
    let public_key = std::env::var("FLOWS_JWT_PUBLIC_KEY").unwrap();

    // Decode the flows_user from jwt
    let state_claim = match decode::<AuthStateClaim>(
        &state,
        &DecodingKey::from_rsa_pem(public_key.as_bytes()).unwrap(),
        &val,
    ) {
        Ok(c) => c,
        Err(e) => {
            return Err(e.to_string());
        }
    };

    let token_resp = auth_token(code).await?;

    let user_resp = get_current_user(token_resp.access_token).await?;

    let pool = &global_state.pool;
    let insert = "
        INSERT INTO guild_author(flows_user, discord_guild_id, discord_guild_name, discord_user_id, discord_username, discord_email)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (flows_user, discord_guild_id, discord_user_id)
        DO UPDATE SET discord_guild_name = excluded.discord_guild_name,
        discord_username = excluded.discord_username,
        discord_email = excluded.discord_email
    ";
    _ = sqlx::query(insert)
        .bind(state_claim.claims.flows_user)
        .bind(token_resp.guild.id)
        .bind(token_resp.guild.name)
        .bind(user_resp.id)
        .bind(user_resp.username)
        .bind(user_resp.email)
        .execute(pool.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(Redirect::temporary(
        "https://flows.network/integration/Discord",
    ))
}

async fn get_current_user(access_token: String) -> Result<UserResponse, String> {
    let url = "https://discord.com/api/users/@me";

    let client = get_client();
    let resp = client
        .get(url)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await;

    match resp {
        Ok(r) => match r.status().is_success() {
            true => r.json::<UserResponse>().await.map_err(|e| e.to_string()),
            false => Err(r.text().await.unwrap_or_else(|e| e.to_string())),
        },
        Err(e) => Err(e.to_string()),
    }
}

async fn auth_token(code: String) -> Result<AuthTokenResponse, String> {
    let client_id = std::env::var("DEFAULT_DISCORD_APP_CLIENT_ID").unwrap();
    let client_secret = std::env::var("DEFAULT_DISCORD_APP_CLIENT_SECRET").unwrap();
    let redirect_uri = std::env::var("DEFAULT_DISCORD_APP_AUTH_REDIRECT_URI").unwrap();

    let url = "https://discord.com/api/oauth2/token";

    let body = AuthTokenRequest {
        client_id,
        client_secret,
        grant_type: String::from("authorization_code"),
        code,
        redirect_uri,
    };

    let client = get_client();
    let resp = client
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(serde_urlencoded::to_string(&body).unwrap())
        .send()
        .await;

    match resp {
        Ok(r) => match r.status().is_success() {
            true => r
                .json::<AuthTokenResponse>()
                .await
                .map_err(|e| e.to_string()),
            false => Err(r.text().await.unwrap_or_else(|e| e.to_string())),
        },
        Err(e) => Err(e.to_string()),
    }
}
