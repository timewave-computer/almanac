{
  description = "Cosmos development environment for macOS with Neutron v6.0.3";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    cosmos-nix = {
      url = "github:timewave-computer/cosmos.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ { flake-parts, rust-overlay, cosmos-nix, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.devshell.flakeModule
      ];

      systems = [ "aarch64-darwin" "x86_64-darwin" ];

      perSystem = { config, self', inputs', pkgs, system, lib, ... }: 
        let
          overlays = [
            rust-overlay.overlays.default
            cosmos-nix.overlays.default
          ];
          
          pkgs' = import inputs.nixpkgs {
            inherit system;
            overlays = overlays;
            config = {
              allowUnfree = true;
            };
          };

          # Build libwasmvm v2.1.5 for Neutron v6.0.3
          wasmvm_2_1_5-src = pkgs.fetchFromGitHub {
            owner = "CosmWasm";
            repo = "wasmvm";
            rev = "v2.1.5";
            sha256 = "1arjjrlxcb2c6fvjl1ga49jbbqmbv2glpajicz0j4m9cnmhc06mb";
          };

          libwasmvm = pkgs.rustPlatform.buildRustPackage {
            pname = "libwasmvm";
            version = "2.1.5";
            
            src = "${wasmvm_2_1_5-src}/libwasmvm";
            
            cargoLock = {
              lockFile = "${wasmvm_2_1_5-src}/libwasmvm/Cargo.lock";
              outputHashes = {
                "cosmwasm-core-2.1.6" = "sha256-xkuvpHL5xxDpeRZIdGNCg+Ub0hW98xXjqdeOubpnwEU=";
              };
            };
            
            nativeBuildInputs = with pkgs; [ 
              rust-bin.stable.latest.default
              pkgs.deterministic-uname
            ];
            
            postInstall = ''
              cp ./bindings.h $out/lib/
              arch=$(uname -m)
              sofile=$out/lib/libwasmvm.so
              if [ -f "$sofile" ]; then
                ln -s libwasmvm.so $out/lib/libwasmvm.$arch.so
              fi
            '';
            
            doCheck = false;
          };

          # Neutron v6.0.3 (built from source)
          neutronPkg = pkgs.buildGoModule rec {
            pname = "neutrond";
            version = "6.0.3";

            src = pkgs.fetchFromGitHub {
              owner = "neutron-org";
              repo = "neutron";
              rev = "v${version}";
              sha256 = "sha256-kduM7WzJLGmw4Yg450UQZvv3s86jc3VXmion/CAdg+Y=";
            };

            # Patch to disable admin module message filtering for testing
            postPatch = ''
              # Remove unused imports
              sed -i '/^import ($/,/^)$/{
                /upgradetypes/d
                /wasmtypes/d
                /authtypes/d
                /banktypes/d
                /consensustypes/d
                /crisistypes/d
                /minttypes/d
                /slashingtypes/d
                /stakingtypes/d
                /icahosttypes/d
                /ibctransfertypes/d
                /ibcclienttypes/d
                /feemarkettypes/d
                /marketmaptypes/d
                /revenuetypes/d
                /harpoontypes/d
                /ibcratelimittypes/d
                /dynamicfeestypes/d
                /globalfeetypes/d
                /contractmanagertypes/d
                /crontypes/d
                /dextypes/d
                /feeburnertypes/d
                /feerefundertypes/d
                /interchainqueriestypes/d
                /interchaintxstypes/d
                /tokenfactorytypes/d
              }' app/proposals_allowlisting.go
              
              # Replace isSdkMessageWhitelisted to always return true
              sed -i '/^func isSdkMessageWhitelisted(msg sdk\.Msg) bool {$/,/^}$/c\
func isSdkMessageWhitelisted(msg sdk.Msg) bool {\
	// PATCHED: Always allow all messages for testing\
	_ = msg\
	return true\
}' app/proposals_allowlisting.go
              
              # Replace IsConsumerProposalAllowlisted to always return true
              sed -i '/^func IsConsumerProposalAllowlisted(content govtypes\.Content) bool {$/,/^}$/c\
func IsConsumerProposalAllowlisted(content govtypes.Content) bool {\
	// PATCHED: Always allow all proposals for testing\
	_ = content\
	return true\
}' app/proposals_allowlisting.go
            '';

            vendorHash = "sha256-wJvvogrVr+rpdZkkHnqwwRlbbuTrlJhKwQ0e71eYXJc=";

            # Build tags
            tags = [ "netgo" "skip_ccv_msg_filter" ];

            # Build flags
            ldflags = [
              "-X github.com/cosmos/cosmos-sdk/version.Name=neutron"
              "-X github.com/cosmos/cosmos-sdk/version.AppName=neutrond"
              "-X github.com/cosmos/cosmos-sdk/version.Version=${version}"
              "-X github.com/cosmos/cosmos-sdk/version.Commit=${src.rev}"
            ];
            
            flags = [ "-trimpath" ];

            doCheck = false;

            subPackages = [ "cmd/neutrond" ];

            nativeBuildInputs = with pkgs; [
              pkg-config
              go
            ] ++ lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.cctools
            ];
            
            buildInputs = with pkgs; [
              openssl
            ] ++ lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
              pkgs.darwin.apple_sdk.frameworks.Security
              pkgs.darwin.apple_sdk.frameworks.CoreFoundation
            ];
            
            # Environment for building
            CGO_ENABLED = "1";
            CGO_LDFLAGS = "-L${libwasmvm}/lib";
            CGO_CFLAGS = "-I${libwasmvm}/lib";
            
            # Post-install setup
            postInstall = ''
              # Ensure binary has proper permissions
              chmod +x $out/bin/neutrond
            '';
            
            postFixup = lib.optionalString pkgs.stdenv.isDarwin ''
              # Fix macOS dynamic library paths
              install_name_tool -add_rpath ${libwasmvm}/lib $out/bin/neutrond
            '';
          };

          # Neutron startup script
          neutronStart = pkgs.writeShellApplication {
            name = "neutron-start";
            runtimeInputs = [ neutronPkg ];
            text = ''
              #!/usr/bin/env bash
              set -e

              # Setup environment
              ADMIN_MNEMONIC="abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"

              # Check if Neutron is already running
              if pgrep -x "neutrond" > /dev/null; then
                  echo "Neutron is already running"
                  exit 0
              fi

              echo "Starting Neutron testnet (v6.0.3)..."

              # Initialize chain
              rm -rf ~/.neutrond
              neutrond init testing --chain-id testing --default-denom untrn

              # Add genesis account
              echo "$ADMIN_MNEMONIC" | neutrond keys add admin --keyring-backend test --recover
              neutrond add-genesis-account admin 10000000000000stake,10000000000000untrn --keyring-backend test

              # Set up consumer chain
              echo "Setting up consumer chain for single node testing..."
              neutrond add-consumer-section

              # Create validator genesis transaction
              echo "Creating validator genesis transaction..."
              neutrond gentx admin 1000000000stake --chain-id testing --keyring-backend test
              neutrond collect-gentxs

              echo "Configuring for development..."

              # Configure for development
              sed -i.bak 's/minimum-gas-prices = ""/minimum-gas-prices = "0.0001untrn"/' ~/.neutrond/config/app.toml
              sed -i.bak 's/enabled-unsafe-cors = false/enabled-unsafe-cors = true/' ~/.neutrond/config/app.toml
              sed -i.bak 's/enable = false/enable = true/' ~/.neutrond/config/app.toml
              sed -i.bak 's/swagger = false/swagger = true/' ~/.neutrond/config/app.toml
              sed -i.bak 's/pruning = "default"/pruning = "everything"/' ~/.neutrond/config/app.toml

              sed -i.bak 's/cors_allowed_origins = \[\]/cors_allowed_origins = ["*"]/' ~/.neutrond/config/config.toml
              sed -i.bak 's/^create_empty_blocks = true/create_empty_blocks = false/' ~/.neutrond/config/config.toml
              sed -i.bak 's/^timeout_commit = "5s"/timeout_commit = "1s"/' ~/.neutrond/config/config.toml

              # Configure feemarket parameters
              sed -i.bak 's/"fee_denom": "stake"/"fee_denom": "untrn"/' ~/.neutrond/config/genesis.json
              sed -i.bak 's/"min_base_gas_price": "[^"]*"/"min_base_gas_price": "0.000100000000000000"/' ~/.neutrond/config/genesis.json
              sed -i.bak 's/"base_gas_price": "[^"]*"/"base_gas_price": "0.000100000000000000"/' ~/.neutrond/config/genesis.json

              echo "Starting Neutron node..."
              neutrond start --rpc.unsafe
            '';
          };

        in {
          packages = {
            neutron = neutronPkg;
            libwasmvm = libwasmvm;
            neutron-start = neutronStart;
            default = neutronPkg;
          };

          devshells.default = {
            name = "cosmos-neutron-v6-macos";
            
            packages = with pkgs; [
              neutronPkg
              go
              rustc
              cargo
              curl
              jq
              nodejs
              yarn
              python3
              git
              protobuf
              llvm
              lld
              clang
            ];
            
            commands = [
              {
                package = neutronPkg;
                help = "Neutron blockchain daemon v6.0.3";
              }
              {
                package = neutronStart;
                help = "Start Neutron testnet with proper configuration";
              }
            ];
            
            env = [
              {
                name = "NEUTRON_RPC_URL";
                value = "http://127.0.0.1:26657";
              }
              {
                name = "NEUTRON_REST_URL";
                value = "http://127.0.0.1:1317";
              }
              {
                name = "NEUTRON_GRPC_URL";
                value = "http://127.0.0.1:9090";
              }
              {
                name = "CGO_ENABLED";
                value = "1";
              }
              {
                name = "GOOS";
                value = "darwin";
              }
              {
                name = "GOARCH";
                value = if pkgs.stdenv.isAarch64 then "arm64" else "amd64";
              }
            ];
            
            motd = ''
              {14}{bold}Neutron v6.0.3 Development Environment (macOS){reset}
              {9}Built from source with quartz-compatible configuration{reset}
              
              {13}Quick Start:{reset}
                neutron-start     # Start Neutron testnet
                neutrond status   # Check node status
                neutrond version  # Should show v6.0.3
              
              {13}Test Accounts:{reset}
                Admin: neutron19rl4cm2hmr8afy4kldpxz3fka4jguq0aclyl9j
                Mnemonic: abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about
              
              {13}Endpoints:{reset}
                RPC: http://localhost:26657
                REST: http://localhost:1317
                gRPC: http://localhost:9090
            '';
          };
        };
    };
}