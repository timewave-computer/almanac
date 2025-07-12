//! Cosmos client implementation using valence-domain-clients
//! 
//! This module provides Cosmos blockchain connectivity and event processing
//! using the valence-domain-clients Cosmos integration for robust chain support.

use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use std::sync::Arc;

#[cfg(feature = "cosmos")]
use valence_domain_clients::clients::noble::NobleClient;
#[cfg(feature = "cosmos")]
use valence_domain_clients::cosmos::base_client::BaseClient;

use indexer_core::types::{Event, UnifiedEvent};
use indexer_core::service::{EventService, EventSubscription};
use indexer_core::types::{ChainId, EventFilter};

use std::time::SystemTime;
use std::any::Any;

pub mod contracts;
pub mod codegen;

// Main cosmos implementation is feature-gated
#[cfg(feature = "cosmos")]
pub mod cosmos_impl;

#[cfg(feature = "cosmos")]
pub use cosmos_impl::*;

// Stub implementation when cosmos feature is disabled
#[cfg(not(feature = "cosmos"))]
pub mod cosmos_stub;

#[cfg(not(feature = "cosmos"))]
pub use cosmos_stub::*;