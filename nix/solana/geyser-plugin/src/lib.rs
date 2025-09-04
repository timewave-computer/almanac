//! Almanac Geyser Plugin for Solana
//! 
//! This plugin streams Solana blockchain data to an Almanac database
//! for historical state tracking and analysis.

use anyhow::Result;
use deadpool_postgres::{Config as PoolConfig, Pool, Runtime};
use log::*;
use serde::{Deserialize, Serialize};
use solana_geyser_plugin_interface::geyser_plugin_interface::{
    GeyserPlugin, GeyserPluginError, ReplicaAccountInfoVersions, ReplicaBlockInfoVersions,
    ReplicaTransactionInfoVersions, SlotStatus, Result as PluginResult,
};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use tokio::runtime::Runtime as TokioRuntime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub database_url: String,
    pub max_connections: Option<u32>,
    pub accounts_selector: AccountsSelector,
    pub transaction_selector: TransactionSelector,
    pub instruction_selector: InstructionSelector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountsSelector {
    pub owners: Vec<String>,
    pub accounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSelector {
    pub mentions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionSelector {
    pub programs: Vec<String>,
}

pub struct AlmanacGeyserPlugin {
    config: Option<PluginConfig>,
    pool: Option<Arc<Pool>>,
    runtime: Option<Arc<TokioRuntime>>,
}

impl Default for AlmanacGeyserPlugin {
    fn default() -> Self {
        Self {
            config: None,
            pool: None,
            runtime: None,
        }
    }
}

impl AlmanacGeyserPlugin {
    fn init_db_pool(&mut self) -> Result<(), GeyserPluginError> {
        let config = self.config.as_ref()
            .ok_or_else(|| GeyserPluginError::Custom("Config not initialized".into()))?;
        
        // Create tokio runtime
        let runtime = TokioRuntime::new()
            .map_err(|e| GeyserPluginError::Custom(format!("Failed to create runtime: {}", e)))?;
        
        // Parse database URL and create pool config
        let mut pool_config = PoolConfig::new();
        pool_config.dbname = Some("almanac".to_string());
        pool_config.host = Some("localhost".to_string());
        pool_config.user = Some("almanac".to_string());
        pool_config.password = Some("almanac".to_string());
        
        if let Some(max_conns) = config.max_connections {
            pool_config.max_size = max_conns as usize;
        }
        
        // Create connection pool
        let pool = runtime.block_on(async {
            pool_config.create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
        }).map_err(|e| GeyserPluginError::Custom(format!("Failed to create pool: {}", e)))?;
        
        self.pool = Some(Arc::new(pool));
        self.runtime = Some(Arc::new(runtime));
        
        // Initialize database schema
        runtime.block_on(self.init_schema())
            .map_err(|e| GeyserPluginError::Custom(format!("Failed to init schema: {}", e)))?;
        
        Ok(())
    }
    
    async fn init_schema(&self) -> Result<()> {
        let pool = self.pool.as_ref().unwrap();
        let client = pool.get().await?;
        
        // Create tables if they don't exist
        client.batch_execute(r#"
            CREATE TABLE IF NOT EXISTS solana_accounts (
                pubkey VARCHAR(44) PRIMARY KEY,
                owner VARCHAR(44) NOT NULL,
                lamports BIGINT NOT NULL,
                slot BIGINT NOT NULL,
                executable BOOLEAN NOT NULL,
                rent_epoch BIGINT NOT NULL,
                data BYTEA,
                write_version BIGINT NOT NULL,
                updated_at TIMESTAMP NOT NULL DEFAULT NOW()
            );
            
            CREATE TABLE IF NOT EXISTS solana_slots (
                slot BIGINT PRIMARY KEY,
                parent_slot BIGINT,
                status VARCHAR(20) NOT NULL,
                updated_at TIMESTAMP NOT NULL DEFAULT NOW()
            );
            
            CREATE TABLE IF NOT EXISTS solana_transactions (
                signature VARCHAR(88) PRIMARY KEY,
                slot BIGINT NOT NULL,
                success BOOLEAN NOT NULL,
                fee BIGINT NOT NULL,
                pre_balances JSONB,
                post_balances JSONB,
                instructions JSONB,
                created_at TIMESTAMP NOT NULL DEFAULT NOW()
            );
            
            CREATE INDEX IF NOT EXISTS idx_accounts_owner ON solana_accounts(owner);
            CREATE INDEX IF NOT EXISTS idx_accounts_slot ON solana_accounts(slot);
            CREATE INDEX IF NOT EXISTS idx_transactions_slot ON solana_transactions(slot);
        "#)?;
        
        Ok(())
    }
    
    fn should_process_account(&self, owner: &[u8]) -> bool {
        let config = match &self.config {
            Some(c) => c,
            None => return false,
        };
        
        let owner_str = bs58::encode(owner).into_string();
        
        // Check if owner is in whitelist
        if !config.accounts_selector.owners.is_empty() {
            return config.accounts_selector.owners.contains(&owner_str);
        }
        
        // If no filters specified, process all accounts
        true
    }
}

impl GeyserPlugin for AlmanacGeyserPlugin {
    fn name(&self) -> &'static str {
        "AlmanacGeyserPlugin"
    }
    
    fn on_load(&mut self, config_file: &str) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        info!("Loading Almanac Geyser plugin from: {}", config_file);
        
        // Read config file
        let mut file = File::open(config_file)
            .map_err(|e| GeyserPluginError::Custom(format!("Failed to open config: {}", e)))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| GeyserPluginError::Custom(format!("Failed to read config: {}", e)))?;
        
        // Skip the outer wrapper that contains libpath
        let config_json: serde_json::Value = serde_json::from_str(&contents)
            .map_err(|e| GeyserPluginError::Custom(format!("Failed to parse config: {}", e)))?;
        
        // Extract our config (everything except libpath)
        let mut our_config = config_json.clone();
        if let Some(obj) = our_config.as_object_mut() {
            obj.remove("libpath");
        }
        
        let config: PluginConfig = serde_json::from_value(our_config)
            .map_err(|e| GeyserPluginError::Custom(format!("Failed to parse plugin config: {}", e)))?;
        
        self.config = Some(config);
        
        // Initialize database connection pool
        self.init_db_pool()?;
        
        info!("Almanac Geyser plugin loaded successfully");
        Ok(())
    }
    
    fn on_unload(&mut self) {
        info!("Unloading Almanac Geyser plugin");
        self.pool = None;
        self.runtime = None;
        self.config = None;
    }
    
    fn update_account(
        &mut self,
        account: ReplicaAccountInfoVersions,
        slot: u64,
        _is_startup: bool,
    ) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        let account_info = match account {
            ReplicaAccountInfoVersions::V0_0_1(info) => info,
            ReplicaAccountInfoVersions::V0_0_2(info) => info,
            ReplicaAccountInfoVersions::V0_0_3(info) => info,
        };
        
        // Check if we should process this account
        if !self.should_process_account(account_info.owner) {
            return Ok(());
        }
        
        let pool = self.pool.clone().unwrap();
        let runtime = self.runtime.as_ref().unwrap();
        
        let pubkey_str = bs58::encode(account_info.pubkey).into_string();
        let owner_str = bs58::encode(account_info.owner).into_string();
        let lamports = account_info.lamports;
        let executable = account_info.executable;
        let rent_epoch = account_info.rent_epoch;
        let data = account_info.data.to_vec();
        let write_version = account_info.write_version;
        
        runtime.spawn(async move {
            if let Ok(client) = pool.get().await {
                let _ = client.execute(
                    r#"
                    INSERT INTO solana_accounts 
                        (pubkey, owner, lamports, slot, executable, rent_epoch, data, write_version)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    ON CONFLICT (pubkey) DO UPDATE SET
                        owner = $2,
                        lamports = $3,
                        slot = $4,
                        executable = $5,
                        rent_epoch = $6,
                        data = $7,
                        write_version = $8,
                        updated_at = NOW()
                    "#,
                    &[
                        &pubkey_str,
                        &owner_str,
                        &(lamports as i64),
                        &(slot as i64),
                        &executable,
                        &(rent_epoch as i64),
                        &data_vec,
                        &(write_version as i64),
                    ],
                ).await;
            }
        });
        
        Ok(())
    }
    
    fn notify_end_of_startup(&mut self) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        info!("End of startup");
        Ok(())
    }
    
    fn update_slot_status(
        &mut self,
        slot: u64,
        parent: Option<u64>,
        status: SlotStatus,
    ) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        let pool = self.pool.clone().unwrap();
        let runtime = self.runtime.as_ref().unwrap();
        
        let status_str = format!("{:?}", status);
        let parent_slot = parent.map(|p| p as i64);
        
        runtime.spawn(async move {
            if let Ok(client) = pool.get().await {
                let _ = client.execute(
                    r#"
                    INSERT INTO solana_slots (slot, parent_slot, status)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (slot) DO UPDATE SET
                        parent_slot = $2,
                        status = $3,
                        updated_at = NOW()
                    "#,
                    &[&(slot as i64), &parent_slot, &status_str],
                ).await;
            }
        });
        
        Ok(())
    }
    
    fn notify_transaction(
        &mut self,
        transaction: ReplicaTransactionInfoVersions,
        slot: u64,
    ) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        let transaction_info = match transaction {
            ReplicaTransactionInfoVersions::V0_0_1(info) => info,
            ReplicaTransactionInfoVersions::V0_0_2(info) => info,
        };
        
        let signature = bs58::encode(&transaction_info.signature).into_string();
        let is_success = transaction_info.is_vote;
        
        let pool = self.pool.clone().unwrap();
        let runtime = self.runtime.as_ref().unwrap();
        
        runtime.spawn(async move {
            if let Ok(client) = pool.get().await {
                let _ = client.execute(
                    r#"
                    INSERT INTO solana_transactions (signature, slot, success, fee, pre_balances, post_balances)
                    VALUES ($1, $2, $3, $4, $5, $6)
                    ON CONFLICT (signature) DO NOTHING
                    "#,
                    &[
                        &signature,
                        &(slot as i64),
                        &is_success,
                        &0i64, // fee would need to be calculated
                        &serde_json::Value::Null,
                        &serde_json::Value::Null,
                    ],
                ).await;
            }
        });
        
        Ok(())
    }
    
    fn notify_block_metadata(
        &mut self,
        _block_info: ReplicaBlockInfoVersions,
    ) -> PluginResult<()> {
        Ok(())
    }
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub unsafe extern "C" fn _create_plugin() -> *mut dyn GeyserPlugin {
    let plugin = AlmanacGeyserPlugin::default();
    let plugin: Box<dyn GeyserPlugin> = Box::new(plugin);
    Box::into_raw(plugin)
}