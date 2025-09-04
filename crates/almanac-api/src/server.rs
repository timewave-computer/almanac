use crate::storage_ext::ExtendedStorageReader;
use crate::{
    auth::{AuthServiceConfig, HttpAuthService},
    config::AuthConfig,
    routes::create_router,
    state::AppState,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to
    pub addr: SocketAddr,

    /// Enable graceful shutdown
    pub graceful_shutdown: bool,

    /// Request timeout in seconds
    pub request_timeout_secs: u64,

    /// Optional authentication configuration
    pub auth_config: Option<AuthConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: ([127, 0, 0, 1], 3000).into(),
            graceful_shutdown: true,
            request_timeout_secs: 30,
            auth_config: None,
        }
    }
}

/// API server
pub struct ApiServer {
    config: ServerConfig,
    state: AppState,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(config: ServerConfig, storage: Arc<dyn ExtendedStorageReader>) -> Self {
        let mut state = AppState::new(storage);

        // Configure authentication if provided
        if let Some(auth_config) = &config.auth_config {
            if auth_config.enabled {
                let auth_service_config: AuthServiceConfig = auth_config.clone().into();
                let auth_service = HttpAuthService::new(auth_service_config);
                state = state.with_auth_service(auth_service);
            }
        }

        Self { config, state }
    }

    /// Create a new API server with pre-configured state
    pub fn with_state(config: ServerConfig, state: AppState) -> Self {
        Self { config, state }
    }

    /// Run the server
    pub async fn run(self) -> anyhow::Result<()> {
        // Create the app with all middleware
        let app = create_router(self.state);

        tracing::info!("Starting API server on {}", self.config.addr);

        // Create the server
        let listener = tokio::net::TcpListener::bind(self.config.addr).await?;

        if self.config.graceful_shutdown {
            // Run with graceful shutdown
            axum::serve(listener, app)
                .with_graceful_shutdown(shutdown_signal())
                .await?;
        } else {
            // Run without graceful shutdown
            axum::serve(listener, app).await?;
        }

        Ok(())
    }
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MemoryStorageReader;

    #[tokio::test]
    async fn test_server_creation() {
        let storage = Arc::new(MemoryStorageReader::new());
        let config = ServerConfig::default();
        let server = ApiServer::new(config, storage);

        assert_eq!(server.config.addr.port(), 3000);
        assert!(server.config.graceful_shutdown);
    }
}
