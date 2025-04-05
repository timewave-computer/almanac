# This file defines a Nix flake for a CosmWasm development environment.
{
  description = "Almanac Project Root (using CosmWasm module)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # REMOVED flake-utils
    flake-parts.url = "github:hercules-ci/flake-parts";
    
    # Inputs required by ./nix/cosmos-module.nix
    wasmd-src = {
      url = "github:CosmWasm/wasmd";
      flake = false;
    };
    # cosmos-nix = { url = "github:informalsystems/cosmos.nix"; };
    
    # REMOVED cosmwasm-dev input
  };

  outputs = inputs@{ self, nixpkgs, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      # Import the module definition
      imports = [
        ./nix/cosmos-module.nix
      ];
      
      systems = ["aarch64-darwin" "x86_64-linux"]; # Specify systems for flake-parts

      # Define perSystem configuration (devShell merged from module)
      perSystem = { config, pkgs, ... }:
      {
        # Define the main dev shell
        # Packages from the module (like wasmd, jq) are automatically included
        devShells.default = pkgs.mkShell {
          # Add packages defined directly in the root flake
          packages = [ 
            pkgs.git 
          ] ++ config.devShells.packages; # Include packages from imported module(s)
          
          shellHook = ''echo "=== Almanac Root Shell (Module) ==="'';
        };

        # Expose packages defined in the module (like wasmd)
        packages = config.packages;
      };
    };
}