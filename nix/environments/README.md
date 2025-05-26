# Almanac Workflow Environments

This directory contains Nix flakes for different development environments that can be used to run Almanac with various blockchain backends.

## Available Workflows

- **Anvil Workflow**: Sets up an Ethereum development environment using Anvil
- **Reth Workflow**: Sets up an Ethereum development environment using Reth
- **CosmWasm Workflow**: Sets up a CosmWasm development environment using wasmd
- **All Workflows**: Runs all workflows in sequence

## Using the Workflow Menu

The main flake provides a menu to select which workflow to run:

```bash
cd nix/environments
nix run .
```

This will display a menu where you can select which workflow to run.

## Running Specific Workflows Directly

If you want to run a specific workflow directly, you can use one of the following commands:

```bash
# Anvil workflow
nix run .#anvil-workflow

# Reth workflow
nix run .#reth-workflow

# CosmWasm workflow
nix run .#cosmwasm-workflow

# All workflows in sequence
nix run .#all-workflows
```

## Development Shell

You can also enter a development shell with all workflow tools available:

```bash
cd nix/environments
nix develop
```

Inside the development shell, you can run any of the workflow scripts directly:

```bash
# Run the workflow menu
workflow-menu

# Or run specific workflows
anvil-workflow
reth-workflow
cosmwasm-workflow
all-workflows
```

## Mocked vs. Real Components

Some components are provided in both real and mocked versions:

### CosmWasm (wasmd)

The CosmWasm workflow supports both real and mocked implementations of the wasmd binary:

- **Real wasmd**: On Linux or x86_64 systems, it attempts to use a real wasmd binary built from source
- **Mock wasmd**: On Apple Silicon Macs, it defaults to a mock implementation that simulates wasmd commands

The workflow automatically detects which implementation is available and adjusts its behavior accordingly. The mock implementation is primarily for development and testing without requiring the full wasmd binary to be built.

If you want to force using the real wasmd binary, you can build it directly:

```bash
nix build ./cosmwasm#real-wasmd
```

### Reth

The Reth workflow currently uses a mock implementation that simulates the basic Ethereum node functionality.

## Troubleshooting

If a workflow doesn't complete successfully, check the respective log files:
- Anvil logs: `/Users/hxrts/projects/timewave/almanac/logs/anvil.log`
- Reth logs: `/Users/hxrts/projects/timewave/almanac/logs/reth.log`
- wasmd logs: `/Users/hxrts/projects/timewave/almanac/logs/wasmd.log`
- PostgreSQL logs: `/Users/hxrts/projects/timewave/almanac/data/postgres/postgres.log` 