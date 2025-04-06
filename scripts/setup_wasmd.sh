#!/bin/bash
# Script to initialize and run a wasmd node with a custom validator key to avoid socket timeout issues

set -e

# Step 1: Set up the home directory 
WASMD_HOME="/Users/hxrts/.wasmd-test"
echo "Setting up wasmd in $WASMD_HOME"

# Step 2: Clean up any existing wasmd instance
pkill -f "wasmd" >/dev/null 2>&1 || true
sleep 2
rm -rf "$WASMD_HOME"
mkdir -p "$WASMD_HOME"

# Step 3: Try to find wasmd in various ways
if command -v wasmd >/dev/null 2>&1; then
  WASMD_CMD="wasmd"
  echo "Using wasmd from PATH: $(which wasmd)"
elif command -v nix >/dev/null 2>&1; then
  if nix develop --command wasmd version >/dev/null 2>&1; then
    echo "Found wasmd in Nix development shell"
    WASMD_CMD="nix develop --command wasmd"
  elif nix run .#wasmd-node >/dev/null 2>&1; then
    echo "Using wasmd-node from flake apps"
    # In this case, our script is not needed since wasmd-node is already available
    echo "wasmd-node is already available as a flake app. Run it directly with:"
    echo "nix run .#wasmd-node"
    exit 0
  else
    echo "Error: wasmd command not found in Nix environment."
    exit 1
  fi
else
  echo "Error: Neither wasmd nor Nix found in PATH."
  exit 1
fi

# Step 4: Initialize wasmd
echo "Initializing wasmd node..."
$WASMD_CMD init --chain-id=wasmchain testing --home="$WASMD_HOME"

# Step 5: Create a custom validator key file
echo "Creating custom validator key..."
cat > "$WASMD_HOME/config/priv_validator_key.json" << EOF
{
  "address": "12617FA635AF8D5E2141BE5FFB161D89B1847771",
  "pub_key": {
    "type": "tendermint/PubKeyEd25519",
    "value": "Ie/6a5+2gFL+jR8418CroiYqgLXEuCkRBV5/aoLkvas="
  },
  "priv_key": {
    "type": "tendermint/PrivKeyEd25519",
    "value": "hVv9jXbgI8K5ua3x8+jroT96l7YlVfq9jjOJ5vKYFD4h7/prn7aAUv6NHzjXwKuiJiqAtcS4KREFX39qguS9qw=="
  }
}
EOF

# Step 6: Create empty validator state file
echo "Creating empty validator state file..."
cat > "$WASMD_HOME/config/priv_validator_state.json" << EOF
{
  "height": "0",
  "round": 0,
  "step": 0
}
EOF

# Step 7: Configure wasmd
echo "Configuring wasmd..."
$WASMD_CMD config chain-id wasmchain --home="$WASMD_HOME"
$WASMD_CMD config keyring-backend test --home="$WASMD_HOME"
$WASMD_CMD config broadcast-mode block --home="$WASMD_HOME"
$WASMD_CMD config node tcp://127.0.0.1:26657 --home="$WASMD_HOME"

# Step 8: Create validator account
echo "Creating validator account..."
$WASMD_CMD keys add validator --keyring-backend=test --home="$WASMD_HOME" || true
VALIDATOR_ADDR=$($WASMD_CMD keys show validator -a --keyring-backend=test --home="$WASMD_HOME")
echo "Validator address: $VALIDATOR_ADDR"

# Step 9: Set up genesis account and transactions
echo "Setting up genesis account..."
$WASMD_CMD add-genesis-account "$VALIDATOR_ADDR" 1000000000stake,1000000000validatortoken --home="$WASMD_HOME"
$WASMD_CMD gentx validator 1000000stake --chain-id=wasmchain --keyring-backend=test --home="$WASMD_HOME"
$WASMD_CMD collect-gentxs --home="$WASMD_HOME"

# Step 10: Update config.toml to disable the priv_validator_laddr
echo "Updating config.toml to disable private validator socket..."
sed -i.bak 's|^priv_validator_laddr = "tcp://127.0.0.1:26658"|priv_validator_laddr = ""|' "$WASMD_HOME/config/config.toml"

# Step 11: Set minimum gas price in app.toml
echo "Setting minimum gas price..."
# Check if the file exists and the line exists, then replace it
if [ -f "$WASMD_HOME/config/app.toml" ]; then
  if grep -q "^minimum-gas-prices" "$WASMD_HOME/config/app.toml"; then
    sed -i.bak 's|^minimum-gas-prices = ".*"|minimum-gas-prices = "0stake"|' "$WASMD_HOME/config/app.toml"
    echo "Updated minimum-gas-prices setting"
  else
    echo "minimum-gas-prices setting not found in app.toml"
  fi
else
  echo "app.toml file not found"
fi

# Step 12: Start the wasmd node
echo "Starting wasmd node..."
echo "IMPORTANT: To stop the node, press Ctrl+C"
$WASMD_CMD start --home="$WASMD_HOME" --priv_validator_laddr="" --x-crisis-skip-assert-invariants --trace
