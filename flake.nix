# This file defines a Nix flake for the Almanac project with CosmWasm support.
{
  description = "Almanac Project Root";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    
    # Input needed for the wasmd build
    wasmd-src = {
      url = "github:CosmWasm/wasmd";
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
        # Default dev shell with cosmos tools
        devShells.default = pkgs.mkShell {
          # Include required packages
          packages = [ 
            pkgs.git 
            # Directly include the cosmos packages
            self'.packages.wasmd
            self'.packages.run-wasmd-node
            self'.packages.test-cosmos-adapter
            pkgs.jq
          ];
          
          shellHook = ''
            echo "=== Almanac Development Environment ==="
            echo "Available commands:"
            echo "  - run-wasmd-node: Start a local wasmd node for testing"
            echo "  - test-cosmos-adapter: Run cosmos adapter tests against local node"
          '';
        };
      };
    };
}