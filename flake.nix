# This file defines a Nix flake for the Almanac project with CosmWasm support.
{
  description = "Almanac Project Root";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    # No longer need direct reth input
    # reth.url = "github:paradigmxyz/reth/main";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    
    # Input needed for flake reference (not used directly anymore)
    wasmd-src = {
      url = "github:CosmWasm/wasmd/v0.31.0";
      flake = false;
    };
    foundry = {
      url = "github:foundry-rs/foundry";
    };
  };

  outputs = inputs@{ self, nixpkgs, flake-parts, wasmd-src, foundry, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      # Apply nixpkgs config and overlays here
      flake = { 
        nixpkgs.config = {
          allowUnfree = true;
          allowUnsupportedSystem = true;
        };
      };
      # Import our modules
      imports = [
        ./nix/cosmos-module.nix
      ];
      
      systems = ["aarch64-darwin" "x86_64-linux"];

      # Define perSystem configuration
      perSystem = { config, self', inputs', system, ... }:
        let 
          # Apply overlay to pkgs for this system
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ inputs.rust-overlay.overlays.default ];
            config = {
              allowUnfree = true;
              allowUnsupportedSystem = true;
            };
          };

          # Define foundry package from nixpkgs
          foundryPkg = pkgs.foundry;

          # Define Reth build logic based on their flake
          rethSrc = pkgs.fetchFromGitHub {
            owner = "paradigmxyz";
            repo = "reth";
            rev = "v1.3.7"; # Use tag name instead of hash
            hash = "sha256-nqahs6zGQG/qG6Qe/tKNIPGLIiQcng1zDZFKrUBpoiM="; # Correct hash
            fetchSubmodules = true;
          };
          cargoTOML = (builtins.fromTOML (builtins.readFile "${rethSrc}/Cargo.toml"));
          packageVersion = cargoTOML.workspace.package.version;
          # Use a specific version known to be available via rust-overlay
          rustVersion = cargoTOML.workspace.package.rust-version;
          rustPkg = pkgs.rust-bin.stable."1.85.0".default.override {
            extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
          };
          macPackages = pkgs.lib.optionals pkgs.stdenv.isDarwin (with pkgs.darwin.apple_sdk.frameworks; [ Security CoreFoundation CoreServices ]);
          linuxPackages = pkgs.lib.optionals pkgs.stdenv.isLinux (with pkgs; [
            libclang.lib
            llvmPackages.libcxxClang
          ]);
          cargoDeps = pkgs.rustPlatform.importCargoLock {
            lockFile = "${rethSrc}/Cargo.lock";
          };
          rustPlatform = pkgs.makeRustPlatform {
            rustc = rustPkg;
            cargo = rustPkg;
          };

          # Define scripts for test apps
          testEthAnvilScript = pkgs.writeShellScript "test-eth-anvil-runner" ''
            export ETH_RPC_URL="http://127.0.0.1:8545"
            # Set target dir to a writable temporary location (use escaped $)
            export CARGO_TARGET_DIR="\$TMPDIR/cargo-target-anvil"
            exec "${self}/scripts/test-ethereum-adapter.sh" "$@"
          '';
          testEthRethScript = pkgs.writeShellScript "test-eth-reth-runner" ''
            export ETH_RPC_URL="http://127.0.0.1:8545" # Assuming default reth port
            # Set target dir to a writable temporary location (use escaped $)
            export CARGO_TARGET_DIR="\$TMPDIR/cargo-target-reth"
            exec "${self}/scripts/test-ethereum-adapter.sh" "$@"
          '';
          
          # Database initialization and testing scripts
          initDatabasesScript = pkgs.writeShellScript "init-databases-runner" ''
            # Set postgres environment variables
            export PGHOST=localhost
            # Export environment variable to indicate nix shell
            export IN_NIX_SHELL=1
            exec "${self}/scripts/init-databases.sh" "$@"
          '';
          testDatabasesScript = pkgs.writeShellScript "test-databases-runner" ''
            # Set postgres environment variables
            export PGHOST=localhost
            # Export environment variable to indicate nix shell
            export IN_NIX_SHELL=1
            exec "${self}/scripts/test-databases.sh" "$@"
          '';

        in
        {
          # Create the default development shell
          devShells.default = pkgs.mkShell {
            packages = [ 
              pkgs.git 
              # Include essential cosmos packages
              self'.packages.wasmd-node
              self'.packages.test-cosmos-adapter
              # Include Ethereum tools
              foundryPkg # Provides anvil
              self'.packages.reth-pkg # Use our manually built reth
              # General dev tools
              pkgs.jq
              pkgs.go
              pkgs.curl
              pkgs.gzip
              pkgs.sqlx-cli
              pkgs.postgresql # Add PostgreSQL server package
            ];
            
            shellHook = ''
              echo "=== Almanac Development Environment ===="
              echo "Available shell commands:"
              echo "  (Cosmos)"
              echo "  - wasmd-node: Start a local wasmd node for testing"
              echo "  - test-cosmos-adapter: Run cosmos adapter tests against local node"
              echo "  (Ethereum)"
              echo "  - anvil: Start local Ethereum test node"
              echo "  - reth node: Start Reth Ethereum node (requires config)"
              echo "  - test-ethereum-adapter-anvil: Run tests against anvil"
              echo "  - test-ethereum-adapter-reth: Run tests against reth"
              echo "  (Database)"
              echo "  - init-databases: Initialize PostgreSQL and RocksDB"
              echo "  - test-databases: Test database connectivity"
              echo ""
              echo "Available nix run commands:"
              echo "  (Cosmos)"
              echo "  - nix run .#wasmd-node"
              echo "  - nix run .#test-cosmos-adapter"
              echo "  (Ethereum)"
              echo "  - nix run .#start-anvil"
              echo "  - nix run .#start-reth"
              echo "  - nix run .#test-ethereum-adapter-anvil"
              echo "  - nix run .#test-ethereum-adapter-reth"
              echo "  (Database)"
              echo "  - nix run .#init-databases"
              echo "  - nix run .#test-databases"
              
              # Create aliases for database commands
              alias init-databases='${self'.packages.init-databases}/bin/init-databases'
              alias test-databases='${self'.packages.test-databases}/bin/test-databases'
            '';
          };

          # Define packages needed for apps
          packages = {
            # Build reth manually
            reth-pkg = rustPlatform.buildRustPackage {
              pname = "reth";
              version = packageVersion;
              cargoLock = {
                lockFile = "${rethSrc}/Cargo.lock";
              };
              checkFlags = [
                #this test breaks Read Only FS sandbox
                "--skip=cli::tests::parse_env_filter_directives"
              ];
              LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
              nativeBuildInputs = (with pkgs;[ libclang ]) ++ macPackages ++ linuxPackages;
              src = rethSrc;
            };

            # Simple wrapper for anvil
             start-anvil = pkgs.stdenv.mkDerivation {
               name = "start-anvil";
               src = pkgs.lib.cleanSource ./.;
               buildInputs = [ pkgs.makeWrapper foundryPkg ];
               installPhase = ''
                 mkdir -p $out/bin
                 makeWrapper ${foundryPkg}/bin/anvil $out/bin/start-anvil
               '';
             };
             # Simple wrapper for reth
             start-reth = pkgs.stdenv.mkDerivation {
               name = "start-reth";
               src = pkgs.lib.cleanSource ./.;
               buildInputs = [ pkgs.makeWrapper self'.packages.reth-pkg pkgs.openssl ]; 
               installPhase = ''
                 mkdir -p $out/bin
                 makeWrapper ${self'.packages.reth-pkg}/bin/reth $out/bin/start-reth --add-flags "node"
               '';
             };
             
             # Database tools
             init-databases = pkgs.stdenv.mkDerivation {
               name = "init-databases";
               src = pkgs.lib.cleanSource ./.;
               buildInputs = [ pkgs.makeWrapper pkgs.postgresql ];
               installPhase = ''
                 mkdir -p $out/bin
                 cp scripts/init-databases.sh $out/bin/init-databases
                 chmod +x $out/bin/init-databases
                 wrapProgram $out/bin/init-databases \
                   --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.postgresql pkgs.sqlx-cli ]}
               '';
             };
             test-databases = pkgs.stdenv.mkDerivation {
               name = "test-databases";
               src = pkgs.lib.cleanSource ./.;
               buildInputs = [ pkgs.makeWrapper pkgs.postgresql ];
               installPhase = ''
                 mkdir -p $out/bin
                 cp scripts/test-databases.sh $out/bin/test-databases
                 chmod +x $out/bin/test-databases
                 wrapProgram $out/bin/test-databases \
                   --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.postgresql pkgs.cargo ]}
               '';
             };
          };

          # Define runnable applications
          apps = {
            # Cosmos Apps
            wasmd-node = {
              type = "app";
              program = "${self'.packages.wasmd-node}/bin/wasmd-node";
            };
            test-cosmos-adapter = {
              type = "app";
              program = "${self'.packages.test-cosmos-adapter}/bin/test-cosmos-adapter";
            };
            # Ethereum Apps
            start-anvil = {
              type = "app";
              program = "${self'.packages.start-anvil}/bin/start-anvil";
            };
            start-reth = {
              type = "app";
              program = "${self'.packages.start-reth}/bin/start-reth";
            };
             # Define test apps directly with inline script
             test-ethereum-adapter-anvil = {
              type = "app";
              program = "${testEthAnvilScript}"; # Reference the script derivation
            };
            test-ethereum-adapter-reth = {
              type = "app";
              program = "${testEthRethScript}"; # Reference the script derivation
            };
            # Database Apps
            init-databases = {
              type = "app";
              program = "${self'.packages.init-databases}/bin/init-databases";
            };
            test-databases = {
              type = "app";
              program = "${self'.packages.test-databases}/bin/test-databases";
            };
          };
        };
    };
}