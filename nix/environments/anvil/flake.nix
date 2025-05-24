{
  description = "Anvil Workflow Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        
        # Script for Anvil workflow
        anvil-workflow-script = pkgs.writeShellScriptBin "anvil-workflow" ''
          #!/usr/bin/env bash
          set -e
          
          # Make scripts executable
          chmod +x simulation/ethereum/setup-anvil.sh
          chmod +x simulation/ethereum/deploy-valence-contracts-anvil.sh
          chmod +x simulation/almanac/index-ethereum-anvil.sh
          chmod +x simulation/stop-all.sh

          # Stop any running services
          echo "Stopping any running services..."
          nix develop --command bash -c "./simulation/stop-all.sh"
          
          # Initialize databases
          echo "Initializing databases..."
          nix develop --command bash -c "init_databases"
          
          # Set up Anvil node
          echo "Setting up Anvil node..."
          nix develop --command bash -c "./simulation/ethereum/setup-anvil.sh"
          sleep 5 # Give Anvil time to fully initialize
          
          # Deploy contracts to Anvil
          echo "Deploying contracts to Anvil..."
          nix develop --command bash -c "./simulation/ethereum/deploy-valence-contracts-anvil.sh"
          
          # Start indexing
          echo "Starting indexer for Anvil..."
          nix develop --command bash -c "./simulation/almanac/index-ethereum-anvil.sh --duration=2m"
          
          echo "Anvil workflow completed!"
        '';
      in {
        packages = {
          default = anvil-workflow-script;
          anvil-workflow = anvil-workflow-script;
        };
        
        apps = {
          default = {
            type = "app";
            program = "${anvil-workflow-script}/bin/anvil-workflow";
          };
        };

        # Add a devShell with the workflow script
        devShells.default = pkgs.mkShell {
          buildInputs = [ anvil-workflow-script ];
          shellHook = ''
            echo "Anvil Workflow Environment"
            echo "Run 'anvil-workflow' to execute the full workflow"
          '';
        };
      }
    );
} 