// HTTP module for HTTP server and REST API functionality
//
// This module provides HTTP server implementation, enhanced HTTP features,
// REST API endpoints, and WebSocket server functionality

pub mod http;
pub mod http_enhanced;
pub mod rest_enhanced;
pub mod websocket;

// Re-export commonly used HTTP types and functions
pub use http::{
    HttpState, EventResponse, EventsResponse, PaginationInfo,
    get_chain_status, get_events_by_address, get_events_by_chain,
    filter_events, get_event_by_id, get_latest_block, get_latest_block_with_status,
    get_block, health_check, get_version, rate_limit_middleware, start_http_server
};
pub use websocket::*; 