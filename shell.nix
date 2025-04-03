# Direct shell.nix without requiring flake-compat
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = with pkgs; [
    # Foundry tools
    foundry
    jq
    
    # Node.js for scripts
    nodejs
    
    # Rust development
    rustc
    cargo
    rustfmt
    clippy
    
    # Build dependencies
    rocksdb
    postgresql_15
  ];
  
  # Set environment variables directly
  MACOSX_DEPLOYMENT_TARGET = "11.0";
  SOURCE_DATE_EPOCH = "1672531200";
  
  shellHook = ''
    # Set environment variables again to ensure they're available in all shell contexts
    export MACOSX_DEPLOYMENT_TARGET="11.0"
    export SOURCE_DATE_EPOCH="1672531200"
    
    # Set other environment variables
    export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/indexer"
    export ROCKSDB_PATH="./data/rocksdb"
    export ETH_RPC_URL="http://localhost:8545"
    export RETH_DATA_DIR="./data/reth"
    export COSMOS_RPC_URL="http://localhost:26657"
    
    # Ensure directories exist
    mkdir -p ./data/rocksdb
    mkdir -p ./data/reth
    mkdir -p ./data/postgres
    mkdir -p ./logs
    
    echo "Environment ready with proper macOS variables set"
    echo "  MACOSX_DEPLOYMENT_TARGET=$MACOSX_DEPLOYMENT_TARGET"
    echo "  SOURCE_DATE_EPOCH=$SOURCE_DATE_EPOCH"
    echo ""
    echo "Run cargo commands directly, the environment is properly set"
  '';
} 