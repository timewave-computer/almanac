/// Common test utilities
use std::time::{SystemTime, Duration};
use std::any::Any;

use indexer_core::event::Event;
use async_trait::async_trait;
use rand::{thread_rng, Rng};

/// Assert that a duration is less than an expected maximum
#[allow(dead_code)]
pub fn assert_duration_less_than(actual: Duration, expected_max: Duration, message: &str) {
    if actual > expected_max {
        panic!("{}: actual duration {:?} exceeds max expected {:?}", 
               message, actual, expected_max);
    }
}

/// Create a single mock event for testing
#[allow(dead_code)]
pub fn create_mock_event(id: &str, chain: &str, block_number: u64) -> Box<dyn Event> {
    Box::new(MockEvent {
        id: id.to_string(),
        chain: chain.to_string(),
        block_number,
        block_hash: format!("block_hash_{}", block_number),
        tx_hash: format!("tx_hash_{}", id),
        timestamp: SystemTime::now(),
        event_type: "mock_event".to_string(),
        raw_data: vec![1, 2, 3, 4],
    })
}

/// Create multiple mock events for testing
#[allow(dead_code)]
pub fn create_mock_events(chain: &str, count: usize) -> Vec<Box<dyn Event>> {
    let mut events = Vec::with_capacity(count);
    
    for i in 0..count {
        let id = format!("event_{}", i);
        let block_number = 100 + i as u64;
        events.push(create_mock_event(&id, chain, block_number));
    }
    
    events
}

/// Mock event for testing
#[derive(Debug, Clone)]
pub struct MockEvent {
    pub id: String,
    pub chain: String,
    pub block_number: u64,
    pub block_hash: String,
    pub tx_hash: String,
    pub timestamp: SystemTime,
    pub event_type: String,
    pub raw_data: Vec<u8>,
}

#[async_trait]
impl Event for MockEvent {
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
        self.timestamp
    }
    
    fn event_type(&self) -> &str {
        &self.event_type
    }
    
    fn raw_data(&self) -> &[u8] {
        &self.raw_data
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Cloned event - workaround for Box<dyn Event> not implementing Clone
#[allow(dead_code)]
pub struct ClonedEvent(pub Box<dyn Event>);

impl ClonedEvent {
    #[allow(dead_code)]
    pub fn new(event: &Box<dyn Event>) -> Self {
        Self(Box::new(MockEvent {
            id: event.id().to_string(),
            chain: event.chain().to_string(),
            block_number: event.block_number(),
            block_hash: event.block_hash().to_string(),
            tx_hash: event.tx_hash().to_string(),
            timestamp: event.timestamp(),
            event_type: event.event_type().to_string(),
            raw_data: event.raw_data().to_vec(),
        }))
    }
    
    #[allow(dead_code)]
    pub fn into_event(self) -> Box<dyn Event> {
        self.0
    }
}

/// Generate random string for testing
#[allow(dead_code)]
pub fn random_string(length: usize) -> String {
    use rand::distributions::Alphanumeric;
    
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Generate a simple timestamp for testing
#[allow(dead_code)]
pub fn random_timestamp() -> SystemTime {
    let seconds_offset = thread_rng().gen_range(0..10_000_000);
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds_offset)
}

/// Generate random data for testing
#[allow(dead_code)]
pub fn random_data(min_size: usize, max_size: usize) -> Vec<u8> {
    let size = thread_rng().gen_range(min_size..=max_size);
    let mut data = Vec::with_capacity(size);
    
    for _ in 0..size {
        data.push(thread_rng().gen::<u8>());
    }
    
    data
} 