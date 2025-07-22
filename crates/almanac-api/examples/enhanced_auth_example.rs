//! Authentication System Example
//!
//! This example demonstrates the comprehensive API key management system
//! with permissions, quotas, and usage tracking.

use almanac_api::auth::{
    EnhancedApiKeyStore, GenerateKeyRequest, ApiPermissions, ApiOperation,
    RateLimitConfig, UsageQuotas, PermissionContext, ApiKeyError,
};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Authentication System Example");
    
    // Create the enhanced API key store
    let key_store = EnhancedApiKeyStore::new();
    
    // Example 1: Generate a read-only API key
    info!("Generating read-only API key...");
    let read_only_request = GenerateKeyRequest {
        name: "Read-Only Client".to_string(),
        developer_id: "dev_001".to_string(),
        permissions: ApiPermissions {
            operations: {
                let mut ops = HashSet::new();
                ops.insert(ApiOperation::ReadEvents);
                ops.insert(ApiOperation::ReadLayouts);
                ops.insert(ApiOperation::ReadSemantics);
                ops
            },
            chains: Some({
                let mut chains = HashSet::new();
                chains.insert("ethereum".to_string());
                chains.insert("solana".to_string());
                chains
            }),
            contracts: None, // Allow all contracts
            scopes: {
                let mut scopes = HashSet::new();
                scopes.insert("read:public".to_string());
                scopes
            },
        },
        rate_limit: Some(RateLimitConfig {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            requests_per_day: 10000,
            burst_limit: 10,
        }),
        quotas: Some(UsageQuotas {
            storage_queries_monthly: Some(50000),
            proof_generations_monthly: Some(0), // No proof generation allowed
            data_egress_monthly: Some(500_000_000), // 500MB
        }),
        expires_at: None, // No expiration
        ip_whitelist: None, // Allow from any IP
        user_agent_pattern: None,
        tags: Some({
            let mut tags = HashMap::new();
            tags.insert("tier".to_string(), "basic".to_string());
            tags.insert("customer".to_string(), "acme-corp".to_string());
            tags
        }),
    };
    
    let read_only_key = key_store.generate_key(read_only_request).await
        .map_err(|e| anyhow::anyhow!("Failed to generate key: {}", e))?;
    info!("Generated read-only key: {}", read_only_key.key_id);
    info!("API Key: {}****", &read_only_key.api_key[..8]); // Show only first 8 chars
    
    // Example 2: Generate an admin API key with full permissions
    info!("Generating admin API key...");
    let admin_request = GenerateKeyRequest {
        name: "Admin Console".to_string(),
        developer_id: "dev_admin".to_string(),
        permissions: ApiPermissions {
            operations: {
                let mut ops = HashSet::new();
                ops.insert(ApiOperation::ReadEvents);
                ops.insert(ApiOperation::ReadLayouts);
                ops.insert(ApiOperation::ReadSemantics);
                ops.insert(ApiOperation::ReadViews);
                ops.insert(ApiOperation::ReadStorage);
                ops.insert(ApiOperation::ReadTrust);
                ops.insert(ApiOperation::WriteLayouts);
                ops.insert(ApiOperation::WriteSemantics);
                ops.insert(ApiOperation::WriteViews);
                ops.insert(ApiOperation::WriteTrust);
                ops.insert(ApiOperation::ManageKeys);
                ops.insert(ApiOperation::ManageUsers);
                ops.insert(ApiOperation::GenerateProofs);
                ops.insert(ApiOperation::VerifyProofs);
                ops
            },
            chains: None, // Allow all chains
            contracts: None, // Allow all contracts
            scopes: {
                let mut scopes = HashSet::new();
                scopes.insert("admin:all".to_string());
                scopes
            },
        },
        rate_limit: Some(RateLimitConfig {
            requests_per_minute: 1000,
            requests_per_hour: 10000,
            requests_per_day: 100000,
            burst_limit: 50,
        }),
        quotas: Some(UsageQuotas {
            storage_queries_monthly: None, // No limits
            proof_generations_monthly: None, // No limits
            data_egress_monthly: None, // No limits
        }),
        expires_at: None,
        ip_whitelist: Some(vec![
            "192.168.1.0/24".to_string(),
            "10.0.0.0/8".to_string(),
        ]),
        user_agent_pattern: Some("AdminConsole/.*".to_string()),
        tags: Some({
            let mut tags = HashMap::new();
            tags.insert("tier".to_string(), "enterprise".to_string());
            tags.insert("role".to_string(), "admin".to_string());
            tags
        }),
    };
    
    let admin_key = key_store.generate_key(admin_request).await
        .map_err(|e| anyhow::anyhow!("Failed to generate admin key: {}", e))?;
    info!("Generated admin key: {}", admin_key.key_id);
    
    // Example 3: Generate a restricted development key
    info!("Generating development API key...");
    let dev_request = GenerateKeyRequest {
        name: "Development Environment".to_string(),
        developer_id: "dev_002".to_string(),
        permissions: ApiPermissions {
            operations: {
                let mut ops = HashSet::new();
                ops.insert(ApiOperation::ReadEvents);
                ops.insert(ApiOperation::ReadLayouts);
                ops.insert(ApiOperation::WriteLayouts);
                ops.insert(ApiOperation::GenerateProofs);
                ops
            },
            chains: Some({
                let mut chains = HashSet::new();
                chains.insert("ethereum-sepolia".to_string()); // Testnet only
                chains
            }),
            contracts: Some({
                let mut contracts = HashSet::new();
                contracts.insert("0x742d35Cc6634C0532925a3b8D80bE5e4eb16F8d6ef".to_string());
                contracts
            }),
            scopes: {
                let mut scopes = HashSet::new();
                scopes.insert("dev:testnet".to_string());
                scopes
            },
        },
        rate_limit: Some(RateLimitConfig {
            requests_per_minute: 100,
            requests_per_hour: 2000,
            requests_per_day: 20000,
            burst_limit: 20,
        }),
        quotas: Some(UsageQuotas {
            storage_queries_monthly: Some(100000),
            proof_generations_monthly: Some(10000),
            data_egress_monthly: Some(1_000_000_000), // 1GB
        }),
        expires_at: Some(Utc::now() + chrono::Duration::days(90)), // 90 day expiry
        ip_whitelist: None,
        user_agent_pattern: None,
        tags: Some({
            let mut tags = HashMap::new();
            tags.insert("tier".to_string(), "developer".to_string());
            tags.insert("environment".to_string(), "development".to_string());
            tags
        }),
    };
    
    let dev_key = key_store.generate_key(dev_request).await
        .map_err(|e| anyhow::anyhow!("Failed to generate dev key: {}", e))?;
    info!("Generated development key: {} (expires in 90 days)", dev_key.key_id);
    
    // Example 4: Test permission validation
    info!("Testing permission validation...");
    
    // Test valid permission
    let context = PermissionContext {
        chain: Some("ethereum".to_string()),
        contract: None,
        developer_id: Some("dev_001".to_string()),
    };
    
    match key_store.validate_key_with_permissions(
        &read_only_key.api_key,
        ApiOperation::ReadEvents,
        &context,
    ).await {
        Ok(key) => {
            info!("✅ Read-only key validated for ReadEvents on ethereum");
            info!("   Key name: {}", key.name);
        },
        Err(e) => error!("❌ Validation failed: {}", e),
    }
    
    // Test invalid permission (write operation with read-only key)
    match key_store.validate_key_with_permissions(
        &read_only_key.api_key,
        ApiOperation::WriteLayouts,
        &context,
    ).await {
        Ok(_) => warn!("⚠️  Unexpected success for write operation with read-only key"),
        Err(ApiKeyError::InsufficientPermissions) => {
            info!("✅ Correctly rejected write operation for read-only key");
        },
        Err(e) => error!("❌ Unexpected error: {}", e),
    }
    
    // Test chain restriction
    let restricted_context = PermissionContext {
        chain: Some("bitcoin".to_string()), // Not allowed for read-only key
        contract: None,
        developer_id: Some("dev_001".to_string()),
    };
    
    match key_store.validate_key_with_permissions(
        &read_only_key.api_key,
        ApiOperation::ReadEvents,
        &restricted_context,
    ).await {
        Ok(_) => warn!("⚠️  Unexpected success for restricted chain"),
        Err(ApiKeyError::ChainNotAllowed) => {
            info!("✅ Correctly rejected access to restricted chain (bitcoin)");
        },
        Err(e) => error!("❌ Unexpected error: {}", e),
    }
    
    // Example 5: Demonstrate usage tracking
    info!("Demonstrating usage tracking...");
    
    // Simulate API usage
    key_store.track_usage(
        &read_only_key.key_id,
        "read_events",
        1024, // 1KB data egress
        true, // Is storage query
        false, // Not proof generation
    ).await;
    
    key_store.track_usage(
        &admin_key.key_id,
        "generate_proof",
        2048, // 2KB data egress
        false, // Not storage query
        true, // Is proof generation
    ).await;
    
    // Check quotas
    let read_only_key_data = key_store.validate_key_with_permissions(
        &read_only_key.api_key,
        ApiOperation::ReadEvents,
        &context,
    ).await?;
    
    match key_store.check_quotas(&read_only_key_data).await {
        Ok(()) => info!("✅ Read-only key is within quotas"),
        Err(e) => warn!("⚠️  Quota exceeded: {}", e),
    }
    
    // Example 6: List keys for developers
    info!("Listing keys by developer...");
    
    let dev_001_keys = key_store.list_developer_keys("dev_001").await;
    info!("Developer dev_001 has {} key(s):", dev_001_keys.len());
    for key_summary in dev_001_keys {
        info!("  - {} ({})", key_summary.name, key_summary.key_id);
        info!("    Created: {}, Last used: {:?}", 
              key_summary.created_at.format("%Y-%m-%d %H:%M"),
              key_summary.last_used_at);
        info!("    Permissions: {}", key_summary.permissions_summary);
    }
    
    let admin_keys = key_store.list_developer_keys("dev_admin").await;
    info!("Developer dev_admin has {} key(s):", admin_keys.len());
    for key_summary in admin_keys {
        info!("  - {} ({})", key_summary.name, key_summary.key_id);
    }
    
    info!("Enhanced authentication system example completed successfully!");
    info!("Key features demonstrated:");
    info!("  ✅ Fine-grained permission control");
    info!("  ✅ Chain and contract restrictions");
    info!("  ✅ Rate limiting configuration");
    info!("  ✅ Usage quotas and tracking");
    info!("  ✅ Key expiration and IP whitelisting");
    info!("  ✅ Developer-scoped key management");
    info!("  ✅ Comprehensive validation and error handling");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_key_generation_and_validation() {
        let store = EnhancedApiKeyStore::new();
        
        let request = GenerateKeyRequest {
            name: "Test Key".to_string(),
            developer_id: "test_dev".to_string(),
            permissions: ApiPermissions {
                operations: {
                    let mut ops = HashSet::new();
                    ops.insert(ApiOperation::ReadEvents);
                    ops
                },
                chains: None,
                contracts: None,
                scopes: HashSet::new(),
            },
            rate_limit: None,
            quotas: None,
            expires_at: None,
            ip_whitelist: None,
            user_agent_pattern: None,
            tags: None,
        };
        
        let key_response = store.generate_key(request).await.unwrap();
        
        let context = PermissionContext {
            chain: None,
            contract: None,
            developer_id: Some("test_dev".to_string()),
        };
        
        let validation_result = store.validate_key_with_permissions(
            &key_response.api_key,
            ApiOperation::ReadEvents,
            &context,
        ).await;
        
        assert!(validation_result.is_ok());
        
        // Test invalid operation
        let invalid_result = store.validate_key_with_permissions(
            &key_response.api_key,
            ApiOperation::WriteLayouts,
            &context,
        ).await;
        
        assert!(matches!(invalid_result, Err(ApiKeyError::InsufficientPermissions)));
    }
    
    #[tokio::test]
    async fn test_usage_tracking() {
        let store = EnhancedApiKeyStore::new();
        
        let request = GenerateKeyRequest {
            name: "Usage Test Key".to_string(),
            developer_id: "usage_test_dev".to_string(),
            permissions: ApiPermissions {
                operations: {
                    let mut ops = HashSet::new();
                    ops.insert(ApiOperation::ReadEvents);
                    ops
                },
                chains: None,
                contracts: None,
                scopes: HashSet::new(),
            },
            rate_limit: None,
            quotas: Some(UsageQuotas {
                storage_queries_monthly: Some(10),
                proof_generations_monthly: None,
                data_egress_monthly: None,
            }),
            expires_at: None,
            ip_whitelist: None,
            user_agent_pattern: None,
            tags: None,
        };
        
        let key_response = store.generate_key(request).await.unwrap();
        
        // Track some usage
        for _ in 0..5 {
            store.track_usage(&key_response.key_id, "read_events", 100, true, false).await;
        }
        
        let context = PermissionContext {
            chain: None,
            contract: None,
            developer_id: Some("usage_test_dev".to_string()),
        };
        
        let key = store.validate_key_with_permissions(
            &key_response.api_key,
            ApiOperation::ReadEvents,
            &context,
        ).await.unwrap();
        
        // Should still be within quota
        assert!(store.check_quotas(&key).await.is_ok());
        
        // Track more usage to exceed quota
        for _ in 0..10 {
            store.track_usage(&key_response.key_id, "read_events", 100, true, false).await;
        }
        
        // Should now exceed quota
        assert!(store.check_quotas(&key).await.is_err());
    }
}