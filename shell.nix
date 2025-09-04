# Unified shell.nix for Almanac development
# Provides backward compatibility with non-flake nix-shell and system detection
# Works on both macOS (local development) and NixOS (remote development)

{ pkgs ? import <nixpkgs> {} }:

let
  # System detection
  isDarwin = pkgs.stdenv.isDarwin;
  isLinux = pkgs.stdenv.isLinux;
  lib = pkgs.lib;
  
  # Use nixos-25.05 for remote NixOS systems to get stable packages compatible with zero.nix
  remotePkgs = if isLinux then 
    import (fetchTarball "https://github.com/NixOS/nixpkgs/archive/nixos-25.05.tar.gz") {}
  else 
    pkgs;
    
  # Choose appropriate package set
  finalPkgs = if isLinux then remotePkgs else pkgs;
  
  # Remote development helper script (only on Linux/NixOS)
  remoteDevScript = finalPkgs.writeShellScriptBin "remote-dev" ''
    #!/usr/bin/env bash
    # Remote development script for Almanac on NixOS
    # Integrated into shell.nix for convenience
    
    set -e
    
    REMOTE_HOST="tw-nix"
    REMOTE_PATH="/root/almanac"
    
    case "''${1:-help}" in
        "sync")
            echo "Syncing project to remote server..."
            rsync -avz --progress \
                --exclude='.git' \
                --exclude='target' \
                --exclude='result*' \
                --exclude='*.log' \
                --exclude='data/' \
                --exclude='out/' \
                --exclude='cache/' \
                ./ ''${REMOTE_HOST}:''${REMOTE_PATH}/
            echo "Sync completed!"
            ;;
        
        "shell")
            echo "Opening remote development shell..."
            ssh -t ''${REMOTE_HOST} "cd ''${REMOTE_PATH} && nix-shell shell.nix"
            ;;
        
        "build")
            echo "Building project on remote server..."
            ssh ''${REMOTE_HOST} "cd ''${REMOTE_PATH} && nix-shell shell.nix --command 'cargo build'"
            ;;
        
        "api")
            echo "Running API server on remote..."
            ssh -t ''${REMOTE_HOST} "cd ''${REMOTE_PATH} && nix-shell shell.nix --command 'cargo run --bin almanac-api'"
            ;;
        
        "test")
            echo "Running tests on remote server..."
            ssh ''${REMOTE_HOST} "cd ''${REMOTE_PATH} && nix-shell shell.nix --command 'cargo test'"
            ;;
        
        "ssh")
            echo "SSH into remote server..."
            ssh -t ''${REMOTE_HOST} "cd ''${REMOTE_PATH} && bash"
            ;;
        
        "status")
            echo "Checking remote server status..."
            ssh ''${REMOTE_HOST} "cd ''${REMOTE_PATH} && pwd && ls -la && rustc --version 2>/dev/null || echo 'Rust not in PATH'"
            ;;
        
        "help"|*)
            echo "Almanac Remote Development Helper"
            echo ""
            echo "Usage: remote-dev <command>"
            echo ""
            echo "Commands:"
            echo "  sync    - Sync local project to remote server"
            echo "  shell   - Open interactive development shell on remote"
            echo "  build   - Build the project on remote server"
            echo "  api     - Run the API server on remote"
            echo "  test    - Run tests on remote server"
            echo "  ssh     - SSH into remote server"
            echo "  status  - Check remote server status"
            echo "  help    - Show this help message"
            echo ""
            echo "Examples:"
            echo "  remote-dev sync && remote-dev shell    # Sync and open shell"
            echo "  remote-dev build                       # Build project remotely"
            echo "  remote-dev api                         # Run API server"
            echo ""
            ;;
    esac
  '';

in finalPkgs.mkShell {
  name = if isLinux then "almanac-remote-development" else "almanac-local-development";
  
  # Development tools - platform-specific packages
  packages = with finalPkgs; [
    # Core Rust toolchain
    rustc
    cargo
    pkg-config 
    openssl
    curl
    jq
    git
    
    # Build tools (common)
    protobuf
    gcc
    clang
    unzip
    cacert
    
  ] ++ lib.optionals isDarwin [
    # macOS-specific packages
    rust-analyzer
    rustfmt
    clippy
    libiconv
    
    # Database tools (macOS)
    postgresql_15
    sqlx-cli
    
  ] ++ lib.optionals isLinux [
    # Linux/NixOS-specific packages
    foundationdb
    go-ethereum  # geth
    rsync
    openssh
    
    # Remote development helper
    remoteDevScript
  ];
  
  # Environment variables - platform-specific
  env = {
    # Common environment
    RUST_BACKTRACE = "1";
    RUST_LOG = "info";
    PROJECT_ROOT = "$PWD";
    
    # Platform-specific environment
  } // lib.optionalAttrs isDarwin {
    # macOS specific environment
    MACOSX_DEPLOYMENT_TARGET = "11.0";
    DATABASE_URL = "postgres://postgres:postgres@localhost:5432/almanac";
    
  } // lib.optionalAttrs isLinux {
    # Linux/NixOS specific environment
    FDB_CLUSTER_FILE = "$PWD/data/fdb.cluster";
    FDB_NETWORK_OPTION_TLS_CERT_PATH = "";
    FDB_NETWORK_OPTION_TLS_KEY_PATH = "";
    FDB_NETWORK_OPTION_TLS_CA_PATH = "";
    
    # Blockchain client environment variables
    ETH_RPC_URL = "http://localhost:8545";
    COSMOS_RPC_URL = "http://localhost:26657";
    SOLANA_RPC_URL = "http://localhost:8899";
    
    # OpenSSL environment variables for rust-openssl
    OPENSSL_DIR = "${finalPkgs.openssl.dev}";
    OPENSSL_LIB_DIR = "${finalPkgs.openssl.out}/lib";
    OPENSSL_INCLUDE_DIR = "${finalPkgs.openssl.dev}/include";
    
    # Protobuf environment variables
    PROTOC = "${finalPkgs.protobuf}/bin/protoc";
    PROTOBUF_INCLUDE = "${finalPkgs.protobuf}/include";
    
    # C/C++ compiler environment
    CC = "${finalPkgs.clang}/bin/clang";
    CXX = "${finalPkgs.clang}/bin/clang++";
    CFLAGS = "-I${finalPkgs.protobuf}/include";
    CXXFLAGS = "-I${finalPkgs.protobuf}/include -std=c++14";
    LDFLAGS = "-L${finalPkgs.protobuf}/lib";
    
    # Certificate bundle
    SSL_CERT_FILE = "${finalPkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
    CURL_CA_BUNDLE = "${finalPkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
  };
  
  # Shell hook - platform-specific welcome messages and setup
  shellHook = if isDarwin then ''
    echo "=== Almanac Development Shell (macOS) ==="
    echo "rust version: $(rustc --version)"
    echo "cargo version: $(cargo --version)"
    echo "MACOSX_DEPLOYMENT_TARGET: $MACOSX_DEPLOYMENT_TARGET"
    echo ""
    echo "Available tools:"
    echo "  cargo build/test/run"
    echo "  rust-analyzer, rustfmt, clippy"
    echo "  postgresql, sqlx-cli"
    echo ""
    echo "Environment:"
    echo "  DATABASE_URL: $DATABASE_URL"
    echo "  PROJECT_ROOT: $PROJECT_ROOT"
    echo ""
    echo "To use the full flake-based development environment, run:"
    echo "  nix develop"
    echo ""
  '' else ''
    echo "=== Almanac Development Shell (NixOS/Remote) ==="
    echo "rust version: $(rustc --version)"
    echo ""
    echo "Available tools:"
    echo "  cargo build/test/run"
    echo "  geth (Ethereum client)"
    echo "  fdbcli (FoundationDB CLI)"
    echo "  remote-dev <command>  # Remote development helper"
    echo ""
    echo "Environment setup:"
    echo "  PROJECT_ROOT=$PROJECT_ROOT"
    echo "  OPENSSL_DIR=$OPENSSL_DIR"
    echo "  PROTOC=$PROTOC"
    echo "  FDB_CLUSTER_FILE=$FDB_CLUSTER_FILE"
    echo ""
    echo "Quick start:"
    echo "  cargo build                    # Build the project"
    echo "  cargo run --bin almanac-api    # Run the API server"
    echo "  fdbcli                         # FoundationDB CLI"
    echo "  remote-dev help                # Show remote development commands"
    echo ""
  '';
}