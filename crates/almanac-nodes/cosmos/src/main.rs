//! Cosmos-specific Almanac indexing node

use almanac_chains::{ChainAdapter, ChainConfig, CosmosAdapter};
use almanac_consensus::{fork_choice::create_fork_choice, BlockInfo, ForkChoiceRule};
use almanac_indexing::pipeline::{IndexingPipeline, PipelineConfig};
use almanac_storage::MemoryStorage;
use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "almanac-cosmos")]
#[command(about = "Almanac indexer for Cosmos chains")]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config/cosmos.toml")]
    config: String,

    /// Node name for identification
    #[arg(long)]
    node_name: String,

    /// Metrics port
    #[arg(long, default_value = "9092")]
    metrics_port: u16,

    /// Start from specific block (overrides config)
    #[arg(long)]
    start_block: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .json()
        .init();

    let args = Args::parse();
    info!("Starting almanac-cosmos node: {}", args.node_name);

    // Load configuration
    let config_content = tokio::fs::read_to_string(&args.config).await?;
    let config: toml::Value = toml::from_str(&config_content)?;

    // Create chain configuration
    let chain_config = ChainConfig {
        chain_id: config["chain"]["id"]
            .as_str()
            .unwrap_or("cosmoshub-4")
            .to_string(),
        rpc_url: config["rpc"]["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing rpc.url in config"))?
            .to_string(),
        api_key: config["rpc"]["api_key"].as_str().map(|s| s.to_string()),
        confirmations: config["chain"]["confirmations"].as_integer().unwrap_or(2) as u32,
        max_retries: config["rpc"]["max_retries"].as_integer().unwrap_or(3) as u32,
        timeout_secs: config["rpc"]["timeout_secs"].as_integer().unwrap_or(30) as u64,
        extra: serde_json::Value::Null,
    };

    // Create Cosmos adapter
    let adapter = Arc::new(CosmosAdapter::new(chain_config.clone()).await?);
    info!("Connected to Cosmos RPC");

    // Connect to FoundationDB
    let _fdb_cluster = config["storage"]["fdb"]["cluster_file"]
        .as_str()
        .map(|s| s.to_string());
    let namespace = format!("almanac/{}", chain_config.chain_id);

    let storage = Arc::new(MemoryStorage::new());
    info!("Connected to FoundationDB namespace: {}", namespace);

    // Create consensus handler
    let fork_choice_rule = match config["consensus"]["fork_choice"].as_str() {
        Some("tendermint_bft") => ForkChoiceRule::TendermintBft,
        Some("casper_ffg") => ForkChoiceRule::CasperFfg,
        _ => ForkChoiceRule::TendermintBft,
    };

    let initial_block_number = adapter.get_latest_block().await?;
    let initial_block = BlockInfo {
        number: initial_block_number,
        hash: "initial".to_string(),
        parent_hash: "".to_string(),
        timestamp: std::time::SystemTime::now(),
        confirmations: 0,
        is_confirmed: false,
        event_count: 0,
        metadata: HashMap::new(),
    };
    let _fork_choice = Arc::new(create_fork_choice(fork_choice_rule, initial_block.clone()));

    // Start metrics server
    let metrics_handle = tokio::spawn(metrics::serve(args.metrics_port));

    // Determine start block
    let start_block = if let Some(block) = args.start_block {
        block
    } else if let Some(block) = config["indexing"]["start_block"].as_integer() {
        block as u64
    } else {
        // Default to latest minus confirmations
        initial_block
            .number
            .saturating_sub(chain_config.confirmations as u64)
    };

    info!("Starting indexing from block {}", start_block);

    // Create indexing configuration
    let batch_size = config["indexing"]["batch_size"].as_integer().unwrap_or(50) as usize;
    let _max_concurrent = config["indexing"]["max_concurrent_batches"]
        .as_integer()
        .unwrap_or(3) as usize;

    // Create and run indexing service
    let config = PipelineConfig {
        chain_id: "cosmos".to_string(),
        batch_size,
        interval_ms: 1000,
        max_concurrent_batches: 10,
        retry_attempts: 3,
        retry_delay_ms: 1000,
        finality_confirmations: 1,
        enable_validation: true,
        enable_ordering: true,
        checkpoint_interval: 1000,
    };
    let indexer = Arc::new(IndexingPipeline::new(config));

    // Start health check endpoint
    let health_adapter = adapter.clone();
    let health_storage = storage.clone();
    let health_indexer = indexer.clone();
    let health_handle = tokio::spawn(async move {
        health::serve(
            args.metrics_port + 1,
            health_adapter,
            health_storage,
            health_indexer,
            args.node_name.clone(),
        )
        .await
    });

    // Create shutdown signal
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_indexer = shutdown.clone();

    // Run indexer
    let indexer_for_task = indexer.clone();
    let indexer_handle = tokio::spawn(async move {
        tokio::select! {
            result = indexer_for_task.start() => {
                if let Err(e) = result {
                    error!("Indexer error: {}", e);
                }
            }
            _ = shutdown_indexer.notified() => {
                info!("Indexer shutting down");
                indexer_for_task.stop();
            }
        }
    });

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
        _ = indexer_handle => {
            error!("Indexer task ended unexpectedly");
        }
        _ = metrics_handle => {
            error!("Metrics server ended unexpectedly");
        }
        _ = health_handle => {
            error!("Health server ended unexpectedly");
        }
    }

    info!("Shutting down...");
    shutdown.notify_waiters();

    // Give tasks time to shutdown gracefully
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    Ok(())
}

mod health {
    use super::*;
    use axum::{extract::State, response::IntoResponse, Json, Router};
    use serde::Serialize;
    use std::sync::Arc;

    #[derive(Clone)]
    struct HealthState {
        adapter: Arc<dyn ChainAdapter>,
        storage: Arc<dyn almanac_core::Storage>,
        indexer: Arc<IndexingPipeline>,
        node_name: String,
    }

    #[derive(Serialize)]
    struct HealthStatus {
        status: &'static str,
        chain: String,
        node_name: String,
        latest_block: u64,
        indexed_block: u64,
        lag: u64,
        version: &'static str,
    }

    pub async fn serve(
        port: u16,
        adapter: Arc<dyn ChainAdapter>,
        storage: Arc<dyn almanac_core::Storage>,
        indexer: Arc<IndexingPipeline>,
        node_name: String,
    ) -> Result<()> {
        let state = HealthState {
            adapter,
            storage,
            indexer,
            node_name,
        };

        let app = Router::new()
            .route("/health", axum::routing::get(health_check))
            .route("/ready", axum::routing::get(readiness_check))
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }

    async fn health_check(State(state): State<HealthState>) -> impl IntoResponse {
        let latest_block = state.adapter.get_latest_block().await.unwrap_or(0);
        let indexed_block = state.indexer.get_last_indexed_block();

        Json(HealthStatus {
            status: "healthy",
            chain: state.adapter.chain_id().to_string(),
            node_name: state.node_name,
            latest_block,
            indexed_block,
            lag: latest_block.saturating_sub(indexed_block),
            version: env!("CARGO_PKG_VERSION"),
        })
    }

    async fn readiness_check(State(state): State<HealthState>) -> impl IntoResponse {
        // Check if we can connect to both chain and storage
        let chain_ok = state.adapter.get_latest_block().await.is_ok();
        let storage_ok = state.storage.get_latest_block("test").await.is_ok();
        let indexer_ok = state.indexer.is_running();

        if chain_ok && storage_ok && indexer_ok {
            (axum::http::StatusCode::OK, "ready")
        } else {
            (axum::http::StatusCode::SERVICE_UNAVAILABLE, "not ready")
        }
    }
}

mod metrics {
    use super::*;
    use axum::{response::IntoResponse, Router};
    use prometheus::{Encoder, TextEncoder};

    pub async fn serve(port: u16) -> Result<()> {
        let app = Router::new().route("/metrics", axum::routing::get(metrics_handler));

        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }

    async fn metrics_handler() -> impl IntoResponse {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}
