# This module adds CosmWasm development tools to our flake
{ self, inputs, ... }:
{
  flake = {
    # Add overlay
    overlays.default = final: prev: {
      # Add our cosmos packages
      almanac-cosmos = self.packages.${prev.system};
    };
  };

  # Define per-system outputs
  perSystem = { config, self', inputs', pkgs, system, ... }: {
    # Define packages for this system
    packages = {
      # Install wasmd via Go
      wasmd-setup = pkgs.stdenv.mkDerivation {
        name = "wasmd-setup";
        src = ./scripts;
        
        buildInputs = with pkgs; [ go cacert jq curl git ];
        nativeBuildInputs = [ pkgs.makeWrapper ];
        
        installPhase = ''
          mkdir -p $out/bin
          cp wasmd-setup/wasmd-setup.sh $out/bin/wasmd-setup
          cp wasmd-setup/wasmd-dummy.sh $out/bin/wasmd-dummy
          chmod +x $out/bin/wasmd-setup
          chmod +x $out/bin/wasmd-dummy
          
          wrapProgram $out/bin/wasmd-setup \
            --prefix PATH : ${pkgs.lib.makeBinPath (with pkgs; [ go cacert jq curl git ])}
        '';
      };
      
      # Run a wasmd test node
      wasmd-node = pkgs.stdenv.mkDerivation {
        name = "wasmd-node";
        src = ./scripts;
        
        buildInputs = with pkgs; [ jq procps ];
        nativeBuildInputs = [ pkgs.makeWrapper ];
        
        installPhase = ''
          mkdir -p $out/bin
          cp wasmd-node/wasmd-node.sh $out/bin/wasmd-node
          chmod +x $out/bin/wasmd-node
          
          wrapProgram $out/bin/wasmd-node \
            --prefix PATH : ${pkgs.lib.makeBinPath (with pkgs; [ jq procps ])}
        '';
      };
      
      # Run cosmos adapter tests against local node
      test-cosmos-adapter = pkgs.stdenv.mkDerivation {
        name = "test-cosmos-adapter";
        src = ./scripts;
        
        buildInputs = with pkgs; [ jq procps curl cargo rustc pkg-config ];
        nativeBuildInputs = [ pkgs.makeWrapper ];
        
        installPhase = ''
          mkdir -p $out/bin
          cp test-cosmos-adapter/test-cosmos-adapter.sh $out/bin/test-cosmos-adapter
          chmod +x $out/bin/test-cosmos-adapter
          
          wrapProgram $out/bin/test-cosmos-adapter \
            --prefix PATH : ${pkgs.lib.makeBinPath (with pkgs; [ jq procps curl cargo rustc pkg-config ])}
        '';
      };
    };
  };
}
