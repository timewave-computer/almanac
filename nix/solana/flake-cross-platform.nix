{
  description = "Cross-platform Solana development environment with Geyser plugin support";

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
        # Platform detection
        isDarwin = pkgs.stdenv.isDarwin;
        isLinux = pkgs.stdenv.isLinux;
        isAarch64 = pkgs.stdenv.isAarch64;
        
        # Simple Almanac Geyser plugin (cross-platform compatible)
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
          ] ++ lib.optionals isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];
          
          # Build as cdylib for plugin loading
          buildType = "release";
          
          postInstall = ''
            mkdir -p $out/lib $out/etc
            
            # Copy the plugin library
            if [ -f target/release/libalmanac_geyser_plugin.dylib ]; then
              cp target/release/libalmanac_geyser_plugin.dylib $out/lib/
            fi
            if [ -f target/release/libalmanac_geyser_plugin.so ]; then
              cp target/release/libalmanac_geyser_plugin.so $out/lib/
            fi
            
            # Create default config
            cat > $out/etc/almanac-geyser-config.json <<EOF
            {
              "libpath": "$out/lib/libalmanac_geyser_plugin.${if isDarwin then "dylib" else "so"}",
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
            description = "Almanac Geyser plugin for cross-platform use";
            license = licenses.mit;
            platforms = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
          };
        };

        # Cross-platform Solana test validator with Geyser support
        solana-test-validator-geyser = pkgs.writeShellApplication {
          name = "solana-test-validator-geyser";
          runtimeInputs = [ pkgs.solana-cli almanac-geyser-plugin ];
          text = ''
            set -euo pipefail
            
            echo "Starting Solana test validator with Almanac Geyser plugin..."
            echo "Platform: ${if isDarwin then "macOS" else "Linux"}"
            echo ""
            
            # Default configuration
            LEDGER_DIR="''${LEDGER_DIR:-./test-ledger}"
            GEYSER_CONFIG="''${GEYSER_CONFIG:-${almanac-geyser-plugin}/etc/almanac-geyser-config.json}"
            
            # Create ledger directory
            mkdir -p "$LEDGER_DIR"
            
            echo "Configuration:"
            echo "  Ledger: $LEDGER_DIR"
            echo "  Geyser config: $GEYSER_CONFIG"
            echo "  Plugin library: ${almanac-geyser-plugin}/lib/"
            echo ""
            
            # Check if PostgreSQL is available
            if command -v psql >/dev/null 2>&1; then
              if psql -U almanac -d almanac -c "SELECT 1;" >/dev/null 2>&1; then
                echo "PostgreSQL connection successful"
              else
                echo "Warning: PostgreSQL not connected - Geyser plugin may not work"
                echo "  To set up PostgreSQL:"
                echo "  1. Install PostgreSQL: nix-shell -p postgresql"
                echo "  2. Create database: createdb almanac"
                echo "  3. Create user: createuser almanac"
              fi
            else
              echo "Warning: PostgreSQL not found - Geyser plugin may not work"
            fi
            echo ""
            
            # Start the validator
            exec solana-test-validator \
              --ledger "$LEDGER_DIR" \
              --rpc-port 8899 \
              --faucet-port 9900 \
              --geyser-plugin-config "$GEYSER_CONFIG" \
              --log \
              "$@"
          '';
        };

        # PostgreSQL setup helper
        setup-postgres-almanac = pkgs.writeShellApplication {
          name = "setup-postgres-almanac";
          runtimeInputs = [ pkgs.postgresql ];
          text = ''
            set -euo pipefail
            
            echo "Setting up PostgreSQL for Almanac Geyser plugin..."
            echo ""
            
            # Platform-specific setup
            ${if isDarwin then ''
              echo "macOS Setup:"
              if command -v brew >/dev/null 2>&1; then
                echo "Using Homebrew PostgreSQL (if available)"
                if brew services list | grep -q "postgresql.*started"; then
                  echo "PostgreSQL already running via Homebrew"
                elif brew services list | grep -q postgresql; then
                  echo "Starting PostgreSQL via Homebrew..."
                  brew services start postgresql
                else
                  echo "Warning: PostgreSQL not installed via Homebrew"
                  echo "Install with: brew install postgresql"
                  exit 1
                fi
              else
                echo "Using Nix PostgreSQL..."
                echo "Note: This requires manual database initialization"
                echo "Run: initdb -D ~/.local/share/postgresql"
              fi
            '' else ''
              echo "Linux Setup:"
              echo "Using Nix PostgreSQL..."
              echo "Initialize with: initdb -D ~/.local/share/postgresql"
              echo "Start with: pg_ctl -D ~/.local/share/postgresql -l logfile start"
            ''}
            
            echo ""
            echo "Creating Almanac database and tables..."
            
            # Create database and user
            createdb almanac || echo "Database 'almanac' already exists"
            createuser -s almanac || echo "User 'almanac' already exists"
            
            # Create tables
            psql -d almanac -c "
            CREATE TABLE IF NOT EXISTS solana_accounts (
                pubkey VARCHAR(44) PRIMARY KEY,
                owner VARCHAR(44) NOT NULL,
                lamports BIGINT NOT NULL,
                slot BIGINT NOT NULL,
                executable BOOLEAN NOT NULL,
                rent_epoch BIGINT NOT NULL,
                data BYTEA,
                write_version BIGINT NOT NULL,
                updated_at TIMESTAMP NOT NULL DEFAULT NOW()
            );
            
            CREATE TABLE IF NOT EXISTS solana_slots (
                slot BIGINT PRIMARY KEY,
                parent_slot BIGINT,
                status VARCHAR(20) NOT NULL,
                updated_at TIMESTAMP NOT NULL DEFAULT NOW()
            );
            
            CREATE TABLE IF NOT EXISTS solana_transactions (
                signature VARCHAR(88) PRIMARY KEY,
                slot BIGINT NOT NULL,
                success BOOLEAN NOT NULL,
                fee BIGINT NOT NULL,
                pre_balances JSONB,
                post_balances JSONB,
                instructions JSONB,
                created_at TIMESTAMP NOT NULL DEFAULT NOW()
            );
            
            -- Create indexes
            CREATE INDEX IF NOT EXISTS idx_accounts_owner ON solana_accounts(owner);
            CREATE INDEX IF NOT EXISTS idx_accounts_slot ON solana_accounts(slot);
            CREATE INDEX IF NOT EXISTS idx_transactions_slot ON solana_transactions(slot);
            "
            
            echo ""
            echo "PostgreSQL setup complete!"
            echo "Test connection with: psql -U almanac -d almanac -c 'SELECT COUNT(*) FROM solana_accounts;'"
          '';
        };

      in {
        packages = {
          solana-cli = pkgs.solana-cli;
          geyser-plugin = almanac-geyser-plugin;
          solana-test-validator-geyser = solana-test-validator-geyser;
          setup-postgres-almanac = setup-postgres-almanac;
        };

        devshells.default = {
          name = "solana-geyser-cross-platform";
          
          commands = [
            {
              package = pkgs.solana-cli;
              help = "Solana CLI tools";
              category = "solana";
            }
            {
              package = self'.packages.solana-test-validator-geyser;
              help = "Start test validator with Geyser plugin";
              category = "solana";
            }
            {
              package = self'.packages.setup-postgres-almanac;
              help = "Set up PostgreSQL for Almanac";
              category = "setup";
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
            {
              package = pkgs.curl;
              help = "HTTP client";
              category = "tools";
            }
          ];
          
          env = [
            {
              name = "GEYSER_PLUGIN_CONFIG";
              value = "${almanac-geyser-plugin}/etc/almanac-geyser-config.json";
            }
            {
              name = "SOLANA_RPC_URL";
              value = "http://localhost:8899";
            }
          ];
          
          motd = ''
            {14}{bold}Cross-Platform Solana + Geyser Environment{reset}
            {9}Platform: ${if isDarwin then "macOS" else "Linux"} (${system}){reset}
            
            {13}Quick start:{reset}
              setup-postgres-almanac        # Set up database
              solana-test-validator-geyser   # Start validator with Geyser
              
            {13}Components:{reset}
              • Solana CLI (nixpkgs)
              • Custom Almanac Geyser plugin
              • PostgreSQL integration
              • Cross-platform compatibility
              
            {11}Geyser plugin: ${almanac-geyser-plugin}/lib/{reset}
          '';
        };
      };
    };
}