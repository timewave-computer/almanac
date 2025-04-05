# Cross-Chain End-to-End Test Specification

This document specifies a comprehensive end-to-end test for validating cross-chain interactions between Ethereum and Cosmos, with the Almanac indexer processing and monitoring events on both chains.

## Test Overview

This test demonstrates a complete cross-chain workflow where:
1. Actions on the Ethereum chain trigger events
2. The indexer processes these events
3. A process detects the events via indexer queries and triggers actions on the Cosmos chain
4. The indexer processes Cosmos chain events
5. Another process detects these events via indexer queries and triggers final actions on Ethereum

## Prerequisites

- Running Ethereum test node (Anvil)
- Running Cosmos test node (wasmd)
- Almanac indexer configured to index both chains
- Valence contracts deployed on both chains
- Test accounts with sufficient funds

## Test Environment Setup

### 1. Ethereum Setup

```bash
# Start Anvil with a deterministic mnemonic and block time
anvil --mnemonic "test test test test test test test test test test test junk" --block-time 2
```

### 2. Cosmos Setup

```bash
# Start local wasmd node
wasmd-node
```

### 3. Indexer Setup

```bash
# Start the indexer in a separate terminal
cargo run --bin indexer -- --config config/test_config.toml
```

## Test Components

1. **Account Setup**
   - Ethereum EOA Account "A" (derived from test mnemonic)
   - Cosmos EOA Account "B" (derived from test mnemonic)
   - Base Account on Ethereum
   - Base Account on Cosmos
   - Processor on Ethereum
   - Processor on Cosmos
   - Authorization Contract on Cosmos

2. **Tokens**
   - "SUN" tokens (ERC-20 on Ethereum)
   - "EARTH" tokens (ERC-20 on Ethereum)
   - "MOON" tokens (CW-20 on Cosmos)

3. **Monitoring Processes**
   - Process 1: Monitors Ethereum → triggers Cosmos actions
   - Process 2: Monitors Cosmos → triggers Ethereum actions

## Detailed Test Procedure

### Phase 1: Initialization

#### 1.1: Account Preparation

```bash
# Generate Ethereum Account A
ETH_ACCOUNT_A="0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266" # First address from test mnemonic

# Generate Cosmos Account B
COSMOS_ACCOUNT_B="cosmos1phaxpevm5wecex2jyaqty2a4v02qj7qmhz2r5e" # First address from test mnemonic
```

#### 1.2: Fund Accounts

```bash
# Anvil automatically funds test accounts with 10000 ETH
echo "Ethereum Account A balance: $(cast balance $ETH_ACCOUNT_A)"

# Fund Cosmos account B with ATOM
wasmd tx bank send validator $COSMOS_ACCOUNT_B 1000000ustake --chain-id=testing --keyring-backend=test --yes
wasmd q bank balances $COSMOS_ACCOUNT_B
```

#### 1.3: Deploy Ethereum Contracts

```bash
# Deploy Processor Contract
echo "Deploying Ethereum Processor Contract..."
ETH_PROCESSOR_ADDRESS=$(forge create --rpc-url http://localhost:8545 --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 contracts/Processor.sol:Processor --constructor-args [constructor_args] | grep -oP 'Deployed to: \K.*')
echo "Ethereum Processor: $ETH_PROCESSOR_ADDRESS"

# Deploy Base Account Contract
echo "Deploying Ethereum Base Account Contract..."
ETH_BASE_ACCOUNT_ADDRESS=$(forge create --rpc-url http://localhost:8545 --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 contracts/BaseAccount.sol:BaseAccount --constructor-args [constructor_args] | grep -oP 'Deployed to: \K.*')
echo "Ethereum Base Account: $ETH_BASE_ACCOUNT_ADDRESS"

# Deploy Token Contracts
echo "Deploying SUN Token Contract..."
SUN_TOKEN_ADDRESS=$(forge create --rpc-url http://localhost:8545 --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 contracts/TestToken.sol:TestToken --constructor-args "SUN Token" "SUN" | grep -oP 'Deployed to: \K.*')
echo "SUN Token: $SUN_TOKEN_ADDRESS"

echo "Deploying EARTH Token Contract..."
EARTH_TOKEN_ADDRESS=$(forge create --rpc-url http://localhost:8545 --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 contracts/TestToken.sol:TestToken --constructor-args "EARTH Token" "EARTH" | grep -oP 'Deployed to: \K.*')
echo "EARTH Token: $EARTH_TOKEN_ADDRESS"
```

#### 1.4: Deploy Cosmos Contracts

```bash
# Store and instantiate Processor Contract
echo "Storing Cosmos Processor Contract..."
PROCESSOR_CODE_ID=$(wasmd tx wasm store artifacts/processor.wasm --from validator --chain-id=testing --gas=4000000 --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
echo "Processor Code ID: $PROCESSOR_CODE_ID"

echo "Instantiating Cosmos Processor Contract..."
COSMOS_PROCESSOR_ADDRESS=$(wasmd tx wasm instantiate $PROCESSOR_CODE_ID '{"owner":"'$COSMOS_ACCOUNT_B'"}' --from validator --chain-id=testing --label "processor" --no-admin --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
echo "Cosmos Processor: $COSMOS_PROCESSOR_ADDRESS"

# Store and instantiate Base Account Contract
echo "Storing Cosmos Base Account Contract..."
BASE_ACCOUNT_CODE_ID=$(wasmd tx wasm store artifacts/base_account.wasm --from validator --chain-id=testing --gas=4000000 --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
echo "Base Account Code ID: $BASE_ACCOUNT_CODE_ID"

echo "Instantiating Cosmos Base Account Contract..."
COSMOS_BASE_ACCOUNT_ADDRESS=$(wasmd tx wasm instantiate $BASE_ACCOUNT_CODE_ID '{"owner":"'$COSMOS_ACCOUNT_B'"}' --from validator --chain-id=testing --label "base_account" --no-admin --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
echo "Cosmos Base Account: $COSMOS_BASE_ACCOUNT_ADDRESS"

# Store and instantiate Authorization Contract
echo "Storing Cosmos Authorization Contract..."
AUTH_CODE_ID=$(wasmd tx wasm store artifacts/authorization.wasm --from validator --chain-id=testing --gas=4000000 --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
echo "Authorization Code ID: $AUTH_CODE_ID"

echo "Instantiating Cosmos Authorization Contract..."
COSMOS_AUTH_ADDRESS=$(wasmd tx wasm instantiate $AUTH_CODE_ID '{"owner":"'$COSMOS_ACCOUNT_B'"}' --from validator --chain-id=testing --label "authorization" --no-admin --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
echo "Cosmos Authorization: $COSMOS_AUTH_ADDRESS"

# Store and instantiate MOON Token Contract (CW20)
echo "Storing CW20 Token Contract..."
CW20_CODE_ID=$(wasmd tx wasm store artifacts/cw20_base.wasm --from validator --chain-id=testing --gas=4000000 --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value')
echo "CW20 Code ID: $CW20_CODE_ID"

echo "Instantiating MOON Token Contract..."
MOON_TOKEN_ADDRESS=$(wasmd tx wasm instantiate $CW20_CODE_ID '{"name":"MOON Token","symbol":"MOON","decimals":6,"initial_balances":[{"address":"'$COSMOS_BASE_ACCOUNT_ADDRESS'","amount":"10000000"}],"mint":{"minter":"'$COSMOS_ACCOUNT_B'"}}' --from validator --chain-id=testing --label "moon_token" --no-admin --output json --keyring-backend=test --yes | jq -r '.logs[0].events[] | select(.type=="instantiate") | .attributes[] | select(.key=="_contract_address") | .value')
echo "MOON Token: $MOON_TOKEN_ADDRESS"
```

#### 1.5: Mint and Transfer Tokens

```bash
# Mint SUN tokens to Ethereum Base Account
echo "Minting 10 SUN tokens to Ethereum Base Account..."
cast send $SUN_TOKEN_ADDRESS "mint(address,uint256)" $ETH_BASE_ACCOUNT_ADDRESS 10000000000000000000 --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
echo "SUN balance: $(cast call $SUN_TOKEN_ADDRESS "balanceOf(address)" $ETH_BASE_ACCOUNT_ADDRESS)"

# Mint EARTH tokens to Ethereum Base Account
echo "Minting 10 EARTH tokens to Ethereum Base Account..."
cast send $EARTH_TOKEN_ADDRESS "mint(address,uint256)" $ETH_BASE_ACCOUNT_ADDRESS 10000000000000000000 --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
echo "EARTH balance: $(cast call $EARTH_TOKEN_ADDRESS "balanceOf(address)" $ETH_BASE_ACCOUNT_ADDRESS)"

# Verify MOON tokens on Cosmos Base Account (should be 10 from initialization)
echo "Checking MOON token balance for Cosmos Base Account..."
wasmd query wasm contract-state smart $MOON_TOKEN_ADDRESS '{"balance":{"address":"'$COSMOS_BASE_ACCOUNT_ADDRESS'"}}' --output json | jq
```

#### 1.6: Set Authorizations

```bash
# Set EOA Account A as authorized for Ethereum Base Account
echo "Authorizing ETH Account A to control ETH Base Account..."
cast send $ETH_BASE_ACCOUNT_ADDRESS "setAuthorized(address,bool)" $ETH_ACCOUNT_A true --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
echo "Authorization status: $(cast call $ETH_BASE_ACCOUNT_ADDRESS "isAuthorized(address)" $ETH_ACCOUNT_A)"

# Set EOA Account B as authorized for Cosmos Base Account
echo "Authorizing COSMOS Account B to control COSMOS Base Account..."
wasmd tx wasm execute $COSMOS_AUTH_ADDRESS '{"grant_permission":{"grant_id":"auth1","grantee":"'$COSMOS_ACCOUNT_B'","permissions":["execute"],"resources":["'$COSMOS_BASE_ACCOUNT_ADDRESS'"]}}' --from validator --chain-id=testing --output json --keyring-backend=test --yes | jq
```

### Phase 2: Test Execution

#### 2.1: Monitor Setup

Create two monitoring processes:

**Process 1: Ethereum to Cosmos Monitor**
```python
#!/usr/bin/env python3
import time
import requests
import subprocess
import json

# Monitor Ethereum events via indexer and trigger Cosmos actions
INDEXER_URL = "http://localhost:8080"
BURN_ADDRESS = "0x000000000000000000000000000000000000dEaD"

ethereum_base_account = "ETH_BASE_ACCOUNT_ADDRESS"  # Replace with actual address
earth_token = "EARTH_TOKEN_ADDRESS"                # Replace with actual address
cosmos_account_b = "COSMOS_ACCOUNT_B"              # Replace with actual address
cosmos_base_account = "COSMOS_BASE_ACCOUNT_ADDRESS" # Replace with actual address
moon_token = "MOON_TOKEN_ADDRESS"                   # Replace with actual address

def check_earth_token_burn():
    # Query indexer for EARTH token transfers from base account to burn address
    query = {
        "query": f"""
        {{
            ethereum {{
                tokenTransfers(
                    where: {{
                        from: "{ethereum_base_account}",
                        to: "{BURN_ADDRESS}",
                        token: "{earth_token}"
                    }}
                ) {{
                    txHash
                    amount
                    blockNumber
                }}
            }}
        }}
        """
    }
    
    try:
        response = requests.post(f"{INDEXER_URL}/graphql", json=query)
        data = response.json()
        
        if 'data' in data and 'ethereum' in data['data'] and 'tokenTransfers' in data['data']['ethereum']:
            transfers = data['data']['ethereum']['tokenTransfers']
            if len(transfers) > 0:
                print(f"EARTH token burn detected in tx: {transfers[0]['txHash']}")
                trigger_cosmos_action()
                return True
        
        return False
    except Exception as e:
        print(f"Error querying indexer: {e}")
        return False

def trigger_cosmos_action():
    # Execute Cosmos transaction to burn MOON tokens
    print("Triggering MOON token burn on Cosmos chain...")
    
    # Prepare the execution message for burning tokens
    execute_msg = {
        "send": {
            "contract": BURN_ADDRESS,
            "amount": "10000000",  # 10 MOON tokens with 6 decimals
            "msg": ""
        }
    }
    
    # First, prepare message to send from base account to MOON token contract
    base_account_msg = {
        "execute": {
            "contract_addr": moon_token,
            "msg": json.dumps(execute_msg),
            "funds": []
        }
    }
    
    # Execute the transaction through Cosmos account B
    cmd = [
        "wasmd", "tx", "wasm", "execute", cosmos_base_account,
        json.dumps(base_account_msg),
        "--from", cosmos_account_b,
        "--chain-id", "testing",
        "--keyring-backend", "test",
        "--yes"
    ]
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    print(f"Transaction result: {result.stdout}")
    if result.stderr:
        print(f"Error: {result.stderr}")

# Main monitoring loop
print("Starting Ethereum to Cosmos monitor...")
while True:
    if check_earth_token_burn():
        print("Monitor 1 task completed!")
        break
    print("Waiting for EARTH token burn event...")
    time.sleep(5)
```

**Process 2: Cosmos to Ethereum Monitor**
```python
#!/usr/bin/env python3
import time
import requests
import subprocess
import json

# Monitor Cosmos events via indexer and trigger Ethereum actions
INDEXER_URL = "http://localhost:8080"
BURN_ADDRESS = "000000000000000000000000000000000000dEaD"  # Cosmos burn address format

ethereum_account_a = "ETH_ACCOUNT_A"                # Replace with actual address
ethereum_base_account = "ETH_BASE_ACCOUNT_ADDRESS"  # Replace with actual address
sun_token = "SUN_TOKEN_ADDRESS"                     # Replace with actual address
cosmos_base_account = "COSMOS_BASE_ACCOUNT_ADDRESS" # Replace with actual address
moon_token = "MOON_TOKEN_ADDRESS"                   # Replace with actual address

def check_moon_token_burn():
    # Query indexer for MOON token transfers from base account to burn address
    query = {
        "query": f"""
        {{
            cosmos {{
                tokenTransfers(
                    where: {{
                        from: "{cosmos_base_account}",
                        to: "{BURN_ADDRESS}",
                        token: "{moon_token}"
                    }}
                ) {{
                    txHash
                    amount
                    blockHeight
                }}
            }}
        }}
        """
    }
    
    try:
        response = requests.post(f"{INDEXER_URL}/graphql", json=query)
        data = response.json()
        
        if 'data' in data and 'cosmos' in data['data'] and 'tokenTransfers' in data['data']['cosmos']:
            transfers = data['data']['cosmos']['tokenTransfers']
            if len(transfers) > 0:
                print(f"MOON token burn detected in tx: {transfers[0]['txHash']}")
                trigger_ethereum_action()
                return True
        
        return False
    except Exception as e:
        print(f"Error querying indexer: {e}")
        return False

def trigger_ethereum_action():
    # Execute Ethereum transaction to burn SUN tokens
    print("Triggering SUN token burn on Ethereum chain...")
    
    # Prepare the calldata for the base account to transfer SUN tokens to burn address
    ethereum_burn_address = "0x000000000000000000000000000000000000dEaD"
    
    # Create the call to the base account to execute the token transfer
    cmd = [
        "cast", "send",
        "--private-key", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        ethereum_base_account,
        "execute(address,bytes)",
        sun_token,
        f"$(cast calldata 'transfer(address,uint256)' {ethereum_burn_address} 10000000000000000000)"
    ]
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    print(f"Transaction result: {result.stdout}")
    if result.stderr:
        print(f"Error: {result.stderr}")

# Main monitoring loop
print("Starting Cosmos to Ethereum monitor...")
while True:
    if check_moon_token_burn():
        print("Monitor 2 task completed!")
        break
    print("Waiting for MOON token burn event...")
    time.sleep(5)
```

#### 2.2: Execute Cross-Chain Events

1. Start both monitoring processes in separate terminals

2. Initiate the first transfer from Ethereum:

```bash
# EOA account A transfers EARTH tokens from Ethereum Base Account to burn address
echo "Initiating EARTH token burn from Ethereum Base Account..."
BURN_ADDRESS="0x000000000000000000000000000000000000dEaD"

# Create calldata for token transfer
TRANSFER_CALLDATA=$(cast calldata "transfer(address,uint256)" $BURN_ADDRESS 10000000000000000000)

# Execute through base account
cast send --private-key 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 $ETH_BASE_ACCOUNT_ADDRESS "execute(address,bytes)" $EARTH_TOKEN_ADDRESS $TRANSFER_CALLDATA

# Verify EARTH token balance is now 0
echo "EARTH balance after burn: $(cast call $EARTH_TOKEN_ADDRESS "balanceOf(address)" $ETH_BASE_ACCOUNT_ADDRESS)"
```

3. Monitoring Process 1 will detect the EARTH token burn via the indexer and trigger the MOON token burn on Cosmos

4. Monitoring Process 2 will detect the MOON token burn via the indexer and trigger the SUN token burn on Ethereum

5. Verify the final state:

```bash
# Check final token balances
echo "Final SUN balance: $(cast call $SUN_TOKEN_ADDRESS "balanceOf(address)" $ETH_BASE_ACCOUNT_ADDRESS)"
echo "Final EARTH balance: $(cast call $EARTH_TOKEN_ADDRESS "balanceOf(address)" $ETH_BASE_ACCOUNT_ADDRESS)"

echo "Final MOON balance:"
wasmd query wasm contract-state smart $MOON_TOKEN_ADDRESS '{"balance":{"address":"'$COSMOS_BASE_ACCOUNT_ADDRESS'"}}' --output json | jq
```

## Success Criteria

The test is considered successful if:

1. The initial transfer of EARTH tokens to the burn address is correctly indexed
2. The monitor correctly detects the EARTH token burn via the indexer
3. The subsequent MOON token burn on Cosmos is executed and indexed
4. The second monitor correctly detects the MOON token burn via the indexer
5. The final SUN token burn on Ethereum is executed
6. All three token balances for the base accounts are zero at the end of the test

## Troubleshooting Tips

1. **Indexer sync issues**: Ensure the indexer is properly detecting and processing events from both chains
2. **Chain connectivity**: Verify that both chain nodes are running and accessible
3. **Transaction failures**: Check for sufficient gas, authorization issues, or contract errors
4. **GraphQL queries**: Test queries directly against the indexer API to verify correct syntax
5. **Log collection**: Capture logs from all components for post-test analysis

## Test Automation

The test can be automated using the provided monitoring scripts along with a master script that:

1. Sets up the test environment
2. Deploys contracts and configures initial state
3. Launches monitoring processes
4. Triggers the initial transaction
5. Waits for all events to complete
6. Verifies the final state
7. Performs cleanup

The test framework should be integrated with the Nix environment to ensure reproducibility and consistent execution across different environments.

## Test Extensions

Future extensions to this test could include:

1. Testing with different token types and amounts
2. Testing failure scenarios (e.g., insufficient funds, revoked authorizations)
3. Testing concurrent cross-chain operations
4. Testing with chain reorganizations on Ethereum
5. Performance benchmarking of the indexer under load

## Script Integration

The test should be integrated into the project's test suite via a Nix-runnable script:

```bash
#!/usr/bin/env bash
set -e

# Cross-chain E2E test runner
echo "Running Cross-Chain End-to-End Test..."

# Setup environment
# ...

# Deploy contracts
# ...

# Start monitoring processes
# ...

# Execute initial transaction
# ...

# Verify final state
# ...

# Cleanup
# ...

echo "Cross-Chain E2E Test completed successfully!"
``` 