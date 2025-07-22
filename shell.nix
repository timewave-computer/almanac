# Shell.nix for backward compatibility with non-flake nix-shell
# This provides a development shell without requiring flakes to be enabled
{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  # Development tools
  packages = with pkgs; [
    # Rust toolchain
    rustc
    cargo
    rust-analyzer
    rustfmt
    clippy
    
    # Build dependencies
    pkg-config
    openssl
    
    # Database tools
    postgresql_15
    sqlx-cli
    
    # Development utilities
    git
    curl
    jq
    
    # Platform-specific dependencies
  ] ++ lib.optionals stdenv.isDarwin [
    # macOS specific
    libiconv
    darwin.apple_sdk.frameworks.Security
    darwin.apple_sdk.frameworks.SystemConfiguration
  ] ++ lib.optionals stdenv.isLinux [
    # Linux specific - including FoundationDB which only works on Linux
    glibc
    foundationdb73
  ];
  
  # Environment variables
  env = {
    # Database configuration
    DATABASE_URL = "postgres://postgres:postgres@localhost:5432/almanac";
    
    # Rust configuration
    RUST_BACKTRACE = "1";
    RUST_LOG = "info";
    
    # FoundationDB configuration
    FDB_CLUSTER_FILE = "$PWD/data/fdb.cluster";
  } // pkgs.lib.optionalAttrs pkgs.stdenv.isDarwin {
    # macOS specific environment
    MACOSX_DEPLOYMENT_TARGET = "11.0";
  };
  
  # Shell hook
  shellHook = ''
    echo "Almanac development shell (non-flake)"
    echo "-----------------------------------"
    echo "Rust version: $(rustc --version)"
    echo "Cargo version: $(cargo --version)"
    ${pkgs.lib.optionalString pkgs.stdenv.isDarwin ''
      echo "MACOSX_DEPLOYMENT_TARGET: $MACOSX_DEPLOYMENT_TARGET"
    ''}
    echo ""
    echo "To use the full flake-based development environment, run:"
    echo "  nix develop"
    echo ""
  '';
}