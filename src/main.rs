mod config;
mod error;
mod models;
mod routes;
mod state;

use axum::Router;
use config::Config;
use mongodb::options::ClientOptions;
use mongodb::Client;
use tracing_subscriber::EnvFilter;

use crate::state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;

    let env_filter = config
        .log_level
        .clone()
        .map(|level| format!("{level}"))
        .unwrap_or_else(|| "info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_new(env_filter).unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    tracing::info!("starting api gateway");

    let mut client_options = ClientOptions::parse(&config.mongodb_uri).await?;
    client_options.app_name = Some("hello_rust_gateway".to_string());
    if let Some(min_pool_size) = config.pool_min_size {
        client_options.min_pool_size = Some(min_pool_size);
    }
    if let Some(max_pool_size) = config.pool_max_size {
        client_options.max_pool_size = Some(max_pool_size);
    }
    if let Some(timeout) = config.connect_timeout {
        client_options.connect_timeout = Some(timeout);
    }
    if let Some(timeout) = config.server_selection_timeout {
        client_options.server_selection_timeout = Some(timeout);
    }

    let client = Client::with_options(client_options)?;
    let state = AppState::new(client, &config);

    let app: Router = routes::router(state);

    let listener = tokio::net::TcpListener::bind(&config.bind_address).await?;
    tracing::info!("listening on {}", config.bind_address);
    axum::serve(listener, app).await?;
    Ok(())
}
