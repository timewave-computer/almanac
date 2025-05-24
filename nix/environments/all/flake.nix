{
  description = "All Workflows Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    anvil.url = "path:../anvil";
    reth.url = "path:../reth";
    cosmwasm.url = "path:../cosmwasm";
  };

  outputs = { self, nixpkgs, flake-utils, anvil, reth, cosmwasm, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        
        # Script for running all workflows
        all-workflows-script = pkgs.writeShellScriptBin "all-workflows" ''
          #!/usr/bin/env bash
          set -e
          
          echo "Starting Anvil workflow..."
          nix run ../anvil
          echo "Anvil workflow completed!"
          
          echo "Starting Reth workflow..."
          nix run ../reth
          echo "Reth workflow completed!"
          
          echo "Starting CosmWasm workflow..."
          nix run ../cosmwasm
          echo "CosmWasm workflow completed!"
          
          echo "All workflows completed successfully!"
        '';
      in {
        packages = {
          default = all-workflows-script;
          all-workflows = all-workflows-script;
        };
        
        apps = {
          default = {
            type = "app";
            program = "${all-workflows-script}/bin/all-workflows";
          };
        };

        # Add a devShell with the workflow script
        devShells.default = pkgs.mkShell {
          buildInputs = [ all-workflows-script ];
          shellHook = ''
            echo "All Workflows Environment"
            echo "Run 'all-workflows' to execute all workflows in sequence"
          '';
        };
      }
    );
} 