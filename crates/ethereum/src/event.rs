use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use std::sync::Arc;

use ethers::types::{Block, Log, Transaction, TransactionReceipt, H256};
use ethers::abi::{Abi, Event as AbiEvent, Token};
use serde::{Deserialize, Serialize};
use indexer_common::{Error, Result};
use tracing::{debug, error, info, warn};

use indexer_core::event::Event;
use indexer_core::types::EventFilter;

/// Represents an Ethereum event with decoded parameters.
#[derive(Clone, Serialize, Deserialize)]
pub struct EthereumEvent {
    /// Unique identifier for the event
    pub id: String,

    /// Chain ID
    pub chain: String,

    /// Block number
    pub block_number: u64,

    /// Block hash
    pub block_hash: String,

    /// Transaction hash
    pub tx_hash: String,

    /// Timestamp (Unix seconds)
    pub timestamp: u64,

    /// Event type (usually the signature hash or decoded name)
    pub event_type: String,

    /// Contract address where the event was emitted
    pub address: String,

    /// Topics of the event (hex strings)
    pub topics: Vec<String>,

    /// Data of the event (bytes)
    pub data: Vec<u8>,

    /// Log index within the block
    pub log_index: u64,

    /// Transaction index within the block
    pub tx_index: u64,

    /// Raw event data (Original ethers Log struct)
    #[serde(skip)]
    pub raw_log: Log,
    
    /// Decoded parameters (if ABI is available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoded_params: Option<HashMap<String, Token>>,
    
    /// Contract name (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_name: Option<String>,
}

impl fmt::Debug for EthereumEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EthereumEvent")
            .field("id", &self.id)
            .field("chain", &self.chain)
            .field("block_number", &self.block_number)
            .field("block_hash", &self.block_hash)
            .field("tx_hash", &self.tx_hash)
            .field("timestamp", &self.timestamp)
            .field("event_type", &self.event_type)
            .field("address", &self.address)
            .field("topics", &self.topics)
            .field("data_len", &self.data.len())
            .field("log_index", &self.log_index)
            .field("tx_index", &self.tx_index)
            .field("decoded_params", &self.decoded_params)
            .field("contract_name", &self.contract_name)
            .finish()
    }
}

impl Event for EthereumEvent {
    fn id(&self) -> &str {
        &self.id
    }

    fn chain(&self) -> &str {
        &self.chain
    }

    fn block_number(&self) -> u64 {
        self.block_number
    }

    fn block_hash(&self) -> &str {
        &self.block_hash
    }

    fn tx_hash(&self) -> &str {
        &self.tx_hash
    }

    fn timestamp(&self) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_secs(self.timestamp)
    }

    fn event_type(&self) -> &str {
        &self.event_type
    }

    fn address(&self) -> Option<&str> {
        Some(&self.address)
    }

    fn raw_data(&self) -> &[u8] {
        &self.raw_log.data
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Event processor for Ethereum events
pub struct EthereumEventProcessor {
    /// Chain ID
    chain_id: String,
    
    /// Contract ABIs by address
    contract_abis: HashMap<String, (String, Arc<Abi>)>,
}

impl EthereumEventProcessor {
    /// Create a new Ethereum event processor
    pub fn new(chain_id: String) -> Self {
        Self {
            chain_id,
            contract_abis: HashMap::new(),
        }
    }
    
    /// Register a contract ABI for event parsing
    pub fn register_contract(&mut self, address: String, name: String, abi: Abi) {
        self.contract_abis.insert(address, (name, Arc::new(abi)));
    }
    
    /// Try to decode an event using the registered ABIs
    fn decode_event(&self, log: &Log) -> Option<(String, HashMap<String, Token>)> {
        let address = log.address.to_string();
        
        // Try to get the contract ABI
        if let Some((contract_name, abi)) = self.contract_abis.get(&address) {
            // Try to find the event in the ABI that matches the first topic
            if !log.topics.is_empty() {
                let event_signature = log.topics[0];
                
                for event in abi.events() {
                    if event.signature() == event_signature.into() {
                        // Try to decode the event
                        if let Ok(decoded) = event.parse_log(log.clone().into()) {
                            let mut params = HashMap::new();
                            
                            for param in decoded.params {
                                params.insert(param.name, param.value);
                            }
                            
                            return Some((event.name.clone(), params));
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// Determine the event type from the log
    fn determine_event_type(&self, log: &Log) -> String {
        // Check if we can decode the event
        if let Some((event_name, _)) = self.decode_event(log) {
            return event_name;
        }
        
        // If we can't decode it, use the first topic as the event signature
        if !log.topics.is_empty() {
            return format!("0x{}", hex::encode(log.topics[0].as_bytes()));
        }
        
        // If there's no topic, use a generic event type
        "unknown".to_string()
    }
    
    /// Process a log from a block and create an Ethereum event
    pub fn process_log(
        &self,
        log: Log, 
        block: &Block<Transaction>, 
        receipt: Option<&TransactionReceipt>
    ) -> Result<EthereumEvent> {
        let timestamp = block.timestamp.as_u64();
        let block_number = block.number.unwrap_or_default().as_u64();
        let block_hash = block.hash.unwrap_or_default().to_string();
        let tx_hash = log.transaction_hash.unwrap_or_default().to_string();
        let log_index = log.log_index.unwrap_or_default().as_u64();
        let tx_index = log.transaction_index.unwrap_or_default().as_u64();
        let address = log.address.to_string();
        
        // Generate a unique ID for the event
        let id = format!("{}-{}-{}", self.chain_id, tx_hash, log_index);
        
        // Get event type and try to decode parameters
        let event_type = self.determine_event_type(&log);
        let (decoded_params, contract_name) = if let Some((name, params)) = self.decode_event(&log) {
            (Some(params), self.contract_abis.get(&address).map(|(name, _)| name.clone()))
        } else {
            (None, None)
        };
        
        // Convert topics to strings
        let topics = log.topics.iter()
            .map(|t| t.to_string())
            .collect();
        
        // Serialize the log to get the raw data
        let raw_data = serde_json::to_vec(&log).unwrap_or_default();
        
        Ok(EthereumEvent {
            id,
            chain: self.chain_id.clone(),
            block_number,
            block_hash,
            tx_hash,
            timestamp,
            event_type,
            address,
            topics,
            data: log.data.to_vec(),
            log_index,
            tx_index,
            raw_log: log,
            decoded_params,
            contract_name,
        })
    }
    
    /// Check if an event matches a filter based on indexer_core::types::EventFilter
    pub fn matches_filter(&self, event: &EthereumEvent, filter: &EventFilter) -> bool {
        // Check chain_id (exact match)
        if let Some(filter_chain_id) = &filter.chain_id {
             if filter_chain_id != &event.chain {
                 return false;
             }
        }
        
        // Check chain name (exact match, case sensitive)
        if let Some(filter_chain_name) = &filter.chain {
             if filter_chain_name != &event.chain {
                 // Assuming event.chain stores the name used for filtering
                 return false;
             }
        }
        
        // Check block range
        if let Some((min_block, max_block)) = filter.block_range {
            if event.block_number < min_block || event.block_number > max_block {
                return false;
            }
        }
        
        // Check time range (Unix seconds)
        if let Some((min_time, max_time)) = filter.time_range {
            if event.timestamp < min_time || event.timestamp > max_time {
                 return false;
            }
        }
        
        // Check event types (matches if event.event_type is in the list)
        if let Some(event_types) = &filter.event_types {
            if !event_types.is_empty() && !event_types.contains(&event.event_type) {
                return false;
            }
        }
        
        // Check custom filters (specifically looking for "address")
        if let Some(filter_address) = filter.custom_filters.get("address") {
            // Perform case-insensitive comparison for Ethereum addresses
            if !event.address.eq_ignore_ascii_case(filter_address) {
                 return false;
            }
        }
        
        // TODO: Implement matching for other custom_filters if needed

        // If all checks passed, the event matches the filter
        true
    }
    
    /// Process logs from a block
    pub fn process_block_logs(
        &self,
        block: &Block<Transaction>,
        logs: Vec<Log>,
        receipts: HashMap<H256, TransactionReceipt>
    ) -> Result<Vec<EthereumEvent>> {
        let mut events = Vec::with_capacity(logs.len());
        
        for log in logs {
            let tx_hash = log.transaction_hash.unwrap_or_default();
            let receipt = receipts.get(&tx_hash);
            
            match self.process_log(log, block, receipt) {
                Ok(event) => events.push(event),
                Err(e) => {
                    error!("Error processing log: {}", e);
                    // Continue processing other logs even if one fails
                    continue;
                }
            }
        }
        
        Ok(events)
    }
}

impl EthereumEvent {
    /// Create a new Ethereum event from a log and block
    pub fn from_log(
        log: Log, 
        block: Block<Transaction>, 
        chain_id: String,
        receipt: Option<TransactionReceipt>
    ) -> Self {
        let timestamp = block.timestamp.as_u64();
        let block_number = block.number.unwrap_or_default().as_u64();
        let block_hash = block.hash.unwrap_or_default().to_string();
        let tx_hash = log.transaction_hash.unwrap_or_default().to_string();
        let log_index = log.log_index.unwrap_or_default().as_u64();
        let tx_index = log.transaction_index.unwrap_or_default().as_u64();
        
        // Generate a unique ID for the event
        let id = format!("{}-{}-{}", chain_id, tx_hash, log_index);
        
        // Determine event type from the first topic if available
        let event_type = if !log.topics.is_empty() {
            format!("0x{}", hex::encode(log.topics[0].as_bytes()))
        } else {
            "unknown".to_string()
        };
        
        // Convert topics to strings
        let topics = log.topics.iter()
            .map(|t| t.to_string())
            .collect();
        
        // Serialize the log to get the raw data
        let raw_data = serde_json::to_vec(&log).unwrap_or_default();
        
        Self {
            id,
            chain: chain_id,
            block_number,
            block_hash,
            tx_hash,
            timestamp,
            event_type,
            address: log.address.to_string(),
            topics,
            data: log.data.to_vec(),
            log_index,
            tx_index,
            raw_log: log,
            decoded_params: None,
            contract_name: None,
        }
    }
} 