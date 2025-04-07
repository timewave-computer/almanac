# reth.nix - Module for running a reth Ethereum node
{ lib, pkgs, ... }:

let
  # Define default configuration for reth
  defaultRethConfig = {
    # Network related settings
    rpcPort = 8545;
    p2pPort = 30303;
    wsPort = 8546;
    httpApi = [ "eth" "net" "web3" "txpool" "debug" "trace" ];
    wsApi = [ "eth" "net" "web3" "txpool" ];
    
    # Chain related settings
    chainId = 31337; # Same as Anvil default for easy swapping
    blockTime = 2; # Block time in seconds (for dev mode)
    dataDir = "~/.reth-dev";
    
    # Dev mode settings
    devMode = true; # Enable development mode
    minerEnabled = true;
    minerCoinbase = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"; # Default dev account
    
    # Genesis settings
    genesisAccounts = [
      { 
        address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
        balance = "10000000000000000000000"; # 10,000 ETH
        privateKey = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
      }
      { 
        address = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8";
        balance = "10000000000000000000000"; # 10,000 ETH
        privateKey = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";
      }
      { 
        address = "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC";
        balance = "10000000000000000000000"; # 10,000 ETH
        privateKey = "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a";
      }
    ];
  };

  # Function to create a reth config file
  makeRethConfig = config:
    let
      # Merge default config with provided config
      finalConfig = defaultRethConfig // config;

      # Generate genesis allocation JSON
      genesisAllocation = builtins.concatStringsSep ",\n" (map 
        (account: ''
          "${account.address}": {
            "balance": "${account.balance}"
          }''
        ) 
        finalConfig.genesisAccounts);

      # Generate a simple genesis configuration
      genesisJson = ''
        {
          "config": {
            "chainId": ${toString finalConfig.chainId},
            "homesteadBlock": 0,
            "eip150Block": 0,
            "eip155Block": 0,
            "eip158Block": 0,
            "byzantiumBlock": 0,
            "constantinopleBlock": 0,
            "petersburgBlock": 0,
            "istanbulBlock": 0,
            "berlinBlock": 0,
            "londonBlock": 0,
            "shanghaiBLock": 0,
            "terminalTotalDifficulty": 0,
            "terminalTotalDifficultyPassed": true
          },
          "difficulty": "1",
          "gasLimit": "30000000",
          "timestamp": "0",
          "extraData": "0x4265207369676e69666963616e74",
          "alloc": {
            ${genesisAllocation}
          },
          "nonce": "0x0000000000000042",
          "mixhash": "0x0000000000000000000000000000000000000000000000000000000000000000",
          "parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
        }
      '';

      # Generate reth config file content
      rethConfig = ''
        [rpc]
        http = true
        http_addr = "0.0.0.0"
        http_port = ${toString finalConfig.rpcPort}
        http_api = [${builtins.concatStringsSep ", " (map (api: "\"${api}\"") finalConfig.httpApi)}]
        ws = true
        ws_addr = "0.0.0.0"
        ws_port = ${toString finalConfig.wsPort}
        ws_api = [${builtins.concatStringsSep ", " (map (api: "\"${api}\"") finalConfig.wsApi)}]
        
        [p2p]
        port = ${toString finalConfig.p2pPort}
        
        [mining]
        enabled = ${if finalConfig.minerEnabled then "true" else "false"}
        block_time = ${toString finalConfig.blockTime}
        coinbase = "${finalConfig.minerCoinbase}"
        
        [chain]
        chain_id = ${toString finalConfig.chainId}
        
        [debug]
        dev_mode = ${if finalConfig.devMode then "true" else "false"}
      '';
    in {
      inherit rethConfig genesisJson;
      
      # Export useful properties
      inherit (finalConfig) rpcPort p2pPort wsPort dataDir chainId;
      httpRpcUrl = "http://localhost:${toString finalConfig.rpcPort}";
      wsRpcUrl = "ws://localhost:${toString finalConfig.wsPort}";
      defaultAccount = builtins.elemAt finalConfig.genesisAccounts 0;
    };

  # Script to start reth with the given config
  makeRethStartScript = { name ? "start-reth", config ? {} }:
    let
      rethCfg = makeRethConfig config;
      expandedDataDir = builtins.replaceStrings ["~"] [builtins.getEnv "HOME"] rethCfg.dataDir;
      rethConfigFile = pkgs.writeTextFile {
        name = "reth-config.toml";
        text = rethCfg.rethConfig;
      };
      genesisFile = pkgs.writeTextFile {
        name = "genesis.json";
        text = rethCfg.genesisJson;
      };
    in
      pkgs.writeShellScriptBin name ''
        #!/usr/bin/env bash
        set -eo pipefail

        echo "Starting reth Ethereum node..."
        
        # Ensure data directory exists
        DATA_DIR="${expandedDataDir}"
        mkdir -p "$DATA_DIR"
        
        # Copy config files
        mkdir -p "$DATA_DIR/config"
        cp "${rethConfigFile}" "$DATA_DIR/config/reth.toml"
        cp "${genesisFile}" "$DATA_DIR/config/genesis.json"
        
        # Initialize if needed
        if [ ! -d "$DATA_DIR/db" ]; then
          echo "Initializing reth node with genesis block..."
          ${pkgs.reth}/bin/reth init --datadir "$DATA_DIR" --chain "$DATA_DIR/config/genesis.json"
        fi
        
        # Start reth
        echo "Starting reth node on RPC port ${toString rethCfg.rpcPort}..."
        echo "JSON-RPC URL: ${rethCfg.httpRpcUrl}"
        echo "WS URL: ${rethCfg.wsRpcUrl}"
        echo "Network ID: ${toString rethCfg.chainId}"
        echo "Default account: ${rethCfg.defaultAccount.address}"
        echo "Private key: ${rethCfg.defaultAccount.privateKey}"
        
        # Export configuration
        export ETH_RPC_URL="${rethCfg.httpRpcUrl}"
        export ETH_PRIVATE_KEY="${rethCfg.defaultAccount.privateKey}"
        export CHAIN_ID="${toString rethCfg.chainId}"
        
        # Create config file in our project format
        mkdir -p $(pwd)/config/reth
        cat > $(pwd)/config/reth/config.json << EOF
        {
          "rpc_url": "${rethCfg.httpRpcUrl}",
          "ws_url": "${rethCfg.wsRpcUrl}",
          "private_key": "${rethCfg.defaultAccount.privateKey}",
          "chain_id": "${toString rethCfg.chainId}"
        }
        EOF
        
        # Start with appropriate parameters
        ${pkgs.reth}/bin/reth node \
          --datadir "$DATA_DIR" \
          --config "$DATA_DIR/config/reth.toml" \
          --dev \
          "$@"
      '';

  # Script to purge reth data
  makeRethCleanScript = { name ? "clean-reth", config ? {} }:
    let
      rethCfg = makeRethConfig config;
      expandedDataDir = builtins.replaceStrings ["~"] [builtins.getEnv "HOME"] rethCfg.dataDir;
    in
      pkgs.writeShellScriptBin name ''
        #!/usr/bin/env bash
        set -eo pipefail
        
        DATA_DIR="${expandedDataDir}"
        
        echo "Removing reth data directory at $DATA_DIR..."
        rm -rf "$DATA_DIR"
        echo "Done."
      '';

  # Import genesis accounts into a running node
  makeRethImportAccountsScript = { name ? "import-reth-accounts", config ? {} }:
    let
      rethCfg = makeRethConfig config;
      expandedDataDir = builtins.replaceStrings ["~"] [builtins.getEnv "HOME"] rethCfg.dataDir;
      
      # Generate import commands for each account
      importCommands = builtins.concatStringsSep "\n" (map 
        (account: ''
          echo "Importing account ${account.address}..."
          ${pkgs.go-ethereum}/bin/geth account import --datadir "${expandedDataDir}" \
            --password <(echo "") <(echo "${account.privateKey}" | sed 's/^0x//') || true
        '') 
        rethCfg.genesisAccounts);
    in
      pkgs.writeShellScriptBin name ''
        #!/usr/bin/env bash
        set -eo pipefail
        
        echo "Importing pre-configured accounts..."
        ${importCommands}
        echo "Done importing accounts."
      '';

  # Script to export reth configuration for integrating with other systems
  makeRethExportConfigScript = { name ? "export-reth-config", config ? {} }:
    let
      rethCfg = makeRethConfig config;
    in
      pkgs.writeShellScriptBin name ''
        #!/usr/bin/env bash
        set -eo pipefail
        
        # Ensure config directory exists
        mkdir -p "$(pwd)/config/reth"
        
        # Export configuration
        cat > "$(pwd)/config/reth/config.json" << EOF
        {
          "rpc_url": "${rethCfg.httpRpcUrl}",
          "ws_url": "${rethCfg.wsRpcUrl}",
          "private_key": "${rethCfg.defaultAccount.privateKey}",
          "chain_id": "${toString rethCfg.chainId}"
        }
        EOF
        
        echo "Exported reth configuration to $(pwd)/config/reth/config.json"
      '';
in
{
  # Export Nix functions for use in other modules
  inherit makeRethConfig makeRethStartScript makeRethCleanScript makeRethImportAccountsScript makeRethExportConfigScript;
  
  # Default configurations for different environments
  testConfig = makeRethConfig {
    dataDir = "~/.reth-test";
    rpcPort = 8545;
    wsPort = 8546;
    p2pPort = 30303;
    chainId = 31337;
  };
  
  devConfig = makeRethConfig {
    dataDir = "~/.reth-dev";
    rpcPort = 8545;
    wsPort = 8546;
    p2pPort = 30303;
    chainId = 31337;
    blockTime = 1; # Faster blocks for development
  };
  
  # Packages available to the flake
  perSystem = { config, self', inputs', pkgs, ... }: {
    packages = {
      # Start a reth node with default test configuration
      start-reth = makeRethStartScript { config = {}; };
      
      # Clean reth data
      clean-reth = makeRethCleanScript { config = {}; };
      
      # Import accounts
      import-reth-accounts = makeRethImportAccountsScript { config = {}; };
      
      # Export configuration
      export-reth-config = makeRethExportConfigScript { config = {}; };
    };
    
    # Apps for the flake
    apps = {
      start-reth = {
        type = "app";
        program = "${self'.packages.start-reth}/bin/start-reth";
      };
      
      clean-reth = {
        type = "app";
        program = "${self'.packages.clean-reth}/bin/clean-reth";
      };
      
      import-reth-accounts = {
        type = "app";
        program = "${self'.packages.import-reth-accounts}/bin/import-reth-accounts";
      };
      
      export-reth-config = {
        type = "app";
        program = "${self'.packages.export-reth-config}/bin/export-reth-config";
      };
    };
  };
} 