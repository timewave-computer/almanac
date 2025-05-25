/// Cross-chain event correlation and management functionality
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::event::Event;
use crate::Result;

/// Configuration for cross-chain correlation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainConfig {
    /// Supported blockchain networks
    pub supported_chains: Vec<ChainConfig>,
    
    /// Maximum time difference for cross-chain correlations
    pub max_time_diff: Duration,
    
    /// Maximum block difference for correlation (per chain)
    pub max_block_diff: HashMap<String, u64>,
    
    /// Cross-chain correlation fields
    pub correlation_fields: Vec<String>,
    
    /// Bridge contract addresses for cross-chain transfers
    pub bridge_contracts: HashMap<String, Vec<String>>,
    
    /// Minimum confidence threshold for correlations
    pub min_confidence: f64,
    
    /// Whether to enable automatic chain synchronization
    pub auto_sync: bool,
    
    /// Chain priority for correlation ordering
    pub chain_priority: HashMap<String, u32>,
}

impl Default for CrossChainConfig {
    fn default() -> Self {
        Self {
            supported_chains: vec![
                ChainConfig {
                    name: "ethereum".to_string(),
                    chain_id: 1,
                    rpc_endpoint: "https://eth.llamarpc.com".to_string(),
                    block_time: Duration::from_secs(12),
                    finality_blocks: 12,
                    native_token: "ETH".to_string(),
                    is_enabled: true,
                },
                ChainConfig {
                    name: "polygon".to_string(),
                    chain_id: 137,
                    rpc_endpoint: "https://polygon.llamarpc.com".to_string(),
                    block_time: Duration::from_secs(2),
                    finality_blocks: 128,
                    native_token: "MATIC".to_string(),
                    is_enabled: true,
                },
            ],
            max_time_diff: Duration::from_secs(300), // 5 minutes
            max_block_diff: HashMap::new(),
            correlation_fields: vec!["tx_hash".to_string(), "sender".to_string(), "amount".to_string()],
            bridge_contracts: HashMap::new(),
            min_confidence: 0.7,
            auto_sync: true,
            chain_priority: HashMap::new(),
        }
    }
}

/// Configuration for a specific blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Chain name
    pub name: String,
    
    /// Chain ID
    pub chain_id: u64,
    
    /// RPC endpoint URL
    pub rpc_endpoint: String,
    
    /// Average block time
    pub block_time: Duration,
    
    /// Number of blocks for finality
    pub finality_blocks: u64,
    
    /// Native token symbol
    pub native_token: String,
    
    /// Whether this chain is enabled for correlation
    pub is_enabled: bool,
}

/// Cross-chain correlation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainCorrelation {
    /// Unique correlation ID
    pub correlation_id: String,
    
    /// Events involved in the correlation
    pub events: Vec<CrossChainEvent>,
    
    /// Correlation type
    pub correlation_type: CrossChainType,
    
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    
    /// Source chain where the correlation originated
    pub source_chain: String,
    
    /// Target chains involved
    pub target_chains: Vec<String>,
    
    /// Time span of the correlation
    pub time_span: (SystemTime, SystemTime),
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    
    /// Bridge information if applicable
    pub bridge_info: Option<BridgeInfo>,
}

/// Types of cross-chain correlations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CrossChainType {
    /// Bridge transfer (lock/mint, burn/unlock)
    BridgeTransfer,
    
    /// Cross-chain message passing
    MessagePassing,
    
    /// Multi-chain transaction (same user, different chains)
    MultiChainTransaction,
    
    /// Arbitrage opportunity
    Arbitrage,
    
    /// Cross-chain DEX operations
    CrossChainSwap,
    
    /// General correlation based on configured fields
    General,
}

/// Bridge transfer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeInfo {
    /// Bridge protocol name
    pub protocol: String,
    
    /// Source chain bridge contract
    pub source_contract: String,
    
    /// Target chain bridge contract
    pub target_contract: String,
    
    /// Token being bridged
    pub token: String,
    
    /// Amount being bridged
    pub amount: String,
    
    /// Bridge transaction hashes
    pub bridge_txs: Vec<String>,
}

/// Cross-chain event wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainEvent {
    /// Original event ID
    pub event_id: String,
    
    /// Chain where event occurred
    pub chain: String,
    
    /// Event type
    pub event_type: String,
    
    /// Block number
    pub block_number: u64,
    
    /// Transaction hash
    pub tx_hash: String,
    
    /// Event timestamp
    pub timestamp: SystemTime,
    
    /// Extracted correlation data
    pub correlation_data: HashMap<String, String>,
    
    /// Event role in correlation
    pub role: EventRole,
}

/// Role of an event in cross-chain correlation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventRole {
    /// Source event that initiates the correlation
    Source,
    
    /// Target event that completes the correlation
    Target,
    
    /// Intermediate event in a multi-step correlation
    Intermediate,
    
    /// Supporting event that provides additional context
    Supporting,
}

/// Cross-chain correlation engine
#[async_trait]
pub trait CrossChainCorrelator: Send + Sync {
    /// Find cross-chain correlations in a set of events
    async fn correlate_cross_chain(
        &self,
        events: Vec<Box<dyn Event>>,
        config: &CrossChainConfig,
    ) -> Result<Vec<CrossChainCorrelation>>;
    
    /// Detect bridge transfers specifically
    async fn detect_bridge_transfers(
        &self,
        events: Vec<Box<dyn Event>>,
        config: &CrossChainConfig,
    ) -> Result<Vec<CrossChainCorrelation>>;
    
    /// Find arbitrage opportunities
    async fn detect_arbitrage(
        &self,
        events: Vec<Box<dyn Event>>,
        config: &CrossChainConfig,
    ) -> Result<Vec<CrossChainCorrelation>>;
}

/// Default cross-chain correlator implementation
pub struct DefaultCrossChainCorrelator;

impl DefaultCrossChainCorrelator {
    pub fn new() -> Self {
        Self
    }
    
    /// Convert event to cross-chain event
    fn to_cross_chain_event(&self, event: &dyn Event, config: &CrossChainConfig) -> CrossChainEvent {
        let mut correlation_data = HashMap::new();
        
        // Extract correlation fields from event
        for field in &config.correlation_fields {
            if let Some(value) = self.extract_field_value(event, field) {
                correlation_data.insert(field.clone(), value);
            }
        }
        
        CrossChainEvent {
            event_id: event.id().to_string(),
            chain: event.chain().to_string(),
            event_type: event.event_type().to_string(),
            block_number: event.block_number(),
            tx_hash: event.tx_hash().to_string(),
            timestamp: event.timestamp(),
            correlation_data,
            role: EventRole::Source, // Will be determined later
        }
    }
    
    /// Extract field value from event
    fn extract_field_value(&self, event: &dyn Event, field: &str) -> Option<String> {
        match field {
            "tx_hash" => Some(event.tx_hash().to_string()),
            "block_hash" => Some(event.block_hash().to_string()),
            "chain" => Some(event.chain().to_string()),
            "event_type" => Some(event.event_type().to_string()),
            _ => {
                // Try to parse from raw data as JSON
                if let Ok(raw_str) = String::from_utf8(event.raw_data().to_vec()) {
                    if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&raw_str) {
                        self.extract_from_json(&json_value, field)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        }
    }
    
    /// Extract value from JSON object
    fn extract_from_json(&self, json: &serde_json::Value, field: &str) -> Option<String> {
        match json {
            serde_json::Value::Object(obj) => {
                obj.get(field).and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s.clone()),
                    serde_json::Value::Number(n) => Some(n.to_string()),
                    serde_json::Value::Bool(b) => Some(b.to_string()),
                    _ => None,
                })
            },
            _ => None,
        }
    }
    
    /// Check if two events can be correlated across chains
    fn can_correlate_cross_chain(
        &self,
        event1: &CrossChainEvent,
        event2: &CrossChainEvent,
        config: &CrossChainConfig,
    ) -> bool {
        // Must be on different chains
        if event1.chain == event2.chain {
            return false;
        }
        
        // Check time difference
        let time_diff = if event1.timestamp > event2.timestamp {
            event1.timestamp.duration_since(event2.timestamp)
        } else {
            event2.timestamp.duration_since(event1.timestamp)
        };
        
        if let Ok(duration) = time_diff {
            if duration > config.max_time_diff {
                return false;
            }
        }
        
        // Check if at least one correlation field matches
        let mut matching_fields = 0;
        for field in &config.correlation_fields {
            if let (Some(val1), Some(val2)) = (
                event1.correlation_data.get(field),
                event2.correlation_data.get(field)
            ) {
                if val1 == val2 {
                    matching_fields += 1;
                }
            }
        }
        
        matching_fields > 0
    }
    
    /// Calculate correlation confidence
    fn calculate_confidence(
        &self,
        events: &[CrossChainEvent],
        correlation_type: &CrossChainType,
        config: &CrossChainConfig,
    ) -> f64 {
        let mut confidence = 0.0;
        
        // Base confidence by correlation type
        confidence += match correlation_type {
            CrossChainType::BridgeTransfer => 0.8,
            CrossChainType::MessagePassing => 0.7,
            CrossChainType::MultiChainTransaction => 0.6,
            CrossChainType::Arbitrage => 0.5,
            CrossChainType::CrossChainSwap => 0.7,
            CrossChainType::General => 0.4,
        };
        
        // Boost confidence based on matching fields
        let total_fields = config.correlation_fields.len() as f64;
        let mut matching_fields = 0.0;
        
        if events.len() >= 2 {
            for field in &config.correlation_fields {
                let values: Vec<_> = events.iter()
                    .filter_map(|e| e.correlation_data.get(field))
                    .collect();
                
                if values.len() >= 2 && values.iter().all(|v| v == &values[0]) {
                    matching_fields += 1.0;
                }
            }
            
            confidence += 0.3 * (matching_fields / total_fields);
        }
        
        // Adjust based on time proximity
        if events.len() >= 2 {
            let time_span = events.iter()
                .map(|e| e.timestamp)
                .max()
                .unwrap()
                .duration_since(
                    events.iter().map(|e| e.timestamp).min().unwrap()
                )
                .unwrap_or_default();
            
            let time_factor = 1.0 - (time_span.as_secs() as f64 / config.max_time_diff.as_secs() as f64);
            confidence += 0.2 * time_factor.max(0.0);
        }
        
        confidence.min(1.0)
    }
    
    /// Determine correlation type based on events
    fn determine_correlation_type(
        &self,
        events: &[CrossChainEvent],
        config: &CrossChainConfig,
    ) -> CrossChainType {
        // Check for bridge transfer patterns
        if self.is_bridge_transfer(events, config) {
            return CrossChainType::BridgeTransfer;
        }
        
        // Check for arbitrage patterns
        if self.is_arbitrage(events) {
            return CrossChainType::Arbitrage;
        }
        
        // Check for cross-chain swap patterns
        if self.is_cross_chain_swap(events) {
            return CrossChainType::CrossChainSwap;
        }
        
        // Check for message passing
        if self.is_message_passing(events) {
            return CrossChainType::MessagePassing;
        }
        
        // Check for multi-chain transaction
        if self.is_multi_chain_transaction(events) {
            return CrossChainType::MultiChainTransaction;
        }
        
        CrossChainType::General
    }
    
    /// Check if events represent a bridge transfer
    fn is_bridge_transfer(&self, events: &[CrossChainEvent], config: &CrossChainConfig) -> bool {
        // Look for lock/mint or burn/unlock patterns
        let has_lock_mint = events.iter().any(|e| {
            e.event_type.contains("lock") || e.event_type.contains("Lock")
        }) && events.iter().any(|e| {
            e.event_type.contains("mint") || e.event_type.contains("Mint")
        });
        
        let has_burn_unlock = events.iter().any(|e| {
            e.event_type.contains("burn") || e.event_type.contains("Burn")
        }) && events.iter().any(|e| {
            e.event_type.contains("unlock") || e.event_type.contains("Unlock")
        });
        
        // Check if any events involve known bridge contracts
        let involves_bridge = events.iter().any(|e| {
            if let Some(contracts) = config.bridge_contracts.get(&e.chain) {
                e.correlation_data.values().any(|addr| contracts.contains(addr))
            } else {
                false
            }
        });
        
        (has_lock_mint || has_burn_unlock) || involves_bridge
    }
    
    /// Check if events represent arbitrage
    fn is_arbitrage(&self, events: &[CrossChainEvent]) -> bool {
        // Look for DEX trades with same token but different prices
        let dex_events: Vec<_> = events.iter()
            .filter(|e| e.event_type.contains("swap") || e.event_type.contains("Swap"))
            .collect();
        
        dex_events.len() >= 2 && 
        dex_events.iter().map(|e| &e.chain).collect::<std::collections::HashSet<_>>().len() >= 2
    }
    
    /// Check if events represent cross-chain swap
    fn is_cross_chain_swap(&self, events: &[CrossChainEvent]) -> bool {
        events.iter().any(|e| e.event_type.contains("CrossChainSwap") || 
                             e.event_type.contains("crossChainSwap"))
    }
    
    /// Check if events represent message passing
    fn is_message_passing(&self, events: &[CrossChainEvent]) -> bool {
        events.iter().any(|e| e.event_type.contains("Message") || 
                             e.event_type.contains("Relay"))
    }
    
    /// Check if events represent multi-chain transaction
    fn is_multi_chain_transaction(&self, events: &[CrossChainEvent]) -> bool {
        // Same user performing transactions on multiple chains
        if let Some(sender_values) = events.first()
            .and_then(|e| e.correlation_data.get("sender")) {
            events.iter().all(|e| {
                e.correlation_data.get("sender") == Some(sender_values)
            }) && events.iter().map(|e| &e.chain).collect::<std::collections::HashSet<_>>().len() >= 2
        } else {
            false
        }
    }
    
    /// Extract bridge information from events
    fn extract_bridge_info(&self, events: &[CrossChainEvent], config: &CrossChainConfig) -> Option<BridgeInfo> {
        if !self.is_bridge_transfer(events, config) {
            return None;
        }
        
        // Try to extract bridge details
        let protocol = "unknown".to_string(); // Would be determined from contract addresses
        let source_contract = events.first()
            .and_then(|e| e.correlation_data.get("contract"))
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let target_contract = events.last()
            .and_then(|e| e.correlation_data.get("contract"))
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let token = events.first()
            .and_then(|e| e.correlation_data.get("token"))
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let amount = events.first()
            .and_then(|e| e.correlation_data.get("amount"))
            .cloned()
            .unwrap_or_else(|| "0".to_string());
        let bridge_txs = events.iter().map(|e| e.tx_hash.clone()).collect();
        
        Some(BridgeInfo {
            protocol,
            source_contract,
            target_contract,
            token,
            amount,
            bridge_txs,
        })
    }
}

#[async_trait]
impl CrossChainCorrelator for DefaultCrossChainCorrelator {
    async fn correlate_cross_chain(
        &self,
        events: Vec<Box<dyn Event>>,
        config: &CrossChainConfig,
    ) -> Result<Vec<CrossChainCorrelation>> {
        let mut correlations = Vec::new();
        let mut processed_pairs = std::collections::HashSet::new();
        
        // Convert events to cross-chain events
        let cross_chain_events: Vec<_> = events.iter()
            .map(|e| self.to_cross_chain_event(e.as_ref(), config))
            .collect();
        
        // Group events by correlation potential
        let mut correlation_groups: HashMap<String, Vec<usize>> = HashMap::new();
        
        for (idx, event) in cross_chain_events.iter().enumerate() {
            // Only process enabled chains
            if !config.supported_chains.iter()
                .any(|c| c.name == event.chain && c.is_enabled) {
                continue;
            }
            
            // Create correlation keys based on shared fields
            for field in &config.correlation_fields {
                if let Some(value) = event.correlation_data.get(field) {
                    let key = format!("{}:{}", field, value);
                    correlation_groups.entry(key).or_default().push(idx);
                }
            }
        }
        
        // Process each potential correlation group
        for (_, event_indices) in correlation_groups {
            if event_indices.len() < 2 {
                continue;
            }
            
            // Check all pairs for cross-chain correlation
            for i in 0..event_indices.len() {
                for j in i+1..event_indices.len() {
                    let idx1 = event_indices[i];
                    let idx2 = event_indices[j];
                    
                    // Create a unique pair key to avoid duplicates
                    let pair_key = if idx1 < idx2 {
                        format!("{}:{}", idx1, idx2)
                    } else {
                        format!("{}:{}", idx2, idx1)
                    };
                    
                    if processed_pairs.contains(&pair_key) {
                        continue;
                    }
                    processed_pairs.insert(pair_key);
                    
                    let event1 = &cross_chain_events[idx1];
                    let event2 = &cross_chain_events[idx2];
                    
                    if self.can_correlate_cross_chain(event1, event2, config) {
                        let correlated_events = vec![event1.clone(), event2.clone()];
                        let correlation_type = self.determine_correlation_type(&correlated_events, config);
                        let confidence = self.calculate_confidence(&correlated_events, &correlation_type, config);
                        
                        if confidence >= config.min_confidence {
                            let correlation_id = format!("cross_{}", uuid::Uuid::new_v4());
                            let time_span = (
                                correlated_events.iter().map(|e| e.timestamp).min().unwrap(),
                                correlated_events.iter().map(|e| e.timestamp).max().unwrap(),
                            );
                            
                            let correlation = CrossChainCorrelation {
                                correlation_id,
                                events: correlated_events.clone(),
                                correlation_type: correlation_type.clone(),
                                confidence,
                                source_chain: event1.chain.clone(),
                                target_chains: vec![event2.chain.clone()],
                                time_span,
                                metadata: HashMap::new(),
                                bridge_info: self.extract_bridge_info(&correlated_events, config),
                            };
                            
                            correlations.push(correlation);
                        }
                    }
                }
            }
        }
        
        // Sort by confidence (highest first)
        correlations.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(correlations)
    }
    
    async fn detect_bridge_transfers(
        &self,
        events: Vec<Box<dyn Event>>,
        config: &CrossChainConfig,
    ) -> Result<Vec<CrossChainCorrelation>> {
        let all_correlations = self.correlate_cross_chain(events, config).await?;
        
        Ok(all_correlations.into_iter()
            .filter(|c| c.correlation_type == CrossChainType::BridgeTransfer)
            .collect())
    }
    
    async fn detect_arbitrage(
        &self,
        events: Vec<Box<dyn Event>>,
        config: &CrossChainConfig,
    ) -> Result<Vec<CrossChainCorrelation>> {
        let all_correlations = self.correlate_cross_chain(events, config).await?;
        
        Ok(all_correlations.into_iter()
            .filter(|c| c.correlation_type == CrossChainType::Arbitrage)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{UNIX_EPOCH, Duration};
    
    // Mock event for testing
    #[derive(Debug, Clone)]
    struct TestEvent {
        id: String,
        chain: String,
        block_number: u64,
        block_hash: String,
        tx_hash: String,
        timestamp: SystemTime,
        event_type: String,
        raw_data: Vec<u8>,
    }
    
    impl Event for TestEvent {
        fn id(&self) -> &str { &self.id }
        fn chain(&self) -> &str { &self.chain }
        fn block_number(&self) -> u64 { self.block_number }
        fn block_hash(&self) -> &str { &self.block_hash }
        fn tx_hash(&self) -> &str { &self.tx_hash }
        fn timestamp(&self) -> SystemTime { self.timestamp }
        fn event_type(&self) -> &str { &self.event_type }
        fn raw_data(&self) -> &[u8] { &self.raw_data }
        fn as_any(&self) -> &dyn std::any::Any { self }
    }
    
    fn create_test_event(
        id: &str,
        chain: &str,
        event_type: &str,
        tx_hash: &str,
        block_number: u64,
        timestamp_offset: u64,
        raw_data: &str,
    ) -> Box<dyn Event> {
        Box::new(TestEvent {
            id: id.to_string(),
            chain: chain.to_string(),
            block_number,
            block_hash: format!("hash_{}", block_number),
            tx_hash: tx_hash.to_string(),
            timestamp: UNIX_EPOCH + Duration::from_secs(timestamp_offset),
            event_type: event_type.to_string(),
            raw_data: raw_data.as_bytes().to_vec(),
        })
    }
    
    #[tokio::test]
    async fn test_cross_chain_correlation() {
        let correlator = DefaultCrossChainCorrelator::new();
        let config = CrossChainConfig::default();
        
        let events = vec![
            create_test_event(
                "1", "ethereum", "Lock", "tx_1", 100, 1000,
                r#"{"sender": "0x123", "amount": "1000", "token": "USDC"}"#
            ),
            create_test_event(
                "2", "polygon", "Mint", "tx_2", 200, 1010,
                r#"{"sender": "0x123", "amount": "1000", "token": "USDC"}"#
            ),
        ];
        
        let correlations = correlator.correlate_cross_chain(events, &config).await.unwrap();
        
        assert_eq!(correlations.len(), 1);
        assert_eq!(correlations[0].events.len(), 2);
        assert_eq!(correlations[0].correlation_type, CrossChainType::BridgeTransfer);
        assert!(correlations[0].confidence >= config.min_confidence);
    }
    
    #[tokio::test]
    async fn test_bridge_transfer_detection() {
        let correlator = DefaultCrossChainCorrelator::new();
        let config = CrossChainConfig::default();
        
        let events = vec![
            create_test_event(
                "1", "ethereum", "TokenLock", "tx_1", 100, 1000,
                r#"{"contract": "0xbridge1", "amount": "500", "token": "DAI"}"#
            ),
            create_test_event(
                "2", "polygon", "TokenMint", "tx_2", 200, 1020,
                r#"{"contract": "0xbridge2", "amount": "500", "token": "DAI"}"#
            ),
        ];
        
        let bridge_transfers = correlator.detect_bridge_transfers(events, &config).await.unwrap();
        
        assert_eq!(bridge_transfers.len(), 1);
        assert_eq!(bridge_transfers[0].correlation_type, CrossChainType::BridgeTransfer);
        assert!(bridge_transfers[0].bridge_info.is_some());
    }
    
    #[test]
    fn test_cross_chain_config_default() {
        let config = CrossChainConfig::default();
        
        assert_eq!(config.supported_chains.len(), 2);
        assert_eq!(config.supported_chains[0].name, "ethereum");
        assert_eq!(config.supported_chains[1].name, "polygon");
        assert_eq!(config.max_time_diff, Duration::from_secs(300));
        assert_eq!(config.min_confidence, 0.7);
    }
    
    #[test]
    fn test_correlation_type_determination() {
        let correlator = DefaultCrossChainCorrelator::new();
        let config = CrossChainConfig::default();
        
        let bridge_events = vec![
            CrossChainEvent {
                event_id: "1".to_string(),
                chain: "ethereum".to_string(),
                event_type: "TokenLock".to_string(),
                block_number: 100,
                tx_hash: "tx_1".to_string(),
                timestamp: SystemTime::now(),
                correlation_data: HashMap::new(),
                role: EventRole::Source,
            },
            CrossChainEvent {
                event_id: "2".to_string(),
                chain: "polygon".to_string(),
                event_type: "TokenMint".to_string(),
                block_number: 200,
                tx_hash: "tx_2".to_string(),
                timestamp: SystemTime::now(),
                correlation_data: HashMap::new(),
                role: EventRole::Target,
            },
        ];
        
        let correlation_type = correlator.determine_correlation_type(&bridge_events, &config);
        assert_eq!(correlation_type, CrossChainType::BridgeTransfer);
    }
} 