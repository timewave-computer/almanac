{
  description = "Almanac simulation orchestrator (simplified)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs @ { flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      perSystem = { config, self', inputs', pkgs, system, lib, ... }: let
        
        # Platform detection
        isDarwin = pkgs.stdenv.isDarwin;
        isLinux = pkgs.stdenv.isLinux;

        # Simple simulation runner that instructs users to use individual flakes
        simRunner = pkgs.writeShellApplication {
          name = "sim-runner";
          runtimeInputs = [ pkgs.coreutils ];
          text = ''
            cat << EOF
            Almanac Multi-Chain Simulation Runner
            
            To run the full simulation environment, use each chain's flake:
            
            1. Foundation DB:
               cd nix/fdb && nix develop
               fdb-manage start
            
            2. Ethereum:
               cd nix/ethereum && nix develop
               anvil-manage start
            
            3. Cosmos:
               cd nix/cosmos && nix develop
               cosmos-manage start
            
            4. Solana:
               cd nix/solana && nix develop
               solana-manage start
            
            Each environment provides its own management commands.
            Storage backend: ${if isLinux then "FoundationDB (with mock fallback)" else "Memory"}
            EOF
          '';
        };

      in {
        packages.default = simRunner;
        
        apps.default = {
          type = "app";
          program = "${simRunner}/bin/sim-runner";
        };

        devShells.default = pkgs.mkShell {
          name = "almanac-simulation";
          
          buildInputs = with pkgs; [
            # Core tools
            rustc
            cargo
            gcc
            openssl
            pkg-config
            curl
            jq
          ];

          shellHook = ''
            echo "Almanac Simulation Environment"
            echo ""
            echo "This is the orchestration environment. For chain-specific tools:"
            echo ""
            echo "  cd ../ethereum && nix develop    # Ethereum/Anvil"
            echo "  cd ../cosmos && nix develop      # Cosmos/Neutron"
            echo "  cd ../solana && nix develop      # Solana validator"
            echo "  cd ../fdb && nix develop         # FoundationDB"
            echo ""
            echo "Each provides management commands like:"
            echo "  anvil-manage start|stop|status"
            echo "  cosmos-manage start|stop|status"
            echo "  solana-manage start|stop|status"
            echo "  fdb-manage start|stop|status"
          '';
        };
      };
    };
}