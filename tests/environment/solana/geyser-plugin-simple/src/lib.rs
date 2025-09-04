//! Simplified Almanac Geyser Plugin for testing

use log::*;
use solana_geyser_plugin_interface::geyser_plugin_interface::{
    GeyserPlugin, GeyserPluginError, ReplicaAccountInfoVersions, ReplicaBlockInfoVersions,
    ReplicaTransactionInfoVersions, SlotStatus, Result as PluginResult,
};

#[derive(Default)]
pub struct AlmanacGeyserPlugin;

impl GeyserPlugin for AlmanacGeyserPlugin {
    fn name(&self) -> &'static str {
        "AlmanacGeyserPlugin"
    }
    
    fn on_load(&mut self, config_file: &str) -> PluginResult<()> {
        info!("Loading Almanac Geyser plugin from: {}", config_file);
        info!("Almanac Geyser plugin loaded successfully (test mode)");
        Ok(())
    }
    
    fn on_unload(&mut self) {
        info!("Unloading Almanac Geyser plugin");
    }
    
    fn update_account(
        &mut self,
        account: ReplicaAccountInfoVersions,
        slot: u64,
        _is_startup: bool,
    ) -> PluginResult<()> {
        match account {
            ReplicaAccountInfoVersions::V0_0_1(info) => {
                debug!("Account update: {} at slot {}", bs58::encode(info.pubkey).into_string(), slot);
            }
            ReplicaAccountInfoVersions::V0_0_2(info) => {
                debug!("Account update v2: {} at slot {}", bs58::encode(info.pubkey).into_string(), slot);
            }
            ReplicaAccountInfoVersions::V0_0_3(info) => {
                debug!("Account update v3: {} at slot {}", bs58::encode(info.pubkey).into_string(), slot);
            }
        }
        Ok(())
    }
    
    fn notify_end_of_startup(&mut self) -> PluginResult<()> {
        info!("End of startup");
        Ok(())
    }
    
    fn update_slot_status(
        &mut self,
        slot: u64,
        _parent: Option<u64>,
        status: SlotStatus,
    ) -> PluginResult<()> {
        debug!("Slot {} status: {:?}", slot, status);
        Ok(())
    }
    
    fn notify_transaction(
        &mut self,
        _transaction: ReplicaTransactionInfoVersions,
        slot: u64,
    ) -> PluginResult<()> {
        debug!("Transaction at slot {}", slot);
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