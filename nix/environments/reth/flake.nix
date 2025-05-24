{
  description = "Reth Workflow Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        
        # Script for Reth workflow
        reth-workflow-script = pkgs.writeShellScriptBin "reth-workflow" ''
          #!/usr/bin/env bash
          set -e
          
          # Make scripts executable
          chmod +x simulation/ethereum/setup-reth.sh
          chmod +x simulation/ethereum/deploy-valence-contracts-reth.sh
          chmod +x simulation/almanac/index-ethereum-reth.sh
          chmod +x simulation/stop-all.sh

          # Stop any running services
          echo "Stopping any running services..."
          nix develop --command bash -c "./simulation/stop-all.sh"
          
          # Initialize databases
          echo "Initializing databases..."
          nix develop --command bash -c "init_databases"
          
          # Set up Reth node
          echo "Setting up Reth node..."
          nix develop --command bash -c "./simulation/ethereum/setup-reth.sh"
          sleep 5 # Give Reth time to fully initialize
          
          # Deploy contracts to Reth
          echo "Deploying contracts to Reth..."
          nix develop --command bash -c "./simulation/ethereum/deploy-valence-contracts-reth.sh"
          
          # Start indexing
          echo "Starting indexer for Reth..."
          nix develop --command bash -c "./simulation/almanac/index-ethereum-reth.sh --duration=2m"
          
          echo "Reth workflow completed!"
        '';
      in {
        packages = {
          default = reth-workflow-script;
          reth-workflow = reth-workflow-script;
        };
        
        apps = {
          default = {
            type = "app";
            program = "${reth-workflow-script}/bin/reth-workflow";
          };
        };

        # Add a devShell with the workflow script
        devShells.default = pkgs.mkShell {
          buildInputs = [ reth-workflow-script ];
          shellHook = ''
            echo "Reth Workflow Environment"
            echo "Run 'reth-workflow' to execute the full workflow"
          '';
        };
      }
    );
} 