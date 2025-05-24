# Direct shell.nix without requiring flake-compat
{ pkgs ? import <nixpkgs> {} }:

let
  # Import the flake to get access to its outputs
  flake = builtins.getFlake (toString ./.);
  
  # Get the default devShell for the current system
  devShell = flake.devShells.${builtins.currentSystem}.default;
  
  # Create a wrapped shell with explicit environment variables
  wrappedShell = pkgs.mkShell {
    # Inherit packages from the flake's devShell
    inputsFrom = [ devShell ];
    
    # Add explicit packages needed for building
    packages = with pkgs; [
      rustc
      cargo
      pkg-config
      openssl
      libiconv
      postgresql_15
      sqlx-cli
    ];
    
    # Explicitly set MACOSX_DEPLOYMENT_TARGET
    shellHook = ''
      # Set macOS deployment target explicitly
      export MACOSX_DEPLOYMENT_TARGET="11.0"
      
      # Then run the original shellHook
      ${devShell.shellHook}
      
      # Make sure the variable is set and not empty
      echo "MACOSX_DEPLOYMENT_TARGET is set to: $MACOSX_DEPLOYMENT_TARGET"
    '';
  };
in wrappedShell 