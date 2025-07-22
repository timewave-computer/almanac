//! Test modules for different CLI command groups

pub mod basic;
pub mod cli_comprehensive;
pub mod config;
pub mod cosmos;
pub mod ethereum;
pub mod ethereum_indexing_e2e;
pub mod multi_chain_simulation_e2e;
pub mod validation;

// Mock types for FDB tests
#[cfg(test)]
pub mod mock_types;

// FDB-related tests (using mocks for now)
#[cfg(test)]
pub mod cosmos_fdb_integration_e2e_simple;
#[cfg(test)]
pub mod cross_chain_fdb_consistency_e2e_simple;
#[cfg(test)]
pub mod ethereum_fdb_integration_e2e_simple;
#[cfg(test)]
pub mod fdb_data_validation_e2e_simple;
#[cfg(test)]
pub mod fdb_tuple_space_coverage_e2e_simple;
pub mod solana_real_accounts;
pub mod solana_real_e2e;
