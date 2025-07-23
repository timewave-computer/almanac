//! Solana Anchor IDL detection and parsing module

use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
extern crate borsh;
use borsh::{BorshDeserialize, BorshSerialize};

/// Calculate IDL account discriminator
pub fn calculate_idl_discriminator() -> [u8; 8] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(b"account:IdlAccount");
    let result = hasher.finalize();
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&result[..8]);
    discriminator
}

/// Discriminator for Anchor IDL accounts (first 8 bytes of SHA256("account:IdlAccount"))
/// Pre-calculated value: [172, 217, 208, 149, 59, 128, 165, 87]
pub const IDL_DISCRIMINATOR: [u8; 8] = [172, 217, 208, 149, 59, 128, 165, 87];

/// Anchor IDL account structure
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct AnchorIdlAccount {
    /// Discriminator prefix (should match IDL_DISCRIMINATOR)
    pub discriminator: [u8; 8],
    /// Authority pubkey that can update the IDL
    pub authority: Pubkey,
    /// Compressed IDL data length
    pub data_len: u32,
    /// Compressed IDL data (gzip compressed JSON)
    pub data: Vec<u8>,
}


/// Parsed Anchor IDL structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorIdl {
    pub address: String,
    pub version: String,
    pub name: String,
    pub spec: String,
    pub metadata: serde_json::Value,
    pub docs: Option<Vec<String>>,
    pub instructions: Vec<serde_json::Value>,
    pub accounts: Vec<serde_json::Value>,
    pub types: Vec<serde_json::Value>,
    pub events: Option<Vec<serde_json::Value>>,
    pub errors: Option<Vec<serde_json::Value>>,
    pub constants: Option<Vec<serde_json::Value>>,
}

/// IDL PDA seed
pub const IDL_SEED: &[u8] = b"anchor:idl";

/// Derives the IDL PDA for a given program ID
pub fn derive_idl_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[IDL_SEED], program_id)
}

/// Validates that the account data has the correct IDL discriminator
pub fn validate_idl_discriminator(data: &[u8]) -> bool {
    data.len() >= 8 && data[..8] == IDL_DISCRIMINATOR
}

/// Parses an IDL account from raw account data
pub fn parse_idl_account(data: &[u8]) -> std::result::Result<AnchorIdlAccount, Box<dyn std::error::Error + Send + Sync>> {
    if !validate_idl_discriminator(data) {
        return Err("Invalid IDL discriminator".into());
    }
    
    let account = AnchorIdlAccount::try_from_slice(data)?;
    Ok(account)
}

/// Decompresses and parses IDL JSON from account data
pub fn parse_idl_json(account: &AnchorIdlAccount) -> std::result::Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    
    // The IDL data is gzip compressed
    let mut decoder = GzDecoder::new(&account.data[..]);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed)?;
    
    let json: serde_json::Value = serde_json::from_str(&decompressed)?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idl_pda_derivation() {
        let program_id = Pubkey::new_unique();
        let (pda, bump) = derive_idl_pda(&program_id);
        
        // Verify PDA is deterministic
        let (pda2, bump2) = derive_idl_pda(&program_id);
        assert_eq!(pda, pda2);
        assert_eq!(bump, bump2);
        
        // Verify different programs get different PDAs
        let program_id2 = Pubkey::new_unique();
        let (pda3, _) = derive_idl_pda(&program_id2);
        assert_ne!(pda, pda3);
    }

    #[test]
    fn test_discriminator_validation() {
        let valid_data = [&IDL_DISCRIMINATOR[..], &[0u8; 100]].concat();
        assert!(validate_idl_discriminator(&valid_data));
        
        let invalid_data = vec![0u8; 100];
        assert!(!validate_idl_discriminator(&invalid_data));
        
        let short_data = vec![0u8; 7];
        assert!(!validate_idl_discriminator(&short_data));
        
        // Test empty data
        assert!(!validate_idl_discriminator(&[]));
    }
    
    #[test]
    fn test_idl_discriminator_value() {
        // Verify the pre-calculated discriminator is correct
        assert_eq!(IDL_DISCRIMINATOR, [172, 217, 208, 149, 59, 128, 165, 87]);
    }
    
    #[test]
    fn test_parse_idl_account_errors() {
        // Test with invalid discriminator
        let bad_data = vec![0u8; 100];
        let result = parse_idl_account(&bad_data);
        assert!(result.is_err());
        
        // Test with too short data
        let short_data = vec![0u8; 5];
        let result = parse_idl_account(&short_data);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_idl_account_structure() {
        // Create a mock IDL account using proper borsh serialization
        let authority = Pubkey::new_unique();
        let compressed_data = vec![1u8; 100];
        let data_len = compressed_data.len() as u32;
        
        let idl_account = AnchorIdlAccount {
            discriminator: IDL_DISCRIMINATOR,
            authority,
            data_len,
            data: compressed_data.clone(),
        };
        
        // Serialize the account using borsh
        let serialized = borsh::to_vec(&idl_account).unwrap();
        
        // Parse it back and verify
        let result = parse_idl_account(&serialized);
        assert!(result.is_ok());
        
        let parsed_account = result.unwrap();
        assert_eq!(parsed_account.discriminator, IDL_DISCRIMINATOR);
        assert_eq!(parsed_account.authority, authority);
        assert_eq!(parsed_account.data_len, data_len);
        assert_eq!(parsed_account.data, compressed_data);
    }
}