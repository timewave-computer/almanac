/// High-performance caching system for Almanac
///
/// This module provides distributed caching, query result caching, smart invalidation,
/// compression, and optimized serialization for the Almanac infrastructure.
pub mod compression;
pub mod distributed;
pub mod serialization;
pub mod simple_cache;

// Re-export core types
pub use almanac_infrastructure_core::types::Event;

// Re-export distributed cache types
pub use distributed::{
    DistributedCache, CacheConfig, CacheStats, 
    RemoteCache
};

// Re-export compression types
pub use compression::{
    CompressionStats
};

// Re-export serialization types
pub use serialization::{
    SerializationFormat, MessageType
};

// Re-export simple cache types
pub use simple_cache::{
    CacheStats as SimpleCacheStats
};