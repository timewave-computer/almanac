#!/bin/bash
# Purpose: Populate RocksDB with sample data for testing

set -e

# Define colors for output
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
ROCKSDB_DATA_DIR="data/rocksdb"
SAMPLE_DATA_DIR="data/sample"

echo -e "${BLUE}=== Populating RocksDB with Sample Data ===${NC}"

# Check if the RocksDB directory exists
if [ ! -d "${ROCKSDB_DATA_DIR}" ]; then
    echo -e "${RED}Error: RocksDB directory not found at ${ROCKSDB_DATA_DIR}${NC}"
    echo -e "${YELLOW}Please set up RocksDB first with: ./simulation/databases/setup-rocksdb.sh${NC}"
    exit 1
fi

# Create sample data directory if it doesn't exist
mkdir -p ${SAMPLE_DATA_DIR}

# Create sample data generation script in Python
cat > ${SAMPLE_DATA_DIR}/generate_sample_data.py << 'EOL'
#!/usr/bin/env python3
"""
Generate sample data for Almanac RocksDB
"""
import json
import os
import random
import sys
import uuid
from datetime import datetime, timedelta

# Configuration
NUM_EVENTS = 100
NUM_BLOCKS = 20
NUM_CONTRACTS = 5
OUTPUT_DIR = sys.argv[1] if len(sys.argv) > 1 else "data/sample"

# Ensure output directory exists
os.makedirs(OUTPUT_DIR, exist_ok=True)

# Generate random Ethereum addresses
def generate_eth_address():
    return "0x" + "".join(random.choice("0123456789abcdef") for _ in range(40))

# Generate random contract ABIs
def generate_contract_abi():
    event_types = ["LogEvent", "Transfer", "Approval", "Deposit", "Withdrawal"]
    methods = ["execute", "transfer", "approve", "deposit", "withdraw"]
    
    abi = []
    # Add random events
    for _ in range(random.randint(2, 5)):
        event_name = random.choice(event_types)
        inputs = []
        for j in range(random.randint(1, 4)):
            input_type = random.choice(["address", "uint256", "bool", "string"])
            input_name = f"param{j}"
            inputs.append({"indexed": random.choice([True, False]), "name": input_name, "type": input_type})
        
        abi.append({
            "anonymous": False,
            "inputs": inputs,
            "name": event_name,
            "type": "event"
        })
    
    # Add random methods
    for _ in range(random.randint(2, 5)):
        method_name = random.choice(methods)
        inputs = []
        for j in range(random.randint(0, 3)):
            input_type = random.choice(["address", "uint256", "bool", "string"])
            input_name = f"param{j}"
            inputs.append({"name": input_name, "type": input_type})
        
        outputs = []
        for j in range(random.randint(0, 2)):
            output_type = random.choice(["address", "uint256", "bool", "string"])
            outputs.append({"name": "", "type": output_type})
        
        abi.append({
            "inputs": inputs,
            "name": method_name,
            "outputs": outputs,
            "stateMutability": random.choice(["view", "nonpayable", "payable"]),
            "type": "function"
        })
    
    return abi

# Generate random blocks
def generate_blocks(num_blocks):
    blocks = []
    timestamp = int(datetime.now().timestamp())
    
    for i in range(1, num_blocks + 1):
        block = {
            "block_id": i,
            "block_hash": "0x" + "".join(random.choice("0123456789abcdef") for _ in range(64)),
            "block_number": i,
            "block_timestamp": timestamp,
            "chain_id": 31337,  # Anvil chain ID
            "parent_hash": "0x" + "".join(random.choice("0123456789abcdef") for _ in range(64))
        }
        blocks.append(block)
        timestamp += random.randint(10, 20)  # Increase timestamp by 10-20 seconds
    
    return blocks

# Generate random contracts
def generate_contracts(num_contracts):
    contracts = []
    contract_types = ["ERC20", "ERC721", "Registry", "Gateway", "Processor"]
    
    for i in range(1, num_contracts + 1):
        contract_address = generate_eth_address()
        contract_type = random.choice(contract_types)
        
        contract = {
            "contract_id": str(uuid.uuid4()),
            "address": contract_address,
            "chain_id": 31337,  # Anvil chain ID
            "contract_type": contract_type,
            "abi": generate_contract_abi(),
            "created_at": int(datetime.now().timestamp())
        }
        contracts.append(contract)
    
    return contracts

# Generate random events
def generate_events(num_events, blocks, contracts):
    events = []
    event_types = ["Transfer", "Approval", "LogEvent", "Deposit", "Withdrawal"]
    
    for i in range(1, num_events + 1):
        block = random.choice(blocks)
        contract = random.choice(contracts)
        event_type = random.choice(event_types)
        
        # Generate random parameters based on event type
        params = {}
        if event_type == "Transfer":
            params = {
                "from": generate_eth_address(),
                "to": generate_eth_address(),
                "value": str(random.randint(1, 1000000))
            }
        elif event_type == "Approval":
            params = {
                "owner": generate_eth_address(),
                "spender": generate_eth_address(),
                "value": str(random.randint(1, 1000000))
            }
        elif event_type == "Deposit":
            params = {
                "user": generate_eth_address(),
                "amount": str(random.randint(1, 1000000))
            }
        elif event_type == "Withdrawal":
            params = {
                "user": generate_eth_address(),
                "amount": str(random.randint(1, 1000000))
            }
        else:  # LogEvent
            params = {
                "sender": generate_eth_address(),
                "message": f"Log message {i}",
                "value": str(random.randint(1, 1000))
            }
        
        event = {
            "event_id": str(uuid.uuid4()),
            "block_id": block["block_id"],
            "transaction_hash": "0x" + "".join(random.choice("0123456789abcdef") for _ in range(64)),
            "transaction_index": random.randint(0, 10),
            "event_index": random.randint(0, 5),
            "contract_id": contract["contract_id"],
            "event_type": event_type,
            "parameters": json.dumps(params),
            "raw_data": "0x" + "".join(random.choice("0123456789abcdef") for _ in range(200)),
            "created_at": block["block_timestamp"]
        }
        events.append(event)
    
    return events

# Main function to generate all data
def generate_all_data():
    print(f"Generating {NUM_BLOCKS} blocks...")
    blocks = generate_blocks(NUM_BLOCKS)
    
    print(f"Generating {NUM_CONTRACTS} contracts...")
    contracts = generate_contracts(NUM_CONTRACTS)
    
    print(f"Generating {NUM_EVENTS} events...")
    events = generate_events(NUM_EVENTS, blocks, contracts)
    
    # Write data to files
    with open(os.path.join(OUTPUT_DIR, "blocks.json"), "w") as f:
        json.dump(blocks, f, indent=2)
    
    with open(os.path.join(OUTPUT_DIR, "contracts.json"), "w") as f:
        json.dump(contracts, f, indent=2)
    
    with open(os.path.join(OUTPUT_DIR, "events.json"), "w") as f:
        json.dump(events, f, indent=2)
    
    print(f"Sample data generated successfully in {OUTPUT_DIR}")
    print(f"- {len(blocks)} blocks")
    print(f"- {len(contracts)} contracts")
    print(f"- {len(events)} events")

if __name__ == "__main__":
    generate_all_data()
EOL

# Make the script executable
chmod +x ${SAMPLE_DATA_DIR}/generate_sample_data.py

# Create RocksDB import script in Python
cat > ${SAMPLE_DATA_DIR}/import_to_rocksdb.py << 'EOL'
#!/usr/bin/env python3
"""
Import sample data into Almanac RocksDB
"""
import json
import os
import sys
import rocksdb
import binascii

# Configuration
SAMPLE_DATA_DIR = sys.argv[1] if len(sys.argv) > 1 else "data/sample"
ROCKSDB_DATA_DIR = sys.argv[2] if len(sys.argv) > 2 else "data/rocksdb"

def open_rocksdb():
    """Open RocksDB database"""
    opts = rocksdb.Options()
    opts.create_if_missing = True
    opts.max_open_files = 300000
    opts.write_buffer_size = 67108864
    opts.max_write_buffer_number = 3
    opts.target_file_size_base = 67108864
    opts.table_factory = rocksdb.BlockBasedTableFactory(
        filter_policy=rocksdb.BloomFilterPolicy(10),
        block_cache=rocksdb.LRUCache(2 * (1024 ** 3)),
        block_cache_compressed=rocksdb.LRUCache(500 * (1024 ** 2)))
    
    try:
        db = rocksdb.DB(ROCKSDB_DATA_DIR, opts)
        return db
    except Exception as e:
        print(f"Error opening RocksDB: {e}")
        sys.exit(1)

def import_blocks(db):
    """Import blocks into RocksDB"""
    blocks_file = os.path.join(SAMPLE_DATA_DIR, "blocks.json")
    if not os.path.exists(blocks_file):
        print(f"Error: {blocks_file} does not exist")
        return
    
    with open(blocks_file, 'r') as f:
        blocks = json.load(f)
    
    batch = rocksdb.WriteBatch()
    for block in blocks:
        # Convert block to binary format for RocksDB
        block_id = str(block["block_id"]).encode()
        block_data = json.dumps(block).encode()
        
        # Create keys for different indexes
        block_id_key = f"block:id:{block['block_id']}".encode()
        block_hash_key = f"block:hash:{block['block_hash']}".encode()
        block_number_key = f"block:number:{block['block_number']}".encode()
        
        # Add to batch
        batch.put(block_id_key, block_data)
        batch.put(block_hash_key, block_id)
        batch.put(block_number_key, block_id)
    
    # Write batch to database
    db.write(batch)
    print(f"Imported {len(blocks)} blocks into RocksDB")

def import_contracts(db):
    """Import contracts into RocksDB"""
    contracts_file = os.path.join(SAMPLE_DATA_DIR, "contracts.json")
    if not os.path.exists(contracts_file):
        print(f"Error: {contracts_file} does not exist")
        return
    
    with open(contracts_file, 'r') as f:
        contracts = json.load(f)
    
    batch = rocksdb.WriteBatch()
    for contract in contracts:
        # Convert contract to binary format for RocksDB
        contract_id = contract["contract_id"].encode()
        contract_data = json.dumps(contract).encode()
        
        # Create keys for different indexes
        contract_id_key = f"contract:id:{contract['contract_id']}".encode()
        contract_address_key = f"contract:address:{contract['address']}:{contract['chain_id']}".encode()
        
        # Add to batch
        batch.put(contract_id_key, contract_data)
        batch.put(contract_address_key, contract_id)
    
    # Write batch to database
    db.write(batch)
    print(f"Imported {len(contracts)} contracts into RocksDB")

def import_events(db):
    """Import events into RocksDB"""
    events_file = os.path.join(SAMPLE_DATA_DIR, "events.json")
    if not os.path.exists(events_file):
        print(f"Error: {events_file} does not exist")
        return
    
    with open(events_file, 'r') as f:
        events = json.load(f)
    
    batch = rocksdb.WriteBatch()
    for event in events:
        # Convert event to binary format for RocksDB
        event_id = event["event_id"].encode()
        event_data = json.dumps(event).encode()
        
        # Create keys for different indexes
        event_id_key = f"event:id:{event['event_id']}".encode()
        event_contract_key = f"event:contract:{event['contract_id']}:{event['event_type']}:{event['event_id']}".encode()
        event_block_key = f"event:block:{event['block_id']}:{event['event_id']}".encode()
        event_tx_key = f"event:tx:{event['transaction_hash']}:{event['event_index']}".encode()
        
        # Add to batch
        batch.put(event_id_key, event_data)
        batch.put(event_contract_key, event_id)
        batch.put(event_block_key, event_id)
        batch.put(event_tx_key, event_id)
    
    # Write batch to database
    db.write(batch)
    print(f"Imported {len(events)} events into RocksDB")

def main():
    print(f"Opening RocksDB at {ROCKSDB_DATA_DIR}...")
    db = open_rocksdb()
    
    print(f"Importing data from {SAMPLE_DATA_DIR}...")
    import_blocks(db)
    import_contracts(db)
    import_events(db)
    
    print("Data import completed successfully!")

if __name__ == "__main__":
    main()
EOL

# Make the script executable
chmod +x ${SAMPLE_DATA_DIR}/import_to_rocksdb.py

# Check if Python is available in the Nix environment
echo -e "${BLUE}Checking for Python in Nix environment...${NC}"
if ! nix develop --command bash -c "command -v python3" > /dev/null 2>&1; then
    echo -e "${RED}Error: Python3 not available in Nix environment${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Python3 is available in Nix environment${NC}"

# Ensure RocksDB directory has correct permissions
echo -e "${BLUE}Ensuring RocksDB directory has correct permissions...${NC}"
chmod -R 755 "${ROCKSDB_DATA_DIR}"
echo -e "${GREEN}✓ Permissions set for RocksDB directory${NC}"

# Generate sample data
echo -e "${BLUE}Generating sample data...${NC}"
nix develop --command bash -c "
    # Check if required Python packages are available
    if ! python3 -c 'import uuid, json, os, random, sys, datetime' 2>/dev/null; then
        echo -e \"${RED}Error: Required Python packages not available${NC}\"
        exit 1
    fi
    
    # Generate sample data
    python3 ${SAMPLE_DATA_DIR}/generate_sample_data.py ${SAMPLE_DATA_DIR}
"

if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Failed to generate sample data${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Sample data generated successfully${NC}"

# Import data into RocksDB
echo -e "${BLUE}Importing data into RocksDB...${NC}"
nix develop --command bash -c "
    # Check if rocksdb Python package is available
    if ! python3 -c 'import rocksdb' 2>/dev/null; then
        echo -e \"${YELLOW}Warning: Python rocksdb package not available, attempting to install...${NC}\"
        pip install python-rocksdb
        
        # Check again after install attempt
        if ! python3 -c 'import rocksdb' 2>/dev/null; then
            echo -e \"${RED}Error: Failed to install Python rocksdb package${NC}\"
            echo -e \"${YELLOW}Please install manually with: pip install python-rocksdb${NC}\"
            exit 1
        fi
    fi
    
    # Import data into RocksDB
    python3 ${SAMPLE_DATA_DIR}/import_to_rocksdb.py ${SAMPLE_DATA_DIR} ${ROCKSDB_DATA_DIR}
"

if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Failed to import data into RocksDB${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Data imported into RocksDB successfully${NC}"

# Verify data was imported
echo -e "${BLUE}Verifying data in RocksDB...${NC}"
nix develop --command bash -c "
    # Create a simple verification script
    cat > ${SAMPLE_DATA_DIR}/verify_rocksdb.py << 'EOF'
import rocksdb
import sys

ROCKSDB_DATA_DIR = '${ROCKSDB_DATA_DIR}'

opts = rocksdb.Options()
opts.create_if_missing = False
db = rocksdb.DB(ROCKSDB_DATA_DIR, opts)

# Count entries in the database
count = 0
it = db.iterkeys()
it.seek_to_first()

for _ in it:
    count += 1

print(f'RocksDB contains approximately {count} entries')

# Try to retrieve a few keys by prefix
prefixes = [b'block:id:', b'contract:id:', b'event:id:']
for prefix in prefixes:
    it = db.iteritems()
    it.seek(prefix)
    
    found = False
    for i, (key, value) in enumerate(it):
        if not key.startswith(prefix):
            break
        found = True
        print(f'Found entry with prefix {prefix.decode()}: {key.decode()}')
        if i >= 2:  # Just show a few examples
            break
    
    if not found:
        print(f'No entries found with prefix {prefix.decode()}')

EOF

    # Run verification script
    python3 ${SAMPLE_DATA_DIR}/verify_rocksdb.py
"

if [ $? -ne 0 ]; then
    echo -e "${RED}Error: Failed to verify data in RocksDB${NC}"
    exit 1
fi

echo -e "${GREEN}✓ RocksDB has been populated with sample data${NC}"
echo -e "${GREEN}✓ Sample data files are available in ${SAMPLE_DATA_DIR}${NC}"
echo -e "${BLUE}=== RocksDB Population Complete ===${NC}" 