// Example showing how to use flexible storage backend selection
//
// This example demonstrates:
// 1. Selecting storage backends based on workload characteristics
// 2. Creating different backend configurations
// 3. Using the hybrid storage mode

use almanac_infrastructure_storage::{
    StorageBackend, StorageFactory, BackendSelector,
    PostgresBackendConfig, RocksBackendConfig, HybridConfig,
    WorkloadCharacteristics, DataVolume, QueryRouting,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Example 1: Automatic backend selection based on workload
    println!("=== Automatic Backend Selection ===");
    
    let write_heavy_workload = WorkloadCharacteristics {
        write_heavy: true,
        complex_queries: false,
        data_volume: DataVolume::Large,
        real_time: true,
        analytics: false,
    };
    
    let recommended = BackendSelector::recommend_backend(&write_heavy_workload);
    println!("Write-heavy workload recommendation: {:?}", recommended);
    
    let analytics_workload = WorkloadCharacteristics {
        write_heavy: false,
        complex_queries: true,
        data_volume: DataVolume::Medium,
        real_time: false,
        analytics: true,
    };
    
    let recommended = BackendSelector::recommend_backend(&analytics_workload);
    println!("Analytics workload recommendation: {:?}", recommended);
    
    // Example 2: PostgreSQL-only configuration
    println!("\n=== PostgreSQL-Only Configuration ===");
    
    let postgres_config = StorageBackend::PostgresOnly {
        config: PostgresBackendConfig {
            url: "postgresql://user:pass@localhost/almanac".to_string(),
            max_connections: 20,
            connection_timeout: 30,
            enable_query_logging: true,
            auto_migrate: true,
        },
    };
    
    println!("Creating PostgreSQL-only storage...");
    match StorageFactory::create(postgres_config).await {
        Ok(_storage) => println!("✓ PostgreSQL storage created successfully"),
        Err(e) => println!("✗ Failed to create PostgreSQL storage: {}", e),
    }
    
    // Example 3: RocksDB-only configuration (if feature enabled)
    #[cfg(feature = "rocks")]
    {
        println!("\n=== RocksDB-Only Configuration ===");
        
        let rocks_config = StorageBackend::RocksOnly {
            config: RocksBackendConfig {
                path: "./data/rocksdb".to_string(),
                cache_size: 1024 * 1024 * 1024, // 1GB
                compression: true,
                num_threads: 8,
                write_buffer_size: 128 * 1024 * 1024, // 128MB
                max_write_buffers: 4,
            },
        };
        
        println!("Creating RocksDB-only storage...");
        match StorageFactory::create(rocks_config).await {
            Ok(_storage) => println!("✓ RocksDB storage created successfully"),
            Err(e) => println!("✗ Failed to create RocksDB storage: {}", e),
        }
    }
    
    // Example 4: Hybrid configuration (if both features enabled)
    #[cfg(all(feature = "postgres", feature = "rocks"))]
    {
        println!("\n=== Hybrid Storage Configuration ===");
        
        let hybrid_config = StorageBackend::Hybrid {
            hot: RocksBackendConfig {
                path: "./data/rocksdb-hot".to_string(),
                cache_size: 2048 * 1024 * 1024, // 2GB for hot data
                compression: false, // No compression for hot data
                num_threads: 16,
                write_buffer_size: 256 * 1024 * 1024, // 256MB
                max_write_buffers: 8,
            },
            cold: PostgresBackendConfig {
                url: "postgresql://user:pass@localhost/almanac_cold".to_string(),
                max_connections: 10,
                connection_timeout: 60,
                enable_query_logging: false,
                auto_migrate: true,
            },
            hybrid_config: HybridConfig {
                hot_blocks: 1000, // Keep last 1000 blocks in hot storage
                hot_time_threshold: 3600, // 1 hour
                auto_migrate: true,
                migration_interval: 300, // Check every 5 minutes
                query_routing: QueryRouting::Smart,
            },
        };
        
        println!("Creating hybrid storage...");
        match StorageFactory::create(hybrid_config).await {
            Ok(_storage) => println!("✓ Hybrid storage created successfully"),
            Err(e) => println!("✗ Failed to create hybrid storage: {}", e),
        }
    }
    
    // Example 5: Configuration from JSON
    println!("\n=== Configuration from JSON ===");
    
    let json_config = r#"{
        "type": "postgres_only",
        "config": {
            "url": "postgresql://localhost/almanac",
            "max_connections": 15,
            "connection_timeout": 30,
            "enable_query_logging": false,
            "auto_migrate": true
        }
    }"#;
    
    let backend: StorageBackend = serde_json::from_str(json_config)?;
    println!("Loaded configuration: {:?}", backend);
    
    // Example 6: Environment-based configuration
    println!("\n=== Environment-based Configuration ===");
    
    let storage_backend = match std::env::var("STORAGE_BACKEND").as_deref() {
        Ok("postgres") => StorageBackend::PostgresOnly {
            config: PostgresBackendConfig {
                url: std::env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgresql://localhost/almanac".to_string()),
                ..Default::default()
            },
        },
        #[cfg(feature = "rocks")]
        Ok("rocks") => StorageBackend::RocksOnly {
            config: RocksBackendConfig {
                path: std::env::var("ROCKS_PATH")
                    .unwrap_or_else(|_| "./data/rocksdb".to_string()),
                ..Default::default()
            },
        },
        #[cfg(all(feature = "postgres", feature = "rocks"))]
        Ok("hybrid") => StorageBackend::Hybrid {
            hot: RocksBackendConfig::default(),
            cold: PostgresBackendConfig::default(),
            hybrid_config: HybridConfig::default(),
        },
        _ => StorageBackend::PostgresOnly {
            config: PostgresBackendConfig::default(),
        },
    };
    
    println!("Environment-based backend: {:?}", storage_backend);
    
    Ok(())
}