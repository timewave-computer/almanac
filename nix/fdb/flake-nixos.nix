{
  description = "FoundationDB development environment for NixOS";

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

      perSystem = { config, self', inputs', pkgs, system, lib, ... }: let
        # FoundationDB version (from nixos-22.11 with compatible glibc)
        fdbVersion = "from nixos-22.11";
        
        # FoundationDB wrapped with glibc 2.38 from nixos-23.11
        foundationdb = let
          # Use nixos-23.11 which has glibc 2.38 (compatible with FoundationDB)
          oldPkgs = import (builtins.fetchTarball {
            url = "https://github.com/NixOS/nixpkgs/archive/nixos-23.11.tar.gz";
            sha256 = "1f5d2g1p6nfwycpmrnnmc2xmcszp804adp16knjvdkj8nz36y1fg";
          }) { system = pkgs.system; };
        in
        # Wrap the pre-built FoundationDB from nixos-23.11 with proper environment
        pkgs.symlinkJoin {
          name = "foundationdb-wrapped";
          paths = [ oldPkgs.foundationdb ];
          buildInputs = [ pkgs.makeWrapper ];
          postBuild = ''
            # Create wrapper scripts for each FDB binary
            for binary in $out/bin/*; do
              if [ -f "$binary" ] && [ -x "$binary" ]; then
                mv "$binary" "$binary-unwrapped"
                makeWrapper "$binary-unwrapped" "$binary" \
                  --set LC_ALL "C.UTF-8" \
                  --set LANG "C.UTF-8" \
                  --set LOCALE_ARCHIVE "${oldPkgs.glibcLocales}/lib/locale/locale-archive" \
                  --prefix LD_LIBRARY_PATH : "${oldPkgs.glibc}/lib:${oldPkgs.stdenv.cc.cc.lib}/lib"
              fi
            done
            
            echo "FoundationDB wrapped with glibc ${oldPkgs.glibc.version} from nixos-23.11"
          '';
          meta = with lib; {
            description = "FoundationDB pre-built from nixos-23.11 with glibc 2.38";
            homepage = "https://www.foundationdb.org";
            license = licenses.asl20;
            platforms = [ "x86_64-linux" "aarch64-linux" ];
          };
        };

        # FoundationDB management script for NixOS
        fdbManageScript = pkgs.writeShellApplication {
          name = "fdb-manage";
          runtimeInputs = [ foundationdb pkgs.coreutils pkgs.procps ];
          text = ''
            PROJECT_ROOT="''${PROJECT_ROOT:-$(pwd)}"
            FDB_CLUSTER_FILE="$PROJECT_ROOT/data/fdb.cluster"
            ACTION="''${1:-help}"
            
            case "$ACTION" in
              "start")
                echo "Starting FoundationDB (NixOS)..."
                mkdir -p "$PROJECT_ROOT/data" "$PROJECT_ROOT/logs"
                echo "almanac_single:almanac_single@127.0.0.1:4500" > "$FDB_CLUSTER_FILE"
                
                if pgrep -x "fdbserver" > /dev/null; then
                  echo "FoundationDB server already running"
                else
                  echo "Starting FoundationDB server..."
                  fdbserver \
                    --cluster-file="$FDB_CLUSTER_FILE" \
                    --datadir="$PROJECT_ROOT/data" \
                    --logdir="$PROJECT_ROOT/logs" \
                    --public-address=127.0.0.1:4500 \
                    --listen-address=0.0.0.0:4500 \
                    > "$PROJECT_ROOT/logs/fdb.log" 2>&1 &
                  echo "FoundationDB server started (PID: $!)"
                fi
                ;;
              "stop")
                echo "Stopping FoundationDB..."
                if pgrep -x "fdbserver" > /dev/null; then
                  pkill -x "fdbserver"
                  echo "FoundationDB server stopped"
                else
                  echo "FoundationDB server not running"
                fi
                ;;
              "status")
                echo "FoundationDB Status (NixOS):"
                if [ -f "$FDB_CLUSTER_FILE" ]; then
                  echo "Cluster file: $FDB_CLUSTER_FILE"
                  fdbcli --cluster-file="$FDB_CLUSTER_FILE" --exec "status" || echo "Server not responding"
                else
                  echo "Cluster file not found. Run 'fdb-manage start' first."
                fi
                ;;
              "cli")
                shift
                [ -f "$FDB_CLUSTER_FILE" ] && fdbcli --cluster-file="$FDB_CLUSTER_FILE" "$@"
                ;;
              *)
                echo "Usage: fdb-manage {start|stop|status|cli|help}"
                echo "Note: Using FoundationDB built with isolated glibc 2.38 for compatibility"
                ;;
            esac
          '';
        };

        # E2E test runner for NixOS
        e2eTestScript = pkgs.writeShellApplication {
          name = "run-e2e-tests";
          runtimeInputs = [ 
            foundationdb pkgs.rustc pkgs.cargo pkgs.gcc pkgs.binutils
            pkgs.coreutils pkgs.openssl pkgs.pkg-config pkgs.protobuf
          ];
          text = ''
            PROJECT_ROOT="''${PROJECT_ROOT:-$(pwd)}"
            
            # Set up environment variables
            export FDB_CLUSTER_FILE="$PROJECT_ROOT/data/fdb.cluster"
            export FDB_INCLUDE_DIR="${foundationdb}/include"
            export OPENSSL_DIR="${pkgs.openssl.dev}"
            export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
            export PKG_CONFIG_PATH="${pkgs.openssl.dev}/lib/pkgconfig"
            export LD_LIBRARY_PATH="${foundationdb}/lib:${pkgs.openssl.out}/lib:''${LD_LIBRARY_PATH:-}"
            
            echo "=== Running Almanac E2E Tests with FoundationDB (NixOS) ==="
            echo "Platform: ${system}"
            echo "FoundationDB: ${fdbVersion}"
            
            TEST_FILTER="''${1:-}"
            if [ -n "$TEST_FILTER" ]; then
              CARGO_ARGS="--filter=$TEST_FILTER"
            else
              CARGO_ARGS=""
            fi
            
            if timeout 600 cargo run --release --package almanac-tests --bin test-runner --all-features -- "$CARGO_ARGS"; then
              echo "=== E2E Tests Completed Successfully ==="
            else
              echo "=== E2E Tests Failed ==="
              exit $?
            fi
          '';
        };

      in {
        packages = {
          inherit foundationdb;
          fdb-manage = fdbManageScript;
          run-e2e-tests = e2eTestScript;
          test-unit = pkgs.writeShellApplication {
            name = "test-unit";
            text = ''exec ${e2eTestScript}/bin/run-e2e-tests "unit"'';
          };
        };

        devshells.default = {
          name = "foundationdb-development-nixos";
          
          commands = [
            { package = foundationdb; help = "FoundationDB binaries (patched)"; }
            { package = self'.packages.fdb-manage; help = "FoundationDB management"; }
            { package = self'.packages.run-e2e-tests; help = "Run E2E tests"; }
            { package = self'.packages.test-unit; help = "Run unit tests"; }
            { package = pkgs.curl; help = "HTTP client"; }
            { package = pkgs.jq; help = "JSON processor"; }
            { package = pkgs.gcc; help = "C/C++ compiler"; }
          ];
          
          env = [
            { name = "FDB_CLUSTER_FILE"; value = "$PWD/data/fdb.cluster"; }
            { name = "FDB_INCLUDE_DIR"; value = "${foundationdb}/include"; }
            { name = "LD_LIBRARY_PATH"; value = "${foundationdb}/lib:${pkgs.openssl.out}/lib:$\{LD_LIBRARY_PATH:-}"; }
          ];
          
          motd = ''
            {14}{bold}FoundationDB development environment (NixOS){reset}
            {9}FoundationDB v${fdbVersion}{reset}
            
            {13}Quick start:{reset}
              fdb-manage start       # Start FoundationDB
              fdb-manage status      # Check status
              run-e2e-tests          # Run tests
              
            {13}Note:{reset} Using FoundationDB built with isolated glibc 2.38 for compatibility
          '';
        };
      };
    };
}