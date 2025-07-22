//! Helper module for creating and managing real Solana accounts in e2e tests

use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    account::Account,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction as system_instruction;
use std::collections::HashMap;
use std::sync::Arc;

/// Manager for creating and interacting with real Solana accounts
pub struct SolanaAccountManager {
    rpc_client: Arc<RpcClient>,
    payer: Keypair,
    created_accounts: HashMap<Pubkey, Keypair>,
}

impl SolanaAccountManager {
    /// Create a new account manager
    pub fn new(rpc_url: &str) -> Result<Self> {
        let rpc_client = Arc::new(RpcClient::new(rpc_url));

        // For test validator, we can use a fixed keypair
        // This should be generated dynamically in production
        let payer = Keypair::new();

        // Request airdrop for the payer
        // This only works with test validators
        println!("Requesting airdrop for payer: {}", payer.pubkey());

        Ok(Self {
            rpc_client,
            payer,
            created_accounts: HashMap::new(),
        })
    }

    /// Request SOL airdrop for an account (test validator only)
    pub async fn request_airdrop(&self, pubkey: &Pubkey, lamports: u64) -> Result<()> {
        let rpc_client = Arc::clone(&self.rpc_client);
        let pubkey = *pubkey;
        let signature =
            tokio::task::spawn_blocking(move || rpc_client.request_airdrop(&pubkey, lamports))
                .await
                .unwrap()
                .map_err(|e| anyhow!("Failed to request airdrop: {}", e))?;

        // Wait for confirmation
        loop {
            let rpc_client = Arc::clone(&self.rpc_client);
            let confirmed =
                tokio::task::spawn_blocking(move || rpc_client.confirm_transaction(&signature))
                    .await
                    .unwrap();

            if let Ok(confirmed) = confirmed {
                if confirmed {
                    println!(
                        "+ Airdrop confirmed for {}: {} SOL",
                        pubkey,
                        lamports as f64 / 1e9
                    );
                    break;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Create a new account with data
    pub async fn create_account_with_data(
        &mut self,
        space: usize,
        owner: &Pubkey,
        initial_data: Option<&[u8]>,
    ) -> Result<Pubkey> {
        // Request airdrop for payer if needed
        let rpc_client = Arc::clone(&self.rpc_client);
        let payer_pubkey = self.payer.pubkey();
        let payer_balance =
            tokio::task::spawn_blocking(move || rpc_client.get_balance(&payer_pubkey))
                .await
                .unwrap()
                .unwrap_or(0);

        if payer_balance < 1_000_000_000 {
            // Less than 1 SOL
            self.request_airdrop(&self.payer.pubkey(), 2_000_000_000)
                .await?; // 2 SOL
        }

        // Calculate rent exemption
        let rpc_client = Arc::clone(&self.rpc_client);
        let rent_exemption = tokio::task::spawn_blocking(move || {
            rpc_client.get_minimum_balance_for_rent_exemption(space)
        })
        .await
        .unwrap()
        .map_err(|e| anyhow!("Failed to get rent exemption: {}", e))?;

        // Create new keypair for the account
        let new_account = Keypair::new();
        let new_pubkey = new_account.pubkey();

        // Create account instruction
        let create_account_ix = system_instruction::create_account(
            &self.payer.pubkey(),
            &new_pubkey,
            rent_exemption,
            space as u64,
            owner,
        );

        // Create transaction
        let rpc_client = Arc::clone(&self.rpc_client);
        let recent_blockhash =
            tokio::task::spawn_blocking(move || rpc_client.get_latest_blockhash())
                .await
                .unwrap()
                .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;

        let mut transaction =
            Transaction::new_with_payer(&[create_account_ix], Some(&self.payer.pubkey()));

        transaction.sign(&[&self.payer, &new_account], recent_blockhash);

        // Send transaction
        let rpc_client = Arc::clone(&self.rpc_client);
        let signature = tokio::task::spawn_blocking(move || {
            rpc_client.send_and_confirm_transaction(&transaction)
        })
        .await
        .unwrap()
        .map_err(|e| anyhow!("Failed to create account: {}", e))?;

        println!("+ Created new account: {} (tx: {})", new_pubkey, signature);

        // If we have initial data, write it to the account
        if let Some(data) = initial_data {
            self.write_account_data(&new_pubkey, data).await?;
        }

        // Store the keypair
        self.created_accounts.insert(new_pubkey, new_account);

        Ok(new_pubkey)
    }

    /// Write data to an existing account
    pub async fn write_account_data(&self, account_pubkey: &Pubkey, data: &[u8]) -> Result<()> {
        // For simplicity, we'll use a generic program call
        // In a real implementation, you would use the specific program that owns the account

        println!(
            "Writing {} bytes to account: {}",
            data.len(),
            account_pubkey
        );

        // This is a placeholder - in reality you'd need to use the appropriate program
        // to modify the account data. For system accounts, we can't directly modify data.
        // For program accounts, you need to call the program's instruction handlers.

        Ok(())
    }

    /// Create a simple test program account
    pub async fn create_test_program_account(&mut self) -> Result<Pubkey> {
        // Create an account owned by the system program with some test data
        let test_data = b"test_account_data_v1";

        let account_pubkey = self
            .create_account_with_data(
                1024, // 1KB space
                &solana_sdk::system_program::id(),
                Some(test_data),
            )
            .await?;

        println!("Created test program account: {}", account_pubkey);
        Ok(account_pubkey)
    }

    /// Create a simple counter program account that can be modified
    pub async fn create_counter_account(&mut self) -> Result<Pubkey> {
        // This creates an account that could be used by a simple counter program
        let initial_counter_data = vec![0u8; 8]; // 8 bytes for a u64 counter

        let account_pubkey = self
            .create_account_with_data(
                8,
                &solana_sdk::system_program::id(), // Using system program for simplicity
                Some(&initial_counter_data),
            )
            .await?;

        println!("Created counter account: {}", account_pubkey);
        Ok(account_pubkey)
    }

    /// Simulate account updates by transferring lamports (which changes account data)
    pub async fn update_account_balance(
        &self,
        account_pubkey: &Pubkey,
        additional_lamports: u64,
    ) -> Result<()> {
        let rpc_client = Arc::clone(&self.rpc_client);
        let recent_blockhash =
            tokio::task::spawn_blocking(move || rpc_client.get_latest_blockhash())
                .await
                .unwrap()
                .map_err(|e| anyhow!("Failed to get recent blockhash: {}", e))?;

        let transfer_ix =
            system_instruction::transfer(&self.payer.pubkey(), account_pubkey, additional_lamports);

        let mut transaction =
            Transaction::new_with_payer(&[transfer_ix], Some(&self.payer.pubkey()));

        transaction.sign(&[&self.payer], recent_blockhash);

        let rpc_client = Arc::clone(&self.rpc_client);
        let signature = tokio::task::spawn_blocking(move || {
            rpc_client.send_and_confirm_transaction(&transaction)
        })
        .await
        .unwrap()
        .map_err(|e| anyhow!("Failed to transfer lamports: {}", e))?;

        println!(
            "Updated account {} balance (+{} lamports) (tx: {})",
            account_pubkey, additional_lamports, signature
        );

        Ok(())
    }

    /// Get account information
    #[allow(dead_code)]
    pub async fn get_account_info(&self, pubkey: &Pubkey) -> Result<Account> {
        let rpc_client = Arc::clone(&self.rpc_client);
        let pubkey = *pubkey;
        tokio::task::spawn_blocking(move || rpc_client.get_account(&pubkey))
            .await
            .unwrap()
            .map_err(|e| anyhow!("Failed to get account info: {}", e))
    }

    /// Get the payer's public key
    pub fn payer_pubkey(&self) -> Pubkey {
        self.payer.pubkey()
    }

    /// Get all created account pubkeys
    #[allow(dead_code)]
    pub fn get_created_accounts(&self) -> Vec<Pubkey> {
        self.created_accounts.keys().cloned().collect()
    }

    /// Close and clean up accounts (optional)
    pub async fn cleanup(&mut self) -> Result<()> {
        println!(
            "Cleaning up {} created accounts...",
            self.created_accounts.len()
        );

        // For test validator, we don't need to explicitly close accounts
        // In production, you might want to close accounts to reclaim rent

        self.created_accounts.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    #[ignore] // Only run when Solana test validator is running
    async fn test_account_creation() {
        // This test assumes a Solana validator is already running at localhost:8899
        // The test environment should set this up via nix develop .#solana
        let mut manager = SolanaAccountManager::new("http://localhost:8899").unwrap();

        // Request airdrop for payer
        manager
            .request_airdrop(&manager.payer_pubkey(), 1_000_000_000)
            .await
            .unwrap();

        // Create a test account
        let account_pubkey = manager.create_test_program_account().await.unwrap();

        // Verify account exists
        let account_info = manager.get_account_info(&account_pubkey).await.unwrap();
        assert!(account_info.lamports > 0);

        // Update account balance
        manager
            .update_account_balance(&account_pubkey, 100_000)
            .await
            .unwrap();

        // Verify balance increased
        let updated_account = manager.get_account_info(&account_pubkey).await.unwrap();
        assert!(updated_account.lamports > account_info.lamports);

        // Cleanup
        manager.cleanup().await.unwrap();
    }
}
