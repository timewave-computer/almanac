{
  description = "FoundationDB development environment";

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

      systems = [ "aarch64-darwin" "x86_64-darwin" "x86_64-linux" "aarch64-linux" ];

      perSystem = { config, self', inputs', pkgs, system, lib, ... }: let
        # Platform detection
        isDarwin = pkgs.stdenv.isDarwin;
        isLinux = pkgs.stdenv.isLinux;
        
        # FoundationDB version
        fdbVersion = "7.3.42";
        
        # Platform-specific FoundationDB package
        foundationdb = if isDarwin then
          # macOS: Use official Apple binaries
          pkgs.stdenv.mkDerivation rec {
            pname = "foundationdb-macos";
            version = fdbVersion;
            
            src = if pkgs.stdenv.hostPlatform.isAarch64 then
              pkgs.fetchurl {
                url = "https://github.com/apple/foundationdb/releases/download/${version}/FoundationDB-${version}_arm64.pkg";
                sha256 = "sha256-kpwXsauHKaorTl3C3YiaGPQv3uk/rluuq1PiKgvKfO8=";
              }
            else
              pkgs.fetchurl {
                url = "https://github.com/apple/foundationdb/releases/download/${version}/FoundationDB-${version}_x86_64.pkg";
                sha256 = pkgs.lib.fakeHash;
              };
            
            nativeBuildInputs = with pkgs; [ xar cpio ];
            
            unpackPhase = ''
              xar -xf $src
              for pkg in FoundationDB-*.pkg; do
                [ -d "$pkg" ] && cd "$pkg" && [ -f Payload ] && cat Payload | gunzip -dc | cpio -i && cd ..
              done
            '';
            
            installPhase = ''
              mkdir -p $out/{bin,lib,include}
              FDB_DIR=$(find . -name "fdbcli" | head -1 | xargs dirname | sed 's|/bin$||') || FDB_DIR="usr/local/foundationdb"
              [ -d "$FDB_DIR/bin" ] && cp -r "$FDB_DIR/bin/"* $out/bin/
              [ -d "$FDB_DIR/lib" ] && cp -r "$FDB_DIR/lib/"* $out/lib/
              [ -d "$FDB_DIR/include" ] && cp -r "$FDB_DIR/include/"* $out/include/
              chmod +x $out/bin/* 2>/dev/null || true
            '';
          }
        else if isLinux then
          # NixOS: Use nixpkgs FoundationDB (runtime wrapper is in flake-nixos.nix)
          pkgs.foundationdb
        else
          throw "FoundationDB not supported on this platform";

        # FoundationDB management script
        fdbManageScript = pkgs.writeShellApplication {
          name = "fdb-manage";
          runtimeInputs = [ foundationdb pkgs.coreutils ] ++ lib.optionals isLinux [ pkgs.procps ];
          text = ''
            PROJECT_ROOT="''${PROJECT_ROOT:-$(pwd)}"
            FDB_CLUSTER_FILE="$PROJECT_ROOT/data/fdb.cluster"
            ACTION="''${1:-help}"
            
            case "$ACTION" in
              "start")
                echo "Starting FoundationDB..."
                mkdir -p "$PROJECT_ROOT/data" "$PROJECT_ROOT/logs"
                echo "almanac_single:almanac_single@127.0.0.1:4500" > "$FDB_CLUSTER_FILE"
                ${if isLinux then ''
                  if ! pgrep -x "fdbserver" > /dev/null; then
                    fdbserver --cluster-file="$FDB_CLUSTER_FILE" --datadir="$PROJECT_ROOT/data" > "$PROJECT_ROOT/logs/fdb.log" 2>&1 &
                  fi
                '' else ""}
                echo "FoundationDB ready for development"
                ;;
              "stop")
                echo "Stopping FoundationDB..."
                ${if isLinux then ''pkill -x "fdbserver" 2>/dev/null || true'' else ''echo "No-op on macOS"''}
                ;;
              "status")
                echo "FoundationDB Status (${system}):"
                [ -f "$FDB_CLUSTER_FILE" ] && echo "Cluster file: $FDB_CLUSTER_FILE"
                fdbcli --cluster-file="$FDB_CLUSTER_FILE" --exec "status" 2>/dev/null || echo "Server not responding"
                ;;
              "cli")
                shift
                [ -f "$FDB_CLUSTER_FILE" ] && fdbcli --cluster-file="$FDB_CLUSTER_FILE" "$@"
                ;;
              *)
                echo "Usage: fdb-manage {start|stop|status|cli}"
                ;;
            esac
          '';
        };

      in {
        packages = {
          inherit foundationdb;
          fdb-manage = fdbManageScript;
        } // lib.optionalAttrs isLinux {
          run-e2e-tests = pkgs.writeShellApplication {
            name = "run-e2e-tests";
            runtimeInputs = [ foundationdb pkgs.rustc pkgs.cargo pkgs.gcc pkgs.openssl pkgs.pkg-config ];
            text = ''
              export FDB_CLUSTER_FILE="''${PROJECT_ROOT:-$(pwd)}/data/fdb.cluster"
              export FDB_INCLUDE_DIR="${foundationdb}/include"
              export LD_LIBRARY_PATH="${foundationdb}/lib:${pkgs.openssl.out}/lib:''${LD_LIBRARY_PATH:-}"
              
              echo "Running E2E tests with FoundationDB mock..."
              cargo run --release --package almanac-tests --bin test-runner --all-features -- "''${1:-}"
            '';
          };
        };

        devshells.default = {
          name = "foundationdb-development";
          
          commands = [
            { package = foundationdb; help = "FoundationDB binaries"; }
            { package = self'.packages.fdb-manage; help = "FoundationDB management"; }
          ] ++ lib.optionals isLinux [
            { package = self'.packages.run-e2e-tests; help = "Run E2E tests"; }
          ] ++ [
            { package = pkgs.curl; help = "HTTP client"; }
            { package = pkgs.jq; help = "JSON processor"; }
          ];
          
          env = [
            { name = "FDB_CLUSTER_FILE"; value = "$PWD/data/fdb.cluster"; }
          ] ++ lib.optionals isDarwin [
            { name = "DYLD_LIBRARY_PATH"; value = "${foundationdb}/lib:$\{DYLD_LIBRARY_PATH:-}"; }
          ] ++ lib.optionals isLinux [
            { name = "LD_LIBRARY_PATH"; value = "${foundationdb}/lib:$\{LD_LIBRARY_PATH:-}"; }
          ];
          
          motd = ''
            {14}{bold}FoundationDB development environment{reset}
            {9}FoundationDB v${fdbVersion} for ${system}{reset}
            
            {13}Quick start:{reset}
              fdb-manage start       # Initialize FoundationDB
              fdb-manage status      # Check status
              ${lib.optionalString isLinux "run-e2e-tests          # Run tests"}
              
            {13}Platform:{reset} ${if isDarwin then "macOS (official binaries)" else "NixOS (patched build)"}
          '';
        };
      };
    };
}