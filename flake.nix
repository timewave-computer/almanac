# This file defines a Nix flake for the Almanac project with CosmWasm support.
{
  description = "Almanac: Indexing and data access layer for cross-chain data";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    # No longer need direct reth input
    # reth.url = "github:paradigmxyz/reth/main";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    # Input needed for flake reference (not used directly anymore)
    wasmd-src = {
      url = "github:CosmWasm/wasmd/v0.31.0";
      flake = false;
    };
    foundry = {
      url = "github:foundry-rs/foundry";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-parts, wasmd-src, foundry, flake-utils, ... }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      # Apply nixpkgs config and overlays here
      flake = { 
        nixpkgs.config = {
          allowUnfree = true;
          allowUnsupportedSystem = true;
        };
      };
      # Import our modules
      imports = [
        # Use relative paths for local modules
        ./nix/rust.nix
        ./nix/database-module.nix
        ./nix/valence-contracts.nix
        ./nix/reth.nix
        ./nix/cosmos-module.nix
        ./nix/cross-chain-module.nix
        foundry.flakeModule
      ];
      
      systems = [ "aarch64-darwin" "x86_64-linux" ]; # Add systems as needed

      # Define perSystem configuration
      perSystem = { config, self', inputs', pkgs, system, ... }:
        let
          # Use crane library provided by rust.nix
          craneLib = config.rust.craneLib;
          
          # Combine all packages from imported modules recursively
          allPackages = pkgs.lib.recursiveUpdate config.packages (
                          pkgs.lib.recursiveUpdate (config.valenceContracts.packages or {}) 
                                              (config.reth.packages or {})
                        ); 

          # Get all devShells defined by modules
          allDevShells = config.devShells;
          
          # Add rust toolchain and hooks to devShells.default if it exists
          defaultDevShell = if allDevShells ? default then
            allDevShells.default.overrideAttrs (old: {
              packages = old.packages ++ [ config.rust.toolchain ];
              # Combine shellHooks from postgres, reth, etc.
              shellHook = pkgs.lib.strings.concatStringsSep "\n" (
                [ old.shellHook or "" 
                  config.postgres.shellHook or "" 
                ] 
                ++ (if config.reth ? shellHook then [config.reth.shellHook] else [])
                # Add other module hooks here if needed
              );
            })
          else pkgs.mkShell { # Create a basic default shell if none exists
            packages = [ config.rust.toolchain ];
            # Combine hooks even for the basic shell
            shellHook = pkgs.lib.strings.concatStringsSep "\n" (
              [ config.postgres.shellHook or "" ]
              ++ (if config.reth ? shellHook then [config.reth.shellHook] else [])
              # Add other module hooks here if needed
            );
          };

          # Combine apps recursively
          allApps = pkgs.lib.recursiveUpdate config.apps (
                      pkgs.lib.recursiveUpdate (config.valenceContracts.apps or {}) 
                                          (config.reth.apps or {})
                    );

        in
        {
          # Make all collected packages available
          packages = allPackages;

          # Make collected devShells available, ensuring default includes Rust toolchain
          devShells = allDevShells // { default = defaultDevShell; };

          # Make combined apps available
          apps = allApps;

          # Checks defined in rust.nix
          checks = config.rust.checks;

          # Formatter defined in rust.nix
          formatter = config.rust.formatter;
        };
    };
}