use ethers::providers::Provider;
use ethers::providers::{Http, Ws};

/// Ethereum provider types
pub enum EthereumProvider {
    /// HTTP provider
    Http(Provider<Http>),
    
    /// WebSocket provider
    Websocket(Provider<Ws>),
} 