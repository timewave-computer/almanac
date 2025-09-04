{
  description = "Ethereum development environment for NixOS 25.05";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
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

      systems = [ "x86_64-linux" "aarch64-linux" ];

      perSystem = { config, self', inputs', pkgs, system, ... }: {
        packages = rec {
          # Use stable system packages from nixos-25.05
          geth = pkgs.go-ethereum;
          lighthouse = pkgs.lighthouse;
          foundry = pkgs.foundry;
        };

        devshells.default = {
          name = "ethereum-development-nixos";
          
          commands = [
            {
              package = self'.packages.geth;
              help = "Ethereum execution client (geth v1.15.11)";
            }
            {
              package = self'.packages.lighthouse;
              help = "Ethereum consensus client (lighthouse v6.0.1)";
            }
            {
              package = self'.packages.foundry;
              help = "Ethereum development framework (forge v1.1.0, cast, anvil)";
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
            {
              package = pkgs.gcc;
              help = "GNU Compiler Collection";
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
              name = "CC";
              value = "${pkgs.gcc}/bin/gcc";
            }
            {
              name = "CXX";
              value = "${pkgs.gcc}/bin/g++";
            }
          ];
          
          motd = ''
            {14}{bold}Ethereum development environment (NixOS 25.05){reset}
            {9}Stable packages from NixOS 25.05 release{reset}
            
            {13}Available tools:{reset}
              - geth: Ethereum execution client (v1.15.11)
              - lighthouse: Ethereum consensus client (v6.0.1)
              - foundry: Complete Ethereum development framework (v1.1.0)
                * forge: Build, test, and deploy contracts
                * cast: Ethereum RPC client and Swiss Army knife
                * anvil: Local Ethereum node for development
              - Node.js/Yarn: For web3.js development
              - Python: For web3.py development
              - GCC: GNU Compiler Collection for native builds
            
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
              - CC: $CC / CXX: $CXX
          '';
        };
      };
    };
}
