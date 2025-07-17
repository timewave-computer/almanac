//! Extended contract state operations using layout resolver
//!
//! This module provides high-level contract state query methods that use
//! the layout resolver to map field names to storage locations.

use super::{TupleFdbStorage, layout_resolver::{LayoutResolver, ResolvedStorageLocation}};
use almanac_infrastructure_core::{Error, Result};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Extended storage implementation with layout resolver
pub struct ExtendedContractState {
    /// The base storage implementation
    storage: Arc<TupleFdbStorage>,
    /// Layout resolver for field mapping
    resolver: Arc<RwLock<LayoutResolver>>,
}

impl ExtendedContractState {
    /// Create a new extended contract state handler
    pub fn new(storage: Arc<TupleFdbStorage>) -> Self {
        Self {
            storage,
            resolver: Arc::new(RwLock::new(LayoutResolver::new())),
        }
    }
    
    /// Register a contract layout for field resolution
    pub async fn register_layout(
        &self,
        chain_id: String,
        contract_address: String,
        layout_data: &[u8],
    ) -> Result<()> {
        let mut resolver = self.resolver.write().await;
        resolver.cache_layout(chain_id, contract_address, layout_data)?;
        Ok(())
    }
    
    /// Get contract state by field name
    pub async fn get_field(
        &self,
        chain_id: &str,
        contract_address: &str,
        field_name: &str,
        block_height: u64,
    ) -> Result<Option<Vec<u8>>> {
        // Resolve field to storage location
        let resolver = self.resolver.read().await;
        let location = resolver.resolve_field(chain_id, contract_address, field_name, block_height)?;
        
        // Get the value from storage
        self.storage.get(location.storage_key).await
    }
    
    /// Get multiple fields in a single operation
    pub async fn get_fields(
        &self,
        chain_id: &str,
        contract_address: &str,
        field_names: &[String],
        block_height: u64,
    ) -> Result<Vec<(String, Option<Vec<u8>>)>> {
        let resolver = self.resolver.read().await;
        let locations = resolver.resolve_fields(chain_id, contract_address, field_names, block_height)?;
        
        let mut results = Vec::new();
        for location in locations {
            let value = self.storage.get(location.storage_key.clone()).await?;
            results.push((location.type_name, value));
        }
        
        Ok(results)
    }
    
    /// Get mapping value by field name and key
    pub async fn get_mapping_value(
        &self,
        chain_id: &str,
        contract_address: &str,
        mapping_name: &str,
        key: &[u8],
        block_height: u64,
    ) -> Result<Option<Vec<u8>>> {
        let resolver = self.resolver.read().await;
        let location = resolver.resolve_mapping(chain_id, contract_address, mapping_name, key, block_height)?;
        
        self.storage.get(location.storage_key).await
    }
    
    /// Set contract state by field name
    pub async fn set_field(
        &self,
        chain_id: &str,
        contract_address: &str,
        field_name: &str,
        block_height: u64,
        value: Vec<u8>,
    ) -> Result<()> {
        let resolver = self.resolver.read().await;
        let location = resolver.resolve_field(chain_id, contract_address, field_name, block_height)?;
        
        // Validate value size
        if value.len() > location.size as usize {
            return Err(Error::Validation(format!(
                "Value size {} exceeds field size {}",
                value.len(), location.size
            )));
        }
        
        self.storage.set(location.storage_key, value).await
    }
    
    /// Get all field names for a contract
    pub async fn get_contract_fields(
        &self,
        chain_id: &str,
        contract_address: &str,
    ) -> Result<Vec<String>> {
        let resolver = self.resolver.read().await;
        resolver.get_all_fields(chain_id, contract_address)
    }
    
    /// Create a view over contract state
    pub async fn create_view(
        &self,
        chain_id: &str,
        contract_address: &str,
        field_names: Vec<String>,
        block_height: u64,
    ) -> Result<ContractView> {
        let mut fields = Vec::new();
        
        for field_name in field_names {
            match self.get_field(chain_id, contract_address, &field_name, block_height).await {
                Ok(value) => {
                    fields.push(ViewField {
                        name: field_name,
                        value,
                        error: None,
                    });
                }
                Err(e) => {
                    fields.push(ViewField {
                        name: field_name,
                        value: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }
        
        Ok(ContractView {
            chain_id: chain_id.to_string(),
            contract_address: contract_address.to_string(),
            block_height,
            fields,
        })
    }
}

/// A view over contract state fields
#[derive(Debug, Clone)]
pub struct ContractView {
    /// Chain ID
    pub chain_id: String,
    /// Contract address
    pub contract_address: String,
    /// Block height
    pub block_height: u64,
    /// Field values
    pub fields: Vec<ViewField>,
}

/// A single field in a contract view
#[derive(Debug, Clone)]
pub struct ViewField {
    /// Field name
    pub name: String,
    /// Field value (if available)
    pub value: Option<Vec<u8>>,
    /// Error message (if field couldn't be retrieved)
    pub error: Option<String>,
}

impl TupleFdbStorage {
    /// Get contract state by field name using layout resolver
    pub async fn get_contract_field(
        &self,
        chain_id: &str,
        contract_address: &str,
        field_name: &str,
        block_height: u64,
        layout_data: &[u8],
    ) -> Result<Option<Vec<u8>>> {
        // Create temporary resolver
        let mut resolver = LayoutResolver::new();
        resolver.cache_layout(chain_id.to_string(), contract_address.to_string(), layout_data)?;
        
        // Resolve field
        let location = resolver.resolve_field(chain_id, contract_address, field_name, block_height)?;
        
        // Get value
        self.get(location.storage_key).await
    }
    
    /// Get mapping value using layout resolver
    pub async fn get_contract_mapping(
        &self,
        chain_id: &str,
        contract_address: &str,
        mapping_name: &str,
        key: &[u8],
        block_height: u64,
        layout_data: &[u8],
    ) -> Result<Option<Vec<u8>>> {
        // Create temporary resolver
        let mut resolver = LayoutResolver::new();
        resolver.cache_layout(chain_id.to_string(), contract_address.to_string(), layout_data)?;
        
        // Resolve mapping
        let location = resolver.resolve_mapping(chain_id, contract_address, mapping_name, key, block_height)?;
        
        // Get value
        self.get(location.storage_key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_extended_contract_state() {
        // This test would require a full FDB setup
        // For now, we're just ensuring the code compiles
    }
}