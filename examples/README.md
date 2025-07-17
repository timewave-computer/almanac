# Almanac Examples

This directory contains example applications demonstrating how to use the Almanac indexer.

## Available Examples

### Basic Indexer

`basic_indexer.rs` - A simple example showing how to set up a cross-chain indexer using Almanac.

This example demonstrates:
- Setting up chain adapters for Ethereum and Cosmos
- Configuring storage backends (Memory, RocksDB, PostgreSQL)
- Processing events from multiple chains
- Basic querying and data retrieval

## Running Examples

All examples can be run directly using cargo:

```bash
# Run the basic indexer example
cargo run --example basic_indexer
```

## Storage Options

Examples support multiple storage backends:

1. **Memory Storage** - Fast, in-memory storage for development and testing
2. **RocksDB Storage** - High-performance persistent storage for production
3. **PostgreSQL Storage** - Rich querying capabilities for complex analysis

## Prerequisites

For examples using PostgreSQL:
- Ensure PostgreSQL is running (use `init_databases` in the nix shell)
- Set `DATABASE_URL` environment variable if using custom connection

For examples using RocksDB:
- No additional setup required - RocksDB is embedded

## Development

To create new examples:

1. Add your example file to the `examples/` directory
2. Use the existing examples as templates
3. Test with different storage backends
4. Document any specific requirements or setup steps 