use axum::{extract::Path, response::Redirect};

pub async fn access(Path(state): Path<String>) -> Redirect {
    let client_id = std::env::var("DEFAULT_DISCORD_APP_CLIENT_ID").unwrap();
    let scope = std::env::var("DEFAULT_DISCORD_APP_AUTH_SCOPE").unwrap();
    let permissions = std::env::var("DEFAULT_DISCORD_APP_AUTH_PERMISSIONS").unwrap();
    let redirect_uri = std::env::var("DEFAULT_DISCORD_APP_AUTH_REDIRECT_URI").unwrap();
    Redirect::permanent(&format!("https://discord.com/oauth2/authorize?client_id={client_id}&permissions={permissions}&scope={scope}&redirect_uri={redirect_uri}&response_type=code&state={state}"))
}
