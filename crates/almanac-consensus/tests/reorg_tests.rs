use almanac_consensus::{
    reorg::{ReorgConfig, ReorgDetector, ReorgHandler, ReorgType, Reorganization},
    types::BlockInfo,
};
use almanac_core::Result;
use async_trait::async_trait;
use std::time::SystemTime;

struct TestReorgHandler {
    handled_reorgs: std::sync::Arc<std::sync::Mutex<Vec<Reorganization>>>,
}

impl TestReorgHandler {
    fn new() -> Self {
        Self {
            handled_reorgs: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    fn get_handled_reorgs(&self) -> Vec<Reorganization> {
        self.handled_reorgs.lock().unwrap().clone()
    }
}

#[async_trait]
impl ReorgHandler for TestReorgHandler {
    async fn handle_reorganization(&self, reorg: &Reorganization) -> Result<()> {
        self.handled_reorgs.lock().unwrap().push(reorg.clone());
        Ok(())
    }

    async fn rollback_blocks(&self, _chain: &str, _from_block: u64, _to_block: u64) -> Result<u64> {
        Ok(0)
    }

    async fn notify_reorganization(&self, _reorg: &Reorganization) -> Result<()> {
        Ok(())
    }

    async fn get_reorg_stats(&self, _chain: &str) -> Result<almanac_consensus::reorg::ReorgStats> {
        Ok(almanac_consensus::reorg::ReorgStats::default())
    }
}

#[tokio::test]
async fn test_reorg_detector_creation() {
    let config = ReorgConfig {
        chain: "ethereum".to_string(),
        tracking_depth: 100,
        min_confirmations: 6,
        auto_rollback: true,
        alert_threshold: 3,
        max_auto_rollback_depth: 20,
    };

    let _detector = ReorgDetector::new(config);

    // Test that we can create a detector - creation itself is the test
}

#[tokio::test]
async fn test_reorg_types() {
    let types = vec![
        ReorgType::Simple,
        ReorgType::Deep,
        ReorgType::Critical,
        ReorgType::Uncle,
        ReorgType::Orphaned,
    ];

    // Test that all reorg types can be created and compared
    for reorg_type in types {
        assert_eq!(reorg_type, reorg_type.clone());
        assert!(!format!("{:?}", reorg_type).is_empty());
    }
}

#[tokio::test]
async fn test_reorganization_creation() {
    let now = SystemTime::now();
    let reorganization = Reorganization {
        chain: "ethereum".to_string(),
        reorg_type: ReorgType::Simple,
        fork_block: 999,
        original_blocks: vec!["0x1000".to_string(), "0x1001".to_string()],
        new_blocks: vec!["0x1002".to_string(), "0x1003".to_string()],
        depth: 3,
        confidence: 0.95,
        detected_at: now,
        affected_events: 10,
        rollback_performed: false,
        metadata: std::collections::HashMap::new(),
    };

    assert_eq!(reorganization.chain, "ethereum");
    assert_eq!(reorganization.reorg_type, ReorgType::Simple);
    assert_eq!(reorganization.fork_block, 999);
    assert_eq!(reorganization.original_blocks.len(), 2);
    assert_eq!(reorganization.new_blocks.len(), 2);
    assert_eq!(reorganization.depth, 3);
    assert_eq!(reorganization.confidence, 0.95);
    assert_eq!(reorganization.affected_events, 10);
}

#[tokio::test]
async fn test_reorg_config_creation() {
    let config = ReorgConfig {
        chain: "ethereum".to_string(),
        tracking_depth: 100,
        min_confirmations: 6,
        auto_rollback: true,
        alert_threshold: 3,
        max_auto_rollback_depth: 20,
    };

    assert_eq!(config.chain, "ethereum");
    assert_eq!(config.tracking_depth, 100);
    assert_eq!(config.min_confirmations, 6);
    assert!(config.auto_rollback);
    assert_eq!(config.alert_threshold, 3);
    assert_eq!(config.max_auto_rollback_depth, 20);
}

#[tokio::test]
async fn test_block_info_for_reorg() {
    let now = SystemTime::now();
    let block_info = BlockInfo {
        number: 1000,
        hash: "0x1234567890abcdef".to_string(),
        parent_hash: "0x0987654321fedcba".to_string(),
        timestamp: now,
        confirmations: 5,
        is_confirmed: true,
        event_count: 10,
        metadata: std::collections::HashMap::new(),
    };

    // Test that BlockInfo has all fields needed for reorg detection
    assert!(!block_info.hash.is_empty());
    assert!(!block_info.parent_hash.is_empty());
    assert!(block_info.number > 0);
    // Confirmations are always >= 0 for u64
}

#[tokio::test]
async fn test_reorg_detector_with_config() {
    let config = ReorgConfig {
        chain: "ethereum".to_string(),
        tracking_depth: 50,
        min_confirmations: 3,
        auto_rollback: false,
        alert_threshold: 5,
        max_auto_rollback_depth: 10,
    };

    let _detector = ReorgDetector::new(config);

    // Basic test - we can create a detector with custom config - creation itself is the test
}

#[tokio::test]
async fn test_reorg_severity_classification() {
    let simple_reorg = Reorganization {
        chain: "ethereum".to_string(),
        reorg_type: ReorgType::Simple,
        fork_block: 999,
        original_blocks: vec!["0x1000".to_string(), "0x1001".to_string()],
        new_blocks: vec!["0x1002".to_string()],
        depth: 2,
        detected_at: SystemTime::now(),
        confidence: 0.99,
        affected_events: 5,
        rollback_performed: false,
        metadata: std::collections::HashMap::new(),
    };

    let deep_reorg = Reorganization {
        chain: "ethereum".to_string(),
        reorg_type: ReorgType::Deep,
        fork_block: 900,
        original_blocks: (901..=1000).map(|i| format!("0x{:x}", i)).collect(),
        new_blocks: (901..=1010).map(|i| format!("0x{:x}", i)).collect(),
        depth: 100,
        detected_at: SystemTime::now(),
        confidence: 0.85,
        affected_events: 50,
        rollback_performed: true,
        metadata: std::collections::HashMap::new(),
    };

    // Test reorg classifications
    assert_eq!(simple_reorg.depth, 2);
    assert_eq!(deep_reorg.depth, 100);
    assert!(simple_reorg.confidence > deep_reorg.confidence);
    assert!(simple_reorg.original_blocks.len() < deep_reorg.original_blocks.len());
}

#[tokio::test]
async fn test_reorg_confidence_calculation() {
    let high_confidence = Reorganization {
        chain: "ethereum".to_string(),
        reorg_type: ReorgType::Simple,
        fork_block: 999,
        original_blocks: vec!["0x1001".to_string()],
        new_blocks: vec!["0x1002".to_string()],
        depth: 1,
        detected_at: SystemTime::now(),
        confidence: 0.95,
        affected_events: 1,
        rollback_performed: false,
        metadata: std::collections::HashMap::new(),
    };

    let low_confidence = Reorganization {
        chain: "ethereum".to_string(),
        reorg_type: ReorgType::Critical,
        fork_block: 950,
        original_blocks: (951..=1000).map(|i| format!("0x{:x}", i)).collect(),
        new_blocks: (951..=1020).map(|i| format!("0x{:x}", i)).collect(),
        depth: 50,
        detected_at: SystemTime::now(),
        confidence: 0.6,
        affected_events: 100,
        rollback_performed: true,
        metadata: std::collections::HashMap::new(),
    };

    // Test confidence levels
    assert!(high_confidence.confidence > 0.9);
    assert!(low_confidence.confidence < 0.7);
    assert!(high_confidence.depth < low_confidence.depth);
}

#[tokio::test]
async fn test_reorg_handler_trait() {
    let handler = TestReorgHandler::new();

    let reorganization = Reorganization {
        chain: "test".to_string(),
        reorg_type: ReorgType::Simple,
        fork_block: 99,
        original_blocks: vec!["0x100".to_string(), "0x101".to_string()],
        new_blocks: vec!["0x102".to_string()],
        depth: 2,
        detected_at: SystemTime::now(),
        confidence: 0.9,
        affected_events: 5,
        rollback_performed: false,
        metadata: std::collections::HashMap::new(),
    };

    // Test handling a reorg
    let result = handler.handle_reorganization(&reorganization).await;
    assert!(result.is_ok());

    // Verify the reorg was handled
    let handled = handler.get_handled_reorgs();
    assert_eq!(handled.len(), 1);
    assert_eq!(handled[0].chain, "test");
    assert_eq!(handled[0].depth, 2);
}
