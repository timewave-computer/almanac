//! In-memory storage implementation for testing

use almanac_core::{BlockStatus, Event, EventFilter, Result, StorageReader, StorageWriter};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::SystemTime;

/// Stored event data that can be cloned and filtered
#[derive(Debug, Clone)]
struct StoredEvent {
    id: String,
    chain: String,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    event_type: String,
    raw_data: Vec<u8>,
    timestamp: SystemTime,
}

impl StoredEvent {
    fn from_event(event: &dyn Event) -> Self {
        Self {
            id: event.id().to_string(),
            chain: event.chain().to_string(),
            block_number: event.block_number(),
            block_hash: event.block_hash().to_string(),
            tx_hash: event.tx_hash().to_string(),
            event_type: event.event_type().to_string(),
            raw_data: event.raw_data().to_vec(),
            timestamp: event.timestamp(),
        }
    }

    fn matches_filter(&self, filter: &EventFilter) -> bool {
        // Check chain filter
        if let Some(ref chain) = filter.chain {
            if self.chain != *chain {
                return false;
            }
        }

        // Check block range
        if let Some((from, to)) = filter.block_range {
            if self.block_number < from || self.block_number > to {
                return false;
            }
        }

        // Check time range
        if let Some((from, to)) = filter.time_range {
            let timestamp_secs = self
                .timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if timestamp_secs < from || timestamp_secs > to {
                return false;
            }
        }

        // Check event types
        if let Some(ref event_types) = filter.event_types {
            if !event_types.contains(&self.event_type) {
                return false;
            }
        }

        // Check addresses
        if let Some(ref _addresses) = filter.addresses {
            // For now, we don't have address info in stored events, so skip this filter
            // In a real implementation, we'd need to store and check contract addresses
        }

        // Check tx hashes
        if let Some(ref tx_hashes) = filter.tx_hashes {
            if !tx_hashes.contains(&self.tx_hash) {
                return false;
            }
        }

        true
    }
}

/// Simple event implementation for returning from memory storage
#[derive(Debug, Clone)]
pub struct MemoryEvent {
    id: String,
    chain: String,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    event_type: String,
    raw_data: Vec<u8>,
    timestamp: SystemTime,
}

impl Event for MemoryEvent {
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
    fn event_type(&self) -> &str {
        &self.event_type
    }
    fn raw_data(&self) -> &[u8] {
        &self.raw_data
    }
    fn timestamp(&self) -> SystemTime {
        self.timestamp
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// In-memory storage implementation
pub struct MemoryStorage {
    events: RwLock<Vec<StoredEvent>>,
    blocks: RwLock<HashMap<String, u64>>,
    block_status: RwLock<HashMap<(String, u64), BlockStatus>>,
}

impl MemoryStorage {
    /// Create a new in-memory storage
    pub fn new() -> Self {
        Self {
            events: RwLock::new(Vec::new()),
            blocks: RwLock::new(HashMap::new()),
            block_status: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageWriter for MemoryStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        let stored_event = StoredEvent::from_event(event.as_ref());

        // Update latest block for this chain
        {
            let mut blocks = self.blocks.write().unwrap();
            let current = blocks.get(&stored_event.chain).copied().unwrap_or(0);
            if stored_event.block_number > current {
                blocks.insert(stored_event.chain.clone(), stored_event.block_number);
            }
        }

        // Store the event
        {
            let mut events = self.events.write().unwrap();
            events.push(stored_event);
        }

        Ok(())
    }

    async fn store_events_batch(&self, events: Vec<Box<dyn Event>>) -> Result<()> {
        let mut blocks = self.blocks.write().unwrap();
        let mut storage_events = self.events.write().unwrap();

        for event in events {
            let stored_event = StoredEvent::from_event(event.as_ref());

            // Update latest block for this chain
            let current = blocks.get(&stored_event.chain).copied().unwrap_or(0);
            if stored_event.block_number > current {
                blocks.insert(stored_event.chain.clone(), stored_event.block_number);
            }

            storage_events.push(stored_event);
        }
        Ok(())
    }

    async fn update_block_status(
        &self,
        chain: &str,
        block_number: u64,
        status: BlockStatus,
    ) -> Result<()> {
        let mut block_status = self.block_status.write().unwrap();
        block_status.insert((chain.to_string(), block_number), status);

        // Also update latest block
        let mut blocks = self.blocks.write().unwrap();
        let current = blocks.get(chain).copied().unwrap_or(0);
        if block_number > current {
            blocks.insert(chain.to_string(), block_number);
        }

        Ok(())
    }

    async fn reorg_chain(&self, chain: &str, from_block: u64) -> Result<()> {
        // Remove all block status entries from from_block onwards
        let mut block_status = self.block_status.write().unwrap();
        block_status.retain(|(c, b), _| c != chain || *b < from_block);

        // Update latest block
        let mut blocks = self.blocks.write().unwrap();
        if let Some(current) = blocks.get_mut(chain) {
            if *current >= from_block {
                *current = from_block.saturating_sub(1);
            }
        }

        Ok(())
    }

    async fn health_check(&self) -> Result<()> {
        Ok(()) // Memory storage is always healthy
    }
}

#[async_trait]
impl StorageReader for MemoryStorage {
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        let events = self.events.read().unwrap();

        let matching_events: Vec<&StoredEvent> = if filters.is_empty() {
            events.iter().collect()
        } else {
            events
                .iter()
                .filter(|stored_event| {
                    filters
                        .iter()
                        .any(|filter| stored_event.matches_filter(filter))
                })
                .collect()
        };

        // Apply limit and offset based on the first filter (if any)
        let mut result_events = matching_events;
        if let Some(filter) = filters.first() {
            if let Some(offset) = filter.offset {
                result_events = result_events.into_iter().skip(offset).collect();
            }
            if let Some(limit) = filter.limit {
                result_events = result_events.into_iter().take(limit).collect();
            }
        }

        let result: Vec<Box<dyn Event>> = result_events
            .into_iter()
            .map(|stored_event| {
                Box::new(MemoryEvent {
                    id: stored_event.id.clone(),
                    chain: stored_event.chain.clone(),
                    block_number: stored_event.block_number,
                    block_hash: stored_event.block_hash.clone(),
                    tx_hash: stored_event.tx_hash.clone(),
                    event_type: stored_event.event_type.clone(),
                    raw_data: stored_event.raw_data.clone(),
                    timestamp: stored_event.timestamp,
                }) as Box<dyn Event>
            })
            .collect();

        Ok(result)
    }

    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        let blocks = self.blocks.read().unwrap();
        Ok(blocks.get(chain).copied().unwrap_or(0))
    }

    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64> {
        let block_status = self.block_status.read().unwrap();
        let latest = block_status
            .iter()
            .filter(|((c, _), s)| c == chain && **s == status)
            .map(|((_, b), _)| *b)
            .max()
            .unwrap_or(0);
        Ok(latest)
    }

    async fn get_events_with_status(
        &self,
        filters: Vec<EventFilter>,
        _status: BlockStatus,
    ) -> Result<Vec<Box<dyn Event>>> {
        // Delegate to get_events for now
        self.get_events(filters).await
    }

    async fn get_latest_block_before(&self, chain: &str, before_block: u64) -> Result<u64> {
        let blocks = self.blocks.read().unwrap();
        let current = blocks.get(chain).copied().unwrap_or(0);
        Ok(current.min(before_block.saturating_sub(1)))
    }

    async fn health_check(&self) -> Result<()> {
        Ok(()) // Memory storage is always healthy
    }
}
