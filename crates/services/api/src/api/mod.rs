// API module for API endpoints and specifications
//
// This module provides various API endpoints including OpenAPI specifications,
// streaming queries, distributed queries, and storage proof APIs

pub mod openapi;
pub mod streaming_query_api;
pub mod distributed_query_api;
pub mod storage_proof_api;

// Re-export API functionality with qualified names to avoid conflicts
pub use openapi::*;

// Streaming query API - prefix with Streaming
pub use streaming_query_api::{
    StreamingQueryState, StreamingQueryRequest, StreamingOptions,
    QueryRequest as StreamingQuery, 
    QueryFilterRequest as StreamingQueryFilter,
    PaginationRequest as StreamingPagination,
    SortingRequest as StreamingSorting,
    create_streaming_query_router,
};

// Distributed query API - prefix with Distributed  
pub use distributed_query_api::{
    DistributedQueryState, LocalQueryExecutor,
    QueryRequest as DistributedQuery,
    QueryFilterRequest as DistributedQueryFilter, 
    PaginationRequest as DistributedPagination,
    SortingRequest as DistributedSorting,
    create_distributed_query_router,
};

// Storage proof API
pub use storage_proof_api::*; 