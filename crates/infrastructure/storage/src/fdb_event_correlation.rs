use async_trait::async_trait;
use foundationdb::{Database, FdbResult, KeySelector, RangeOption, Transaction};
use futures::TryStreamExt;
use indexer_core::distributed_correlation::{
    EventCorrelation, EventCorrelationService, EventReference, CorrelationStats,
};
use indexer_core::{Event, Result};
use serde_json;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

const CORRELATIONS_PREFIX: &[u8] = b"/correlations/";
const EVENT_CORRELATIONS_PREFIX: &[u8] = b"/event_correlations/";
const CORRELATION_INDEX_PREFIX: &[u8] = b"/correlation_index/";

pub struct FdbEventCorrelationService {
    db: Database,
    namespace: String,
}

impl FdbEventCorrelationService {
    pub fn new(db: Database, namespace: String) -> Self {
        Self { db, namespace }
    }

    fn namespace_key(&self, prefix: &[u8], key: &[u8]) -> Vec<u8> {
        let mut full_key = Vec::with_capacity(self.namespace.len() + prefix.len() + key.len() + 2);
        full_key.push(b'/');
        full_key.extend_from_slice(self.namespace.as_bytes());
        full_key.extend_from_slice(prefix);
        full_key.extend_from_slice(key);
        full_key
    }

    fn correlation_key(&self, correlation_id: &str) -> Vec<u8> {
        self.namespace_key(CORRELATIONS_PREFIX, correlation_id.as_bytes())
    }

    fn event_correlation_key(&self, event_ref: &EventReference) -> Vec<u8> {
        let key = format!(
            "{}/{}/{}/{}",
            event_ref.chain_id,
            event_ref.block_number,
            event_ref.transaction_hash,
            event_ref.log_index
        );
        self.namespace_key(EVENT_CORRELATIONS_PREFIX, key.as_bytes())
    }

    fn correlation_index_key(&self, correlation_type: &str, correlation_id: &str) -> Vec<u8> {
        let key = format!("{}/{}", correlation_type, correlation_id);
        self.namespace_key(CORRELATION_INDEX_PREFIX, key.as_bytes())
    }

    async fn transact<F, R>(&self, f: F) -> FdbResult<R>
    where
        F: Fn(Transaction) -> FdbResult<R>,
    {
        self.db.run(f).await
    }

    fn event_to_reference(event: &dyn Event) -> EventReference {
        EventReference {
            chain_id: event.chain_id(),
            block_number: event.block_number(),
            transaction_hash: event.transaction_hash().unwrap_or_default(),
            log_index: event.log_index().unwrap_or(0),
            event_type: format!("{:?}", event.event_type()),
        }
    }
}

#[async_trait]
impl EventCorrelationService for FdbEventCorrelationService {
    async fn correlate_event(&self, event: &dyn Event) -> Result<Vec<EventCorrelation>> {
        let event_ref = Self::event_to_reference(event);
        
        // Look for existing correlations involving this event
        let existing_correlations = self.get_correlations(&event_ref).await?;
        
        // Here we would implement correlation logic based on event data
        // For now, return existing correlations
        Ok(existing_correlations)
    }

    async fn get_correlations(&self, event_ref: &EventReference) -> Result<Vec<EventCorrelation>> {
        let key = self.event_correlation_key(event_ref);
        
        let correlation_ids = self
            .transact(|trx| {
                let get_future = trx.get(&key, false);
                
                futures::executor::block_on(async {
                    match get_future.await {
                        Ok(Some(value)) => {
                            serde_json::from_slice::<Vec<String>>(&value).unwrap_or_default()
                        }
                        _ => Vec::new(),
                    }
                })
            })
            .await
            .map_err(|e| indexer_core::Error::Generic(format!("FDB error: {}", e)))?;

        let mut correlations = Vec::new();
        for correlation_id in correlation_ids {
            let key = self.correlation_key(&correlation_id);
            
            let result = self
                .transact(|trx| Ok(trx.get(&key, false)))
                .await
                .map_err(|e| indexer_core::Error::Generic(format!("FDB error: {}", e)))?
                .await
                .map_err(|e| indexer_core::Error::Generic(format!("FDB error: {}", e)))?;
                
            if let Some(value) = result {
                if let Ok(correlation) = serde_json::from_slice::<EventCorrelation>(&value) {
                    correlations.push(correlation);
                }
            }
        }

        Ok(correlations)
    }

    async fn store_correlation(&self, correlation: EventCorrelation) -> Result<()> {
        let correlation_key = self.correlation_key(&correlation.correlation_id);
        let correlation_value = serde_json::to_vec(&correlation)
            .map_err(|e| indexer_core::Error::Serialization(e.to_string()))?;

        self.transact(|mut trx| {
            // Store the correlation
            trx.set(&correlation_key, &correlation_value);
            
            // Update event correlation indices for source event
            let source_key = self.event_correlation_key(&correlation.source_event);
            let get_source = trx.get(&source_key, false);
            
            futures::executor::block_on(async {
                let mut correlation_ids = match get_source.await {
                    Ok(Some(value)) => {
                        serde_json::from_slice::<Vec<String>>(&value).unwrap_or_default()
                    }
                    _ => Vec::new(),
                };
                
                if !correlation_ids.contains(&correlation.correlation_id) {
                    correlation_ids.push(correlation.correlation_id.clone());
                    if let Ok(value) = serde_json::to_vec(&correlation_ids) {
                        trx.set(&source_key, &value);
                    }
                }
            });
            
            // Update indices for target events
            for target_event in &correlation.target_events {
                let target_key = self.event_correlation_key(target_event);
                let get_target = trx.get(&target_key, false);
                
                futures::executor::block_on(async {
                    let mut correlation_ids = match get_target.await {
                        Ok(Some(value)) => {
                            serde_json::from_slice::<Vec<String>>(&value).unwrap_or_default()
                        }
                        _ => Vec::new(),
                    };
                    
                    if !correlation_ids.contains(&correlation.correlation_id) {
                        correlation_ids.push(correlation.correlation_id.clone());
                        if let Ok(value) = serde_json::to_vec(&correlation_ids) {
                            trx.set(&target_key, &value);
                        }
                    }
                });
            }
            
            // Update correlation type index
            let correlation_type = format!("{:?}", correlation.correlation_type);
            let index_key = self.correlation_index_key(&correlation_type, &correlation.correlation_id);
            trx.set(&index_key, b"1");
            
            Ok(())
        })
        .await
        .map_err(|e| indexer_core::Error::Generic(format!("FDB error: {}", e)))?;

        info!(
            "Stored correlation {} with {} target events",
            correlation.correlation_id,
            correlation.target_events.len()
        );

        Ok(())
    }

    async fn find_correlated_events(&self, correlation_id: &str) -> Result<Vec<EventCorrelation>> {
        let key = self.correlation_key(correlation_id);
        
        let result = self
            .transact(|trx| Ok(trx.get(&key, false)))
            .await
            .map_err(|e| indexer_core::Error::Generic(format!("FDB error: {}", e)))?
            .await
            .map_err(|e| indexer_core::Error::Generic(format!("FDB error: {}", e)))?;
            
        match result {
            Some(value) => {
                let correlation = serde_json::from_slice(&value)
                    .map_err(|e| indexer_core::Error::Serialization(e.to_string()))?;
                Ok(vec![correlation])
            }
            None => Ok(Vec::new()),
        }
    }

    async fn get_correlation_stats(&self) -> Result<CorrelationStats> {
        let start_key = self.namespace_key(CORRELATIONS_PREFIX, b"");
        let end_key = self.namespace_key(CORRELATIONS_PREFIX, b"\xff");

        let range = RangeOption {
            begin: KeySelector::first_greater_or_equal(&start_key),
            end: KeySelector::first_greater_or_equal(&end_key),
            mode: foundationdb::options::StreamingMode::Iterator,
            limit: None,
            reverse: false,
        };

        let correlations = self
            .transact(|trx| {
                let mut correlations = Vec::new();
                let iter = trx.get_range(&range, 1, false);
                
                futures::executor::block_on(async {
                    let mut stream = iter;
                    while let Ok(Some(kv)) = stream.try_next().await {
                        if let Ok(correlation) = serde_json::from_slice::<EventCorrelation>(kv.value()) {
                            correlations.push(correlation);
                        }
                    }
                });
                
                Ok(correlations)
            })
            .await
            .map_err(|e| indexer_core::Error::Generic(format!("FDB error: {}", e)))?;

        let total_correlations = correlations.len() as u64;
        let mut correlations_by_type = HashMap::new();
        let mut correlations_by_chain_pair = HashMap::new();
        let mut total_correlation_time = Duration::from_secs(0);

        for correlation in &correlations {
            // Count by type
            let correlation_type = format!("{:?}", correlation.correlation_type);
            *correlations_by_type.entry(correlation_type).or_insert(0) += 1;
            
            // Count by chain pair
            for target in &correlation.target_events {
                let chain_pair = format!("{}->{}", correlation.source_event.chain_id, target.chain_id);
                *correlations_by_chain_pair.entry(chain_pair).or_insert(0) += 1;
            }
            
            // Calculate correlation time (simplified - would need more data in practice)
            if let Some(first_target) = correlation.target_events.first() {
                let time_diff = first_target.block_number.saturating_sub(correlation.source_event.block_number);
                total_correlation_time += Duration::from_secs(time_diff * 12); // Assume 12s blocks
            }
        }

        let avg_correlation_time = if total_correlations > 0 {
            total_correlation_time / total_correlations as u32
        } else {
            Duration::from_secs(0)
        };

        Ok(CorrelationStats {
            total_correlations,
            correlations_by_type,
            correlations_by_chain_pair,
            avg_correlation_time,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexer_core::distributed_correlation::{CorrelationType, EventCorrelation};
    use indexer_core::{EventType, UnifiedEvent};

    #[tokio::test]
    async fn test_event_correlation_basic_operations() {
        // Skip if FDB not available
        let db = match Database::default() {
            Ok(db) => db,
            Err(_) => {
                eprintln!("FoundationDB not available, skipping test");
                return;
            }
        };

        let service = FdbEventCorrelationService::new(db, "test".to_string());

        // Create a correlation
        let correlation = EventCorrelation {
            correlation_id: "test-correlation-1".to_string(),
            source_event: EventReference {
                chain_id: "ethereum".to_string(),
                block_number: 100,
                transaction_hash: "0x123".to_string(),
                log_index: 0,
                event_type: "Transfer".to_string(),
            },
            target_events: vec![EventReference {
                chain_id: "polygon".to_string(),
                block_number: 200,
                transaction_hash: "0x456".to_string(),
                log_index: 1,
                event_type: "Received".to_string(),
            }],
            correlation_type: CorrelationType::CrossChainMessage {
                message_id: "msg-1".to_string(),
                source_chain: "ethereum".to_string(),
                destination_chain: "polygon".to_string(),
            },
            metadata: HashMap::new(),
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Store correlation
        service.store_correlation(correlation.clone()).await.unwrap();

        // Get correlations for source event
        let correlations = service.get_correlations(&correlation.source_event).await.unwrap();
        assert_eq!(correlations.len(), 1);
        assert_eq!(correlations[0].correlation_id, "test-correlation-1");

        // Find by correlation ID
        let found = service.find_correlated_events("test-correlation-1").await.unwrap();
        assert_eq!(found.len(), 1);

        // Get stats
        let stats = service.get_correlation_stats().await.unwrap();
        assert_eq!(stats.total_correlations, 1);
    }
}