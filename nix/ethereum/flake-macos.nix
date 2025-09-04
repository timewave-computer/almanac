{
  description = "Ethereum development environment for macOS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ { flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.devshell.flakeModule
      ];

      systems = [ "aarch64-darwin" "x86_64-darwin" ];

      perSystem = { config, self', inputs', pkgs, system, ... }: {
        packages = rec {
          # Use system packages optimized for macOS
          geth = pkgs.go-ethereum;
          lighthouse = if (pkgs ? lighthouse && !pkgs.lighthouse.meta.broken or false) 
            then pkgs.lighthouse 
            else pkgs.writeShellApplication {
              name = "lighthouse";
              text = ''
                echo "Lighthouse not available on macOS in current nixpkgs"
                echo "Install via:"
                echo "  brew install lighthouse"
                echo "  or download from: https://github.com/sigp/lighthouse/releases"
                exit 1
              '';
            };
          foundry = pkgs.foundry;
        };

        devshells.default = {
          name = "ethereum-development-macos";
          
          commands = [
            {
              package = self'.packages.geth;
              help = "Ethereum execution client (geth)";
            }
            {
              package = self'.packages.lighthouse;
              help = "Ethereum consensus client (lighthouse)";
            }
            {
              package = self'.packages.foundry;
              help = "Ethereum development framework (forge, cast, anvil)";
            }
            {
              package = pkgs.curl;
              help = "HTTP client for API testing";
            }
            {
              package = pkgs.jq;
              help = "JSON processor for API responses";
            }
            {
              package = pkgs.openssl;
              help = "SSL/TLS toolkit";
            }
            {
              package = pkgs.nodejs;
              help = "Node.js for web3 development";
            }
            {
              package = pkgs.yarn;
              help = "Package manager for Node.js";
            }
            {
              package = pkgs.python3;
              help = "Python for web3.py development";
            }
            {
              package = pkgs.git;
              help = "Version control";
            }
          ];
          
          env = [
            {
              name = "ETH_RPC_URL";
              value = "http://127.0.0.1:8545";
            }
            {
              name = "ETH_WS_URL";
              value = "ws://127.0.0.1:8546";
            }
            {
              name = "FOUNDRY_PROFILE";
              value = "default";
            }
            {
              name = "ANVIL_IP_ADDR";
              value = "0.0.0.0";
            }
            {
              name = "MACOSX_DEPLOYMENT_TARGET";
              value = "10.15";
            }
          ];
          
          motd = ''
            {14}{bold}Ethereum development environment (macOS){reset}
            {9}Optimized for Apple Silicon and Intel Macs{reset}
            
            {13}Available tools:{reset}
              - geth: Ethereum execution client (${self'.packages.geth.version or "latest"})
              - lighthouse: Ethereum consensus client (install via brew if not available)
              - foundry: Complete Ethereum development framework
                * forge: Build, test, and deploy contracts
                * cast: Ethereum RPC client and Swiss Army knife
                * anvil: Local Ethereum node for development
              - Node.js/Yarn: For web3.js development
              - Python: For web3.py development
            
            {13}Quick start:{reset}
              # Start local development node
              anvil                     # Local Ethereum node (port 8545)
              
              # Ethereum mainnet/testnet
              geth --sepolia --datadir ./data/geth --http --ws
              lighthouse bn --network sepolia --datadir ./data/lighthouse
              
              # Smart contract development
              forge init my-project     # Create new Foundry project
              cd my-project
              forge build               # Compile contracts
              forge test                # Run tests
              forge script script/Deploy.s.sol --rpc-url $ETH_RPC_URL --broadcast
              
            {13}Environment variables:{reset}
              - ETH_RPC_URL: $ETH_RPC_URL
              - ETH_WS_URL: $ETH_WS_URL
              - FOUNDRY_PROFILE: $FOUNDRY_PROFILE
              - ANVIL_IP_ADDR: $ANVIL_IP_ADDR
              - MACOSX_DEPLOYMENT_TARGET: $MACOSX_DEPLOYMENT_TARGET
          '';
        };
      };
    };
}
