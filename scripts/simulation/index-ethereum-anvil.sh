#!/bin/bash
# Purpose: Index Ethereum Valence contracts with Almanac, connecting to an Anvil node

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
ANVIL_PORT=8545
ANVIL_HOST="localhost"
ANVIL_URL="http://${ANVIL_HOST}:${ANVIL_PORT}"
ANVIL_CHAIN_ID=31337
CONTRACTS_DIR="data/contracts/ethereum/anvil"
CONTRACT_ADDRESSES_FILE="${CONTRACTS_DIR}/contract-addresses.env"
LOG_DIR="logs"
LOG_FILE="${LOG_DIR}/almanac-ethereum-anvil.log"
INDEX_DURATION="15m"  # Default indexing duration

echo -e "${BLUE}=== Indexing Ethereum Valence Contracts with Almanac (Anvil) ===${NC}"

# Create necessary directories
mkdir -p ${LOG_DIR}

# Check if the contract addresses file exists
if [ ! -f "${CONTRACT_ADDRESSES_FILE}" ]; then
    echo -e "${RED}Error: Contract addresses file not found at ${CONTRACT_ADDRESSES_FILE}${NC}"
    echo -e "${YELLOW}Please deploy Valence contracts first using: ./simulation/ethereum/deploy-valence-contracts-anvil.sh${NC}"
    exit 1
fi

# Source the contract addresses
source "${CONTRACT_ADDRESSES_FILE}"

# Check for required contract addresses
if [ -z "${VALENCE_REGISTRY_ADDRESS}" ] || [ -z "${VALENCE_GATEWAY_ADDRESS}" ]; then
    echo -e "${RED}Error: Required contract addresses not found in ${CONTRACT_ADDRESSES_FILE}${NC}"
    echo -e "${YELLOW}Please redeploy Valence contracts using: ./simulation/ethereum/deploy-valence-contracts-anvil.sh${NC}"
    exit 1
fi

echo -e "${GREEN}Found contract addresses:${NC}"
echo -e "${GREEN}• Registry: ${VALENCE_REGISTRY_ADDRESS}${NC}"
echo -e "${GREEN}• Gateway: ${VALENCE_GATEWAY_ADDRESS}${NC}"

# Check if Anvil is running
if ! curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' "${ANVIL_URL}" > /dev/null; then
    echo -e "${RED}Error: Anvil node is not running at ${ANVIL_URL}${NC}"
    echo -e "${YELLOW}Please start Anvil first with: ./simulation/ethereum/setup-anvil.sh${NC}"
    exit 1
fi

# Verify chain ID matches expected
CHAIN_ID_HEX=$(curl -s -X POST -H "Content-Type: application/json" --data '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' "${ANVIL_URL}" | grep -o '"result":"0x[0-9a-f]*"' | cut -d'"' -f4)
if [ "$CHAIN_ID_HEX" != "0x7a69" ]; then
    echo -e "${RED}Error: Unexpected chain ID: ${CHAIN_ID_HEX}. Expected: 0x7a69${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Anvil node is running at ${ANVIL_URL}${NC}"

# Check if PostgreSQL is running
if ! nix develop --command bash -c 'pg_isready -h localhost -p 5432 -U postgres' > /dev/null 2>&1; then
    echo -e "${RED}Error: PostgreSQL is not running${NC}"
    echo -e "${YELLOW}Please start PostgreSQL first with: ./simulation/databases/setup-postgres.sh or use 'nix develop --command bash -c \"init_databases\"'${NC}"
    exit 1
fi

echo -e "${GREEN}✓ PostgreSQL is running${NC}"

# Check if RocksDB has been set up
if [ ! -d "data/rocksdb" ]; then
    echo -e "${RED}Error: RocksDB directory not found${NC}"
    echo -e "${YELLOW}Please set up RocksDB first with: ./simulation/databases/setup-rocksdb.sh${NC}"
    exit 1
fi

echo -e "${GREEN}✓ RocksDB is set up${NC}"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --duration=*)
            INDEX_DURATION="${1#*=}"
            ;;
        --help)
            echo -e "Usage: $0 [options]"
            echo -e "Options:"
            echo -e "  --duration=DURATION  Duration to run the indexer (e.g., 5m, 1h, default: ${INDEX_DURATION})"
            echo -e "  --help               Display this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            echo -e "Use --help for usage information"
            exit 1
            ;;
    esac
    shift
done

echo -e "${BLUE}Preparing to index Ethereum contracts for ${INDEX_DURATION}...${NC}"

# Generate indexer configuration for Gateway contract
cat > config.json << EOL
{
  "rpc_urls": ["${ANVIL_URL}"],
  "chain_id": ${ANVIL_CHAIN_ID},
  "starting_block": 1,
  "contract_addresses": {
    "valence_registry": "${VALENCE_REGISTRY_ADDRESS}",
    "valence_gateway": "${VALENCE_GATEWAY_ADDRESS}"
  }
}
EOL

echo -e "${GREEN}✓ Created indexer configuration${NC}"

# Check if almanac-indexer is built in the cargo workspace
if [ -f "target/debug/almanac-indexer" ]; then
    ALMANAC_INDEXER="$(pwd)/target/debug/almanac-indexer"
    echo -e "${GREEN}Using locally built almanac-indexer at ${ALMANAC_INDEXER}${NC}"
elif [ -f "target/release/almanac-indexer" ]; then
    ALMANAC_INDEXER="$(pwd)/target/release/almanac-indexer"
    echo -e "${GREEN}Using locally built almanac-indexer at ${ALMANAC_INDEXER}${NC}"
else
    # Try to build it if it doesn't exist
    echo -e "${YELLOW}almanac-indexer not found. Attempting to build it...${NC}"
    nix develop --command bash -c "cargo build --bin almanac-indexer"
    
    if [ -f "target/debug/almanac-indexer" ]; then
        ALMANAC_INDEXER="$(pwd)/target/debug/almanac-indexer"
        echo -e "${GREEN}Successfully built almanac-indexer at ${ALMANAC_INDEXER}${NC}"
    else
        echo -e "${RED}Error: almanac-indexer could not be built or found.${NC}"
        echo -e "${YELLOW}Please build it manually with: nix develop --command bash -c \"cargo build --bin almanac-indexer\"${NC}"
        exit 1
    fi
fi

# Start almanac-indexer with Ethereum configuration
echo -e "${BLUE}Starting almanac-indexer for Ethereum contracts...${NC}"
nix develop --command bash -c "
    # Clean databases first to ensure fresh indexing
    echo -e \"${BLUE}Cleaning existing index data...${NC}\"
    ${ALMANAC_INDEXER} --config config.json drop-tables
    
    # Start the indexer
    echo -e \"${BLUE}Starting indexer for ${INDEX_DURATION}...${NC}\"
    ${ALMANAC_INDEXER} --config config.json index > ${LOG_FILE} 2>&1 &
    INDEXER_PID=\$!
    
    # Wait for the specified duration
    echo -e \"${GREEN}Indexer started with PID \${INDEXER_PID}, running for ${INDEX_DURATION}...${NC}\"
    sleep ${INDEX_DURATION}
    
    # Stop the indexer
    echo -e \"${BLUE}Stopping indexer...${NC}\"
    kill -SIGINT \${INDEXER_PID} 2>/dev/null || true
    
    # Wait for the indexer to finish gracefully
    wait \${INDEXER_PID} 2>/dev/null || true
    
    # Check indexing status
    tail -n 20 ${LOG_FILE} 2>/dev/null || echo \"No log file found\"
    
    echo -e \"${GREEN}Indexing completed after ${INDEX_DURATION}${NC}\"
" || {
    echo -e "${RED}Error: Failed to run almanac-indexer${NC}"
    exit 1
}

echo -e "${GREEN}✓ Indexing of Ethereum Valence contracts on Anvil completed${NC}"
echo -e "${BLUE}Log file available at: ${LOG_FILE}${NC}"
echo -e "${BLUE}To query the indexed data, you can use the almanac-indexer query CLI or connect directly to PostgreSQL${NC}"
echo -e "${BLUE}=== Indexing Complete ===${NC}" 