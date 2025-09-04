{
  description = "Solana development environment for NixOS - Properly vendored build with Geyser plugin";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
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

      perSystem = { config, self', inputs', pkgs, system, lib, ... }: 
      let
        # Use rust-overlay for specific Rust version
        rustToolchain = inputs'.rust-overlay.packages.rust_1_76_0.override {
          extensions = [ "rust-src" "rustfmt" "clippy" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # Build Solana from source with vendored dependencies and socket patch
        solana-source = pkgs.rustPlatform.buildRustPackage rec {
          pname = "solana-cli";
          version = "1.18.26";
          
          src = pkgs.fetchFromGitHub {
            owner = "solana-labs";
            repo = "solana";
            rev = "v${version}";
            sha256 = "sha256-sJ0Zn5GMi64/S8zqomL/dYRVW8SOQWsP+bpcdatJC0A=";
            fetchSubmodules = true;
          };

          # Copy the Cargo.lock file
          # Apply socket patch for Digital Ocean compatibility
          patches = [
            (pkgs.writeText "socket-patch.patch" ''
              diff --git a/streamer/src/nonblocking/quic.rs b/streamer/src/nonblocking/quic.rs
              index abc123..def456 100644
              --- a/streamer/src/nonblocking/quic.rs
              +++ b/streamer/src/nonblocking/quic.rs
              @@ -172,8 +172,16 @@ pub fn receiver(
                   socket: Arc<UdpSocket>,
                   stats: Arc<StatsCollector>,
               ) -> JoinHandle<()> {
              -    let res = socket.set_read_timeout(Some(Duration::new(1, 0)));
              -    assert!(res.is_ok(), "streamer::receiver set_read_timeout error");
              +    match socket.set_read_timeout(Some(Duration::new(1, 0))) {
              +        Ok(_) => debug!("Socket read timeout set to 1 second"),
              +        Err(e) => {
              +            warn!("Failed to set socket read timeout: {:?}", e);
              +            warn!("This is common in KVM/virtualized environments");
              +            warn!("Attempting to continue with non-blocking mode...");
              +            let _ = socket.set_nonblocking(true);
              +        }
              +    }
              +
                   tokio::spawn(async move {
                       let mut packets = vec![Packet::default(); packets_per_batch];
                       loop {
            '')
          ];

          cargoLock = {
            lockFile = "${src}/Cargo.lock";
            outputHashes = {
              # All git dependencies with their hashes
              "aes-gcm-siv-0.10.3" = "0vwd5dh342aa8f79crwp1j7ghi9zfdy0pb054ms1xq5hyz36jnip";
              "crossbeam-epoch-0.9.5" = "02w3c4kq8mkqh7n4vlk6kzs3jd72x55wnpg7czi2a9i0pdm13z95";
              "curve25519-dalek-3.2.1" = "0d1nrzr74fqr79v4y8q2lga5zim1w5yv4gm85bm7bj42w4b4vr8n";
              "tokio-1.29.1" = "0scclfq2qcv0vrixm48x014lcvi8hlk720fsbm9zb45aq301xyb7";
              "tokio-macros-2.1.0" = "0scclfq2qcv0vrixm48x014lcvi8hlk720fsbm9zb45aq301xyb7";
            };
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            protobuf
            rustPlatform.bindgenHook
            perl
            python3
            cmake
            rustToolchain
          ];
          
          buildInputs = with pkgs; [
            openssl
            zlib
            systemd
            libudev-zero
            libclang.lib
            hidapi
            rocksdb
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];
          
          # Conservative build flags for Digital Ocean virtualized CPU compatibility
          RUSTFLAGS = "-C target-cpu=x86-64 -C opt-level=2";
          
          # Build configuration
          buildType = "release";
          
          # Skip tests to speed up build
          doCheck = false;
          checkType = "debug";
          
          # Set up build environment
          preBuild = ''
            export HOME=$TMPDIR
            export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
            export PROTOC="${pkgs.protobuf}/bin/protoc"
            export PROTOC_INCLUDE="${pkgs.protobuf}/include"
            export OPENSSL_DIR="${pkgs.openssl.dev}"
            export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
            export OPENSSL_INCLUDE_DIR="${pkgs.openssl.dev}/include"
          '';
          
          # Build all necessary binaries including validator
          cargoBuildFlags = [
            "--package" "solana-cli"
            "--package" "solana-keygen"  
            "--package" "solana-test-validator"
            "--package" "solana-validator"
            "--package" "solana-faucet"
          ];
          
          # Don't install everything, just what we built
          postInstall = ''
            # Remove any duplicate binaries
            rm -f $out/bin/*.so
            
            # Only keep the binaries we need
            for bin in $out/bin/*; do
              case "$(basename "$bin")" in
                solana|solana-keygen|solana-test-validator|solana-validator|solana-faucet|solana-genesis|solana-ledger-tool)
                  echo "Keeping $bin"
                  ;;
                *)
                  echo "Removing $bin"
                  rm -f "$bin"
                  ;;
              esac
            done
          '';
          
          meta = with lib; {
            description = "Solana CLI built with conservative CPU flags";
            homepage = "https://solana.com";
            license = licenses.asl20;
            platforms = [ "x86_64-linux" "aarch64-linux" ];
          };
        };

        # Almanac Geyser plugin
        almanac-geyser-plugin = pkgs.rustPlatform.buildRustPackage rec {
          pname = "almanac-geyser-plugin";
          version = "0.1.0";
          
          src = ./geyser-plugin;
          
          cargoLock = {
            lockFile = ./geyser-plugin/Cargo.lock;
          };
          
          nativeBuildInputs = with pkgs; [
            pkg-config
            protobuf
          ];
          
          buildInputs = with pkgs; [
            openssl
            postgresql
          ];
          
          # Build as cdylib for plugin
          buildType = "release";
          
          postInstall = ''
            mkdir -p $out/lib
            cp target/release/libalmanac_geyser_plugin.so $out/lib/
            
            # Create default config
            mkdir -p $out/etc
            cat > $out/etc/almanac-geyser-config.json <<EOF
            {
              "libpath": "$out/lib/libalmanac_geyser_plugin.so",
              "database_url": "postgresql://almanac:almanac@localhost/almanac",
              "accounts_selector": {
                "owners": ["11111111111111111111111111111111"],
                "accounts": []
              },
              "transaction_selector": {
                "mentions": []
              },
              "instruction_selector": {
                "programs": []
              }
            }
            EOF
          '';
          
          meta = with lib; {
            description = "Geyser plugin for Almanac database integration";
            license = licenses.mit;
            platforms = platforms.linux;
          };
        };

        # Test validator wrapper
        solana-test-validator-safe = pkgs.writeShellApplication {
          name = "solana-test-validator-safe";
          runtimeInputs = [ solana-source ];
          text = ''
            echo "Starting Solana test validator (built from source)..."
            echo "Version: $(solana --version)"
            
            # Setup directories
            LEDGER_DIR="''${LEDGER_DIR:-$PWD/test-ledger}"
            mkdir -p "$LEDGER_DIR"
            
            # Configure if needed
            if [ ! -f "$HOME/.config/solana/id.json" ]; then
              echo "Creating keypair..."
              solana-keygen new --no-passphrase --force --silent
            fi
            
            solana config set --url http://localhost:8899 >/dev/null 2>&1
            
            # Run validator with conservative settings
            exec solana-test-validator \
              --ledger "$LEDGER_DIR" \
              --rpc-port 8899 \
              --faucet-port 9900 \
              --limit-ledger-size 50000000 \
              --slots-per-epoch 50 \
              --quiet \
              --reset \
              "$@"
          '';
        };

        # Test validator wrapper with Geyser support
        solana-test-validator-geyser = pkgs.writeShellApplication {
          name = "solana-test-validator-geyser";
          runtimeInputs = [ solana-source almanac-geyser-plugin pkgs.postgresql ];
          text = ''
            echo "Starting Solana test validator with Almanac Geyser plugin..."
            echo "Version: $(solana --version)"
            
            # Setup directories
            LEDGER_DIR="''${LEDGER_DIR:-$PWD/test-ledger}"
            CONFIG_DIR="''${CONFIG_DIR:-$PWD/config}"
            mkdir -p "$LEDGER_DIR" "$CONFIG_DIR"
            
            # Check if PostgreSQL is running
            if ! pg_isready -q 2>/dev/null; then
              echo "Warning: PostgreSQL doesn't appear to be running"
              echo "The Geyser plugin will fail to connect to the database"
              echo "Start PostgreSQL and create the almanac database first"
              echo ""
              echo "To set up PostgreSQL:"
              echo "  sudo systemctl start postgresql"
              echo "  sudo -u postgres createdb almanac"
              echo "  sudo -u postgres createuser almanac"
              echo ""
            fi
            
            # Configure if needed
            if [ ! -f "$HOME/.config/solana/id.json" ]; then
              echo "Creating keypair..."
              solana-keygen new --no-passphrase --force --silent
            fi
            
            solana config set --url http://localhost:8899 >/dev/null 2>&1
            
            # Create Geyser config
            GEYSER_CONFIG="$CONFIG_DIR/geyser-config.json"
            if [ ! -f "$GEYSER_CONFIG" ]; then
              echo "Creating Geyser config at $GEYSER_CONFIG"
              cat > "$GEYSER_CONFIG" <<EOF
            {
              "libpath": "${almanac-geyser-plugin}/lib/libalmanac_geyser_plugin.so",
              "database_url": "''${DATABASE_URL:-postgresql://almanac:almanac@localhost/almanac}",
              "accounts_selector": {
                "owners": ["11111111111111111111111111111111"],
                "accounts": []
              },
              "transaction_selector": {
                "mentions": []
              },
              "instruction_selector": {
                "programs": []
              }
            }
            EOF
            fi
            
            # Run validator with Geyser plugin
            exec solana-test-validator \
              --ledger "$LEDGER_DIR" \
              --rpc-port 8899 \
              --faucet-port 9900 \
              --limit-ledger-size 50000000 \
              --slots-per-epoch 50 \
              --geyser-plugin-config "$GEYSER_CONFIG" \
              --quiet \
              --reset \
              "$@"
          '';
        };

        # Full validator wrapper with Geyser
        solana-validator-geyser = pkgs.writeShellApplication {
          name = "solana-validator-geyser";
          runtimeInputs = [ solana-source almanac-geyser-plugin ];
          text = ''
            echo "Starting Solana validator with Almanac Geyser plugin..."
            
            # Set up default directories
            LEDGER_DIR="''${LEDGER_DIR:-$PWD/ledger}"
            CONFIG_DIR="''${CONFIG_DIR:-$PWD/config}"
            
            mkdir -p "$LEDGER_DIR" "$CONFIG_DIR"
            
            # Use custom Geyser config if provided, otherwise use default
            GEYSER_CONFIG="''${GEYSER_CONFIG:-$CONFIG_DIR/geyser-config.json}"
            
            if [ ! -f "$GEYSER_CONFIG" ]; then
              echo "Creating Geyser config at $GEYSER_CONFIG"
              cp ${almanac-geyser-plugin}/etc/almanac-geyser-config.json "$GEYSER_CONFIG"
              echo "Please edit $GEYSER_CONFIG with your database credentials"
            fi
            
            echo "Using Geyser config: $GEYSER_CONFIG"
            
            # Start validator with Geyser plugin
            exec solana-validator \
              --ledger "$LEDGER_DIR" \
              --geyser-plugin-config "$GEYSER_CONFIG" \
              "$@"
          '';
        };

        # PostgreSQL setup helper
        setup-almanac-db = pkgs.writeShellApplication {
          name = "setup-almanac-db";
          runtimeInputs = [ pkgs.postgresql ];
          text = ''
            echo "Setting up Almanac database..."
            
            if ! pg_isready -q 2>/dev/null; then
              echo "PostgreSQL is not running. Please start it first:"
              echo "  sudo systemctl start postgresql"
              exit 1
            fi
            
            # Create database and user
            sudo -u postgres psql <<EOF || true
            CREATE DATABASE almanac;
            CREATE USER almanac WITH PASSWORD 'almanac';
            GRANT ALL PRIVILEGES ON DATABASE almanac TO almanac;
            \q
            EOF
            
            echo "Database setup complete!"
            echo "Connection string: postgresql://almanac:almanac@localhost/almanac"
          '';
        };
        
      in {
        packages = {
          solana-cli = solana-source;
          geyser-plugin = almanac-geyser-plugin;
          solana-test-validator-safe = solana-test-validator-safe;
          solana-test-validator-geyser = solana-test-validator-geyser;
          solana-validator-geyser = solana-validator-geyser;
          setup-almanac-db = setup-almanac-db;
          rust = rustToolchain;
        };

        devshells.default = {
          name = "solana-geyser";
          
          commands = [
            {
              package = self'.packages.solana-cli;
              help = "Solana CLI (built from source)";
              category = "solana";
            }
            {
              package = self'.packages.solana-test-validator-safe;
              help = "Start test validator (without Geyser)";
              category = "solana";
            }
            {
              package = self'.packages.solana-test-validator-geyser;
              help = "Start test validator with Geyser plugin";
              category = "solana";
            }
            {
              package = self'.packages.solana-validator-geyser;
              help = "Start full validator with Geyser plugin";
              category = "solana";
            }
            {
              package = self'.packages.setup-almanac-db;
              help = "Set up PostgreSQL database for Almanac";
              category = "setup";
            }
            {
              package = self'.packages.rust;
              help = "Rust 1.76.0 toolchain";
              category = "development";
            }
            {
              package = pkgs.protobuf;
              help = "Protocol buffers compiler";
              category = "tools";
            }
            {
              package = pkgs.postgresql;
              help = "PostgreSQL client tools";
              category = "database";
            }
            {
              package = pkgs.jq;
              help = "JSON processor";
              category = "tools";
            }
          ];
          
          env = [
            { name = "SOLANA_RPC_URL"; value = "http://127.0.0.1:8899"; }
            { name = "RUST_BACKTRACE"; value = "1"; }
            { name = "RUSTFLAGS"; value = "-C target-cpu=x86-64"; }
            { name = "DATABASE_URL"; value = "postgresql://almanac:almanac@localhost/almanac"; }
            { name = "SOLANA_METRICS_CONFIG"; value = ""; }
          ];
          
          motd = ''
            {14}{bold}Solana with Almanac Geyser Plugin (NixOS){reset}
            {9}Socket-patched for Digital Ocean compatibility{reset}
            
            {13}Version:{reset}
              Solana CLI v1.18.26 (patched)
              Geyser plugin v0.1.0
              
            {13}Quick start:{reset}
              # First time setup
              setup-almanac-db      # Create PostgreSQL database
              
              # Start test validator with Geyser
              solana-test-validator-geyser
              
              # Or without Geyser
              solana-test-validator-safe
              
              # Monitor Geyser logs
              tail -f test-ledger/validator.log | grep -i geyser
              
            {13}Geyser configuration:{reset}
              Config: ./config/geyser-config.json
              Database: $DATABASE_URL
              
            {13}Features:{reset}
              Socket timeout patch for KVM/Digital Ocean
              Almanac database integration
              Account state tracking
              Transaction recording
              Conservative CPU flags (x86-64)
          '';
        };
      };
    };
}