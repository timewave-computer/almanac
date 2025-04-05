# This file defines a Nix flake for the Almanac project with CosmWasm support.
{
  description = "Almanac Project Root";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    
    # Input needed for flake reference (not used directly anymore)
    wasmd-src = {
      url = "github:CosmWasm/wasmd/v0.31.0";
      flake = false;
    };
  };

  outputs = inputs@{ self, nixpkgs, flake-parts, wasmd-src, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      # Import our modules
      imports = [
        ./nix/cosmos-module.nix
      ];
      
      systems = ["aarch64-darwin" "x86_64-linux"];

      # Define perSystem configuration
      perSystem = { config, self', pkgs, ... }: {
        # Create the default development shell
        devShells.default = pkgs.mkShell {
          packages = [ 
            pkgs.git 
            # Include essential cosmos packages
            self'.packages.wasmd-setup
            self'.packages.wasmd-node
            self'.packages.test-cosmos-adapter
            pkgs.jq
            pkgs.go
          ];
          
          shellHook = ''
            echo "=== Almanac Development Environment ==="
            echo "Available commands:"
            echo "  - wasmd-setup: Install wasmd from source via Go"
            echo "  - wasmd-node: Start a local wasmd node for testing"
            echo "  - test-cosmos-adapter: Run cosmos adapter tests against local node"
            
            # Check if wasmd is already installed
            if [ -f "$HOME/go/bin/wasmd" ]; then
              echo "✓ wasmd is already installed"
            else
              echo "ℹ Run 'wasmd-setup' to install wasmd"
            fi
          '';
        };
      };
    };
}