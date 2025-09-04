//! Nix environment utilities for cross-platform E2E testing
//!
//! This module provides utilities to detect the operating system and select
//! the appropriate Nix flake for running blockchain environments in tests.

use std::process::Command;
use anyhow::{anyhow, Result};

/// Supported operating systems for E2E testing
#[derive(Debug, Clone, PartialEq)]
pub enum OperatingSystem {
    MacOS,
    NixOS,
    Linux,
}

/// Nix environment configuration for blockchain testing
#[derive(Debug, Clone)]
pub struct NixEnvironment {
    pub os: OperatingSystem,
    pub ethereum_flake: String,
    pub cosmos_flake: String,
    pub solana_flake: String,
    pub fdb_flake: String,
}

impl NixEnvironment {
    /// Detect the current operating system and return appropriate Nix environment
    pub fn detect() -> Result<Self> {
        let os = detect_operating_system()?;
        
        match os {
            OperatingSystem::MacOS => Ok(Self {
                os,
                ethereum_flake: "../nix/ethereum/flake-macos.nix".to_string(),
                cosmos_flake: "../nix/cosmos/flake-macos.nix".to_string(),
                solana_flake: "../nix/solana/flake-macos.nix".to_string(),
                fdb_flake: "../nix/fdb/flake-macos.nix".to_string(),
            }),
            OperatingSystem::NixOS | OperatingSystem::Linux => Ok(Self {
                os,
                ethereum_flake: "../nix/ethereum/flake-nixos.nix".to_string(),
                cosmos_flake: "../nix/cosmos/flake-nixos.nix".to_string(),
                solana_flake: "../nix/solana/flake-nixos.nix".to_string(),
                fdb_flake: "../nix/fdb/flake-nixos.nix".to_string(),
            }),
        }
    }

    /// Run a command in the appropriate Ethereum Nix environment
    pub fn run_ethereum_command(&self, command: &str, args: &[&str]) -> Result<std::process::Child> {
        self.run_nix_command(&self.ethereum_flake, command, args)
    }

    /// Run a command in the appropriate Cosmos Nix environment
    pub fn run_cosmos_command(&self, command: &str, args: &[&str]) -> Result<std::process::Child> {
        self.run_nix_command(&self.cosmos_flake, command, args)
    }

    /// Run a command in the appropriate Solana Nix environment
    pub fn run_solana_command(&self, command: &str, args: &[&str]) -> Result<std::process::Child> {
        self.run_nix_command(&self.solana_flake, command, args)
    }

    /// Run a command in the FoundationDB Nix environment
    pub fn run_fdb_command(&self, command: &str, args: &[&str]) -> Result<std::process::Child> {
        self.run_nix_command(&self.fdb_flake, command, args)
    }

    /// Run a command in a specific Nix environment
    fn run_nix_command(&self, flake_path: &str, command: &str, args: &[&str]) -> Result<std::process::Child> {
        let mut nix_args = vec!["develop", flake_path, "--command", command];
        nix_args.extend(args);

        Command::new("nix")
            .args(&nix_args)
            .spawn()
            .map_err(|e| anyhow!("Failed to run command '{}' in Nix environment '{}': {}", command, flake_path, e))
    }

    /// Get the appropriate Nix develop command for a blockchain environment
    pub fn get_nix_develop_command(&self, blockchain: &str) -> Result<Vec<String>> {
        let flake_path = match blockchain {
            "ethereum" => &self.ethereum_flake,
            "cosmos" => &self.cosmos_flake,
            "solana" => &self.solana_flake,
            "fdb" => &self.fdb_flake,
            _ => return Err(anyhow!("Unsupported blockchain: {}", blockchain)),
        };

        Ok(vec![
            "nix".to_string(),
            "develop".to_string(),
            flake_path.clone(),
            "--command".to_string(),
        ])
    }

    /// Check if a blockchain environment is available
    pub fn check_blockchain_availability(&self, blockchain: &str) -> Result<bool> {
        let flake_path = match blockchain {
            "ethereum" => &self.ethereum_flake,
            "cosmos" => &self.cosmos_flake,
            "solana" => &self.solana_flake,
            "fdb" => &self.fdb_flake,
            _ => return Err(anyhow!("Unsupported blockchain: {}", blockchain)),
        };

        // Check if the flake file exists
        std::path::Path::new(flake_path).exists().then_some(true).ok_or_else(|| {
            anyhow!("Blockchain environment '{}' not available: flake '{}' not found", blockchain, flake_path)
        })
    }
}

/// Detect the current operating system
fn detect_operating_system() -> Result<OperatingSystem> {
    // First check if we're on NixOS specifically
    if is_nixos()? {
        return Ok(OperatingSystem::NixOS);
    }

    // Then check the general OS
    match std::env::consts::OS {
        "macos" => Ok(OperatingSystem::MacOS),
        "linux" => Ok(OperatingSystem::Linux),
        os => Err(anyhow!("Unsupported operating system: {}", os)),
    }
}

/// Check if the current system is NixOS
fn is_nixos() -> Result<bool> {
    // Check for NixOS-specific files/directories
    if std::path::Path::new("/etc/nixos").exists() {
        return Ok(true);
    }

    // Check /etc/os-release for NixOS
    if let Ok(contents) = std::fs::read_to_string("/etc/os-release") {
        if contents.contains("ID=nixos") || contents.contains("NAME=\"NixOS\"") {
            return Ok(true);
        }
    }

    // Check if nixos-rebuild is available
    if let Ok(output) = Command::new("which").arg("nixos-rebuild").output() {
        if output.status.success() {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_detection() {
        let env = NixEnvironment::detect();
        assert!(env.is_ok(), "Should be able to detect operating system");
        
        let env = env.unwrap();
        println!("Detected OS: {:?}", env.os);
        println!("Ethereum flake: {}", env.ethereum_flake);
        println!("Cosmos flake: {}", env.cosmos_flake);
        println!("Solana flake: {}", env.solana_flake);
        println!("FoundationDB flake: {}", env.fdb_flake);
    }

    #[test]
    fn test_blockchain_availability() {
        let env = NixEnvironment::detect().unwrap();
        
        // These should work if the flakes exist
        let ethereum_available = env.check_blockchain_availability("ethereum");
        let cosmos_available = env.check_blockchain_availability("cosmos");
        let solana_available = env.check_blockchain_availability("solana");
        let fdb_available = env.check_blockchain_availability("fdb");
        
        println!("Ethereum available: {:?}", ethereum_available);
        println!("Cosmos available: {:?}", cosmos_available);
        println!("Solana available: {:?}", solana_available);
        println!("FoundationDB available: {:?}", fdb_available);
    }

    #[test]
    fn test_nix_develop_command() {
        let env = NixEnvironment::detect().unwrap();
        
        let ethereum_cmd = env.get_nix_develop_command("ethereum").unwrap();
        assert!(ethereum_cmd.contains(&"nix".to_string()));
        assert!(ethereum_cmd.contains(&"develop".to_string()));
        
        println!("Ethereum command: {:?}", ethereum_cmd);
    }
}
