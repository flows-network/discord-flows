use discord_flows::http::HttpBuilder;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let client = HttpBuilder::new("DEFAULT_BOT").build();

    let me = client.get_current_user().await;

    println!("{:#?}", me);
}
