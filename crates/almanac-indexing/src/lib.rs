//! Event indexing service for Almanac
//!
//! This crate provides the event indexing functionality,
//! processing blockchain events and storing them.

pub mod batch;
pub mod finality;
pub mod ingestion;
pub mod ordering;
pub mod pipeline;
pub mod validation;

// Re-exports
pub use pipeline::{IndexingPipeline, PipelineConfig};