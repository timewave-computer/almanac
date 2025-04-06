{
  config,
  lib,
  pkgs,
  ...
}:

with lib;

let
  cfg = config.ethereum;
  
  # Define packages
  packages = {
    # Start anvil node
    start-anvil = pkgs.writeShellScriptBin "start-anvil" ''
      ${pkgs.foundry}/bin/anvil --host 0.0.0.0
    '';
    
    # Deploy contracts
    deploy-contract = pkgs.writeShellScriptBin "deploy-contract" ''
      # Set up environment
      export RPC_URL="${cfg.rpcUrl}"
      export PRIVATE_KEY="${cfg.privateKey}"
      
      # Create deployment directory
      DEPLOYMENT_DIR="./deployments"
      mkdir -p $DEPLOYMENT_DIR
      
      # Deploy Faucet contract as default
      echo "Deploying Faucet contract..."
      ${pkgs.foundry}/bin/forge create --rpc-url "$RPC_URL" \
        --private-key "$PRIVATE_KEY" \
        ${lib.optionalString (cfg.gasPrice != null) "--gas-price ${cfg.gasPrice}"} \
        ${lib.optionalString (cfg.gasLimit != null) "--gas-limit ${toString cfg.gasLimit}"} \
        contracts/solidity/Faucet.sol:Faucet \
        --json > $DEPLOYMENT_DIR/Faucet.json
      
      CONTRACT_ADDRESS=$(${pkgs.jq}/bin/jq -r '.deployedTo' $DEPLOYMENT_DIR/Faucet.json)
      echo "Faucet deployed to: $CONTRACT_ADDRESS"
      echo $CONTRACT_ADDRESS > $DEPLOYMENT_DIR/Faucet_address.txt
    '';
    
    # Mint tokens
    mint-tokens = pkgs.writeShellScriptBin "mint-tokens" ''
      # Check if enough arguments are provided
      if [ $# -lt 2 ]; then
        echo "Usage: $0 <address> <amount>"
        echo "Example: $0 0x1234... 100"
        exit 1
      fi
      
      RECIPIENT_ADDRESS="$1"
      AMOUNT="$2"
      
      # Set up environment
      export RPC_URL="${cfg.rpcUrl}"
      export PRIVATE_KEY="${cfg.privateKey}"
      
      # Get the contract address
      DEPLOYMENT_DIR="./deployments"
      if [ ! -f "$DEPLOYMENT_DIR/Faucet_address.txt" ]; then
        echo "Error: Faucet contract not deployed yet"
        echo "Please run 'nix run .#deploy-contract' first"
        exit 1
      fi
      
      CONTRACT_ADDRESS=$(cat "$DEPLOYMENT_DIR/Faucet_address.txt")
      
      # Mint tokens
      echo "Minting $AMOUNT tokens to $RECIPIENT_ADDRESS..."
      ${pkgs.foundry}/bin/cast send \
        --rpc-url "$RPC_URL" \
        --private-key "$PRIVATE_KEY" \
        $CONTRACT_ADDRESS \
        "mint(address,uint256)" \
        $RECIPIENT_ADDRESS \
        $(${pkgs.foundry}/bin/cast --to-wei "$AMOUNT" eth)
      
      echo "Tokens minted successfully"
      
      # Get balance
      echo "Checking balance..."
      BALANCE_WEI=$(${pkgs.foundry}/bin/cast call \
        --rpc-url "$RPC_URL" \
        $CONTRACT_ADDRESS \
        "balanceOf(address)(uint256)" \
        $RECIPIENT_ADDRESS)
      
      BALANCE_ETH=$(${pkgs.foundry}/bin/cast --from-wei "$BALANCE_WEI" eth)
      echo "Current balance of $RECIPIENT_ADDRESS: $BALANCE_ETH FCT"
    '';
    
    # End-to-end test
    e2e-test = pkgs.writeShellScriptBin "e2e-test" ''
      # Set up environment
      export RPC_URL="${cfg.rpcUrl}"
      export PRIVATE_KEY="${cfg.privateKey}"
      
      # Create a temporary copy of the script that we can make executable
      TEMP_SCRIPT=$(mktemp)
      cp ${../scripts/e2e-test.sh} $TEMP_SCRIPT
      chmod +x $TEMP_SCRIPT
      
      # Run the script
      $TEMP_SCRIPT
      EXIT_CODE=$?
      
      # Clean up
      rm -f $TEMP_SCRIPT
      
      exit $EXIT_CODE
    '';
  };
  
  # Define apps
  apps = {
    start-anvil = {
      type = "app";
      program = "${packages.start-anvil}/bin/start-anvil";
    };
    
    deploy-contract = {
      type = "app";
      program = "${packages.deploy-contract}/bin/deploy-contract";
    };
    
    mint-tokens = {
      type = "app";
      program = "${packages.mint-tokens}/bin/mint-tokens";
    };
    
    e2e-test = {
      type = "app";
      program = "${packages.e2e-test}/bin/e2e-test";
    };
  };
in
{
  options.ethereum = {
    rpcUrl = mkOption {
      type = types.str;
      description = "RPC URL for the Ethereum node";
      example = "http://localhost:8545";
    };
    
    privateKey = mkOption {
      type = types.str;
      description = "Private key for transaction signing";
    };
    
    gasPrice = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Gas price for transactions (null for auto)";
    };
    
    gasLimit = mkOption {
      type = types.nullOr types.int;
      default = null;
      description = "Gas limit for transactions (null for auto)";
    };

    contracts = mkOption {
      type = types.attrsOf (types.submodule {
        options = {
          path = mkOption {
            type = types.str;
            description = "Path to the contract source file and contract name";
          };
          
          constructorArgs = mkOption {
            type = types.listOf types.str;
            default = [];
            description = "Constructor arguments for contract deployment";
          };
          
          verifyContract = mkOption {
            type = types.bool;
            default = false;
            description = "Whether to verify the contract on Etherscan";
          };
          
          etherscanApiKey = mkOption {
            type = types.nullOr types.str;
            default = null;
            description = "Etherscan API key for contract verification";
          };
        };
      });
      default = {};
      description = "Ethereum contracts to deploy";
    };
  };
  
  config = mkIf (config ? ethereum) {
    inherit packages apps;
  };
} 