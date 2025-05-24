/// WebSocket API implementation for real-time event streaming
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        ConnectInfo, Path, Query, State,
    },
    http::HeaderMap,
    response::Response,
};
use base64::prelude::*;
use futures::{
    sink::SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::SocketAddr;
use tokio::sync::{mpsc, RwLock, broadcast};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use indexer_core::{
    event::Event,
    service::{BoxedEventService, EventSubscription},
    types::{ChainId, EventFilter as CoreEventFilter},
    Error, Result,
};
use crate::{
    auth::{AuthState, OptionalUser, UserRole},
    http::HttpState,
};

/// Subscription persistence storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSubscription {
    pub id: String,
    pub connection_id: String,
    pub user_id: Option<String>,
    pub filters: EventFilters,
    pub created_at: u64,
    pub event_count: usize,
    pub active: bool,
}

/// In-memory subscription storage (for development/testing)
#[derive(Debug, Clone)]
pub struct InMemorySubscriptionStorage {
    subscriptions: Arc<RwLock<HashMap<String, PersistedSubscription>>>,
}

impl InMemorySubscriptionStorage {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn save_subscription(&self, subscription: &PersistedSubscription) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;
        subscriptions.insert(subscription.id.clone(), subscription.clone());
        debug!("Saved subscription {} to storage", subscription.id);
        Ok(())
    }
    
    pub async fn load_subscriptions(&self, connection_id: &str) -> Result<Vec<PersistedSubscription>> {
        let subscriptions = self.subscriptions.read().await;
        let results = subscriptions
            .values()
            .filter(|s| s.connection_id == connection_id && s.active)
            .cloned()
            .collect();
        Ok(results)
    }
    
    pub async fn load_all_subscriptions(&self) -> Result<Vec<PersistedSubscription>> {
        let subscriptions = self.subscriptions.read().await;
        let results = subscriptions
            .values()
            .filter(|s| s.active)
            .cloned()
            .collect();
        Ok(results)
    }
    
    pub async fn update_subscription_count(&self, subscription_id: &str, count: usize) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;
        if let Some(subscription) = subscriptions.get_mut(subscription_id) {
            subscription.event_count = count;
        }
        Ok(())
    }
    
    pub async fn deactivate_subscription(&self, subscription_id: &str) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;
        if let Some(subscription) = subscriptions.get_mut(subscription_id) {
            subscription.active = false;
            debug!("Deactivated subscription {}", subscription_id);
        }
        Ok(())
    }
    
    pub async fn cleanup_old_subscriptions(&self, older_than_hours: u64) -> Result<()> {
        let mut subscriptions = self.subscriptions.write().await;
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() - (older_than_hours * 3600);
        
        subscriptions.retain(|_, sub| sub.active || sub.created_at > cutoff_time);
        debug!("Cleaned up old subscriptions older than {} hours", older_than_hours);
        Ok(())
    }
}

/// WebSocket message types for the event streaming API
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// Client subscribes to events with filters
    Subscribe {
        id: String,
        filters: EventFilters,
    },
    /// Client unsubscribes from a subscription
    Unsubscribe {
        id: String,
    },
    /// Server sends an event to the client
    Event {
        subscription_id: String,
        event: EventData,
    },
    /// Server confirms subscription
    Subscribed {
        id: String,
        status: String,
    },
    /// Server confirms unsubscription
    Unsubscribed {
        id: String,
    },
    /// Error message
    Error {
        id: Option<String>,
        error: String,
        code: u16,
    },
    /// Heartbeat/ping message
    Ping {
        timestamp: u64,
    },
    /// Heartbeat response
    Pong {
        timestamp: u64,
    },
    /// Authentication message
    Auth {
        token: String,
    },
    /// Authentication response
    AuthResponse {
        authenticated: bool,
        user: Option<String>,
        role: Option<String>,
    },
}

/// Event filters for WebSocket subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilters {
    /// Chain ID filter
    pub chain_id: Option<String>,
    /// Contract address filter
    pub address: Option<String>,
    /// Event type filter
    pub event_type: Option<String>,
    /// Block range filter (from_height, to_height)
    pub block_range: Option<(u64, u64)>,
    /// Custom attribute filters
    pub attributes: Option<HashMap<String, Value>>,
    /// Maximum number of events to send
    pub limit: Option<usize>,
}

/// Event data for WebSocket messages
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventData {
    pub id: String,
    pub chain_id: String,
    pub block_number: u64,
    pub block_hash: String,
    pub tx_hash: String,
    pub event_type: String,
    pub timestamp: u64,
    pub raw_data: String, // Base64 encoded
    pub attributes: HashMap<String, Value>,
}

/// Convert core event to WebSocket event data
impl From<&dyn Event> for EventData {
    fn from(event: &dyn Event) -> Self {
        Self {
            id: event.id().to_string(),
            chain_id: event.chain().to_string(),
            block_number: event.block_number(),
            block_hash: event.block_hash().to_string(),
            tx_hash: event.tx_hash().to_string(),
            event_type: event.event_type().to_string(),
            timestamp: event
                .timestamp()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            raw_data: BASE64_STANDARD.encode(event.raw_data()),
            attributes: HashMap::new(), // TODO: Extract from event data
        }
    }
}

/// Convert WebSocket filters to core filters
impl From<EventFilters> for CoreEventFilter {
    fn from(filters: EventFilters) -> Self {
        let mut core_filter = CoreEventFilter::new();
        
        core_filter.chain_ids = filters.chain_id.as_ref().map(|id| vec![ChainId(id.clone())]);
        
        if let Some((from, to)) = filters.block_range {
            core_filter.block_range = Some((from, to));
        }
        
        if let Some(event_type) = filters.event_type {
            core_filter.event_types = Some(vec![event_type]);
        }
        
        // Convert attributes to custom_filters
        if let Some(attributes) = filters.attributes {
            for (key, value) in attributes {
                if let Some(str_value) = value.as_str() {
                    core_filter.custom_filters.insert(key, str_value.to_string());
                }
            }
        }
        
        // Add address as a custom filter if provided
        if let Some(address) = filters.address {
            core_filter.custom_filters.insert("address".to_string(), address);
        }
        
        core_filter.limit = filters.limit;
        
        core_filter
    }
}

/// Active subscription information
#[derive(Debug, Clone)]
pub struct Subscription {
    pub id: String,
    pub filters: EventFilters,
    pub created_at: SystemTime,
    pub event_count: usize,
    pub user_id: Option<String>,
}

impl Subscription {
    /// Convert to persisted subscription
    pub fn to_persisted(&self, connection_id: String) -> PersistedSubscription {
        PersistedSubscription {
            id: self.id.clone(),
            connection_id,
            user_id: self.user_id.clone(),
            filters: self.filters.clone(),
            created_at: self.created_at
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            event_count: self.event_count,
            active: true,
        }
    }
    
    /// Create from persisted subscription
    pub fn from_persisted(persisted: &PersistedSubscription) -> Self {
        Self {
            id: persisted.id.clone(),
            filters: persisted.filters.clone(),
            created_at: UNIX_EPOCH + std::time::Duration::from_secs(persisted.created_at),
            event_count: persisted.event_count,
            user_id: persisted.user_id.clone(),
        }
    }
}

/// WebSocket connection state
pub struct ConnectionState {
    /// Connection ID
    pub id: String,
    /// Client IP address
    pub addr: SocketAddr,
    /// Authenticated user (if any)
    pub user: Option<crate::auth::User>,
    /// Active subscriptions
    pub subscriptions: HashMap<String, Subscription>,
    /// Connection start time
    pub connected_at: SystemTime,
    /// Last activity time
    pub last_activity: SystemTime,
    /// Number of messages sent
    pub messages_sent: usize,
    /// Number of messages received
    pub messages_received: usize,
    /// Sender for messages to this connection
    pub sender: Option<mpsc::UnboundedSender<WsMessage>>,
}

impl ConnectionState {
    pub fn new(id: String, addr: SocketAddr) -> Self {
        Self {
            id,
            addr,
            user: None,
            subscriptions: HashMap::new(),
            connected_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            messages_sent: 0,
            messages_received: 0,
            sender: None,
        }
    }

    pub fn has_permission(&self, required_role: &UserRole) -> bool {
        self.user.as_ref()
            .map(|user| user.role.has_permission(required_role))
            .unwrap_or(false)
    }

    pub fn update_activity(&mut self) {
        self.last_activity = SystemTime::now();
    }
}

/// Connection manager for WebSocket connections
#[derive(Clone)]
pub struct ConnectionManager {
    /// Active connections
    connections: Arc<RwLock<HashMap<String, ConnectionState>>>,
    /// Event service
    event_service: BoxedEventService,
    /// Authentication state
    auth_state: AuthState,
    /// Event broadcaster
    event_broadcast: broadcast::Sender<(String, EventData)>,
    /// Subscription storage
    subscription_storage: Arc<InMemorySubscriptionStorage>,
}

impl ConnectionManager {
    pub fn new(event_service: BoxedEventService, auth_state: AuthState) -> Self {
        let (event_broadcast, _) = broadcast::channel(1000);
        
        let manager = Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            event_service,
            auth_state,
            event_broadcast,
            subscription_storage: Arc::new(InMemorySubscriptionStorage::new()),
        };
        
        // Start the event streaming background task
        manager.start_event_streaming();
        
        // Start the subscription cleanup task
        manager.start_cleanup_task();
        
        manager
    }

    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    pub async fn register_connection(&self, connection_id: String, addr: SocketAddr) {
        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), ConnectionState::new(connection_id, addr));
    }

    pub async fn set_connection_sender(&self, connection_id: &str, sender: mpsc::UnboundedSender<WsMessage>) {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(connection_id) {
            connection.sender = Some(sender);
        }
    }

    pub async fn remove_connection(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.remove(connection_id) {
            info!("Removed WebSocket connection: {} ({} subscriptions)", 
                  connection_id, connection.subscriptions.len());
                  
            // Deactivate all subscriptions for this connection
            for subscription_id in connection.subscriptions.keys() {
                if let Err(e) = self.subscription_storage.deactivate_subscription(subscription_id).await {
                    warn!("Failed to deactivate subscription {}: {}", subscription_id, e);
                }
            }
        }
    }

    pub async fn authenticate_connection(&self, connection_id: &str, headers: &HeaderMap) -> bool {
        let user = self.auth_state.authenticate(headers).await;
        
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(connection_id) {
            connection.user = user;
            connection.user.is_some()
        } else {
            false
        }
    }

    pub async fn add_subscription(
        &self,
        connection_id: &str,
        subscription_id: String,
        filters: EventFilters,
    ) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        if let Some(connection) = connections.get_mut(connection_id) {
            let subscription = Subscription {
                id: subscription_id.clone(),
                filters,
                created_at: SystemTime::now(),
                event_count: 0,
                user_id: connection.user.as_ref().map(|u| u.id.clone()),
            };
            
            // Save to persistent storage
            let persisted = subscription.to_persisted(connection_id.to_string());
            self.subscription_storage.save_subscription(&persisted).await?;
            
            connection.subscriptions.insert(subscription_id.clone(), subscription);
            info!("Added subscription {} for connection {}", subscription_id, connection_id);
            Ok(())
        } else {
            Err(Error::generic("Connection not found"))
        }
    }

    pub async fn remove_subscription(&self, connection_id: &str, subscription_id: &str) -> Result<()> {
        let mut connections = self.connections.write().await;
        
        if let Some(connection) = connections.get_mut(connection_id) {
            connection.subscriptions.remove(subscription_id);
            
            // Deactivate in persistent storage
            self.subscription_storage.deactivate_subscription(subscription_id).await?;
            
            info!("Removed subscription {} for connection {}", subscription_id, connection_id);
            Ok(())
        } else {
            Err(Error::generic("Connection not found"))
        }
    }

    pub async fn update_activity(&self, connection_id: &str, sent: bool) {
        let mut connections = self.connections.write().await;
        if let Some(connection) = connections.get_mut(connection_id) {
            connection.update_activity();
            if sent {
                connection.messages_sent += 1;
            } else {
                connection.messages_received += 1;
            }
        }
    }

    /// Recover subscriptions for a reconnected client
    pub async fn recover_subscriptions(&self, connection_id: &str) -> Result<Vec<Subscription>> {
        let persisted_subs = self.subscription_storage.load_subscriptions(connection_id).await?;
        let subscriptions: Vec<Subscription> = persisted_subs
            .iter()
            .map(Subscription::from_persisted)
            .collect();
        
        if !subscriptions.is_empty() {
            info!("Recovered {} subscriptions for connection {}", subscriptions.len(), connection_id);
        }
        
        Ok(subscriptions)
    }

    pub async fn get_stats(&self) -> Value {
        let connections = self.connections.read().await;
        let total_connections = connections.len();
        let authenticated_connections = connections.values()
            .filter(|c| c.user.is_some())
            .count();
        let total_subscriptions: usize = connections.values()
            .map(|c| c.subscriptions.len())
            .sum();

        // Get persistent subscription count
        let persistent_subscriptions = self.subscription_storage
            .load_all_subscriptions()
            .await
            .map(|subs| subs.len())
            .unwrap_or(0);

        json!({
            "total_connections": total_connections,
            "authenticated_connections": authenticated_connections,
            "total_subscriptions": total_subscriptions,
            "persistent_subscriptions": persistent_subscriptions,
            "average_subscriptions_per_connection": if total_connections > 0 {
                total_subscriptions as f64 / total_connections as f64
            } else {
                0.0
            }
        })
    }

    /// Start the background task for streaming events to subscribers
    fn start_event_streaming(&self) {
        let connections = self.connections.clone();
        let event_service = self.event_service.clone();
        let _event_broadcast = self.event_broadcast.clone();
        let subscription_storage = self.subscription_storage.clone();

        tokio::spawn(async move {
            // Subscribe to events from the event service
            match event_service.subscribe().await {
                Ok(mut subscription) => {
                    info!("Started WebSocket event streaming task");
                    
                    // Process events in a loop
                    while let Some(event) = subscription.next().await {
                        let event_data = EventData::from(event.as_ref());
                        
                        // Broadcast to all connections with matching subscriptions
                        let connections_read = connections.read().await;
                        for (connection_id, connection) in connections_read.iter() {
                            for (sub_id, subscription) in connection.subscriptions.iter() {
                                if Self::event_matches_filter(&event_data, &subscription.filters) {
                                    if let Some(sender) = &connection.sender {
                                        let ws_message = WsMessage::Event {
                                            subscription_id: sub_id.clone(),
                                            event: event_data.clone(),
                                        };
                                        
                                        if let Err(e) = sender.send(ws_message) {
                                            debug!("Failed to send event to connection {}: {}", connection_id, e);
                                            // Connection is likely closed, but we'll let the cleanup handle it
                                        } else {
                                            // Update event count in storage
                                            let new_count = subscription.event_count + 1;
                                            if let Err(e) = subscription_storage.update_subscription_count(sub_id, new_count).await {
                                                warn!("Failed to update subscription count: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to subscribe to events: {}", e);
                }
            }
        });
    }

    /// Start the background cleanup task
    fn start_cleanup_task(&self) {
        let subscription_storage = self.subscription_storage.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600)); // Run every hour
            
            loop {
                interval.tick().await;
                
                // Clean up subscriptions older than 24 hours
                if let Err(e) = subscription_storage.cleanup_old_subscriptions(24).await {
                    warn!("Failed to cleanup old subscriptions: {}", e);
                } else {
                    debug!("Cleaned up old inactive subscriptions");
                }
            }
        });
    }

    /// Check if an event matches the subscription filters
    fn event_matches_filter(event: &EventData, filters: &EventFilters) -> bool {
        // Check chain ID filter
        if let Some(ref chain_filter) = filters.chain_id {
            if event.chain_id != *chain_filter {
                return false;
            }
        }

        // Check event type filter
        if let Some(ref event_type_filter) = filters.event_type {
            if event.event_type != *event_type_filter {
                return false;
            }
        }

        // Check block range filter
        if let Some((from_block, to_block)) = filters.block_range {
            if event.block_number < from_block || event.block_number > to_block {
                return false;
            }
        }

        // Check address filter (stored in attributes for now)
        if let Some(ref _address_filter) = filters.address {
            // This would need to be implemented based on how addresses are stored in events
            // For now, we'll assume it matches
        }

        // Check custom attribute filters
        if let Some(ref attr_filters) = filters.attributes {
            for (key, value) in attr_filters {
                if let Some(event_value) = event.attributes.get(key) {
                    if event_value != value {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }

        true
    }
}

/// WebSocket handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<HttpState>,
) -> Response {
    info!("WebSocket connection attempt from: {}", addr);
    
    ws.on_upgrade(move |socket| handle_websocket(socket, addr, state))
}

/// Handle WebSocket connection
async fn handle_websocket(socket: WebSocket, addr: SocketAddr, state: HttpState) {
    let connection_id = Uuid::new_v4().to_string();
    
    // Create connection manager
    let manager = ConnectionManager::new(state.event_service.clone(), state.auth_state.clone());
    
    // Register connection
    manager.register_connection(connection_id.clone(), addr).await;
    
    // Split socket
    let (ws_sender, mut receiver) = socket.split();
    
    // Create channels for communication
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();
    let (ping_tx, mut ping_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    
    // Set the sender in the connection manager for event streaming
    manager.set_connection_sender(&connection_id, tx.clone()).await;
    
    // Spawn sender task
    let connection_id_clone = connection_id.clone();
    let manager_clone = manager.clone();
    let mut sender = ws_sender;
    let sender_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Handle outgoing messages
                msg = rx.recv() => {
                    match msg {
                        Some(ws_msg) => {
                            let json_msg = match serde_json::to_string(&ws_msg) {
                                Ok(json) => json,
                                Err(e) => {
                                    error!("Failed to serialize WebSocket message: {}", e);
                                    continue;
                                }
                            };
                            
                            if sender.send(Message::Text(json_msg)).await.is_err() {
                                error!("Failed to send WebSocket message");
                                break;
                            }
                            
                            manager_clone.update_activity(&connection_id_clone, true).await;
                        }
                        None => break,
                    }
                }
                // Handle ping responses
                pong_data = ping_rx.recv() => {
                    match pong_data {
                        Some(data) => {
                            if sender.send(Message::Pong(data)).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }
    });
    
    // Handle incoming messages
    let connection_id_clone = connection_id.clone();
    let manager_clone = manager.clone();
    let tx_clone = tx.clone();
    let receiver_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    manager_clone.update_activity(&connection_id_clone, false).await;
                    
                    match serde_json::from_str::<WsMessage>(&text) {
                        Ok(ws_msg) => {
                            if let Err(e) = handle_websocket_message(
                                ws_msg,
                                &connection_id_clone,
                                &manager_clone,
                                &tx_clone,
                            ).await {
                                error!("Error handling WebSocket message: {}", e);
                                let error_msg = WsMessage::Error {
                                    id: None,
                                    error: e.to_string(),
                                    code: 400,
                                };
                                let _ = tx_clone.send(error_msg);
                            }
                        }
                        Err(e) => {
                            error!("Failed to parse WebSocket message: {}", e);
                            let error_msg = WsMessage::Error {
                                id: None,
                                error: "Invalid message format".to_string(),
                                code: 400,
                            };
                            let _ = tx_clone.send(error_msg);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed by client: {}", addr);
                    break;
                }
                Ok(Message::Ping(data)) => {
                    let _ = ping_tx.send(data);
                }
                Ok(Message::Pong(_)) => {
                    // Handle pong
                }
                Ok(Message::Binary(_)) => {
                    warn!("Received binary message on WebSocket, ignoring");
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }
    });
    
    // Wait for either task to complete
    tokio::select! {
        _ = sender_task => {
            debug!("WebSocket sender task completed");
        }
        _ = receiver_task => {
            debug!("WebSocket receiver task completed");
        }
    }
    
    // Cleanup
    manager.remove_connection(&connection_id).await;
    info!("WebSocket connection cleanup completed for: {}", addr);
}

/// Handle individual WebSocket messages
async fn handle_websocket_message(
    msg: WsMessage,
    connection_id: &str,
    manager: &ConnectionManager,
    tx: &mpsc::UnboundedSender<WsMessage>,
) -> Result<()> {
    match msg {
        WsMessage::Subscribe { id, filters } => {
            // Add subscription
            manager.add_subscription(connection_id, id.clone(), filters).await?;
            
            // Send confirmation
            let response = WsMessage::Subscribed {
                id: id.clone(),
                status: "active".to_string(),
            };
            tx.send(response).map_err(|e| Error::generic(&format!("Failed to send message: {}", e)))?;
            
            // Event streaming is handled automatically by the background task
            debug!("Subscription created: {}", id);
        }
        
        WsMessage::Unsubscribe { id } => {
            // Remove subscription
            manager.remove_subscription(connection_id, &id).await?;
            
            // Send confirmation
            let response = WsMessage::Unsubscribed { id };
            tx.send(response).map_err(|e| Error::generic(&format!("Failed to send message: {}", e)))?;
        }
        
        WsMessage::Ping { timestamp } => {
            // Send pong
            let response = WsMessage::Pong { timestamp };
            tx.send(response).map_err(|e| Error::generic(&format!("Failed to send message: {}", e)))?;
        }
        
        WsMessage::Auth { token } => {
            // Create headers with the token for authentication
            let mut headers = HeaderMap::new();
            let auth_header = format!("Bearer {}", token);
            if let Ok(header_value) = auth_header.parse() {
                headers.insert("authorization", header_value);
                
                // Authenticate the connection
                let authenticated = manager.authenticate_connection(connection_id, &headers).await;
                
                                 // Get user info if authenticated
                let (user, role) = if authenticated {
                    let connections = manager.connections.read().await;
                    if let Some(connection) = connections.get(connection_id) {
                        if let Some(ref user) = connection.user {
                            (Some(user.username.clone()), Some(format!("{:?}", user.role)))
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                };
                
                let response = WsMessage::AuthResponse {
                    authenticated,
                    user,
                    role,
                };
                tx.send(response).map_err(|e| Error::generic(&format!("Failed to send message: {}", e)))?;
            } else {
                let response = WsMessage::AuthResponse {
                    authenticated: false,
                    user: None,
                    role: None,
                };
                tx.send(response).map_err(|e| Error::generic(&format!("Failed to send message: {}", e)))?;
            }
        }
        
        _ => {
            return Err(Error::generic("Unsupported message type"));
        }
    }
    
    Ok(())
}

/// WebSocket statistics endpoint
pub async fn websocket_stats(State(state): State<HttpState>) -> axum::response::Json<Value> {
    let manager = ConnectionManager::new(state.event_service, state.auth_state);
    let stats = manager.get_stats().await;
    axum::response::Json(stats)
} 