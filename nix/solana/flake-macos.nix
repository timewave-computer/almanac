{
  description = "Solana development environment for macOS";

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

      perSystem = { config, self', inputs', pkgs, system, ... }: {
        packages = rec {
          # Use system packages optimized for macOS
          solana-cli = pkgs.solana-cli;
          anchor = pkgs.anchor;
        };

        devshells.default = {
          name = "solana-development-macos";
          
          commands = [
            {
              package = self'.packages.solana-cli;
              help = "Solana CLI tools";
            }
            {
              package = self'.packages.anchor;
              help = "Anchor framework for Solana";
            }
            {
              package = pkgs.rustc;
              help = "Rust compiler";
            }
            {
              package = pkgs.cargo;
              help = "Rust package manager";
            }
            {
              package = pkgs.curl;
              help = "HTTP client for API testing";
            }
            {
              package = pkgs.jq;
              help = "JSON processor";
            }
            {
              package = pkgs.nodejs;
              help = "Node.js for Solana web3.js";
            }
            {
              package = pkgs.yarn;
              help = "Package manager for Node.js";
            }
            {
              package = pkgs.python3;
              help = "Python for Solana Python SDK";
            }
            {
              package = pkgs.git;
              help = "Version control";
            }
            {
              package = pkgs.llvm;
              help = "LLVM toolchain for wasm32 compilation";
            }
            {
              package = pkgs.lld;
              help = "LLVM linker for wasm32 targets";
            }
            {
              package = pkgs.clang;
              help = "Clang compiler (part of LLVM)";
            }
          ];
          
          env = [
            {
              name = "SOLANA_RPC_URL";
              value = "http://127.0.0.1:8899";
            }
            {
              name = "SOLANA_WS_URL";
              value = "ws://127.0.0.1:8900";
            }
            {
              name = "ANCHOR_PROVIDER_URL";
              value = "http://127.0.0.1:8899";
            }
            {
              name = "ANCHOR_WALLET";
              value = "$HOME/.config/solana/id.json";
            }
            {
              name = "RUST_BACKTRACE";
              value = "1";
            }
            {
              name = "SOLANA_LOG_LEVEL";
              value = "info";
            }
            {
              name = "MACOSX_DEPLOYMENT_TARGET";
              value = "10.15";
            }
            {
              name = "CC_wasm32_unknown_unknown";
              value = "${pkgs.clang}/bin/clang";
            }
            {
              name = "TARGET_CC";
              value = "${pkgs.clang}/bin/clang";
            }
            {
              name = "CFLAGS_wasm32_unknown_unknown";
              value = "-O2 -fPIC";
            }
            {
              name = "TARGET_CFLAGS";
              value = "-O2 -fPIC";
            }
            {
              name = "NIX_HARDENING_ENABLE";
              value = "";
            }
          ];
          
          motd = ''
            {14}{bold}Solana development environment (macOS){reset}
            {9}Optimized for Apple Silicon and Intel Macs{reset}
            
            {13}Available tools:{reset}
              - solana: Solana CLI (${self'.packages.solana-cli.version or "latest"})
              - anchor: Anchor framework (${self'.packages.anchor.version or "latest"})
              - spl-token: SPL Token CLI (included with solana-cli)
              - cargo: Rust toolchain with Solana BPF support
              - llvm/lld/clang: LLVM toolchain for wasm32 compilation
              - Node.js/Yarn: For web3.js development
              - Python: For Solana Python SDK
            
            {13}Quick start:{reset}
              # Setup Solana environment
              solana config set --url localhost
              solana-keygen new --outfile ~/.config/solana/id.json
              
              # Start local validator
              solana-test-validator
              
              # Create new Anchor project
              anchor init my-project
              cd my-project
              anchor build              # Build the program
              anchor test               # Run tests
              anchor deploy             # Deploy to local validator
              
              # SPL Token operations
              spl-token create-token
              spl-token create-account <TOKEN_MINT>
              spl-token mint <TOKEN_MINT> <AMOUNT>
              
            {13}Environment variables:{reset}
              - SOLANA_RPC_URL: $SOLANA_RPC_URL
              - SOLANA_WS_URL: $SOLANA_WS_URL
              - ANCHOR_PROVIDER_URL: $ANCHOR_PROVIDER_URL
              - ANCHOR_WALLET: $ANCHOR_WALLET
              - RUST_BACKTRACE: $RUST_BACKTRACE
              - SOLANA_LOG_LEVEL: $SOLANA_LOG_LEVEL
              - MACOSX_DEPLOYMENT_TARGET: $MACOSX_DEPLOYMENT_TARGET
          '';
        };
      };
    };
}
