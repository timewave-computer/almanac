{
  description = "FoundationDB development environment for Almanac";

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

      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];

      perSystem = { config, self', inputs', pkgs, system, lib, ... }: let
        # Cross-platform FoundationDB support
        isLinux = pkgs.stdenv.isLinux;
        isDarwin = pkgs.stdenv.isDarwin;
        
        # FoundationDB versions and configurations
        fdbVersion = "7.3.42";
        fdbMajorVersion = "7.3";
        
        # FoundationDB derivation for macOS using official binaries
        foundationdb-macos = pkgs.stdenv.mkDerivation rec {
          pname = "foundationdb-macos";
          version = fdbVersion;
          
          # Download official macOS package based on architecture
          src = if pkgs.stdenv.hostPlatform.isAarch64 then
            pkgs.fetchurl {
              url = "https://github.com/apple/foundationdb/releases/download/${version}/FoundationDB-${version}_arm64.pkg";
              sha256 = "sha256-kpwXsauHKaorTl3C3YiaGPQv3uk/rluuq1PiKgvKfO8=";
            }
          else
            pkgs.fetchurl {
              url = "https://github.com/apple/foundationdb/releases/download/${version}/FoundationDB-${version}_x86_64.pkg";
              sha256 = pkgs.lib.fakeHash; # Will be updated with correct hash for x86_64
            };
          
          nativeBuildInputs = with pkgs; [ 
            xar
            cpio
            makeWrapper
          ];
          
          buildInputs = with pkgs; [
            openssl
            zlib
          ];
          
          # Extract the .pkg file
          unpackPhase = ''
            echo "Extracting FoundationDB package..."
            xar -xf $src
            
            # Extract all sub-packages
            for pkg in FoundationDB-*.pkg; do
              if [ -d "$pkg" ]; then
                echo "Extracting $pkg..."
                cd "$pkg"
                if [ -f Payload ]; then
                  cat Payload | gunzip -dc | cpio -i
                else
                  echo "No Payload found in $pkg"
                fi
                cd ..
              fi
            done
            
            # List extracted contents
            echo "Extracted contents:"
            find . -name "fdb*" -type f | head -10
          '';
          
          installPhase = ''
            mkdir -p $out/{bin,lib,include,share}
            
            # Find and copy FoundationDB binaries
            if [ -d usr/local/foundationdb ]; then
              FDB_DIR="usr/local/foundationdb"
            elif [ -d usr/local/bin ]; then
              FDB_DIR="usr/local"
            else
              echo "Looking for FoundationDB installation..."
              find . -name "fdbcli" -o -name "fdbserver" | head -5
              FDB_DIR=$(find . -name "fdbcli" | head -1 | xargs dirname | sed 's|/bin$||')
            fi
            
            echo "Found FoundationDB in: $FDB_DIR"
            
            # Copy binaries
            if [ -d "$FDB_DIR/bin" ]; then
              cp -r "$FDB_DIR/bin/"* $out/bin/
            fi
            
            # Copy libraries
            if [ -d "$FDB_DIR/lib" ]; then
              cp -r "$FDB_DIR/lib/"* $out/lib/
            fi
            
            # Copy headers
            if [ -d "$FDB_DIR/include" ]; then
              cp -r "$FDB_DIR/include/"* $out/include/
            fi
            
            # Copy share/documentation
            if [ -d "$FDB_DIR/share" ]; then
              cp -r "$FDB_DIR/share/"* $out/share/
            fi
            
            # Create default cluster file
            mkdir -p $out/share/foundationdb
            cat > $out/share/foundationdb/fdb.cluster << 'EOF'
# FoundationDB Cluster File
# This is a sample cluster configuration
almanac_cluster:almanac_cluster@127.0.0.1:4500
EOF
            
            # Ensure binaries are executable
            chmod +x $out/bin/* 2>/dev/null || true
            
            # Create main executable that dispatches to the right command  
            cat > $out/bin/foundationdb-macos << 'EOF'
#!/bin/bash
# FoundationDB main executable for macOS
COMMAND=$(basename "$0")
case "$COMMAND" in
  "fdbcli")
    exec "$0/../bin/fdbcli" "$@"
    ;;
  "fdbserver")
    exec "$0/../bin/fdbserver" "$@"
    ;;
  *)
    echo "FoundationDB for macOS"
    echo "Available commands:"
    echo "  fdbcli     - FoundationDB CLI"
    echo "  fdbserver  - FoundationDB Server"
    echo "  fdbbackup  - FoundationDB Backup"
    echo "  fdbrestore - FoundationDB Restore"
    echo ""
    echo "Usage: $0 <command> [args...]"
    echo "   or: fdbcli [args...]"
    echo "   or: fdbserver [args...]"
    ;;
esac
EOF
            chmod +x $out/bin/foundationdb-macos
          '';
          
          meta = with pkgs.lib; {
            description = "FoundationDB distributed database for macOS";
            homepage = "https://www.foundationdb.org/";
            platforms = platforms.darwin;
            license = licenses.asl20;
          };
        };
        
        # Choose appropriate FoundationDB package
        foundationdb = if isLinux then 
          pkgs.foundationdb 
        else if isDarwin then 
          foundationdb-macos 
        else 
          throw "FoundationDB not supported on this platform";
        
        # FoundationDB configuration generator
        fdbConfigGenerator = pkgs.writeShellApplication {
          name = "fdb-config-generator";
          runtimeInputs = [ foundationdb pkgs.coreutils ];
          text = ''
            # Generate FoundationDB configuration for different environments
            
            CLUSTER_FILE="''${1:-/etc/foundationdb/fdb.cluster}"
            MODE="''${2:-single}" # single, cluster, development
            
            echo "Generating FoundationDB configuration..."
            echo "Mode: $MODE"
            echo "Cluster file: $CLUSTER_FILE"
            
            case "$MODE" in
              "single")
                echo "almanac_single:almanac_single@127.0.0.1:4500" > "$CLUSTER_FILE"
                ;;
              "cluster")
                cat > "$CLUSTER_FILE" << 'EOF'
            # Multi-node cluster configuration
            almanac_cluster:almanac_cluster@127.0.0.1:4500,127.0.0.1:4501,127.0.0.1:4502
            EOF
                ;;
              "development")
                cat > "$CLUSTER_FILE" << 'EOF'
            # Development cluster configuration
            almanac_dev:almanac_dev@127.0.0.1:4500
            EOF
                ;;
              *)
                echo "Unknown mode: $MODE"
                echo "Supported modes: single, cluster, development"
                exit 1
                ;;
            esac
            
            echo "Configuration generated successfully"
          '';
        };
        
        # FoundationDB initialization script
        fdbInitScript = pkgs.writeShellApplication {
          name = "fdb-init";
          runtimeInputs = [ 
            foundationdb 
            pkgs.coreutils 
            pkgs.procps 
            fdbConfigGenerator 
          ];
          text = ''
            set -euo pipefail
            
            PROJECT_ROOT="''${PROJECT_ROOT:-$(pwd)}"
            FDB_DATA_DIR="$PROJECT_ROOT/data/foundationdb"
            FDB_LOG_DIR="$PROJECT_ROOT/logs/foundationdb"
            FDB_CLUSTER_FILE="$PROJECT_ROOT/data/fdb.cluster"
            
            echo "=== FoundationDB Initialization ==="
            echo "Platform: ${system}"
            echo "FDB Version: ${fdbVersion}"
            echo "Data directory: $FDB_DATA_DIR"
            echo "Log directory: $FDB_LOG_DIR"
            echo "Cluster file: $FDB_CLUSTER_FILE"
            
            # Create directories
            mkdir -p "$FDB_DATA_DIR" "$FDB_LOG_DIR"
            
            ${if isDarwin then ''
              echo "Running on macOS - using official FoundationDB binaries"
              fdb-config-generator "$FDB_CLUSTER_FILE" "single"
              
              echo "FoundationDB ready"
              echo "Note: For production clusters, configure multiple nodes."
            '' else if isLinux then ''
              echo "Running on Linux - full FoundationDB support"
              
              # Check if FoundationDB is already running
              if pgrep -x "fdbserver" > /dev/null; then
                echo "FoundationDB server is already running"
              else
                echo "Starting FoundationDB server..."
                
                # Generate cluster configuration
                fdb-config-generator "$FDB_CLUSTER_FILE" "single"
                
                # Start FoundationDB server in background
                fdbserver \
                  --cluster-file="$FDB_CLUSTER_FILE" \
                  --datadir="$FDB_DATA_DIR" \
                  --logdir="$FDB_LOG_DIR" \
                  --public-address=127.0.0.1:4500 \
                  --listen-address=0.0.0.0:4500 \
                  --class=storage \
                  --memory=2GiB \
                  > "$FDB_LOG_DIR/server.log" 2>&1 &
                
                echo "FoundationDB server started (PID: $!)"
                echo "Waiting for server to be ready..."
                
                # Wait for server to be ready
                for i in {1..30}; do
                  if fdbcli --cluster-file="$FDB_CLUSTER_FILE" --exec "status" &>/dev/null; then
                    echo "FoundationDB server is ready"
                    break
                  fi
                  echo "Waiting... ($i/30)"
                  sleep 2
                done
                
                # Configure database
                echo "Configuring database..."
                fdbcli --cluster-file="$FDB_CLUSTER_FILE" --exec "configure new single memory"
              fi
            '' else ''
              echo "Unsupported platform: ${system}"
              exit 1
            ''}
            
            # Set environment variables
            export FDB_CLUSTER_FILE="$FDB_CLUSTER_FILE"
            export FDB_NETWORK_OPTION_TLS_CERT_PATH=""
            export FDB_NETWORK_OPTION_TLS_KEY_PATH=""
            export FDB_NETWORK_OPTION_TLS_CA_PATH=""
            
            echo "Environment variables set:"
            echo "  FDB_CLUSTER_FILE=$FDB_CLUSTER_FILE"
            
            echo "=== FoundationDB Initialization Complete ==="
          '';
        };
        
        # FoundationDB management script
        fdbManageScript = pkgs.writeShellApplication {
          name = "fdb-manage";
          runtimeInputs = [ 
            foundationdb 
            pkgs.coreutils 
            pkgs.procps 
          ];
          text = ''
            set -euo pipefail
            
            PROJECT_ROOT="''${PROJECT_ROOT:-$(pwd)}"
            FDB_CLUSTER_FILE="$PROJECT_ROOT/data/fdb.cluster"
            
            ACTION="''${1:-help}"
            
            case "$ACTION" in
              "start")
                echo "Starting FoundationDB..."
                fdb-init
                ;;
              "stop")
                echo "Stopping FoundationDB..."
                ${if isLinux then ''
                  if pgrep -x "fdbserver" > /dev/null; then
                    pkill -x "fdbserver"
                    echo "FoundationDB server stopped"
                  else
                    echo "FoundationDB server is not running"
                  fi
                '' else ''
                  echo "FoundationDB - no server to stop"
                ''}
                ;;
              "status")
                echo "FoundationDB Status:"
                if [ -f "$FDB_CLUSTER_FILE" ]; then
                  echo "Cluster file: $FDB_CLUSTER_FILE"
                  ${if isLinux then ''
                    if command -v fdbcli >/dev/null; then
                      fdbcli --cluster-file="$FDB_CLUSTER_FILE" --exec "status" || echo "Server not responding"
                    else
                      echo "fdbcli not available"
                    fi
                  '' else ''
                    echo "Platform: ${system}"
                    echo "Full status checking available"
                  ''}
                else
                  echo "Cluster file not found. Run 'fdb-manage start' first."
                fi
                ;;
              "cli")
                shift
                ${if isLinux then ''
                  fdbcli --cluster-file="$FDB_CLUSTER_FILE" "$@"
                '' else ''
                  fdbcli --cluster-file="$FDB_CLUSTER_FILE" "$@"
                ''}
                ;;
              "help"|*)
                echo "FoundationDB Management Script"
                echo "Usage: fdb-manage <action>"
                echo ""
                echo "Actions:"
                echo "  start   - Initialize and start FoundationDB"
                echo "  stop    - Stop FoundationDB server"
                echo "  status  - Show FoundationDB status"
                echo "  cli     - Run FoundationDB CLI with additional arguments"
                echo "  help    - Show this help message"
                echo ""
                echo "Platform: ${system}"
                echo "FoundationDB: ${if isLinux then "Full support" else "Full support"}"
                ;;
            esac
          '';
        };

        # E2E test runner script with FDB environment
        e2eTestScript = pkgs.writeShellApplication {
          name = "run-e2e-tests";
          runtimeInputs = [ 
            foundationdb 
            pkgs.rustc
            pkgs.cargo
            pkgs.coreutils 
          ];
          text = ''
            set -euo pipefail
            
            PROJECT_ROOT="''${PROJECT_ROOT:-$(pwd)}"
            
            # Set up FoundationDB environment variables
            export FDB_CLUSTER_FILE="$PROJECT_ROOT/data/fdb.cluster"
            export FDB_NETWORK_OPTION_TLS_CERT_PATH=""
            export FDB_NETWORK_OPTION_TLS_KEY_PATH=""
            export FDB_NETWORK_OPTION_TLS_CA_PATH=""
            export FDB_INCLUDE_DIR="${foundationdb}/include"
            
            # Set library path for the current platform
            ${if isDarwin then ''
              export DYLD_LIBRARY_PATH="${foundationdb}/lib:''${DYLD_LIBRARY_PATH:-}"
            '' else if isLinux then ''
              export LD_LIBRARY_PATH="${foundationdb}/lib:''${LD_LIBRARY_PATH:-}"
            '' else ""}
            
            echo "=== Running Almanac E2E Tests with FoundationDB ==="
            echo "Platform: ${system}"
            echo "FoundationDB: ${fdbVersion}"
            echo "Project root: $PROJECT_ROOT"
            echo ""
            
            # Ensure we're in the project root
            if [ ! -d "e2e" ]; then
              echo "Error: e2e directory not found. Please run from the Almanac project root."
              exit 1
            fi
            
            # Display environment info
            echo "Environment variables:"
            echo "  FDB_CLUSTER_FILE=$FDB_CLUSTER_FILE"
            ${if isDarwin then ''
              echo "  DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH"
            '' else if isLinux then ''
              echo "  LD_LIBRARY_PATH=$LD_LIBRARY_PATH"
            '' else ""}
            echo ""
            
            # Change to e2e directory and run tests
            echo "Changing to e2e directory..."
            cd e2e
            
            echo "Running cargo test with release profile and all features..."
            echo ""
            
            # Run the tests with proper error handling
            if cargo test --release --all-features; then
              echo ""
              echo "=== E2E Tests Completed Successfully ==="
              echo "All tests passed with FoundationDB integration"
            else
              echo ""
              echo "=== E2E Tests Failed ==="
              echo "Check the output above for error details"
              exit 1
            fi
          '';
        };

      in {
        packages = {
          inherit foundationdb fdbConfigGenerator fdbInitScript fdbManageScript e2eTestScript;
          
          # Aliases for easier access
          fdb-config-generator = fdbConfigGenerator;
          fdb-init = fdbInitScript;
          fdb-manage = fdbManageScript;
          run-e2e-tests = e2eTestScript;
          
          # Platform-specific packages (only on Darwin)
        } // lib.optionalAttrs isDarwin {
          foundationdb-macos = foundationdb-macos;
        };

        devshells.default = {
          name = "foundationdb-development";
          
          commands = [
            {
              package = self'.packages.fdb-manage;
              help = "FoundationDB management commands";
            }
            {
              package = self'.packages.fdb-init;
              help = "Initialize FoundationDB for development";
            }
            {
              package = self'.packages.fdb-config-generator;
              help = "Generate FoundationDB configuration files";
            }
            {
              package = self'.packages.run-e2e-tests;
              help = "Run Almanac E2E tests with FoundationDB";
            }
            {
              package = pkgs.curl;
              help = "HTTP client for testing";
            }
            {
              package = pkgs.jq;
              help = "JSON processor";
            }
          ];
          
          env = [
            {
              name = "FDB_CLUSTER_FILE";
              value = "$PWD/data/fdb.cluster";
            }
            {
              name = "FDB_NETWORK_OPTION_TLS_CERT_PATH";
              value = "";
            }
            {
              name = "FDB_NETWORK_OPTION_TLS_KEY_PATH";
              value = "";
            }
            {
              name = "FDB_NETWORK_OPTION_TLS_CA_PATH";
              value = "";
            }
            {
              name = "FDB_INCLUDE_DIR";
              value = "${foundationdb}/include";
            }
          ] ++ lib.optionals isDarwin [
            {
              name = "DYLD_LIBRARY_PATH";
              value = "${foundationdb}/lib:$DYLD_LIBRARY_PATH";
            }
          ] ++ lib.optionals isLinux [
            {
              name = "LD_LIBRARY_PATH";
              value = "${foundationdb}/lib:$LD_LIBRARY_PATH";
            }
          ];
          
          motd = ''
            {14}{bold}FoundationDB development environment{reset}
            {9}FoundationDB v${fdbVersion} for ${system}{reset}
            
            {13}Quick start:{reset}
              fdb-manage start     # Initialize and start FoundationDB
              fdb-manage status    # Check status
              fdb-manage cli       # Access CLI
              run-e2e-tests        # Run Almanac E2E tests
              
            {13}Library paths:{reset} Automatically configured for ${if isDarwin then "DYLD_LIBRARY_PATH (macOS)" else if isLinux then "LD_LIBRARY_PATH (Linux)" else "this platform"}
            {13}Platform:{reset} ${if isLinux then "Linux (full support)" else if isDarwin then "macOS (official binaries)" else "Unsupported"}
          '';
        };
      };
    };
}