//! In-memory storage implementation for testing

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;
use almanac_core::{Event, EventFilter, Storage, BlockStatus, Result, Error};

/// In-memory storage implementation
pub struct MemoryStorage {
    events: RwLock<Vec<Box<dyn Event>>>,
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
impl Storage for MemoryStorage {
    async fn store_event(&self, event: Box<dyn Event>) -> Result<()> {
        let mut events = self.events.write()
            .map_err(|_| Error::storage("Failed to acquire write lock"))?;
        
        // Update latest block
        let mut blocks = self.blocks.write()
            .map_err(|_| Error::storage("Failed to acquire blocks write lock"))?;
        let chain = event.chain().to_string();
        let block_number = event.block_number();
        
        blocks.entry(chain)
            .and_modify(|b| *b = (*b).max(block_number))
            .or_insert(block_number);
        
        events.push(event);
        Ok(())
    }
    
    async fn get_events(&self, filters: Vec<EventFilter>) -> Result<Vec<Box<dyn Event>>> {
        let events = self.events.read()
            .map_err(|_| Error::storage("Failed to acquire read lock"))?;
        
        let mut result = Vec::new();
        
        for event in events.iter() {
            let mut matches = true;
            
            for filter in &filters {
                if !filter.matches_chain(event.chain()) ||
                   !filter.matches_block(event.block_number()) ||
                   !filter.matches_event_type(event.event_type()) ||
                   !filter.matches_address(event.contract_address()) ||
                   !filter.matches_tx_hash(event.tx_hash()) {
                    matches = false;
                    break;
                }
            }
            
            if matches {
                // Clone the event - this is inefficient but works for testing
                result.push(create_event_clone(event.as_ref()));
            }
        }
        
        // Apply pagination
        if let Some(filter) = filters.first() {
            if let Some(offset) = filter.offset {
                result = result.into_iter().skip(offset).collect();
            }
            if let Some(limit) = filter.limit {
                result.truncate(limit);
            }
        }
        
        Ok(result)
    }
    
    async fn get_latest_block(&self, chain: &str) -> Result<u64> {
        let blocks = self.blocks.read()
            .map_err(|_| Error::storage("Failed to acquire read lock"))?;
        
        blocks.get(chain)
            .copied()
            .ok_or_else(|| Error::storage(format!("No blocks found for chain {}", chain)))
    }
    
    async fn get_latest_block_with_status(&self, chain: &str, status: BlockStatus) -> Result<u64> {
        let block_status = self.block_status.read()
            .map_err(|_| Error::storage("Failed to acquire read lock"))?;
        
        let mut latest = 0u64;
        for ((c, block), s) in block_status.iter() {
            if c == chain && *s == status && *block > latest {
                latest = *block;
            }
        }
        
        if latest == 0 {
            Err(Error::storage(format!("No blocks with status {:?} found for chain {}", status, chain)))
        } else {
            Ok(latest)
        }
    }
    
    async fn update_block_status(&self, chain: &str, block_number: u64, status: BlockStatus) -> Result<()> {
        let mut block_status = self.block_status.write()
            .map_err(|_| Error::storage("Failed to acquire write lock"))?;
        
        block_status.insert((chain.to_string(), block_number), status);
        Ok(())
    }
    
    async fn get_events_with_status(&self, filters: Vec<EventFilter>, status: BlockStatus) -> Result<Vec<Box<dyn Event>>> {
        let events = self.get_events(filters).await?;
        let block_status = self.block_status.read()
            .map_err(|_| Error::storage("Failed to acquire read lock"))?;
        
        let filtered: Vec<_> = events.into_iter()
            .filter(|e| {
                block_status.get(&(e.chain().to_string(), e.block_number()))
                    .is_some_and(|s| *s == status)
            })
            .collect();
        
        Ok(filtered)
    }
    
    async fn reorg_chain(&self, chain: &str, from_block: u64) -> Result<()> {
        // Remove events from the specified block onwards
        let mut events = self.events.write()
            .map_err(|_| Error::storage("Failed to acquire write lock"))?;
        
        events.retain(|e| !(e.chain() == chain && e.block_number() >= from_block));
        
        // Update block status
        let mut block_status = self.block_status.write()
            .map_err(|_| Error::storage("Failed to acquire write lock"))?;
        
        let keys_to_update: Vec<_> = block_status.keys()
            .filter(|(c, b)| c == chain && *b >= from_block)
            .cloned()
            .collect();
        
        for key in keys_to_update {
            block_status.insert(key, BlockStatus::Reorged);
        }
        
        Ok(())
    }
}

// Helper function to clone events (simplified for testing)
fn create_event_clone(event: &dyn Event) -> Box<dyn Event> {
    use almanac_core::UnifiedEvent;
    
    Box::new(UnifiedEvent {
        id: event.id().to_string(),
        chain: event.chain().to_string(),
        block_number: event.block_number(),
        block_hash: event.block_hash().to_string(),
        tx_hash: event.tx_hash().to_string(),
        timestamp: event.timestamp(),
        event_type: event.event_type().to_string(),
        contract_address: Some(event.contract_address().to_string()).filter(|s| !s.is_empty()),
        raw_data: event.raw_data().to_vec(),
        attributes: event.attributes(),
        source: almanac_core::EventSource::Chain,
    })
}