/// WebSocket API tests
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use tokio::time::Duration;

use indexer_api::{
    websocket::{WsMessage, EventFilters, EventData, InMemorySubscriptionStorage, PersistedSubscription},
};
use indexer_core::{
    event::Event,
};

// Mock event for testing
#[derive(Debug, Clone)]
struct MockEvent {
    id: String,
    chain: String,
    block_number: u64,
    block_hash: String,
    tx_hash: String,
    timestamp: SystemTime,
    event_type: String,
    raw_data: Vec<u8>,
}

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
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn create_test_event(id: &str, chain: &str, block_number: u64, event_type: &str) -> MockEvent {
    MockEvent {
        id: id.to_string(),
        chain: chain.to_string(),
        block_number,
        block_hash: format!("0x{:064x}", block_number),
        tx_hash: format!("0x{:064x}", block_number + 1000),
        timestamp: UNIX_EPOCH + Duration::from_secs(1640000000 + block_number * 60),
        event_type: event_type.to_string(),
        raw_data: format!("event-data-{}", id).into_bytes(),
    }
}

#[test]
fn test_ws_message_serialization() {
    // Test Subscribe message
    let subscribe_msg = WsMessage::Subscribe {
        id: "sub-1".to_string(),
        filters: EventFilters {
            chain_id: Some("ethereum".to_string()),
            address: Some("0x123".to_string()),
            event_type: Some("Transfer".to_string()),
            block_range: Some((100, 200)),
            attributes: None,
            limit: Some(10),
        },
    };
    
    let json = serde_json::to_string(&subscribe_msg).unwrap();
    assert!(json.contains("subscribe"));
    assert!(json.contains("sub-1"));
    assert!(json.contains("ethereum"));
    assert!(json.contains("0x123"));
    assert!(json.contains("Transfer"));
    
    // Test deserialization
    let deserialized = serde_json::from_str::<WsMessage>(&json).unwrap();
    match deserialized {
        WsMessage::Subscribe { id, filters } => {
            assert_eq!(id, "sub-1");
            assert_eq!(filters.chain_id, Some("ethereum".to_string()));
            assert_eq!(filters.address, Some("0x123".to_string()));
            assert_eq!(filters.event_type, Some("Transfer".to_string()));
            assert_eq!(filters.block_range, Some((100, 200)));
            assert_eq!(filters.limit, Some(10));
        }
        _ => panic!("Expected Subscribe message"),
    }
}

#[test]
fn test_event_filters_validation() {
    // Test valid filter
    let valid_filter = EventFilters {
        chain_id: Some("ethereum".to_string()),
        address: Some("0x123".to_string()),
        event_type: Some("Transfer".to_string()),
        block_range: Some((100, 200)),
        attributes: Some(HashMap::new()),
        limit: Some(10),
    };
    
    assert_eq!(valid_filter.chain_id, Some("ethereum".to_string()));
    assert_eq!(valid_filter.address, Some("0x123".to_string()));
    assert_eq!(valid_filter.event_type, Some("Transfer".to_string()));
    assert_eq!(valid_filter.block_range, Some((100, 200)));
    assert_eq!(valid_filter.limit, Some(10));
    
    // Test empty filter
    let empty_filter = EventFilters {
        chain_id: None,
        address: None,
        event_type: None,
        block_range: None,
        attributes: None,
        limit: None,
    };
    
    assert!(empty_filter.chain_id.is_none());
    assert!(empty_filter.address.is_none());
    assert!(empty_filter.event_type.is_none());
    assert!(empty_filter.block_range.is_none());
    assert!(empty_filter.attributes.is_none());
    assert!(empty_filter.limit.is_none());
}

#[test]
fn test_event_data_creation() {
    let event = create_test_event("event-1", "ethereum", 100, "Transfer");
    let event_data = EventData::from(&event as &dyn Event);
    
    assert_eq!(event_data.id, "event-1");
    assert_eq!(event_data.chain_id, "ethereum");
    assert_eq!(event_data.block_number, 100);
    assert_eq!(event_data.event_type, "Transfer");
    assert!(!event_data.raw_data.is_empty());
}

#[tokio::test]
async fn test_subscription_storage() {
    let storage = InMemorySubscriptionStorage::new();
    
    // Create a test subscription
    let subscription = PersistedSubscription {
        id: "sub-1".to_string(),
        connection_id: "conn-1".to_string(),
        user_id: Some("user-1".to_string()),
        filters: EventFilters {
            chain_id: Some("ethereum".to_string()),
            address: None,
            event_type: Some("Transfer".to_string()),
            block_range: None,
            attributes: None,
            limit: Some(100),
        },
        created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        event_count: 0,
        active: true,
    };
    
    // Test saving subscription
    let result = storage.save_subscription(&subscription).await;
    assert!(result.is_ok());
    
    // Test loading subscriptions for connection
    let loaded = storage.load_subscriptions("conn-1").await.unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "sub-1");
    assert_eq!(loaded[0].connection_id, "conn-1");
    
    // Test updating event count
    let result = storage.update_subscription_count("sub-1", 5).await;
    assert!(result.is_ok());
    
    // Test deactivating subscription
    let result = storage.deactivate_subscription("sub-1").await;
    assert!(result.is_ok());
    
    // Test that deactivated subscription is not loaded
    let loaded = storage.load_subscriptions("conn-1").await.unwrap();
    assert!(loaded.is_empty());
}

#[test]
fn test_ws_message_types() {
    // Test all message types can be created and serialized
    
    // Subscribe message
    let subscribe = WsMessage::Subscribe {
        id: "sub-1".to_string(),
        filters: EventFilters {
            chain_id: Some("ethereum".to_string()),
            address: None,
            event_type: None,
            block_range: None,
            attributes: None,
            limit: None,
        },
    };
    assert!(serde_json::to_string(&subscribe).is_ok());
    
    // Unsubscribe message
    let unsubscribe = WsMessage::Unsubscribe {
        id: "sub-1".to_string(),
    };
    assert!(serde_json::to_string(&unsubscribe).is_ok());
    
    // Event message
    let event = WsMessage::Event {
        subscription_id: "sub-1".to_string(),
        event: EventData {
            id: "event-1".to_string(),
            chain_id: "ethereum".to_string(),
            block_number: 100,
            block_hash: "0xabc".to_string(),
            tx_hash: "0xdef".to_string(),
            event_type: "Transfer".to_string(),
            timestamp: 1640000000,
            raw_data: "dGVzdA==".to_string(), // base64 encoded "test"
            attributes: HashMap::new(),
        },
    };
    assert!(serde_json::to_string(&event).is_ok());
    
    // Error message
    let error = WsMessage::Error {
        id: Some("req-1".to_string()),
        error: "Invalid filter".to_string(),
        code: 400,
    };
    assert!(serde_json::to_string(&error).is_ok());
    
    // Ping message
    let ping = WsMessage::Ping {
        timestamp: 1640000000,
    };
    assert!(serde_json::to_string(&ping).is_ok());
    
    // Pong message
    let pong = WsMessage::Pong {
        timestamp: 1640000000,
    };
    assert!(serde_json::to_string(&pong).is_ok());
    
    // Auth message
    let auth = WsMessage::Auth {
        token: "jwt-token".to_string(),
    };
    assert!(serde_json::to_string(&auth).is_ok());
    
    // AuthResponse message
    let auth_response = WsMessage::AuthResponse {
        authenticated: true,
        user: Some("user-1".to_string()),
        role: Some("admin".to_string()),
    };
    assert!(serde_json::to_string(&auth_response).is_ok());
}

#[test]
fn test_event_filters_json() {
    let filters = EventFilters {
        chain_id: Some("ethereum".to_string()),
        address: Some("0x123".to_string()),
        event_type: Some("Transfer".to_string()),
        block_range: Some((100, 200)),
        attributes: Some({
            let mut attrs = HashMap::new();
            attrs.insert("from".to_string(), json!("0x456"));
            attrs.insert("to".to_string(), json!("0x789"));
            attrs
        }),
        limit: Some(50),
    };
    
    // Test serialization
    let json = serde_json::to_string(&filters).unwrap();
    assert!(json.contains("ethereum"));
    assert!(json.contains("0x123"));
    assert!(json.contains("Transfer"));
    assert!(json.contains("100"));
    assert!(json.contains("200"));
    assert!(json.contains("0x456"));
    assert!(json.contains("0x789"));
    assert!(json.contains("50"));
    
    // Test deserialization
    let deserialized = serde_json::from_str::<EventFilters>(&json).unwrap();
    assert_eq!(deserialized.chain_id, Some("ethereum".to_string()));
    assert_eq!(deserialized.address, Some("0x123".to_string()));
    assert_eq!(deserialized.event_type, Some("Transfer".to_string()));
    assert_eq!(deserialized.block_range, Some((100, 200)));
    assert_eq!(deserialized.limit, Some(50));
    
    if let Some(attrs) = deserialized.attributes {
        assert_eq!(attrs.get("from"), Some(&json!("0x456")));
        assert_eq!(attrs.get("to"), Some(&json!("0x789")));
    } else {
        panic!("Attributes should be present");
    }
}

#[test]
fn test_event_data_json() {
    let event_data = EventData {
        id: "event-123".to_string(),
        chain_id: "ethereum".to_string(),
        block_number: 1000,
        block_hash: "0xabc123".to_string(),
        tx_hash: "0xdef456".to_string(),
        event_type: "Transfer".to_string(),
        timestamp: 1640000000,
        raw_data: "dGVzdCBkYXRh".to_string(), // base64 encoded "test data"
        attributes: {
            let mut attrs = HashMap::new();
            attrs.insert("from".to_string(), json!("0x111"));
            attrs.insert("to".to_string(), json!("0x222"));
            attrs.insert("value".to_string(), json!("1000000000000000000"));
            attrs
        },
    };
    
    // Test serialization
    let json = serde_json::to_string(&event_data).unwrap();
    assert!(json.contains("event-123"));
    assert!(json.contains("ethereum"));
    assert!(json.contains("1000"));
    assert!(json.contains("Transfer"));
    assert!(json.contains("0x111"));
    assert!(json.contains("0x222"));
    assert!(json.contains("1000000000000000000"));
    
    // Test deserialization
    let deserialized = serde_json::from_str::<EventData>(&json).unwrap();
    assert_eq!(deserialized.id, "event-123");
    assert_eq!(deserialized.chain_id, "ethereum");
    assert_eq!(deserialized.block_number, 1000);
    assert_eq!(deserialized.event_type, "Transfer");
    assert_eq!(deserialized.timestamp, 1640000000);
    
    assert_eq!(deserialized.attributes.get("from"), Some(&json!("0x111")));
    assert_eq!(deserialized.attributes.get("to"), Some(&json!("0x222")));
    assert_eq!(deserialized.attributes.get("value"), Some(&json!("1000000000000000000")));
}

#[tokio::test]
async fn test_subscription_cleanup() {
    let storage = InMemorySubscriptionStorage::new();
    
    // Create old and new subscriptions
    let old_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() - (25 * 3600); // 25 hours ago
    
    let new_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() - 3600; // 1 hour ago
    
    let old_subscription = PersistedSubscription {
        id: "old-sub".to_string(),
        connection_id: "conn-1".to_string(),
        user_id: None,
        filters: EventFilters {
            chain_id: Some("ethereum".to_string()),
            address: None,
            event_type: None,
            block_range: None,
            attributes: None,
            limit: None,
        },
        created_at: old_time,
        event_count: 0,
        active: false, // Inactive old subscription
    };
    
    let new_subscription = PersistedSubscription {
        id: "new-sub".to_string(),
        connection_id: "conn-2".to_string(),
        user_id: None,
        filters: EventFilters {
            chain_id: Some("ethereum".to_string()),
            address: None,
            event_type: None,
            block_range: None,
            attributes: None,
            limit: None,
        },
        created_at: new_time,
        event_count: 0,
        active: true, // Active new subscription
    };
    
    // Save both subscriptions
    storage.save_subscription(&old_subscription).await.unwrap();
    storage.save_subscription(&new_subscription).await.unwrap();
    
    // Verify both are present
    let all_subs = storage.load_all_subscriptions().await.unwrap();
    assert_eq!(all_subs.len(), 1); // Only active subscription
    
    // Clean up old subscriptions (older than 24 hours)
    storage.cleanup_old_subscriptions(24).await.unwrap();
    
    // Verify old subscription was removed and new one remains
    let remaining_subs = storage.load_all_subscriptions().await.unwrap();
    assert_eq!(remaining_subs.len(), 1);
    assert_eq!(remaining_subs[0].id, "new-sub");
} 