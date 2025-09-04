{
  description = "Cosmos SDK development environment for NixOS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    cosmos-nix = {
      url = "github:informalsystems/cosmos.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
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

      systems = [ "x86_64-linux" "aarch64-linux" ];

      perSystem = { config, self', inputs', pkgs, system, ... }: 
      let
        # Get cosmos packages from cosmos-nix
        cosmos = inputs'.cosmos-nix.packages;
        
        # Wrapper script for neutrond with test configuration
        neutron-test-wrapper = pkgs.writeShellApplication {
          name = "neutron-test";
          runtimeInputs = [ cosmos.neutron ];
          text = ''
            echo "Starting Neutron test node..."
            
            # Setup test directory
            TEST_DIR="''${TEST_DIR:-$PWD/neutron-test}"
            mkdir -p "$TEST_DIR"
            cd "$TEST_DIR"
            
            # Initialize if needed
            if [ ! -f config/genesis.json ]; then
              echo "Initializing Neutron test chain..."
              neutrond init test --chain-id test-chain --home .
              
              # Add test account
              neutrond keys add test --keyring-backend test --home .
              neutrond genesis add-genesis-account test 1000000000stake --keyring-backend test --home .
              neutrond genesis gentx test 100000000stake --keyring-backend test --chain-id test-chain --home .
              neutrond genesis collect-gentxs --home .
            fi
            
            # Start the node
            exec neutrond start --home . "$@"
          '';
        };

        # Wrapper for gaia (Cosmos Hub)
        gaia-test-wrapper = pkgs.writeShellApplication {
          name = "gaia-test";
          runtimeInputs = [ cosmos.gaia ];
          text = ''
            echo "Starting Gaia test node..."
            
            TEST_DIR="''${TEST_DIR:-$PWD/gaia-test}"
            mkdir -p "$TEST_DIR"
            cd "$TEST_DIR"
            
            if [ ! -f config/genesis.json ]; then
              echo "Initializing Gaia test chain..."
              gaiad init test --chain-id test-gaia --home .
              gaiad keys add test --keyring-backend test --home .
              gaiad genesis add-genesis-account test 1000000000uatom --keyring-backend test --home .
              gaiad genesis gentx test 100000000uatom --keyring-backend test --chain-id test-gaia --home .
              gaiad genesis collect-gentxs --home .
            fi
            
            exec gaiad start --home . "$@"
          '';
        };
        
        # Create wasmd wrapper from git for compatibility
        wasmd-patched = pkgs.buildGoModule rec {
          pname = "wasmd";
          version = "0.53.0";
          
          src = pkgs.fetchFromGitHub {
            owner = "CosmWasm";
            repo = "wasmd";
            rev = "v${version}";
            sha256 = "sha256-bz33oNM6sPLwkJFuSl3VnE1dJzdFgvr3JZW7K4aliU0=";
          };
          
          vendorHash = "sha256-IqmUYOHQvoFdVTB8GV9/kSRfJgunNLi/A5aAKL0J3ZU=";
          
          preBuild = ''
            export GOWORK=off
          '';
          
          ldflags = [
            "-s"
            "-w"
            "-X github.com/cosmos/cosmos-sdk/version.Name=wasmd"
            "-X github.com/cosmos/cosmos-sdk/version.AppName=wasmd"
            "-X github.com/cosmos/cosmos-sdk/version.Version=${version}"
            "-X github.com/cosmos/cosmos-sdk/version.Commit=nixpkgs"
          ];
          
          doCheck = false;
          
          meta = with pkgs.lib; {
            description = "CosmWasm smart contract enabled Cosmos SDK blockchain daemon";
            homepage = "https://github.com/CosmWasm/wasmd";
            license = licenses.asl20;
          };
        };

        # Test wrapper for wasmd
        wasmd-test-wrapper = pkgs.writeShellApplication {
          name = "wasmd-test";
          runtimeInputs = [ wasmd-patched ];
          text = ''
            echo "Starting wasmd test node..."
            
            TEST_DIR="''${TEST_DIR:-$PWD/wasmd-test}"
            mkdir -p "$TEST_DIR"
            cd "$TEST_DIR"
            
            if [ ! -f config/genesis.json ]; then
              echo "Initializing wasmd test chain..."
              wasmd init test --chain-id test-wasmd --home .
              wasmd keys add test --keyring-backend test --home .
              wasmd genesis add-genesis-account test 1000000000stake --keyring-backend test --home .
              wasmd genesis gentx test 100000000stake --keyring-backend test --chain-id test-wasmd --home .
              wasmd genesis collect-gentxs --home .
            fi
            
            exec wasmd start --home . "$@"
          '';
        };
        
        # All-in-one test runner
        cosmos-test-all = pkgs.writeShellApplication {
          name = "cosmos-test-all";
          runtimeInputs = [ pkgs.tmux ];
          text = ''
            echo "Starting all Cosmos test nodes in tmux..."
            
            # Create tmux session
            tmux new-session -d -s cosmos-test
            
            # Neutron
            tmux send-keys -t cosmos-test:0 "${neutron-test-wrapper}/bin/neutron-test" C-m
            
            # Gaia
            tmux new-window -t cosmos-test:1 -n gaia
            tmux send-keys -t cosmos-test:1 "${gaia-test-wrapper}/bin/gaia-test" C-m
            
            # Wasmd
            tmux new-window -t cosmos-test:2 -n wasmd
            tmux send-keys -t cosmos-test:2 "${wasmd-test-wrapper}/bin/wasmd-test" C-m
            
            echo "All nodes started in tmux session 'cosmos-test'"
            echo "Attach with: tmux attach -t cosmos-test"
          '';
        };
        
      in {
        packages = {
          # Core cosmos binaries from cosmos-nix
          inherit (cosmos) 
            gaia 
            neutron 
            ibc-go 
            cosmos-sdk 
            osmosis 
            juno
            evmos
            regen
            agoric
            umee;
          
          # Our custom packages
          wasmd = wasmd-patched;
          
          # Test wrappers
          neutron-test = neutron-test-wrapper;
          gaia-test = gaia-test-wrapper;
          wasmd-test = wasmd-test-wrapper;
          cosmos-test-all = cosmos-test-all;
          
          # Default package
          default = cosmos.gaia;
        };

        devshells.default = {
          name = "cosmos";
          
          commands = [
            { package = self'.packages.gaia; help = "Cosmos Hub (Gaia) blockchain"; }
            { package = self'.packages.neutron; help = "Neutron CosmWasm chain"; }
            { package = self'.packages.wasmd; help = "CosmWasm enabled chain"; }
            { package = self'.packages.ibc-go; help = "IBC relayer"; }
            { package = self'.packages.osmosis; help = "Osmosis DEX chain"; }
            { package = self'.packages.neutron-test; help = "Start Neutron test node"; category = "test"; }
            { package = self'.packages.gaia-test; help = "Start Gaia test node"; category = "test"; }
            { package = self'.packages.wasmd-test; help = "Start wasmd test node"; category = "test"; }
            { package = self'.packages.cosmos-test-all; help = "Start all test nodes in tmux"; category = "test"; }
            { package = pkgs.go_1_22; help = "Go programming language"; category = "development"; }
            { package = pkgs.jq; help = "JSON processor"; category = "tools"; }
            { package = pkgs.tmux; help = "Terminal multiplexer"; category = "tools"; }
          ];
          
          env = [
            { name = "GOPATH"; eval = "$PRJ_ROOT/.go"; }
            { name = "GOPROXY"; value = "https://proxy.golang.org,direct"; }
            { name = "PATH"; prefix = "$PRJ_ROOT/.go/bin"; }
          ];
          
          motd = ''
            {14}Cosmos SDK Development Environment{reset}
            {2}Available chains: gaia, neutron, wasmd, osmosis, juno, evmos{reset}
            
            {6}Quick start:{reset}
              # Start a test node
              neutron-test
              gaia-test
              wasmd-test
              
              # Start all test nodes
              cosmos-test-all
              
              # Check versions
              gaiad version
              neutrond version
              wasmd version
            
            {4}Type 'menu' to see all available commands{reset}
          '';
        };
      };
    };
}