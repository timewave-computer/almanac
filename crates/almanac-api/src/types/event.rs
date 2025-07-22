use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Minimal Event trait with only essential methods
pub trait ApiEventTrait: Send + Sync {
    /// Get the chain this event belongs to
    fn chain(&self) -> &str;

    /// Get the block number this event was emitted in
    fn block_number(&self) -> u64;

    /// Get the event type identifier
    fn event_type(&self) -> &str;

    /// Get the raw event data
    fn data(&self) -> &[u8];
}

/// Simple API event representation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiEvent {
    /// Unique event ID
    pub id: String,

    /// Chain identifier
    pub chain: String,

    /// Block number
    pub block_number: u64,

    /// Event type
    pub event_type: String,

    /// Transaction hash (optional)
    pub tx_hash: Option<String>,

    /// Event index in block
    pub event_index: u32,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Raw event data (encoded)
    #[serde(with = "base64")]
    pub data: Vec<u8>,
}

impl ApiEventTrait for ApiEvent {
    fn chain(&self) -> &str {
        &self.chain
    }

    fn block_number(&self) -> u64 {
        self.block_number
    }

    fn event_type(&self) -> &str {
        &self.event_type
    }

    fn data(&self) -> &[u8] {
        &self.data
    }
}

/// Base64 encoding for binary data in JSON
mod base64 {
    use serde::{Deserialize, Serialize};
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        use base64::{engine::general_purpose, Engine as _};
        let base64 = general_purpose::STANDARD.encode(v);
        String::serialize(&base64, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        use base64::{engine::general_purpose, Engine as _};
        let base64 = String::deserialize(d)?;
        general_purpose::STANDARD
            .decode(base64)
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

impl ApiEvent {
    /// Create a new API event
    pub fn new(
        chain: impl Into<String>,
        block_number: u64,
        event_type: impl Into<String>,
        data: Vec<u8>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            chain: chain.into(),
            block_number,
            event_type: event_type.into(),
            tx_hash: None,
            event_index: 0,
            timestamp: Utc::now(),
            data,
        }
    }

    /// Set the transaction hash
    pub fn with_tx_hash(mut self, tx_hash: impl Into<String>) -> Self {
        self.tx_hash = Some(tx_hash.into());
        self
    }

    /// Set the event index
    pub fn with_event_index(mut self, index: u32) -> Self {
        self.event_index = index;
        self
    }

    /// Set the timestamp
    pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = timestamp;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_event_creation() {
        let event = ApiEvent::new("ethereum", 12345, "Transfer", vec![1, 2, 3]);

        assert_eq!(event.chain(), "ethereum");
        assert_eq!(event.block_number(), 12345);
        assert_eq!(event.event_type(), "Transfer");
        assert_eq!(event.data(), &[1, 2, 3]);
    }

    #[test]
    fn test_api_event_serialization() {
        let event = ApiEvent::new("ethereum", 12345, "Transfer", vec![1, 2, 3])
            .with_tx_hash("0x123")
            .with_event_index(5);

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ApiEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.chain, event.chain);
        assert_eq!(deserialized.tx_hash, Some("0x123".to_string()));
        assert_eq!(deserialized.event_index, 5);
    }
}
