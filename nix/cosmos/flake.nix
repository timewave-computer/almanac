{
  description = "Cosmos development environment with neutron for Darwin and Linux";

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

      perSystem = { config, self', inputs', pkgs, system, lib, ... }: {
        packages = let
          # Build libwasmvm for Darwin following the zero.nix approach
          libwasmvm = pkgs.rustPlatform.buildRustPackage rec {
            pname = "libwasmvm";
            version = "2.1.5";

            src = pkgs.fetchFromGitHub {
              owner = "CosmWasm";
              repo = "wasmvm";
              rev = "v${version}";
              hash = "sha256-qxrAYLUsVSLBZ1GqS5/Yq+K1ZCLqBSq3M0ws1mmWMqs=";
            };

            sourceRoot = "${src.name}/libwasmvm";
            
            # Use the newer fetch method for git dependencies
            useFetchCargoVendor = true;
            cargoHash = "sha256-Q7Enw2pwZB7kzdf/tde/Q/dZhiDCUp19YWd2UY0Hq8w=";
            
            # Override cargo build to ensure cdylib is built
            buildPhase = ''
              runHook preBuild
              
              # Build both rlib and cdylib as specified in Cargo.toml
              cargo build --release
              
              runHook postBuild
            '';
            
            # Disable the automatic cargo install
            dontCargoInstall = true;

            nativeBuildInputs = lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.cctools
            ];

            buildInputs = lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.Security
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
              pkgs.libiconv
            ];

            # Set macOS deployment target for compatibility
            preBuild = lib.optionalString pkgs.stdenv.isDarwin ''
              export MACOSX_DEPLOYMENT_TARGET="11.0"
            '';

            # Use a simple, clean install phase
            installPhase = ''
              runHook preInstall
              
              # Create output directories
              mkdir -p $out/lib $out/include
              
              # Install the library file  
              EXT="${if pkgs.stdenv.isDarwin then "dylib" else "so"}"
              cp target/release/libwasmvm.$EXT $out/lib/
              
              # Create a simple pkg-config file for the library
              mkdir -p $out/lib/pkgconfig
              cat > $out/lib/pkgconfig/wasmvm.pc << EOF
              prefix=$out
              exec_prefix=\''${prefix}
              libdir=\''${exec_prefix}/lib
              includedir=\''${prefix}/include

              Name: wasmvm
              Description: WebAssembly VM for Cosmos chains
              Version: ${version}
              Libs: -L\''${libdir} -lwasmvm
              Cflags: -I\''${includedir}
              EOF
              
              runHook postInstall
            '';

            doCheck = false;

            meta = {
              description = "WebAssembly VM for Cosmos chains";
              homepage = "https://github.com/CosmWasm/wasmvm";
              license = lib.licenses.asl20;
              platforms = lib.platforms.unix;
            };
          };

          # Build neutrond with Darwin support (from zero.nix)
          neutrond = pkgs.buildGoModule rec {
            pname = "neutrond";
            version = "6.0.3";

            src = pkgs.fetchFromGitHub {
              owner = "neutron-org";
              repo = "neutron";
              rev = "v${version}";
              hash = "sha256-kduM7WzJLGmw4Yg450UQZvv3s86jc3VXmion/CAdg+Y=";
            };

            vendorHash = "sha256-wJvvogrVr+rpdZkkHnqwwRlbbuTrlJhKwQ0e71eYXJc=";

            # Enable CGO for libwasmvm
            env.CGO_ENABLED = "1";

            # Set build tags
            tags = [ "netgo" ];

            # Only build the main binary, not test packages
            subPackages = [ "cmd/neutrond" ];

            # Darwin-specific build inputs
            nativeBuildInputs = [ pkgs.makeWrapper ] ++ lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.cctools
            ];

            buildInputs = [ libwasmvm ] ++ lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.Security
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
              pkgs.darwin.apple_sdk.frameworks.CoreFoundation
              pkgs.darwin.apple_sdk.frameworks.CoreServices
              pkgs.libiconv
            ];

            # Set environment variables for build
            preBuild = ''
              export CGO_CFLAGS="-I${libwasmvm}/include"
              export CGO_LDFLAGS="-L${libwasmvm}/lib -lwasmvm"
              ${lib.optionalString pkgs.stdenv.isDarwin ''
                export MACOSX_DEPLOYMENT_TARGET="11.0"
              ''}
            '';

            # Fix dynamic library paths on Darwin
            postInstall = lib.optionalString pkgs.stdenv.isDarwin ''
              install_name_tool -add_rpath ${libwasmvm}/lib $out/bin/neutrond
              install_name_tool -change @rpath/libwasmvm.dylib ${libwasmvm}/lib/libwasmvm.dylib $out/bin/neutrond
            '';

            # Create wrapper with proper library paths
            postFixup = ''
              wrapProgram $out/bin/neutrond \
                --prefix ${if pkgs.stdenv.isDarwin then "DYLD_LIBRARY_PATH" else "LD_LIBRARY_PATH"} : "${libwasmvm}/lib"
            '';

            doCheck = false;

            meta = {
              description = "The most secure permissionless smart contract platform";
              homepage = "https://neutron.org";
              license = lib.licenses.asl20;
              platforms = lib.platforms.unix;
            };
          };

          # Create a smart wrapper following the zero.nix pattern
          neutronWrapper = pkgs.writeShellScriptBin "neutron" ''
            # Function to check if a command exists
            command_exists() {
              command -v "$1" >/dev/null 2>&1
            }
            
            # First, try the Nix-built neutrond
            if [ -x "${neutrond}/bin/neutrond" ]; then
              export ${if pkgs.stdenv.isDarwin then "DYLD_LIBRARY_PATH" else "LD_LIBRARY_PATH"}="${libwasmvm}/lib:''${${if pkgs.stdenv.isDarwin then "DYLD_LIBRARY_PATH" else "LD_LIBRARY_PATH"}:+:''${${if pkgs.stdenv.isDarwin then "DYLD_LIBRARY_PATH" else "LD_LIBRARY_PATH"}}}"
              exec "${neutrond}/bin/neutrond" "$@"
            # Fall back to system neutrond if available
            elif command_exists neutrond; then
              echo "Warning: Using system neutrond. Some features may not work as expected." >&2
              exec neutrond "$@"
            else
              echo "Error: neutrond not found. Please install neutron or ensure it's in your PATH." >&2
              exit 1
            fi
          '';

        in {
          inherit libwasmvm neutrond neutronWrapper;
          
          # Export the main wrapper as the default neutron package
          neutron = neutronWrapper;
        };

        devshells.default = {
          name = "cosmos-development";
          
          commands = [
            {
              package = self'.packages.neutron;
              help = "Neutron blockchain node";
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
            {14}{bold}Cosmos development environment{reset}
            {9}Neutron v6.0.3 with Darwin and Linux support{reset}
            
            {13}Quick start:{reset}
              neutron init mynode --chain-id neutron-1
              neutron start
          '';
        };
      };
    };
}