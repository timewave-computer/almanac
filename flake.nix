{
  description = "Almanac: Cross-chain event indexer and processor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    
    # Blockchain development environments
    fdb = {
      url = "path:./nix/fdb";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    ethereum = {
      url = "path:./nix/ethereum";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    cosmos = {
      url = "path:./nix/cosmos";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    solana = {
      url = "path:./nix/solana";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    simulation = {
      url = "path:./nix/simulation";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    swagger-ui = {
      url = "path:./nix/swagger-ui";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ self, nixpkgs, flake-utils, fdb, ethereum, cosmos, solana, simulation, swagger-ui, ... }:
    flake-utils.lib.eachSystem [ "aarch64-darwin" "x86_64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        lib = pkgs.lib;
        
        # For macOS, set a deployment target
        darwinDeploymentTarget = "11.0";
        
        # Import packages from subflakes
        fdb-packages = fdb.packages.${system};
        ethereum-packages = ethereum.packages.${system};
        cosmos-packages = cosmos.packages.${system};
        solana-packages = solana.packages.${system};
        simulation-packages = simulation.packages.${system};
        swagger-ui-packages = swagger-ui.packages.${system};
        
        # Create a set of common environment variables
        commonEnv = {
          MACOSX_DEPLOYMENT_TARGET = darwinDeploymentTarget;
          DEVELOPER_DIR = "";
        };
        
        # Combined development shell with all blockchain tools
        combinedDevShell = pkgs.mkShell {
          name = "almanac-development";
          
          packages = [ 
            # Rust toolchain
            pkgs.rustc
            pkgs.cargo
            pkgs.pkg-config 
            pkgs.openssl
            pkgs.curl
            pkgs.jq
            
            # FoundationDB
            fdb-packages.foundationdb
            fdb-packages.fdb-manage
            fdb-packages.fdb-init
            fdb-packages.fdb-config-generator
            
            # Ethereum tools
            ethereum-packages.geth
            ethereum-packages.lighthouse
            ethereum-packages.forge
            
            # Cosmos tools
            cosmos-packages.neutron
            
            # Solana tools
            solana-packages.solana-tools
            
            # Simulation tools
            simulation-packages.sim-cluster
            simulation-packages.sim-config
            
            # Swagger UI tools
            swagger-ui-packages.swagger-ui
            swagger-ui-packages.swagger-static-server
            swagger-ui-packages.generate-openapi-spec
          ];
          
          shellHook = ''
            # Set macOS deployment target for Rust builds
            export MACOSX_DEPLOYMENT_TARGET="${darwinDeploymentTarget}"
            
            # Clear the DEVELOPER_DIR variable to fix macOS linking issues
            unset DEVELOPER_DIR
            unset DEVELOPER_DIR_aarch64_apple_darwin
            
            # FoundationDB environment setup
            export PROJECT_ROOT="$(pwd)"
            export FDB_CLUSTER_FILE="$PROJECT_ROOT/data/fdb.cluster"
            export FDB_NETWORK_OPTION_TLS_CERT_PATH=""
            export FDB_NETWORK_OPTION_TLS_KEY_PATH=""
            export FDB_NETWORK_OPTION_TLS_CA_PATH=""
            
            # Set FoundationDB compilation environment for Rust
            export BINDGEN_EXTRA_CLANG_ARGS="-I${fdb-packages.foundationdb}/include"
            export C_INCLUDE_PATH="${fdb-packages.foundationdb}/include:''${C_INCLUDE_PATH:-}"
            export CPLUS_INCLUDE_PATH="${fdb-packages.foundationdb}/include:''${CPLUS_INCLUDE_PATH:-}"
            export LIBRARY_PATH="${fdb-packages.foundationdb}/lib:''${LIBRARY_PATH:-}"
            export LD_LIBRARY_PATH="${fdb-packages.foundationdb}/lib:''${LD_LIBRARY_PATH:-}"
            export DYLD_LIBRARY_PATH="${fdb-packages.foundationdb}/lib:''${DYLD_LIBRARY_PATH:-}"
            
            # Fix PKG_CONFIG_PATH to include OpenSSL and FoundationDB
            export PKG_CONFIG_PATH="${fdb-packages.foundationdb}/lib/pkgconfig:${pkgs.openssl.dev}/lib/pkgconfig:''${PKG_CONFIG_PATH:-}"
            
            # Set OpenSSL environment variables for rust-openssl
            export OPENSSL_DIR="${pkgs.openssl.dev}"
            export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
            export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include"
            
            # Create local include directory for FoundationDB headers
            if command -v fdbcli >/dev/null 2>&1; then
              FDB_LOCAL_INCLUDE="$PWD/.fdb_include"
              if [ ! -d "$FDB_LOCAL_INCLUDE/foundationdb" ]; then
                echo "Setting up local FoundationDB headers..."
                mkdir -p "$FDB_LOCAL_INCLUDE/foundationdb"
                
                # Find and copy FDB headers from Nix store
                FDB_NIX_INCLUDE=$(find ${fdb-packages.foundationdb} -name "fdb_c.h" -type f 2>/dev/null | head -1 | xargs dirname 2>/dev/null || echo "")
                if [ -n "$FDB_NIX_INCLUDE" ] && [ -d "$FDB_NIX_INCLUDE" ]; then
                  cp -r "$FDB_NIX_INCLUDE"/* "$FDB_LOCAL_INCLUDE/foundationdb/" 2>/dev/null || true
                fi
                
                # Create stub fdb.options if missing
                if [ ! -f "$FDB_LOCAL_INCLUDE/foundationdb/fdb.options" ]; then
                  cat > "$FDB_LOCAL_INCLUDE/foundationdb/fdb.options" << 'EOFFDB'
# FoundationDB Options File (stub for compilation)
# Generated for Nix build environment
EOFFDB
                fi
              fi
              
              # Export paths for build
              export FDB_INCLUDE_DIR="$FDB_LOCAL_INCLUDE"
              export CPATH="$FDB_LOCAL_INCLUDE:''${CPATH:-}"
              export C_INCLUDE_PATH="$FDB_LOCAL_INCLUDE:''${C_INCLUDE_PATH:-}"
            fi
            
            # Set up blockchain client environment variables
            export ETH_RPC_URL="http://localhost:8545"
            export GETH_DATA_DIR="$PWD/data/ethereum/geth"
            export LIGHTHOUSE_DATA_DIR="$PWD/data/ethereum/lighthouse"
            export COSMOS_RPC_URL="http://localhost:26657"
            export SOLANA_RPC_URL="http://localhost:8899"
            
            echo "=== Almanac Cross-Chain Development Environment ==="
            echo "rust version: $(rustc --version)"
            echo ""
            echo "Available blockchain environments:"
            echo "  FoundationDB: fdb-manage start/stop/status"
            echo "  Ethereum: geth, lighthouse, forge"
            echo "  Cosmos: neutron"  
            echo "  Solana: solana, anchor"
            echo ""
            echo "API Documentation:"
            echo "  nix run .#swagger-ui    # Start Swagger UI server"
            echo "  nix run .#swagger-ui -- 8080  # Custom port"
            echo ""
            echo "Simulation tools:"
            echo "  sim-cluster init [chains]  # Initialize simulation"
            echo "  sim-cluster start          # Start all services"
            echo "  sim-cluster stop           # Stop all services"
            echo "  sim-cluster status         # Check service status"
            echo ""
            echo "Quick start:"
            echo "  fdb-manage start     # Start FoundationDB"
            echo "  geth --sepolia       # Start Ethereum testnet"
            echo "  neutron init mynode  # Initialize Cosmos node"
            echo "  solana-test-validator # Start Solana validator"
            echo "  nix run .#swagger-ui # Start API docs server"
            echo ""
          '';
          
          inherit (commonEnv) MACOSX_DEPLOYMENT_TARGET;
        };
        
      in {
        # Main packages - expose all subflake packages
        packages = {
          # FoundationDB packages
          foundationdb = fdb-packages.foundationdb;
          fdb-manage = fdb-packages.fdb-manage;
          fdb-init = fdb-packages.fdb-init;
          fdb-config-generator = fdb-packages.fdb-config-generator;
          run-e2e-tests = fdb-packages.run-e2e-tests;
          test-unit = fdb-packages.test-unit;
          test-multi-chain = fdb-packages.test-multi-chain;
          
          # Ethereum packages
          geth = ethereum-packages.geth;
          lighthouse = ethereum-packages.lighthouse;
          forge = ethereum-packages.forge;
          
          # Cosmos packages
          neutron = cosmos-packages.neutron;
          
          # Solana packages
          solana-tools = solana-packages.solana-tools;
          solana-node = solana-packages.solana-node;
          anchor = solana-packages.anchor;
          
          # Simulation packages
          sim-cluster = simulation-packages.sim-cluster;
          sim-config = simulation-packages.sim-config;
          sim-ethereum = simulation-packages.sim-ethereum;
          sim-cosmos = simulation-packages.sim-cosmos;
          sim-solana = simulation-packages.sim-solana;
          
          # Swagger UI packages from subflake
          swagger-ui = swagger-ui-packages.swagger-ui;
          swagger-static-server = swagger-ui-packages.swagger-static-server;
          generate-openapi-spec = swagger-ui-packages.generate-openapi-spec;
          
          # Default package
          default = fdb-packages.foundationdb;
        };
        
        # Development shells
        devShells = {
          default = combinedDevShell;
          
          # Individual blockchain development shells
          fdb = fdb.devShells.${system}.default;
          ethereum = ethereum.devShells.${system}.default;
          cosmos = cosmos.devShells.${system}.default;
          solana = solana.devShells.${system}.default;
          swagger-ui = swagger-ui.devShells.${system}.default;
        };
        
        # Apps for easy access
        apps = {
          fdb-manage = {
            type = "app";
            program = "${fdb-packages.fdb-manage}/bin/fdb-manage";
          };
          
          run-e2e-tests = {
            type = "app";
            program = "${fdb-packages.run-e2e-tests}/bin/run-e2e-tests";
          };
          
          test-unit = {
            type = "app";
            program = "${fdb-packages.test-unit}/bin/test-unit";
          };
          
          test-multi-chain = {
            type = "app";
            program = "${fdb-packages.test-multi-chain}/bin/test-multi-chain";
          };
          
          geth = {
            type = "app";
            program = "${ethereum-packages.geth}/bin/geth";
          };
          
          neutron = {
            type = "app";
            program = "${cosmos-packages.neutron}/bin/neutron";
          };
          
          solana = {
            type = "app";
            program = "${solana-packages.solana-node}/bin/solana";
          };
          
          sim-cluster = {
            type = "app";
            program = "${simulation-packages.sim-cluster}/bin/sim-cluster";
          };
          
          swagger-ui = {
            type = "app";
            program = "${swagger-ui-packages.swagger-ui}/bin/almanac-swagger-ui";
          };
        };
        
        formatter = pkgs.nixpkgs-fmt;
      }
    );
}