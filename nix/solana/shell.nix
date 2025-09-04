# Shell.nix wrapper that selects the appropriate flake based on platform
let
  pkgs = import <nixpkgs> {};
  system = builtins.currentSystem;
  
  # Select flake based on system
  flakeFile = if pkgs.stdenv.isLinux then
    ./flake-nixos.nix
  else if pkgs.stdenv.isDarwin then
    ./flake-macos.nix
  else
    throw "Unsupported system: ${system}";
    
  # Import and evaluate the flake
  flake = import flakeFile;
  
  # Create dummy inputs for the flake
  inputs = {
    self = null;
    nixpkgs = pkgs.path;
    flake-parts = builtins.fetchTarball {
      url = "https://github.com/hercules-ci/flake-parts/archive/main.tar.gz";
    };
    devshell = builtins.fetchTarball {
      url = "https://github.com/numtide/devshell/archive/main.tar.gz";
    };
    rust-overlay = if pkgs.stdenv.isLinux then builtins.fetchTarball {
      url = "https://github.com/oxalica/rust-overlay/archive/master.tar.gz";
    } else null;
  };
  
  # Get the outputs
  outputs = flake.outputs inputs;
  
  # Get the default devshell for current system
  devShell = outputs.devShells.${system}.default or null;
  
in
  # Return the devshell or a basic shell
  if devShell != null then devShell else pkgs.mkShell {
    buildInputs = with pkgs; [ 
      solana-cli
      anchor
    ];
  }