{
  description = "Solana development environment with solana node and development tools";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
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
        # Apply rust-overlay to pkgs for rust-bin support
        rustPkgs = pkgs.appendOverlays [
          inputs.rust-overlay.overlays.rust-overlay
        ];
        
        # Solana and Anchor versions - pinned for stability (from zero.nix)
        sol-version = "2.0.22";
        anchor-version = "0.31.1";
        platform-tools-version = "1.48";
        
        # macOS deployment target (used for all Darwin systems)
        darwinDeploymentTarget = "11.0";
        
        # Common environment variables for Solana development
        commonEnv = {
          SOURCE_DATE_EPOCH = "1686858254"; # Fixed value for reproducible builds
          SOLANA_INSTALL_DIR = "$HOME/.cache/solana";
          ANCHOR_VERSION = anchor-version;
          SOLANA_VERSION = sol-version;
          RUST_BACKTRACE = "1";
        } // lib.optionalAttrs pkgs.stdenv.isDarwin {
          MACOSX_DEPLOYMENT_TARGET = darwinDeploymentTarget;
        };

        # Nightly Rust environment specifically for IDL generation
        nightly-rust = rustPkgs.rust-bin.nightly."2024-12-01".default.override {
          extensions = [ "rust-src" "llvm-tools-preview" ];
        };

        # Unified Solana node derivation including platform tools and CLI tools (from zero.nix)
        solana-node = pkgs.stdenv.mkDerivation rec {
          pname = "solana-node";
          version = sol-version;

          # Primary Solana CLI source
          solana-src = pkgs.fetchurl {
            url = "https://github.com/anza-xyz/agave/releases/download/v${version}/solana-release-${
              if pkgs.stdenv.isDarwin then
                if pkgs.stdenv.isAarch64 then "aarch64-apple-darwin" else "x86_64-apple-darwin"
              else
                "x86_64-unknown-linux-gnu"
            }.tar.bz2";
            sha256 = if pkgs.stdenv.isDarwin then
              "sha256-upgxwAEvh11+IKVQ1FaZGlx8Z8Ps0CEScsbu4Hv3WH0="  # v2.0.22 macOS ARM64 hash
            else
              "sha256-kDXSnCXZHYSQt18AtnuBHcqyK9wxEemNx9on7CKW328=";  # v2.0.22 Linux x86_64 hash
          };

          # Platform tools source
          platform-tools-src = pkgs.fetchurl {
            url = if pkgs.stdenv.isDarwin then
              "https://github.com/anza-xyz/platform-tools/releases/download/v${platform-tools-version}/platform-tools-osx-aarch64.tar.bz2"
            else
              "https://github.com/anza-xyz/platform-tools/releases/download/v${platform-tools-version}/platform-tools-linux-x86_64.tar.bz2";
            sha256 = if pkgs.stdenv.isDarwin then
              "sha256-eZ5M/O444icVXIP7IpT5b5SoQ9QuAcA1n7cSjiIW0t0="  # v1.48 macOS ARM64 hash
            else
              "sha256-qdMVf5N9X2+vQyGjWoA14PgnEUpmOwFQ20kuiT7CdZc=";  # v1.48 Linux x86_64 hash
          };

          nativeBuildInputs = with pkgs; [
            makeWrapper
          ] ++ lib.optionals stdenv.isLinux [
            autoPatchelfHook
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.cctools
            darwin.sigtool
          ];

          buildInputs = with pkgs; [
            stdenv.cc.cc.lib
            zlib
            openssl
            libffi
          ] ++ lib.optionals stdenv.isLinux [
            glibc
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];
          
          # Disable stripping to avoid issues with text files and scripts
          dontStrip = true;

          unpackPhase = ''
            runHook preUnpack
            
            # Create separate directories for each source
            mkdir -p solana-cli platform-tools
            
            # Extract Solana CLI
            cd solana-cli
            tar -xf ${solana-src}
            cd ..
            
            # Extract platform tools
            cd platform-tools
            tar -xf ${platform-tools-src}
            cd ..
            
            runHook postUnpack
          '';

          installPhase = ''
            runHook preInstall
            
            mkdir -p $out/bin $out/lib $out/platform-tools
            
            # Install Solana CLI tools
            if [ -d "solana-cli/solana-release/bin" ]; then
              cp -r solana-cli/solana-release/bin/* $out/bin/
            elif [ -d "solana-cli/bin" ]; then
              cp -r solana-cli/bin/* $out/bin/
            else
              # Look for any directories with binaries
              for dir in solana-cli/*/; do
                if [ -d "$dir/bin" ]; then
                  cp -r "$dir/bin"/* $out/bin/
                  break
                fi
              done
            fi
            
            # Install platform tools
            cp -r platform-tools/* $out/platform-tools/
            
            # Make platform tools binaries available in PATH
            if [ -d "$out/platform-tools/rust/bin" ]; then
              for tool in $out/platform-tools/rust/bin/*; do
                if [ -f "$tool" ] && [ -x "$tool" ]; then
                  tool_name=$(basename "$tool")
                  # Don't override main Solana CLI tools
                  if [ ! -f "$out/bin/$tool_name" ]; then
                    ln -sf "$tool" "$out/bin/$tool_name"
                  fi
                fi
              done
            fi
            
            if [ -d "$out/platform-tools/llvm/bin" ]; then
              for tool in $out/platform-tools/llvm/bin/*; do
                if [ -f "$tool" ] && [ -x "$tool" ]; then
                  tool_name=$(basename "$tool")
                  # Don't override main Solana CLI tools
                  if [ ! -f "$out/bin/$tool_name" ]; then
                    ln -sf "$tool" "$out/bin/$tool_name"
                  fi
                fi
              done
            fi
            
            # Ensure all binaries are executable
            find $out -type f -executable -exec chmod +x {} \; 2>/dev/null || true
            
            # Fix broken symlinks
            find $out -type l ! -exec test -e {} \; -delete 2>/dev/null || true
            
            # Use wrapProgram to wrap key tools with proper environment
            for tool in solana solana-keygen solana-test-validator; do
              if [ -f "$out/bin/$tool" ]; then
                wrapProgram "$out/bin/$tool" \
                  --set PLATFORM_TOOLS_DIR "$out/platform-tools" \
                  --set SBF_SDK_PATH "$out/platform-tools" \
                  --prefix PATH : "$out/platform-tools/rust/bin" \
                  --prefix PATH : "$out/platform-tools/llvm/bin"
              fi
            done
            
            runHook postInstall
          '';

          meta = with pkgs.lib; {
            description = "Complete Solana node with platform tools and CLI";
            homepage = "https://solana.com";
            license = licenses.asl20;
            platforms = [ "x86_64-linux" "x86_64-darwin" "aarch64-darwin" ];
            maintainers = [ ];
          };
        };

        # Pre-fetch the macOS-compatible affinity source to avoid git network calls during build
        affinity-macos-src = pkgs.fetchFromGitHub {
          owner = "elast0ny";
          repo = "affinity";
          rev = "67b925db00575a35d839455964baea494ac86ec2";
          hash = "sha256-54Z45C751GCTadHJVorxWz40Igmk2D2QsQZwIQ9yAfc=";
        };

        # Build Yellowstone Geyser plugin for Solana with macOS affinity patch
        yellowstone-grpc-geyser = pkgs.rustPlatform.buildRustPackage rec {
          pname = "yellowstone-grpc-geyser";
          version = "8.0.0+solana.2.3.3";

          src = pkgs.fetchFromGitHub {
            owner = "rpcpool";
            repo = "yellowstone-grpc";
            rev = "v${version}";
            hash = "sha256-rbGS0NLljGrv5Ffap0T+28tLN7sRYclMQYJA/BlmiNs=";
          };

          cargoHash = "sha256-KWY0qQya9k674WPm5Oj3rOdKl2PfFnAGT5pS66rQUFc=";

          # Patch the workspace Cargo.toml to use pre-fetched affinity source
          postPatch = ''
            # Copy the macOS-compatible affinity source into the vendor directory
            mkdir -p vendor/affinity
            cp -r ${affinity-macos-src}/* vendor/affinity/
            
            # Find and patch the workspace Cargo.toml to override affinity dependency
            if [ -f Cargo.toml ]; then
              echo "Patching workspace Cargo.toml..."
              sed -i 's|affinity = "0\.1\.2"|affinity = { path = "./vendor/affinity" }|g' Cargo.toml
              
              echo "Workspace Cargo.toml affinity section:"
              grep -A2 -B2 'affinity.*=' Cargo.toml || echo "affinity not found in workspace Cargo.toml"
            fi
            
            echo "Patched affinity dependency to use local macOS-compatible version"
          '';

          # Build only the Geyser plugin library
          cargoBuildFlags = [ "--package" "yellowstone-grpc-geyser" ];
          
          # Skip tests for faster builds  
          doCheck = false;

          nativeBuildInputs = with pkgs; [
            pkg-config
            protobuf
            git
          ];

          buildInputs = with pkgs; [
            openssl
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          # Set library type to cdylib for plugin loading
          CARGO_BUILD_TARGET = if pkgs.stdenv.isDarwin then
            "${if pkgs.stdenv.isAarch64 then "aarch64" else "x86_64"}-apple-darwin"
          else
            "${if pkgs.stdenv.isAarch64 then "aarch64" else "x86_64"}-unknown-linux-gnu";

          # Set git version info for build script
          VERGEN_GIT_DESCRIBE = "v${version}";
          VERGEN_GIT_SHA = "unknown";
          SOURCE_DATE_EPOCH = "1686858254";

          # Set up fake git repo for build script
          preBuild = ''
            git init
            git config user.email "nix@build.local"
            git config user.name "Nix Build"
            git add .
            git commit -m "nix build" --allow-empty
            git tag "v${version}"
          '';

          # Post-install to ensure the plugin library is accessible
          postInstall = ''
            # Create symlinks for easier plugin discovery
            mkdir -p $out/plugins
            if [ -f $out/lib/libyellowstone_grpc_geyser.dylib ]; then
              ln -sf $out/lib/libyellowstone_grpc_geyser.dylib $out/plugins/
            fi
            if [ -f $out/lib/libyellowstone_grpc_geyser.so ]; then
              ln -sf $out/lib/libyellowstone_grpc_geyser.so $out/plugins/
            fi
          '';

          meta = with pkgs.lib; {
            description = "Yellowstone gRPC Geyser plugin for Solana";
            homepage = "https://github.com/rpcpool/yellowstone-grpc";
            license = licenses.asl20;
            platforms = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
          };
        };

        # Build anchor CLI from source with rust-overlay (from zero.nix)
        anchor = pkgs.rustPlatform.buildRustPackage rec {
          pname = "anchor-cli";
          version = anchor-version;

          src = pkgs.fetchFromGitHub {
            owner = "coral-xyz";
            repo = "anchor";
            rev = "v${version}";
            hash = "sha256-pvD0v4y7DilqCrhT8iQnAj5kBxGQVqNvObJUBzFLqzA=";
            fetchSubmodules = true;
          };

          useFetchCargoVendor = true;
          cargoHash = "sha256-fjhLA+utQdgR75wg+/N4VwASW6+YBHglRPj14sPHmGA=";

          # Build the CLI package specifically
          cargoBuildFlags = [ "--package" "anchor-cli" ];
          cargoTestFlags = [ "--package" "anchor-cli" ];
          
          # Skip tests for faster builds
          doCheck = false;

          nativeBuildInputs = with pkgs; [
            pkg-config
            rustc
            cargo
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          buildInputs = with pkgs; [
            openssl
            libiconv
          ] ++ lib.optionals stdenv.isLinux [
            libudev-zero
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.CoreFoundation
            darwin.apple_sdk.frameworks.CoreServices
          ];

          # Environment variables for building
          OPENSSL_NO_VENDOR = "1";
          SOURCE_DATE_EPOCH = commonEnv.SOURCE_DATE_EPOCH;

          # Platform-specific environment variables
          MACOSX_DEPLOYMENT_TARGET = lib.optionalString pkgs.stdenv.isDarwin darwinDeploymentTarget;
          RUSTFLAGS = lib.optionalString pkgs.stdenv.isDarwin 
            "-C link-arg=-undefined -C link-arg=dynamic_lookup";

          # Clean build without toolchain installation patches
          postPatch = ''
            # Remove any toolchain installation code that might interfere
            find . -name "*.rs" -type f -exec sed -i 's/install_toolchain_if_needed.*;//g' {} \;
          '';

          meta = with pkgs.lib; {
            description = "Solana Sealevel Framework CLI";
            homepage = "https://github.com/coral-xyz/anchor";
            license = licenses.asl20;
            maintainers = [ ];
            platforms = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
          };
        };

        # Setup script for Solana environment (from zero.nix)
        # Platform-specific setup script (only on supported platforms)
        setup-solana = if (pkgs.stdenv.system != "aarch64-linux") then
          pkgs.writeShellScriptBin "setup-solana" ''
            set -e
            
            echo "Setting up Solana development environment..."
            echo "Platform tools v${platform-tools-version} location: ${solana-node}"
            echo "Solana CLI v${sol-version} location: ${solana-node}"
            echo "Anchor CLI v${anchor-version} location: ${anchor}"
            
            # Create cache directories with proper permissions
            SOLANA_CACHE_DIR="$HOME/.cache/solana"
            mkdir -p "$SOLANA_CACHE_DIR/v${platform-tools-version}/cargo" "$SOLANA_CACHE_DIR/v${platform-tools-version}/rustup"
            mkdir -p "$SOLANA_CACHE_DIR/releases" "$SOLANA_CACHE_DIR/config"
            chmod -R 755 "$SOLANA_CACHE_DIR"
            
            echo "Solana development environment setup complete!"
          ''
        else
          pkgs.writeShellScriptBin "setup-solana" ''
            echo "Solana node is not available on aarch64-linux (ARM64 Linux)"
            echo "Available tools: anchor CLI for smart contract development"
            echo "For full Solana functionality, use x86_64-linux, x86_64-darwin, or aarch64-darwin"
          '';

        # Platform-specific anchor wrapper
        anchor-wrapper = if (pkgs.stdenv.system != "aarch64-linux") then
          pkgs.writeShellScriptBin "anchor" ''
            set -e
            
            # Set up platform tools environment for SBF compilation  
            export PLATFORM_TOOLS_DIR=${solana-node}/platform-tools
            export SBF_SDK_PATH=${solana-node}/platform-tools
            export PATH="${solana-node}/platform-tools/rust/bin:${solana-node}/bin:$PATH"
          
            # Set required environment variables
            export SOURCE_DATE_EPOCH="${commonEnv.SOURCE_DATE_EPOCH}" 
            export RUST_BACKTRACE=1
            export PROTOC=${pkgs.protobuf}/bin/protoc
            
            # Platform-specific environment variables
            ${lib.optionalString pkgs.stdenv.isDarwin ''
              export MACOSX_DEPLOYMENT_TARGET="${darwinDeploymentTarget}"
              export CARGO_BUILD_TARGET="${if pkgs.stdenv.isAarch64 then "aarch64" else "x86_64"}-apple-darwin"
              export RUSTFLAGS="-C link-arg=-undefined -C link-arg=dynamic_lookup"
            ''}
            ${lib.optionalString pkgs.stdenv.isLinux ''
              export CARGO_BUILD_TARGET="${if pkgs.stdenv.isAarch64 then "aarch64" else "x86_64"}-unknown-linux-gnu"
            ''}
            
            # Critical: Set cargo/rustup cache directories to prevent redownloading
            export CARGO_HOME="$HOME/.cache/solana/v${platform-tools-version}/cargo"
            export RUSTUP_HOME="$HOME/.cache/solana/v${platform-tools-version}/rustup"
            
            # Ensure cache directories exist
            mkdir -p "$CARGO_HOME" "$RUSTUP_HOME"
            
            # Check if this is for IDL generation and use nightly rust
            case " $@ " in
              *" idl "*)
                export PATH="${nightly-rust}/bin:${solana-node}/bin:$PATH"
                export RUSTC="${nightly-rust}/bin/rustc"
                export CARGO="${nightly-rust}/bin/cargo"
                ;;
            esac
            
            # Run anchor with platform tools environment
            exec "${anchor}/bin/anchor" "$@"
          ''
        else
          pkgs.writeShellScriptBin "anchor" ''
            # Fallback anchor wrapper for unsupported platforms
            exec "${anchor}/bin/anchor" "$@"
          '';

        # Combined solana development environment
        solana-tools = pkgs.symlinkJoin {
          name = "solana-tools";
          paths = [
            anchor-wrapper
            setup-solana
            yellowstone-grpc-geyser
          ] ++ lib.optionals (pkgs.stdenv.system != "aarch64-linux") [
            solana-node
          ];
        };

      in {
        packages = {
          inherit anchor setup-solana nightly-rust anchor-wrapper solana-tools yellowstone-grpc-geyser;
        } // lib.optionalAttrs (pkgs.stdenv.system != "aarch64-linux") {
          inherit solana-node;
        };

        devshells.default = {
          name = "solana-development";
          
          commands = [
          ] ++ lib.optionals (pkgs.stdenv.system != "aarch64-linux") [
            {
              package = self'.packages.solana-node;
              help = "Solana CLI and test validator";
            }
          ] ++ [
            {
              package = self'.packages.anchor-wrapper;
              help = "Anchor framework for Solana development";
            }
            {
              package = self'.packages.setup-solana;
              help = "Setup Solana development environment";
            }
            {
              package = self'.packages.yellowstone-grpc-geyser;
              help = "Yellowstone gRPC Geyser plugin for real-time data streaming";
            }
            {
              package = pkgs.curl;
              help = "HTTP client for API testing";
            }
            {
              package = pkgs.jq;
              help = "JSON processor for API responses";
            }
            {
              package = pkgs.nodejs;
              help = "Node.js runtime for development tools";
            }
          ];
          
          motd = ''
            {14}{bold}Solana development environment{reset}
            {9}Solana v${sol-version} + Anchor v${anchor-version}{reset}
            
            {13}Quick start:{reset}
              setup-solana
              solana-test-validator
              anchor init my-project
              cd my-project && anchor build
          '';
        };
      };
    };
}