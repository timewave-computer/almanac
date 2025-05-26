# Almanac Simulation Scripts

This directory contains scripts for setting up and running Almanac with various blockchain backends for development and testing purposes.

## Quick Start

To start all services:

```bash
./simulation/start-all.sh
```

To stop all services:

```bash
./simulation/stop-all.sh
```

## Directory Structure

- `databases/`: Database setup scripts for PostgreSQL and RocksDB
- `ethereum/`: Ethereum node setup and contract deployment scripts
- `cosmos/`: CosmWasm node setup and contract deployment scripts
- `start-all.sh`: Start all services
- `stop-all.sh`: Stop all services
- `make-scripts-executable.sh`: Make all scripts executable

## Available Workflows

### Ethereum with Anvil

1. Set up an Anvil node:
   ```bash
   ./simulation/ethereum/setup-anvil.sh
   ```

2. Deploy Valence contracts to Anvil:
   ```bash
   ./simulation/ethereum/deploy-valence-contracts-anvil.sh
   ```

### Ethereum with Reth

1. Set up a Reth node:
   ```bash
   ./simulation/ethereum/setup-reth.sh
   ```

2. Deploy Valence contracts to Reth:
   ```bash
   ./simulation/ethereum/deploy-valence-contracts-reth.sh
   ```

### CosmWasm with wasmd

1. Set up a wasmd node:
   ```bash
   ./simulation/cosmos/setup-wasmd.sh
   ```

2. Deploy Valence contracts to wasmd:
   ```bash
   ./simulation/cosmos/deploy-valence-contracts-wasmd.sh
   ```

## Database Management

- Set up PostgreSQL:
  ```bash
  ./simulation/databases/setup-postgres.sh
  ```

- Create PostgreSQL tables:
  ```bash
  ./simulation/databases/create-postgres-tables.sh
  ```

- Set up RocksDB:
  ```bash
  ./simulation/databases/setup-rocksdb.sh
  ```

- Reset all databases:
  ```bash
  ./simulation/databases/reset-databases.sh
  ```

## Notes

- All services will run in the background
- Log files are stored in the `logs/` directory
- Contract addresses are stored in:
  - `data/contracts/ethereum/anvil/contract-addresses.env`
  - `data/contracts/ethereum/reth/contract-addresses.env`
  - `data/contracts/cosmos/wasmd/contract-addresses.env`
- Make sure you have PostgreSQL, Foundry (for Anvil), and optionally Reth and wasmd installed
- The scripts will attempt to use mock implementations if the real binaries are not found

## Troubleshooting

If you encounter issues:

1. Check the log files in the `logs/` directory
2. Make sure all dependencies are installed
3. Try running `simulation/stop-all.sh` and then restart the services
4. For database issues, try `simulation/databases/reset-databases.sh` 