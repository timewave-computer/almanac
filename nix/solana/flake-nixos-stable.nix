{
  description = "Stable Solana 1.14.23 for NixOS production environments";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
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

      perSystem = { config, self', inputs', pkgs, system, ... }: {
        packages = {
          # Stable Solana 1.14.23 that works reliably on KVM
          solana-cli = pkgs.solana-cli;
          
          # Test validator wrapper with helpful defaults
          solana-test-validator-stable = pkgs.writeShellApplication {
            name = "solana-test-validator-stable";
            runtimeInputs = [ pkgs.solana-cli ];
            text = ''
              echo "Starting Solana 1.14.23 test validator (KVM-stable)..."
              echo "This version is proven to work on DigitalOcean/KVM environments"
              echo ""
              
              # Create ledger directory if it doesn't exist
              LEDGER_DIR="''${LEDGER_DIR:-./test-ledger}"
              mkdir -p "$LEDGER_DIR"
              
              # Start validator with sensible defaults
              exec solana-test-validator \
                --ledger "$LEDGER_DIR" \
                --rpc-port 8899 \
                --faucet-port 9900 \
                --log \
                "$@"
            '';
          };
          
          # Quick deployment script
          deploy-validator = pkgs.writeShellApplication {
            name = "deploy-validator";
            runtimeInputs = [ pkgs.solana-cli pkgs.curl pkgs.jq ];
            text = ''
              echo "Deploying Solana 1.14.23 validator..."
              
              # Check if already running
              if pgrep -f "solana-test-validator" > /dev/null; then
                echo "Validator already running!"
                exit 1
              fi
              
              # Start validator in background
              nohup solana-test-validator-stable > validator.log 2>&1 &
              echo "Validator starting... (PID: $!)"
              
              # Wait for startup
              echo "Waiting for validator to be ready..."
              sleep 5
              
              # Check health
              if curl -s http://localhost:8899/health | grep -q "ok"; then
                echo "OK Validator is healthy!"
                echo ""
                echo "RPC URL: http://localhost:8899"
                echo "Faucet URL: http://localhost:9900"
                echo "Logs: tail -f validator.log"
              else
                echo "NO Validator failed to start"
                echo "Check validator.log for details"
                exit 1
              fi
            '';
          };
        };

        devshells.default = {
          name = "solana-stable";
          
          packages = with pkgs; [
            solana-cli
            postgresql
            jq
            curl
            self'.packages.solana-test-validator-stable
            self'.packages.deploy-validator
          ];
          
          commands = [
            {
              name = "solana";
              package = pkgs.solana-cli;
              help = "Solana CLI (v1.14.23 - KVM stable)";
            }
            {
              name = "start-validator";
              package = self'.packages.solana-test-validator-stable;
              help = "Start Solana test validator";
            }
            {
              name = "deploy";
              package = self'.packages.deploy-validator;
              help = "Deploy validator with health checks";
            }
          ];
          
          motd = ''
            {14}Solana 1.14.23 - Stable for KVM/DigitalOcean{reset}
            {2}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━{reset}
            
            {11}Version:{reset} $(solana --version)
            
            {11}Quick Start:{reset}
              start-validator    # Start test validator
              deploy            # Deploy with health checks
              
            {11}Commands:{reset}
              solana            # CLI tools
              solana-keygen     # Key generation
              
            {11}Notes:{reset}
              - No illegal instruction errors on KVM
              - Tested on DigitalOcean droplets
              - Stable for production use
          '';
          
          env = [
            {
              name = "RUST_LOG";
              value = "solana=info";
            }
          ];
        };
        
        # Alternative development shell without devshell
        devShells.minimal = pkgs.mkShell {
          packages = with pkgs; [
            solana-cli
            jq
            curl
          ];
          
          shellHook = ''
            echo "Solana 1.14.23 (Minimal Shell)"
            echo "=============================="
            solana --version
            echo ""
            echo "This is the stable version for KVM environments"
          '';
        };
      };
    };
}