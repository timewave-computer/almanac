/// Ethereum blockchain indexer implementation
/// 
/// This module provides indexing capabilities for Ethereum-compatible blockchains
/// using the valence-domain-clients EVM integration for robust chain support.
use std::sync::Arc;
use async_trait::async_trait;
use std::any::Any;
use std::time::SystemTime;
use indexer_core::{Result, Error};
use indexer_core::types::{Event, UnifiedEvent};
use indexer_core::service::{EventService, EventSubscription};
use indexer_core::types::{ChainId, EventFilter};

#[cfg(feature = "ethereum")]
use valence_domain_clients::evm::base_client::EvmBaseClient;
#[cfg(feature = "ethereum")]
use valence_domain_clients::clients::ethereum::EthereumClient as ValenceEthereumClient;
#[cfg(feature = "ethereum")]
use valence_domain_clients::common::transaction::TransactionResponse;

pub mod codegen;

// Main ethereum implementation is feature-gated
#[cfg(feature = "ethereum")]
pub mod ethereum_impl;

#[cfg(feature = "ethereum")]
pub use ethereum_impl::*;

// Stub implementation when ethereum feature is disabled
#[cfg(not(feature = "ethereum"))]
pub mod ethereum_stub;

#[cfg(not(feature = "ethereum"))]
pub use ethereum_stub::*; 