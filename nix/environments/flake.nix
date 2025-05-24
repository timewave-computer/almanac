{
  description = "Almanac Workflow Environments";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    anvil.url = "path:./anvil";
    reth.url = "path:./reth";
    cosmwasm.url = "path:./cosmwasm";
    all.url = "path:./all";
  };

  outputs = { self, nixpkgs, flake-utils, anvil, reth, cosmwasm, all, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        
        # Script for workflow menu
        workflow-menu-script = pkgs.writeShellScriptBin "workflow-menu" ''
          #!/usr/bin/env bash
          
          echo "=== Almanac Workflow Menu ==="
          echo "1) Anvil Workflow"
          echo "2) Reth Workflow"
          echo "3) CosmWasm Workflow"
          echo "4) Run All Workflows"
          echo ""
          echo -n "Select a workflow (1-4): "
          read choice
          
          case $choice in
            1)
              echo "Running Anvil workflow..."
              nix run ./anvil
              ;;
            2)
              echo "Running Reth workflow..."
              nix run ./reth
              ;;
            3)
              echo "Running CosmWasm workflow..."
              nix run ./cosmwasm
              ;;
            4)
              echo "Running all workflows..."
              nix run ./all
              ;;
            *)
              echo "Invalid choice. Please select a number between 1 and 4."
              exit 1
              ;;
          esac
        '';
      in {
        packages = {
          default = workflow-menu-script;
          workflow-menu = workflow-menu-script;
          anvil-workflow = anvil.packages.${system}.default;
          reth-workflow = reth.packages.${system}.default;
          cosmwasm-workflow = cosmwasm.packages.${system}.default;
          all-workflows = all.packages.${system}.default;
        };
        
        apps = {
          default = {
            type = "app";
            program = "${workflow-menu-script}/bin/workflow-menu";
          };
          anvil = anvil.apps.${system}.default;
          reth = reth.apps.${system}.default;
          cosmwasm = cosmwasm.apps.${system}.default;
          all = all.apps.${system}.default;
        };

        # Add a devShell with all workflow scripts
        devShells.default = pkgs.mkShell {
          buildInputs = [
            workflow-menu-script
            anvil.packages.${system}.default
            reth.packages.${system}.default
            cosmwasm.packages.${system}.default
            all.packages.${system}.default
          ];
          shellHook = ''
            echo "Almanac Workflow Environments"
            echo "Run 'workflow-menu' to select a workflow to run"
          '';
        };
      }
    );
} 