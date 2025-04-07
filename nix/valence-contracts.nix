# valence-contracts.nix - Module for building and deploying Valence contracts
{ lib, pkgs, inputs, ... }:

let
  # Common utilities for Valence deployment
  valenceUtils = rec {
    # Create a wasmd config file to avoid duplication
    makeWasmdConfig = { rpcUrl ? "http://localhost:26657"
                     , apiUrl ? "http://localhost:1317"
                     , chainId ? "wasmchain"
                     , keyringBackend ? "test"
                     , nodeHome ? "${builtins.getEnv "HOME"}/.wasmd-test"
                     , broadcastMode ? "block"
                     , gasPrices ? "0.025stake"
                     }:
      pkgs.writeTextFile {
        name = "wasmd-config.json";
        text = builtins.toJSON {
          chain_id = chainId;
          rpc_url = rpcUrl;
          api_url = apiUrl;
          keyring_backend = keyringBackend;
          node_home = nodeHome;
          broadcast_mode = broadcastMode;
          gas_prices = gasPrices;
        };
      };

    # Create an anvil config file
    makeAnvilConfig = { rpcUrl ? "http://localhost:8545"
                      , privateKey ? "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                      , chainId ? "31337"
                      }:
      pkgs.writeTextFile {
        name = "anvil-config.json";
        text = builtins.toJSON {
          rpc_url = rpcUrl;
          private_key = privateKey;
          chain_id = chainId;
        };
      };

    # Generate a deployment script that uses the specified config files
    makeDeployScript = { name, script }:
      pkgs.writeShellScriptBin name ''
        set -e
        ${script}
      '';

    # Function to create a standard contract deployment script
    deployContract = { contractName, initMsg ? "{}", extraCommands ? "" }:
      let
        capitalizedName = lib.strings.toUpper (builtins.substring 0 1 contractName) + 
                          builtins.substring 1 (builtins.stringLength contractName) contractName;
      in ''
        PROJECT_ROOT="$(pwd)"
        WASMD_CONFIG="$PROJECT_ROOT/config/wasmd/config.json"
        VALENCE_DIR="$PROJECT_ROOT/valence-protocol"
        BUILD_DIR="$PROJECT_ROOT/build/valence-contracts"
        
        # Parse wasmd config
        if [ ! -f "$WASMD_CONFIG" ]; then
          echo "wasmd configuration not found at $WASMD_CONFIG"
          echo "Please run 'nix run .#valence-contract-integration'"
          exit 1
        fi
        
        CHAIN_ID=$(jq -r '.chain_id' "$WASMD_CONFIG")
        RPC_URL=$(jq -r '.rpc_url' "$WASMD_CONFIG")
        API_URL=$(jq -r '.api_url' "$WASMD_CONFIG")
        KEYRING_BACKEND=$(jq -r '.keyring_backend' "$WASMD_CONFIG")
        NODE_HOME=$(jq -r '.node_home' "$WASMD_CONFIG")
        BROADCAST_MODE=$(jq -r '.broadcast_mode' "$WASMD_CONFIG")
        GAS_PRICES=$(jq -r '.gas_prices' "$WASMD_CONFIG")
        
        echo "Deploying ${capitalizedName} contract..."
        
        # Check if wasmd node is running
        if ! curl -s "$RPC_URL/status" > /dev/null; then
          echo "wasmd node is not running. Please start it with 'nix run .#wasmd-node'"
          exit 1
        fi
        
        # Get validator address
        VALIDATOR_ADDR=$(wasmd keys show validator -a --keyring-backend="$KEYRING_BACKEND" --home="$NODE_HOME")
        echo "Using validator address: $VALIDATOR_ADDR"
        
        # Deploy the contract
        WASM_FILE="$VALENCE_DIR/target/wasm32-unknown-unknown/release/valence_${contractName}.wasm"
        
        if [ ! -f "$WASM_FILE" ]; then
          echo "Contract WASM file not found at $WASM_FILE"
          echo "Please build Valence contracts first using 'nix run .#build-valence-contracts'"
          exit 1
        fi
        
        echo "Storing contract code on chain..."
        TX_HASH=$(wasmd tx wasm store "$WASM_FILE" \
          --from validator \
          --gas auto --gas-adjustment 1.3 \
          --gas-prices "$GAS_PRICES" \
          --broadcast-mode "$BROADCAST_MODE" \
          --chain-id "$CHAIN_ID" \
          --keyring-backend "$KEYRING_BACKEND" \
          --home "$NODE_HOME" \
          --output json -y | jq -r '.txhash')
        
        echo "Contract uploaded with tx hash: $TX_HASH"
        echo "Waiting for transaction to be included in a block..."
        sleep 6
        
        # Get code ID from transaction result
        CODE_ID=$(wasmd query tx "$TX_HASH" --chain-id "$CHAIN_ID" --node "$RPC_URL" --output json | \
          jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
        
        if [ -z "$CODE_ID" ]; then
          echo "Failed to get code ID for ${capitalizedName} contract"
          exit 1
        fi
        
        echo "${capitalizedName} contract stored with code ID: $CODE_ID"
        mkdir -p "$BUILD_DIR"
        echo "$CODE_ID" > "$BUILD_DIR/${contractName}_code_id.txt"
        
        # Initialize the contract
        INIT_MSG="${initMsg}"
        if [[ "$INIT_MSG" == *"\$VALIDATOR_ADDR"* ]]; then
          INIT_MSG=$(echo "$INIT_MSG" | sed "s|\$VALIDATOR_ADDR|$VALIDATOR_ADDR|g")
        fi
        
        echo "Initializing ${capitalizedName} contract with code ID: $CODE_ID"
        echo "Using init message: $INIT_MSG"
        
        TX_HASH=$(wasmd tx wasm instantiate "$CODE_ID" "$INIT_MSG" \
          --label "Valence ${capitalizedName}" \
          --admin "$VALIDATOR_ADDR" \
          --from validator \
          --gas auto --gas-adjustment 1.3 \
          --gas-prices "$GAS_PRICES" \
          --broadcast-mode "$BROADCAST_MODE" \
          --chain-id "$CHAIN_ID" \
          --keyring-backend "$KEYRING_BACKEND" \
          --home "$NODE_HOME" \
          --output json -y | jq -r '.txhash')
        
        echo "Contract instantiated with tx hash: $TX_HASH"
        echo "Waiting for transaction to be included in a block..."
        sleep 6
        
        # Get contract address from transaction result
        CONTRACT_ADDR=$(wasmd query tx "$TX_HASH" --chain-id "$CHAIN_ID" --node "$RPC_URL" --output json | \
          jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
        
        if [ -z "$CONTRACT_ADDR" ]; then
          echo "Failed to get contract address for ${capitalizedName} contract"
          exit 1
        fi
        
        echo "${capitalizedName} contract instantiated at address: $CONTRACT_ADDR"
        echo "$CONTRACT_ADDR" > "$BUILD_DIR/${contractName}_contract_addr.txt"
        
        # Query the contract state to verify
        echo "Querying contract state to verify initialization..."
        wasmd query wasm contract-state smart "$CONTRACT_ADDR" '{"get_owner":{}}' --node "$RPC_URL" --output json
        
        ${extraCommands}
        
        echo "${capitalizedName} contract deployed and initialized successfully!"
      '';
  };

  # Build contracts script content
  buildContractsScript = ''
    PROJECT_ROOT="$(pwd)"
    VALENCE_DIR="$PROJECT_ROOT/valence-protocol"
    
    if [ ! -d "$VALENCE_DIR" ]; then
      echo "Valence protocol directory not found at $VALENCE_DIR"
      echo "Cloning Valence protocol repository..."
      git clone https://github.com/timewave-computer/valence-protocol.git "$VALENCE_DIR"
    fi
    
    cd "$VALENCE_DIR"
    echo "Building Valence WASM contracts..."
    cargo wasm
    echo "Building Valence Solidity contracts..."
    (cd solidity && forge build)
    cd "$PROJECT_ROOT"
    echo "Valence contracts built successfully!"
  '';

  # Account contract deployment script content
  deployAccountScript = valenceUtils.deployContract {
    contractName = "account";
    initMsg = ''{"owner":"$VALIDATOR_ADDR"}'';
    extraCommands = ''
      # Get contract address
      CONTRACT_ADDR=$(cat "$BUILD_DIR/account_contract_addr.txt")
      
      # Create a test account
      echo "Creating a test account..."
      TEST_ACCOUNT_ID="test-account-1"
      
      EXECUTE_MSG="{\"create_account\":{\"id\":\"$TEST_ACCOUNT_ID\"}}"
      TX_HASH=$(wasmd tx wasm execute "$CONTRACT_ADDR" "$EXECUTE_MSG" \
        --from validator \
        --gas auto --gas-adjustment 1.3 \
        --gas-prices "$GAS_PRICES" \
        --broadcast-mode "$BROADCAST_MODE" \
        --chain-id "$CHAIN_ID" \
        --keyring-backend "$KEYRING_BACKEND" \
        --home "$NODE_HOME" \
        --output json -y | jq -r '.txhash')
      
      echo "Test account created with tx hash: $TX_HASH"
      echo "Waiting for transaction to be included in a block..."
      sleep 6
      
      # Query to verify account creation
      echo "Querying contract state to verify account creation..."
      wasmd query wasm contract-state smart "$CONTRACT_ADDR" "{\"get_account\":{\"id\":\"$TEST_ACCOUNT_ID\"}}" --node "$RPC_URL" --output json
      
      # Save the test account ID
      echo "$TEST_ACCOUNT_ID" > "$BUILD_DIR/test_account_id.txt"
      echo "Test account '$TEST_ACCOUNT_ID' created successfully"
    '';
  };

  # Processor contract deployment script content
  deployProcessorScript = valenceUtils.deployContract {
    contractName = "processor";
    initMsg = ''{"owner":"$VALIDATOR_ADDR","allowed_sources":["ethereum","cosmos"],"allowed_targets":["ethereum","cosmos"]}'';
    extraCommands = ''
      # Get contract address
      CONTRACT_ADDR=$(cat "$BUILD_DIR/processor_contract_addr.txt")
      
      # Register chains
      echo "Registering ethereum chain..."
      EXECUTE_MSG="{\"register_chain\":{\"chain_id\":\"ethereum\",\"config\":{\"verification_threshold\":1,\"verifiers\":[\"$VALIDATOR_ADDR\"]}}}"
      wasmd tx wasm execute "$CONTRACT_ADDR" "$EXECUTE_MSG" \
        --from validator \
        --gas auto --gas-adjustment 1.3 \
        --gas-prices "$GAS_PRICES" \
        --broadcast-mode "$BROADCAST_MODE" \
        --chain-id "$CHAIN_ID" \
        --keyring-backend "$KEYRING_BACKEND" \
        --home "$NODE_HOME" \
        --output json -y > /dev/null
      
      echo "Registering cosmos chain..."
      EXECUTE_MSG="{\"register_chain\":{\"chain_id\":\"cosmos\",\"config\":{\"verification_threshold\":1,\"verifiers\":[\"$VALIDATOR_ADDR\"]}}}"
      wasmd tx wasm execute "$CONTRACT_ADDR" "$EXECUTE_MSG" \
        --from validator \
        --gas auto --gas-adjustment 1.3 \
        --gas-prices "$GAS_PRICES" \
        --broadcast-mode "$BROADCAST_MODE" \
        --chain-id "$CHAIN_ID" \
        --keyring-backend "$KEYRING_BACKEND" \
        --home "$NODE_HOME" \
        --output json -y > /dev/null
      
      # Verify chain registration
      echo "Verifying ethereum chain registration..."
      wasmd query wasm contract-state smart "$CONTRACT_ADDR" '{"get_chain_config":{"chain_id":"ethereum"}}' --node "$RPC_URL" --output json
      
      echo "Verifying cosmos chain registration..."
      wasmd query wasm contract-state smart "$CONTRACT_ADDR" '{"get_chain_config":{"chain_id":"cosmos"}}' --node "$RPC_URL" --output json
      
      # Submit test message
      echo "Submitting test cross-chain message..."
      # Base64 encode a simple message payload
      PAYLOAD=$(echo -n '{"action":"test_action","data":"hello_cosmos"}' | base64)
      EXECUTE_MSG="{\"process_message\":{\"message\":{\"source_chain\":\"ethereum\",\"target_chain\":\"cosmos\",\"sender\":\"0x123...\",\"recipient\":\"cosmos1...\",\"nonce\":1,\"payload\":\"$PAYLOAD\"},\"proof\":{\"signature\":\"abc...\",\"signer\":\"$VALIDATOR_ADDR\"}}}"
      wasmd tx wasm execute "$CONTRACT_ADDR" "$EXECUTE_MSG" \
        --from validator \
        --gas auto --gas-adjustment 1.3 \
        --gas-prices "$GAS_PRICES" \
        --broadcast-mode "$BROADCAST_MODE" \
        --chain-id "$CHAIN_ID" \
        --keyring-backend "$KEYRING_BACKEND" \
        --home "$NODE_HOME" \
        --output json -y > /dev/null
      
      echo "Querying processed messages..."
      sleep 6 # Give time for processing
      wasmd query wasm contract-state smart "$CONTRACT_ADDR" '{"get_processed_messages":{"limit":10}}' --node "$RPC_URL" --output json
    '';
  };

  # Authorization contract deployment script content
  deployAuthorizationScript = valenceUtils.deployContract {
    contractName = "authorization";
    initMsg = ''{"owner":"$VALIDATOR_ADDR"}'';
    extraCommands = ''
      # Get contract address
      CONTRACT_ADDR=$(cat "$BUILD_DIR/authorization_contract_addr.txt")
      
      # Create a test policy
      echo "Creating a test authorization policy..."
      POLICY_ID="test-policy-v1"
      POLICY_CONTENT="{\"rules\":[{\"grantee\":\"cosmos1...\",\"resource\":\"data/*\",\"permissions\":[\"read\"]}]}"
      POLICY_HASH=$(echo -n "$POLICY_CONTENT" | sha256sum | awk '{print $1}')
      
      EXECUTE_MSG="{\"create_policy\":{\"policy_id\":\"$POLICY_ID\",\"content_hash\":\"$POLICY_HASH\",\"activate\":true}}"
      wasmd tx wasm execute "$CONTRACT_ADDR" "$EXECUTE_MSG" \
        --from validator \
        --gas auto --gas-adjustment 1.3 \
        --gas-prices "$GAS_PRICES" \
        --broadcast-mode "$BROADCAST_MODE" \
        --chain-id "$CHAIN_ID" \
        --keyring-backend "$KEYRING_BACKEND" \
        --home "$NODE_HOME" \
        --output json -y > /dev/null
      
      echo "Waiting for policy creation..."
      sleep 6
      
      # Verify policy creation
      echo "Querying contract state to verify policy creation..."
      wasmd query wasm contract-state smart "$CONTRACT_ADDR" "{\"get_policy\":{\"policy_id\":\"$POLICY_ID\"}}" --node "$RPC_URL" --output json
      
      # Grant permission
      echo "Granting test permission..."
      GRANTEE_ADDR="$VALIDATOR_ADDR" # Grant to self for testing
      RESOURCE="data/test_resource"
      PERMISSION="read"
      EXECUTE_MSG="{\"grant\":{\"grantee\":\"$GRANTEE_ADDR\",\"resource\":\"$RESOURCE\",\"permission\":\"$PERMISSION\"}}"
      wasmd tx wasm execute "$CONTRACT_ADDR" "$EXECUTE_MSG" \
        --from validator \
        --gas auto --gas-adjustment 1.3 \
        --gas-prices "$GAS_PRICES" \
        --broadcast-mode "$BROADCAST_MODE" \
        --chain-id "$CHAIN_ID" \
        --keyring-backend "$KEYRING_BACKEND" \
        --home "$NODE_HOME" \
        --output json -y > /dev/null
      
      echo "Waiting for grant..."
      sleep 6
      
      # Verify grant
      echo "Querying contract state to verify grant..."
      wasmd query wasm contract-state smart "$CONTRACT_ADDR" "{\"check_permission\":{\"grantee\":\"$GRANTEE_ADDR\",\"resource\":\"$RESOURCE\",\"permission\":\"$PERMISSION\"}}" --node "$RPC_URL" --output json
    '';
  };

  # Library contract deployment script content
  deployLibraryScript = valenceUtils.deployContract {
    contractName = "library";
    initMsg = ''{"owner":"$VALIDATOR_ADDR","library_type":"test_library"}'';
    extraCommands = ''
      # Get contract address
      CONTRACT_ADDR=$(cat "$BUILD_DIR/library_contract_addr.txt")
      
      # Publish a new version
      echo "Publishing library version 1..."
      VERSION=1
      CODE_HASH=$(echo -n "test_code_v1" | sha256sum | awk '{print $1}')
      FEATURES='["feature_a","feature_b"]';
      
      EXECUTE_MSG="{\"publish_version\":{\"version\":$VERSION,\"code_hash\":\"$CODE_HASH\",\"features\":$FEATURES,\"activate\":true}}"
      wasmd tx wasm execute "$CONTRACT_ADDR" "$EXECUTE_MSG" \
        --from validator \
        --gas auto --gas-adjustment 1.3 \
        --gas-prices "$GAS_PRICES" \
        --broadcast-mode "$BROADCAST_MODE" \
        --chain-id "$CHAIN_ID" \
        --keyring-backend "$KEYRING_BACKEND" \
        --home "$NODE_HOME" \
        --output json -y > /dev/null
      
      echo "Waiting for version publishing..."
      sleep 6
      
      # Verify version publishing
      echo "Querying contract state to verify version publishing..."
      wasmd query wasm contract-state smart "$CONTRACT_ADDR" "{\"get_version\":{\"version\":$VERSION}}" --node "$RPC_URL" --output json
      
      # Record usage
      echo "Recording library usage..."
      ACCOUNT_ADDR=$(cat "$BUILD_DIR/account_contract_addr.txt") # Assuming account contract is deployed
      EXECUTE_MSG="{\"record_usage\":{\"account_address\":\"$ACCOUNT_ADDR\",\"function_name\":\"test_function\",\"success\":true}}"
      wasmd tx wasm execute "$CONTRACT_ADDR" "$EXECUTE_MSG" \
        --from validator \
        --gas auto --gas-adjustment 1.3 \
        --gas-prices "$GAS_PRICES" \
        --broadcast-mode "$BROADCAST_MODE" \
        --chain-id "$CHAIN_ID" \
        --keyring-backend "$KEYRING_BACKEND" \
        --home "$NODE_HOME" \
        --output json -y > /dev/null
      
      echo "Waiting for usage recording..."
      sleep 6
      
      # Verify usage recording
      echo "Querying contract usage history..."
      wasmd query wasm contract-state smart "$CONTRACT_ADDR" '{"get_usage_history":{"limit":10}}' --node "$RPC_URL" --output json
    '';
  };

  # Deploy all contracts script content
  deployAllContractsScript = ''
    echo "Deploying all Valence Cosmos contracts..."
    nix run .#deploy-valence-account
    nix run .#deploy-valence-processor
    nix run .#deploy-valence-authorization
    nix run .#deploy-valence-library
    echo "All Valence Cosmos contracts deployed successfully!"
  '';
  
  # Deploy Ethereum contracts script content (Anvil)
  deployEthereumContractsScript = ''
    PROJECT_ROOT="$(pwd)"
    ANVIL_CONFIG="$PROJECT_ROOT/config/anvil/config.json"
    BUILD_DIR="$PROJECT_ROOT/build/valence-contracts/ethereum"
    ETH_DIR="$PROJECT_ROOT/valence-protocol/solidity"
    
    if [ ! -d "$ETH_DIR" ]; then
      echo "Valence Solidity contracts directory not found at $ETH_DIR"
      echo "Please build Valence contracts first using 'nix run .#build-valence-contracts'"
      exit 1
    fi
    
    if [ ! -f "$ANVIL_CONFIG" ]; then
      echo "Anvil config not found at $ANVIL_CONFIG"
      echo "Please run 'nix run .#valence-contract-integration' first to generate config"
      exit 1
    fi
    
    RPC_URL=$(jq -r '.rpc_url' "$ANVIL_CONFIG")
    PRIVATE_KEY=$(jq -r '.private_key' "$ANVIL_CONFIG")
    CHAIN_ID=$(jq -r '.chain_id' "$ANVIL_CONFIG")
    
    echo "Deploying Ethereum contracts to Anvil ($RPC_URL)..."
    
    # Ensure build directory exists
    mkdir -p "$BUILD_DIR"
    DEPLOYMENT_INFO="$BUILD_DIR/deployment-info.json"
    echo "{}" > "$DEPLOYMENT_INFO" # Start with empty JSON
    
    # Function to deploy and record
    deploy_and_record() {
      local contract_name=$1
      local contract_path=$2
      local constructor_args=$3
      
      echo "Deploying ${contract_name}..."
      DEPLOY_OUTPUT=$(forge create "$ETH_DIR/$contract_path" --rpc-url "$RPC_URL" --private-key "$PRIVATE_KEY" --chain-id "$CHAIN_ID" --broadcast $constructor_args)
      
      CONTRACT_ADDRESS=$(echo "$DEPLOY_OUTPUT" | grep "Deployed to:" | awk '{print $3}')
      DEPLOYER=$(echo "$DEPLOY_OUTPUT" | grep "Deployer:" | awk '{print $2}')
      TX_HASH=$(echo "$DEPLOY_OUTPUT" | grep "Transaction hash:" | awk '{print $3}')
      
      if [ -z "$CONTRACT_ADDRESS" ]; then
        echo "Failed to deploy ${contract_name}"
        echo "Output: $DEPLOY_OUTPUT"
        exit 1
      fi
      
      echo "Deployed ${contract_name} to $CONTRACT_ADDRESS (tx: $TX_HASH)"
      
      # Update deployment info JSON
      jq --arg name "$contract_name" --arg addr "$CONTRACT_ADDRESS" --arg tx "$TX_HASH" \
         '.[$name] = {address: $addr, tx_hash: $tx}' "$DEPLOYMENT_INFO" > temp.json && mv temp.json "$DEPLOYMENT_INFO"
    }
    
    # Deploy contracts
    deploy_and_record "EthereumProcessor" "EthereumProcessor.sol:EthereumProcessor"
    deploy_and_record "BaseAccount" "BaseAccount.sol:BaseAccount"
    deploy_and_record "UniversalGateway" "UniversalGateway.sol:UniversalGateway"
    deploy_and_record "TestToken_SUN" "TestToken.sol:TestToken" '--constructor-args "Sun Token" SUN 18'
    deploy_and_record "TestToken_EARTH" "TestToken.sol:TestToken" '--constructor-args "Earth Token" EARTH 18'
    
    echo "Ethereum contracts deployed successfully to Anvil!"
    echo "Deployment info saved to $DEPLOYMENT_INFO"
  '';
  
  # Deploy Ethereum contracts script content (Reth)
  deployEthereumContractsRethScript = ''
    PROJECT_ROOT="$(pwd)"
    RETH_CONFIG="$PROJECT_ROOT/config/reth/config.json"
    BUILD_DIR="$PROJECT_ROOT/build/valence-contracts/ethereum-reth"
    ETH_DIR="$PROJECT_ROOT/valence-protocol/solidity"
    
    if [ ! -d "$ETH_DIR" ]; then
      echo "Valence Solidity contracts directory not found at $ETH_DIR"
      echo "Please build Valence contracts first using 'nix run .#build-valence-contracts'"
      exit 1
    fi
    
    if [ ! -f "$RETH_CONFIG" ]; then
      echo "Reth config not found at $RETH_CONFIG"
      echo "Please ensure reth is running and config is exported."
      exit 1
    fi
    
    RPC_URL=$(jq -r '.rpc_url' "$RETH_CONFIG")
    PRIVATE_KEY=$(jq -r '.private_key' "$RETH_CONFIG")
    CHAIN_ID=$(jq -r '.chain_id' "$RETH_CONFIG")
    
    echo "Deploying Ethereum contracts to Reth ($RPC_URL)..."
    
    # Ensure build directory exists
    mkdir -p "$BUILD_DIR"
    DEPLOYMENT_INFO="$BUILD_DIR/deployment-info.json"
    echo "{}" > "$DEPLOYMENT_INFO" # Start with empty JSON
    
    # Function to deploy and record (same as Anvil version)
    deploy_and_record() {
      local contract_name=$1
      local contract_path=$2
      local constructor_args=$3
      
      echo "Deploying ${contract_name}..."
      DEPLOY_OUTPUT=$(forge create "$ETH_DIR/$contract_path" --rpc-url "$RPC_URL" --private-key "$PRIVATE_KEY" --chain-id "$CHAIN_ID" --broadcast $constructor_args)
      
      CONTRACT_ADDRESS=$(echo "$DEPLOY_OUTPUT" | grep "Deployed to:" | awk '{print $3}')
      DEPLOYER=$(echo "$DEPLOY_OUTPUT" | grep "Deployer:" | awk '{print $2}')
      TX_HASH=$(echo "$DEPLOY_OUTPUT" | grep "Transaction hash:" | awk '{print $3}')
      
      if [ -z "$CONTRACT_ADDRESS" ]; then
        echo "Failed to deploy ${contract_name}"
        echo "Output: $DEPLOY_OUTPUT"
        exit 1
      fi
      
      echo "Deployed ${contract_name} to $CONTRACT_ADDRESS (tx: $TX_HASH)"
      
      # Update deployment info JSON
      jq --arg name "$contract_name" --arg addr "$CONTRACT_ADDRESS" --arg tx "$TX_HASH" \
         '.[$name] = {address: $addr, tx_hash: $tx}' "$DEPLOYMENT_INFO" > temp.json && mv temp.json "$DEPLOYMENT_INFO"
    }
    
    # Deploy contracts
    deploy_and_record "EthereumProcessor" "EthereumProcessor.sol:EthereumProcessor"
    deploy_and_record "BaseAccount" "BaseAccount.sol:BaseAccount"
    deploy_and_record "UniversalGateway" "UniversalGateway.sol:UniversalGateway"
    deploy_and_record "TestToken_SUN" "TestToken.sol:TestToken" '--constructor-args "Sun Token" SUN 18'
    deploy_and_record "TestToken_EARTH" "TestToken.sol:TestToken" '--constructor-args "Earth Token" EARTH 18'
    
    echo "Ethereum contracts deployed successfully to Reth!"
    echo "Deployment info saved to $DEPLOYMENT_INFO"
  '';
  
  # Integration setup script
  integrationSetupScript = ''
    PROJECT_ROOT="$(pwd)"
    CONFIG_DIR="$PROJECT_ROOT/config"
    mkdir -p "$CONFIG_DIR/wasmd"
    mkdir -p "$CONFIG_DIR/anvil"
    mkdir -p "$CONFIG_DIR/reth"
    
    echo "Generating default configurations for test environments..."
    
    # Generate wasmd config
    echo "Generating wasmd config..."
    nix run .#valence-contract-make-wasmd-config > "$CONFIG_DIR/wasmd/config.json"
    
    # Generate anvil config
    echo "Generating anvil config..."
    nix run .#valence-contract-make-anvil-config > "$CONFIG_DIR/anvil/config.json"
    
    # Generate reth config (assuming reth module provides a way to export config)
    # This might require running a reth-specific command if available
    echo "Generating reth config (requires reth node running or pre-configured)..."
    if nix run .#export-reth-config -- --output-file="$CONFIG_DIR/reth/config.json"; then
      echo "Reth config exported successfully."
    else
      echo "Failed to export reth config. Please ensure reth is configured or export manually."
    fi
    
    # Build contracts if not already done
    if [ ! -d "$PROJECT_ROOT/valence-protocol/target" ]; then
      echo "Building Valence contracts..."
      nix run .#build-valence-contracts
    fi
    
    echo "Valence contract integration setup complete!"
    echo "Configuration files generated in $CONFIG_DIR"
  '';
  
  # Helper function to create Nix package from script content
  makeScriptPackage = { name, scriptContent, runtimeInputs ? [] }:
    pkgs.writeShellApplication {
      inherit name runtimeInputs;
      text = scriptContent;
    };

in
{
  # Define per-system outputs
  perSystem = { config, self', inputs', pkgs, system, ... }:
  let 
    # Define packages in a let block for clarity
    definedPackages = {
      # Build Valence contracts
      build-valence-contracts = makeScriptPackage {
        name = "build-valence-contracts";
        scriptContent = buildContractsScript;
        runtimeInputs = [ pkgs.git pkgs.cargo pkgs.rustc pkgs.wasm-pack ];
      };
      
      # Deploy Account contract
      deploy-valence-account = makeScriptPackage { 
        name = "deploy-valence-account"; 
        scriptContent = deployAccountScript; 
        runtimeInputs = [ pkgs.jq pkgs.curl pkgs.git config.packages.wasmd-cli ]; # Use merged wasmd-cli
      };
      
      # Deploy Processor contract
      deploy-valence-processor = makeScriptPackage { 
        name = "deploy-valence-processor"; 
        scriptContent = deployProcessorScript; 
        runtimeInputs = [ pkgs.jq pkgs.curl pkgs.git config.packages.wasmd-cli ]; 
      };
      
      # Deploy Authorization contract
      deploy-valence-authorization = makeScriptPackage { 
        name = "deploy-valence-authorization"; 
        scriptContent = deployAuthorizationScript; 
        runtimeInputs = [ pkgs.jq pkgs.curl pkgs.git config.packages.wasmd-cli ]; 
      };
      
      # Deploy Library contract
      deploy-valence-library = makeScriptPackage { 
        name = "deploy-valence-library"; 
        scriptContent = deployLibraryScript; 
        runtimeInputs = [ pkgs.jq pkgs.curl pkgs.git config.packages.wasmd-cli ]; 
      };
      
      # Deploy all Cosmos contracts
      deploy-valence-cosmos-contracts = makeScriptPackage { 
        name = "deploy-valence-cosmos-contracts"; 
        scriptContent = deployAllContractsScript; 
        # Depends on other deploy scripts, runtime inputs are implicitly handled
        # Needs access to other nix run commands, provided by being in the shell
        runtimeInputs = [ pkgs.nix ]; 
      };
      
      # Deploy Ethereum contracts to Anvil
      deploy-valence-ethereum-contracts = makeScriptPackage { 
        name = "deploy-valence-ethereum-contracts"; 
        scriptContent = deployEthereumContractsScript; 
        runtimeInputs = [ inputs.foundry.packages.${system}.default pkgs.jq pkgs.curl pkgs.git ]; 
      };
      
      # Deploy Ethereum contracts to Reth
      deploy-valence-ethereum-contracts-reth = makeScriptPackage { 
        name = "deploy-valence-ethereum-contracts-reth"; 
        scriptContent = deployEthereumContractsRethScript; 
        runtimeInputs = [ inputs.foundry.packages.${system}.default pkgs.jq pkgs.curl pkgs.git ]; 
      };
      
      # Valence contract integration setup
      valence-contract-integration = makeScriptPackage { 
        name = "valence-contract-integration"; 
        scriptContent = integrationSetupScript; 
        runtimeInputs = [ pkgs.jq pkgs.curl pkgs.git pkgs.nix ]; # Needs nix run
      };

      # Config generation utilities (exposed as packages/apps)
      valence-contract-make-wasmd-config = valenceUtils.makeWasmdConfig {};
      valence-contract-make-anvil-config = valenceUtils.makeAnvilConfig {};
    };
  in
  {
    # Add packages for building and deploying Valence contracts
    packages = definedPackages;
    
    # Add apps for the Nix flake, mapped from the locally defined packages
    apps = lib.mapAttrs' (name: pkg: { 
              name = name; 
              value = { type = "app"; program = "${pkg}/bin/${name}"; }; 
            }) definedPackages;
  };
} 