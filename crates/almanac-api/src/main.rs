use almanac_api::{run_server, AppConfig};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "almanac_api=info,tower_http=debug".into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration from environment
    let config = AppConfig::from_env()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    tracing::info!("Starting Almanac API server on {}", addr);

    // Run the server
    run_server(config).await
}
