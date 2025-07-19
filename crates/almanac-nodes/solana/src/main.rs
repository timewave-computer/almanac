//! Solana-specific Almanac indexing node

mod health;
mod metrics;

use almanac_chains::{ChainAdapter, ChainConfig, SolanaAdapter};
use almanac_config::Config;
use almanac_consensus::{create_fork_choice, ForkChoiceRule};
use almanac_indexing::IndexingService;
use almanac_storage::FoundationDBBackend;
use anyhow::Result;
use clap::Parser;
use std::sync::Arc;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "almanac-solana")]
#[command(about = "Almanac indexer for Solana")]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "config/solana.toml")]
    config: String,
    
    /// Node name for identification
    #[arg(long, env = "NODE_NAME")]
    node_name: String,
    
    /// Metrics port
    #[arg(long, default_value = "9093")]
    metrics_port: u16,
    
    /// Start from specific slot (overrides config)
    #[arg(long)]
    start_slot: Option<u64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .json()
        .init();
    
    let args = Args::parse();
    info!("Starting almanac-solana node: {}", args.node_name);
    
    // Load configuration
    let config_content = tokio::fs::read_to_string(&args.config).await?;
    let config: toml::Value = toml::from_str(&config_content)?;
    
    // Create chain configuration
    let chain_config = ChainConfig {
        chain_id: config["chain"]["id"].as_str().unwrap_or("solana-mainnet").to_string(),
        rpc_url: config["rpc"]["url"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing rpc.url in config"))?
            .to_string(),
        api_key: config["rpc"]["api_key"].as_str().map(|s| s.to_string()),
        confirmations: config["chain"]["confirmations"].as_integer().unwrap_or(32) as u32,
        max_retries: config["rpc"]["max_retries"].as_integer().unwrap_or(3) as u32,
        timeout_secs: config["rpc"]["timeout_secs"].as_integer().unwrap_or(30) as u64,
        extra: serde_json::Value::Null,
    };
    
    // Create Solana adapter
    let adapter = Arc::new(SolanaAdapter::new(chain_config.clone()).await?);
    info!("Connected to Solana RPC");
    
    // Connect to FoundationDB
    let fdb_cluster = config["storage"]["fdb"]["cluster_file"]
        .as_str()
        .map(|s| s.to_string());
    let namespace = format!("almanac/{}", chain_config.chain_id);
    
    let storage = Arc::new(FoundationDBBackend::new(
        fdb_cluster.as_deref(),
        &namespace,
    ).await?);
    info!("Connected to FoundationDB namespace: {}", namespace);
    
    // Create consensus handler (Solana uses Tower BFT)
    let fork_choice_rule = ForkChoiceRule::TowerBft;
    let initial_slot = adapter.get_latest_block().await?;
    let fork_choice = Arc::new(create_fork_choice(fork_choice_rule, initial_slot));
    
    // Start metrics server
    let metrics_handle = tokio::spawn(metrics::serve(args.metrics_port));
    
    // Start health check endpoint
    let health_adapter = adapter.clone();
    let health_storage = storage.clone();
    let health_handle = tokio::spawn(async move {
        health::serve(
            args.metrics_port + 1,
            health_adapter,
            health_storage,
            args.node_name.clone(),
        ).await
    });
    
    // Determine start slot
    let start_slot = if let Some(slot) = args.start_slot {
        slot
    } else if let Some(slot) = config["indexing"]["start_slot"].as_integer() {
        slot as u64
    } else {
        // Default to latest minus confirmations
        initial_slot.saturating_sub(chain_config.confirmations as u64)
    };
    
    info!("Starting indexing from slot {}", start_slot);
    
    // Create indexing configuration
    let batch_size = config["indexing"]["batch_size"].as_integer().unwrap_or(25) as usize;
    let max_concurrent = config["indexing"]["max_concurrent_batches"].as_integer().unwrap_or(2) as usize;
    
    // Create and run indexing service
    let indexer = IndexingService::builder()
        .chain_adapter(adapter)
        .storage(storage)
        .fork_choice(fork_choice)
        .node_name(&args.node_name)
        .batch_size(batch_size)
        .max_concurrent_batches(max_concurrent)
        .start_block(start_slot)
        .build();
    
    // Create shutdown signal
    let shutdown = Arc::new(tokio::sync::Notify::new());
    let shutdown_indexer = shutdown.clone();
    
    // Run indexer
    let indexer_handle = tokio::spawn(async move {
        tokio::select! {
            result = indexer.run() => {
                if let Err(e) = result {
                    error!("Indexer error: {}", e);
                }
            }
            _ = shutdown_indexer.notified() => {
                info!("Indexer shutting down");
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
    use axum::{Router, Json, response::IntoResponse, extract::State};
    use serde::Serialize;
    use std::sync::Arc;

    #[derive(Clone)]
    struct HealthState {
        adapter: Arc<dyn ChainAdapter>,
        storage: Arc<dyn almanac_core::Storage>,
        node_name: String,
    }

    #[derive(Serialize)]
    struct HealthStatus {
        status: &'static str,
        chain: String,
        node_name: String,
        latest_slot: u64,
        indexed_slot: u64,
        lag: u64,
        version: &'static str,
    }

    pub async fn serve(
        port: u16,
        adapter: Arc<dyn ChainAdapter>,
        storage: Arc<dyn almanac_core::Storage>,
        node_name: String,
    ) -> Result<()> {
        let state = HealthState {
            adapter,
            storage,
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
        let latest_slot = state.adapter.get_latest_block().await.unwrap_or(0);
        let indexed_slot = 0; // TODO: Get from storage
        
        Json(HealthStatus {
            status: "healthy",
            chain: state.adapter.chain_id().to_string(),
            node_name: state.node_name,
            latest_slot,
            indexed_slot,
            lag: latest_slot.saturating_sub(indexed_slot),
            version: env!("CARGO_PKG_VERSION"),
        })
    }
    
    async fn readiness_check(State(state): State<HealthState>) -> impl IntoResponse {
        // Check if we can connect to both chain and storage
        let chain_ok = state.adapter.get_latest_block().await.is_ok();
        let storage_ok = true; // TODO: Add storage health check
        
        if chain_ok && storage_ok {
            (axum::http::StatusCode::OK, "ready")
        } else {
            (axum::http::StatusCode::SERVICE_UNAVAILABLE, "not ready")
        }
    }
}

mod metrics {
    use super::*;
    use axum::Router;
    use prometheus::{Encoder, TextEncoder};

    pub async fn serve(port: u16) -> Result<()> {
        let app = Router::new()
            .route("/metrics", axum::routing::get(metrics_handler));
        
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