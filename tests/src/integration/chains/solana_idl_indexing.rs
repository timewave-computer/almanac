//! Integration tests for Solana IDL indexing

#[tokio::test]
async fn test_basic_solana_idl_functionality() -> Result<(), Box<dyn std::error::Error>> {
    // Basic test to verify IDL functionality is working
    // For now, just test that imports are working
    Ok(())
}

#[tokio::test]
async fn test_idl_discriminator_validation() -> Result<(), Box<dyn std::error::Error>> {
    use almanac_chains::adapters::solana_idl::{validate_idl_discriminator, IDL_DISCRIMINATOR};
    
    // Test valid discriminator
    let valid_data = [&IDL_DISCRIMINATOR[..], &[0u8; 100]].concat();
    assert!(validate_idl_discriminator(&valid_data));
    
    // Test invalid discriminator
    let invalid_data = vec![0u8; 100];
    assert!(!validate_idl_discriminator(&invalid_data));
    
    // Test short data
    let short_data = vec![0u8; 7];
    assert!(!validate_idl_discriminator(&short_data));
    
    Ok(())
}

#[tokio::test]
async fn test_idl_pda_derivation() -> Result<(), Box<dyn std::error::Error>> {
    use almanac_chains::adapters::solana_idl::derive_idl_pda;
    use solana_sdk::pubkey::Pubkey;
    
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
    
    Ok(())
}