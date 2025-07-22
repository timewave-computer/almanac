use almanac_consensus::{
    finality::{AdvancedFinalityProcessor, FinalityConfig, FinalityLevel},
    types::BlockInfo,
};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

#[tokio::test]
async fn test_finality_processor_creation() {
    let _processor = AdvancedFinalityProcessor::new();

    // Test that we can create a processor - processor created successfully
}

#[tokio::test]
async fn test_finality_processor_default() {
    let _processor = AdvancedFinalityProcessor::default();

    // Test that default implementation works - processor created successfully
}

#[tokio::test]
async fn test_finality_config_creation() {
    let config = FinalityConfig {
        chain_id: "ethereum".to_string(),
        finality_requirements: {
            let mut requirements = HashMap::new();
            requirements.insert(FinalityLevel::Confirmed, 1);
            requirements.insert(FinalityLevel::Safe, 6);
            requirements.insert(FinalityLevel::Finalized, 12);
            requirements
        },
        time_requirements: {
            let mut requirements = HashMap::new();
            requirements.insert(FinalityLevel::Confirmed, Duration::from_secs(60));
            requirements.insert(FinalityLevel::Safe, Duration::from_secs(300));
            requirements.insert(FinalityLevel::Finalized, Duration::from_secs(600));
            requirements
        },
        max_reorg_depth: 100,
        probabilistic_finality: true,
        reorg_detection: true,
        auto_finality_timeout: Duration::from_secs(3600),
    };

    assert_eq!(config.chain_id, "ethereum");
    assert_eq!(config.finality_requirements.len(), 3);
    assert_eq!(config.max_reorg_depth, 100);
    assert_eq!(config.auto_finality_timeout, Duration::from_secs(3600));
}

#[tokio::test]
async fn test_finality_levels() {
    let levels = vec![
        FinalityLevel::Confirmed,
        FinalityLevel::Safe,
        FinalityLevel::Justified,
        FinalityLevel::Finalized,
    ];

    // Test that all levels can be created and compared
    for level in levels {
        assert_eq!(level, level.clone());
        assert!(!format!("{:?}", level).is_empty());
    }
}

#[tokio::test]
async fn test_finality_requirements() {
    let mut requirements = HashMap::new();
    requirements.insert(FinalityLevel::Safe, 6);
    requirements.insert(FinalityLevel::Finalized, 12);

    assert_eq!(requirements[&FinalityLevel::Safe], 6);
    assert_eq!(requirements[&FinalityLevel::Finalized], 12);
}

#[tokio::test]
async fn test_add_chain_config() {
    let processor = AdvancedFinalityProcessor::new();

    let config = FinalityConfig {
        chain_id: "ethereum".to_string(),
        finality_requirements: {
            let mut requirements = HashMap::new();
            requirements.insert(FinalityLevel::Confirmed, 1);
            requirements
        },
        time_requirements: {
            let mut requirements = HashMap::new();
            requirements.insert(FinalityLevel::Confirmed, Duration::from_secs(60));
            requirements
        },
        max_reorg_depth: 100,
        probabilistic_finality: true,
        reorg_detection: true,
        auto_finality_timeout: Duration::from_secs(3600),
    };

    // Test that we can add a chain config
    processor.add_chain_config(config);

    // Basic test - we can't easily verify the internal state without getters - config added successfully
}

#[tokio::test]
async fn test_block_info_creation() {
    let now = SystemTime::now();
    let block_info = BlockInfo {
        number: 1000,
        hash: "0x1234567890abcdef".to_string(),
        parent_hash: "0x0987654321fedcba".to_string(),
        timestamp: now,
        confirmations: 5,
        is_confirmed: true,
        event_count: 10,
        metadata: HashMap::new(),
    };

    assert_eq!(block_info.number, 1000);
    assert_eq!(block_info.hash, "0x1234567890abcdef");
    assert_eq!(block_info.confirmations, 5);
    assert!(block_info.is_confirmed);
    assert_eq!(block_info.event_count, 10);
}

#[tokio::test]
async fn test_ethereum_finality_config() {
    let config = FinalityConfig {
        chain_id: "ethereum".to_string(),
        finality_requirements: {
            let mut requirements = HashMap::new();
            requirements.insert(FinalityLevel::Confirmed, 1);
            requirements.insert(FinalityLevel::Safe, 6);
            requirements.insert(FinalityLevel::Finalized, 12);
            requirements
        },
        time_requirements: {
            let mut requirements = HashMap::new();
            requirements.insert(FinalityLevel::Confirmed, Duration::from_secs(15));
            requirements.insert(FinalityLevel::Safe, Duration::from_secs(90));
            requirements.insert(FinalityLevel::Finalized, Duration::from_secs(180));
            requirements
        },
        max_reorg_depth: 64,
        probabilistic_finality: true,
        reorg_detection: true,
        auto_finality_timeout: Duration::from_secs(1800),
    };

    // Test Ethereum-specific finality requirements
    assert_eq!(config.finality_requirements[&FinalityLevel::Confirmed], 1);
    assert_eq!(config.finality_requirements[&FinalityLevel::Safe], 6);
    assert_eq!(config.finality_requirements[&FinalityLevel::Finalized], 12);
}

#[tokio::test]
async fn test_cosmos_finality_config() {
    let config = FinalityConfig {
        chain_id: "cosmos".to_string(),
        finality_requirements: {
            let mut requirements = HashMap::new();
            requirements.insert(FinalityLevel::Confirmed, 1);
            requirements.insert(FinalityLevel::Finalized, 1); // Cosmos has instant finality
            requirements
        },
        time_requirements: {
            let mut requirements = HashMap::new();
            requirements.insert(FinalityLevel::Confirmed, Duration::from_secs(7));
            requirements.insert(FinalityLevel::Finalized, Duration::from_secs(7));
            requirements
        },
        max_reorg_depth: 0, // Cosmos doesn't have reorgs
        probabilistic_finality: false,
        reorg_detection: false,
        auto_finality_timeout: Duration::from_secs(600),
    };

    // Test Cosmos-specific finality requirements
    assert_eq!(config.max_reorg_depth, 0);
    assert_eq!(config.finality_requirements.len(), 2);
}

#[tokio::test]
async fn test_finality_level_ordering() {
    // Test that finality levels are ordered properly
    assert!(FinalityLevel::Confirmed < FinalityLevel::Safe);
    assert!(FinalityLevel::Safe < FinalityLevel::Justified);
    assert!(FinalityLevel::Justified < FinalityLevel::Finalized);

    // Test the all_up_to method
    let levels = FinalityLevel::Finalized.all_up_to();
    assert_eq!(levels.len(), 4);
    assert_eq!(levels[0], FinalityLevel::Confirmed);
    assert_eq!(levels[3], FinalityLevel::Finalized);
}
