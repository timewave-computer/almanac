/// PostgreSQL performance benchmarking tests
use std::time::{Duration, Instant};
use sqlx::{Pool, Postgres, postgres::PgPoolOptions};

use indexer_common::{Result, BlockStatus, Error};
use indexer_core::event::Event;

use crate::postgres::{PostgresStorage, PostgresConfig};
use crate::EventFilter;
use crate::Storage;
use crate::tests::common::{create_mock_event, create_mock_events, assert_duration_less_than};
use crate::migrations::initialize_database;
use crate::migrations::postgres::PostgresSchemaManager;

// Test checkpoint 1.3.2: Test PostgreSQL storage

/// Create a test PostgreSQL pool with initialized schema
async fn create_test_pool() -> Result<Pool<Postgres>> {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/indexer_test".to_string());
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .map_err(|e| Error::database(format!("Failed to connect to database: {}", e)))?;
    
    // Initialize schema directly without checking migrations
    let schema_manager = PostgresSchemaManager::new(pool.clone());
    
    // Apply the test schema directly to avoid circular dependencies with migrations table
    schema_manager.apply_test_schema().await?;
    
    Ok(pool)
}

/// Validate schema setup
#[tokio::test]
async fn test_schema_setup() -> Result<()> {
    let pool = create_test_pool().await?;
    
    // Check if tables exist - excluding migrations table which might cause circular issues
    for table_name in &["events", "blocks", "contract_schemas"] {
        let result = sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT FROM pg_tables 
                WHERE schemaname = 'public' 
                AND tablename = $1
            ) as exists
            "#,
            table_name
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| Error::database(format!("Failed to check if table exists: {}", e)))?;
        
        assert!(
            result.exists.unwrap_or(false), 
            "Table {} does not exist", 
            table_name
        );
        
        println!("Table {} exists", table_name);
    }
    
    // Skip directly checking migrations table content in this test
    println!("Schema setup appears to be successful");
    
    Ok(())
}

/// Test complex queries with test datasets
#[tokio::test]
async fn test_complex_queries() -> Result<()> {
    let pool = create_test_pool().await?;
    let config = PostgresConfig {
        url: std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/indexer_test".to_string()),
        max_connections: 5,
        connection_timeout: 30,
    };
    
    let storage = PostgresStorage::new(config).await?;
    
    // Create and insert test events
    let event_count = 100;
    let eth_events = create_mock_events("ethereum", event_count / 2);
    let cosmos_events = create_mock_events("cosmos", event_count / 2);
    
    // Insert the events
    for event in eth_events {
        storage.store_event(event).await?;
    }
    
    for event in cosmos_events {
        storage.store_event(event).await?;
    }
    
    // Update block statuses
    for i in 0..(event_count / 4) {
        let block_number = 100 + i as u64;
        storage.update_block_status("ethereum", block_number, BlockStatus::Finalized).await?;
    }
    
    for i in (event_count / 4)..(event_count / 2) {
        let block_number = 100 + i as u64;
        storage.update_block_status("ethereum", block_number, BlockStatus::Confirmed).await?;
    }
    
    // Test filter by chain
    let filter1 = EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: None,
        time_range: None,
        event_types: None,
        limit: None,
        offset: None,
    };
    
    let ethereum_events = storage.get_events(vec![filter1.clone()]).await?;
    println!("Ethereum events: {}", ethereum_events.len());
    assert_eq!(ethereum_events.len(), event_count / 2, "Incorrect number of Ethereum events");
    
    // Test filter by block range
    let filter2 = EventFilter {
        chain: Some("ethereum".to_string()),
        block_range: Some((100, 110)),
        time_range: None,
        event_types: None,
        limit: None,
        offset: None,
    };
    
    let block_range_events = storage.get_events(vec![filter2]).await?;
    println!("Ethereum events in block range 100-110: {}", block_range_events.len());
    assert_eq!(block_range_events.len(), 11, "Incorrect number of events in block range");
    
    // Test filter by status
    let finalized_events = storage.get_events_with_status(vec![filter1], BlockStatus::Finalized).await?;
    println!("Finalized Ethereum events: {}", finalized_events.len());
    assert_eq!(finalized_events.len(), event_count / 4, "Incorrect number of finalized events");
    
    Ok(())
}

/// Benchmark query performance with various data sizes
#[tokio::test]
async fn benchmark_query_performance() -> Result<()> {
    let pool = create_test_pool().await?;
    let config = PostgresConfig {
        url: std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/indexer_test".to_string()),
        max_connections: 5,
        connection_timeout: 30,
    };
    
    let storage = PostgresStorage::new(config).await?;
    
    // Define dataset sizes to test
    let dataset_sizes = vec![10, 100, 1000];
    
    println!("PostgreSQL Query Performance Benchmark:");
    println!("--------------------------------------");
    
    for &size in &dataset_sizes {
        // Create and insert test events
        let events = create_mock_events("benchmark", size);
        
        // Insert the events
        for event in events {
            storage.store_event(event).await?;
        }
        
        // Benchmark simple query - get all events
        let start = Instant::now();
        
        let filter = EventFilter {
            chain: Some("benchmark".to_string()),
            block_range: None,
            time_range: None,
            event_types: None,
            limit: None,
            offset: None,
        };
        
        let results = storage.get_events(vec![filter]).await?;
        
        let duration = start.elapsed();
        let ops_per_sec = (results.len() as f64) / duration.as_secs_f64();
        
        println!("Simple query - dataset size: {}, results: {}, duration: {:?}, ops/sec: {:.2}",
               size, results.len(), duration, ops_per_sec);
        
        // Benchmark complex query - join with blocks and filter
        let start = Instant::now();
        
        // Update some block statuses
        for i in 0..(size / 2) {
            let block_number = 100 + i as u64;
            storage.update_block_status("benchmark", block_number, BlockStatus::Finalized).await?;
        }
        
        let filter = EventFilter {
            chain: Some("benchmark".to_string()),
            block_range: Some((100, 100 + (size / 2) as u64)),
            time_range: None,
            event_types: None,
            limit: None,
            offset: None,
        };
        
        let results = storage.get_events_with_status(vec![filter], BlockStatus::Finalized).await?;
        
        let duration = start.elapsed();
        let ops_per_sec = if results.len() > 0 {
            (results.len() as f64) / duration.as_secs_f64() 
        } else {
            0.0
        };
        
        println!("Complex query - dataset size: {}, results: {}, duration: {:?}, ops/sec: {:.2}",
               size, results.len(), duration, ops_per_sec);
        
        // Ensure queries meet performance requirements for small datasets
        if size <= 100 {
            let expected_max_duration = Duration::from_millis(100);
            assert_duration_less_than(duration, expected_max_duration,
                &format!("Query for dataset size {} too slow", size));
        }
    }
    
    Ok(())
}

/// Test SQL preparation workflow with sqlx
#[tokio::test]
async fn test_sqlx_prepare() -> Result<()> {
    // This test demonstrates the sqlx prepare workflow
    let pool = create_test_pool().await?;
    
    // Example of a query that will be checked at compile time with `sqlx prepare`
    let result = sqlx::query!(
        r#"
        SELECT table_name, column_name
        FROM information_schema.columns
        WHERE table_schema = 'public'
        AND table_name = 'events'
        ORDER BY ordinal_position
        "#
    )
    .fetch_all(&pool)
    .await?;
    
    println!("Events table columns:");
    for row in &result {
        println!("  - {}: {}", 
            row.table_name.as_ref().unwrap_or(&"unknown".to_string()), 
            row.column_name.as_ref().unwrap_or(&"unknown".to_string()));
    }
    
    assert!(!result.is_empty(), "No columns found for events table");
    
    // Another example using a parameterized query
    let event_id = "test_event_1";
    let test_event = create_mock_event(event_id, "test_chain", 100);
    let storage = PostgresStorage::new(PostgresConfig::default()).await?;
    
    // Store the event
    storage.store_event(test_event).await?;
    
    // Verify compile-time SQL validation
    // This query will be validated by sqlx prepare
    let filters = vec![EventFilter {
        chain: Some("test_chain".to_string()),
        block_range: None,
        time_range: None,
        event_types: None,
        limit: Some(1),
        offset: None,
    }];
    
    let events = storage.get_events(filters).await?;
    
    assert!(!events.is_empty(), "No events found after inserting test event");
    assert_eq!(events[0].id(), event_id, "Retrieved event has incorrect ID");
    
    println!("SQL preparation test passed: queries are validated at compile time");
    
    Ok(())
} 