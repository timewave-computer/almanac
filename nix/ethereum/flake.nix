{
  description = "Ethereum development environment with geth, lighthouse, and forge";

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

      perSystem = { config, self', inputs', pkgs, system, ... }: {
        packages = {
          # Geth - Ethereum execution client (from zero.nix)
          geth = pkgs.buildGoModule rec {
            pname = "geth";
            version = "1.15.6";
            
            src = pkgs.fetchFromGitHub {
              owner = "ethereum";
              repo = "go-ethereum";
              rev = "v${version}";
              hash = "sha256-BdNv0rx+9/F0leNj2AAej8psy8X8HysDrIXheVOOkSo=";
            };
            
            vendorHash = "sha256-KRVI1DxjoABZFJkmjGaMVlmxIHvtSFuvmpuMuvr8Pws=";
            
            doCheck = false;
            
            subPackages = [ "cmd/geth" ];
            
            # Build configuration
            env.CGO_ENABLED = if pkgs.stdenv.isLinux then "1" else "0";
            buildFlags = [ "-mod=readonly" ];
            
            # Add necessary inputs for CGO on Linux
            buildInputs = with pkgs; lib.optionals stdenv.isLinux [
              glibc
              gcc
            ];
            
            nativeBuildInputs = with pkgs; lib.optionals stdenv.isLinux [
              pkg-config
            ];
            
            meta = with pkgs.lib; {
              description = "Official Go implementation of the Ethereum protocol";
              homepage = "https://github.com/ethereum/go-ethereum";
              license = licenses.lgpl3Plus;
              mainProgram = "geth";
            };
          };
          
          # Lighthouse - Ethereum consensus client (from zero.nix)
          lighthouse = pkgs.rustPlatform.buildRustPackage rec {
            pname = "lighthouse";
            version = "5.3.0";
            
            src = pkgs.fetchFromGitHub {
              owner = "sigp";
              repo = "lighthouse";
              rev = "v${version}";
              hash = "sha256-wIj+YabyUrgLjWCfjCAH/Xb8jUG6ss+5SwnE2M82a+4=";
            };
            
            cargoHash = "sha256-v/gOTbkzcwmqV8XCzkLzAl6LyshVBWxUclZxx1mr53o=";
            useFetchCargoVendor = true;
            
            buildInputs = with pkgs; [
              openssl
            ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.Security
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            ];
            
            nativeBuildInputs = with pkgs; [
              pkg-config
              protobuf
              cmake
            ];
            
            # Disable tests that require network access
            doCheck = false;
            
            # Build only the lighthouse binary
            cargoBuildFlags = [ "--bin" "lighthouse" ];
            
            meta = with pkgs.lib; {
              description = "Ethereum consensus client written in Rust";
              homepage = "https://github.com/sigp/lighthouse";
              license = licenses.asl20;
              mainProgram = "lighthouse";
            };
          };

          # Forge from Foundry toolchain
          forge = pkgs.foundry-bin;
        };

        devshells.default = {
          name = "ethereum-development";
          
          commands = [
            {
              package = self'.packages.geth;
              help = "Ethereum execution client";
            }
            {
              package = self'.packages.lighthouse;
              help = "Ethereum consensus client";
            }
            {
              package = self'.packages.forge;
              help = "Ethereum development framework";
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
              package = pkgs.openssl;
              help = "SSL/TLS toolkit";
            }
          ];
          
          motd = ''
            {14}{bold}Ethereum development environment{reset}
            {9}Network: sepolia (default){reset}
            
            {13}Quick start:{reset}
              geth --sepolia --datadir ./data/geth --http --ws
              lighthouse bn --network sepolia --datadir ./data/lighthouse
              forge init my-project
          '';
        };
      };
    };
}