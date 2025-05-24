{
  description = "CosmWasm Workflow Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        
        # Script for CosmWasm workflow
        cosmwasm-workflow-script = pkgs.writeShellScriptBin "cosmwasm-workflow" ''
          #!/usr/bin/env bash
          set -e
          
          # Make scripts executable
          chmod +x simulation/cosmos/setup-wasmd.sh
          chmod +x simulation/cosmos/deploy-cw721-contracts.sh
          chmod +x simulation/almanac/index-cosmwasm.sh
          chmod +x simulation/stop-all.sh

          # Stop any running services
          echo "Stopping any running services..."
          nix develop --command bash -c "./simulation/stop-all.sh"
          
          # Initialize databases
          echo "Initializing databases..."
          nix develop --command bash -c "init_databases"
          
          # Set up wasmd node
          echo "Setting up wasmd node..."
          nix develop --command bash -c "./simulation/cosmos/setup-wasmd.sh"
          sleep 5 # Give wasmd time to fully initialize
          
          # Deploy contracts to wasmd
          echo "Deploying contracts to wasmd..."
          nix develop --command bash -c "./simulation/cosmos/deploy-cw721-contracts.sh"
          
          # Start indexing
          echo "Starting indexer for CosmWasm..."
          nix develop --command bash -c "./simulation/almanac/index-cosmwasm.sh --duration=2m"
          
          echo "CosmWasm workflow completed!"
        '';
      in {
        packages = {
          default = cosmwasm-workflow-script;
          cosmwasm-workflow = cosmwasm-workflow-script;
        };
        
        apps = {
          default = {
            type = "app";
            program = "${cosmwasm-workflow-script}/bin/cosmwasm-workflow";
          };
        };

        # Add a devShell with the workflow script
        devShells.default = pkgs.mkShell {
          buildInputs = [ cosmwasm-workflow-script ];
          shellHook = ''
            echo "CosmWasm Workflow Environment"
            echo "Run 'cosmwasm-workflow' to execute the full workflow"
          '';
        };
      }
    );
}