# Almanac Examples

This directory contains example applications demonstrating how to use the Almanac indexer.

## Available Examples

### Basic Indexer

`basic_indexer.rs` - A simple example showing how to set up a cross-chain indexer using Almanac.

This example uses MemoryStorage instead of RocksDB or PostgreSQL to avoid compilation issues that currently exist in the storage implementations.

## Using Examples

The examples in this directory are provided as reference implementations rather than directly runnable applications. Due to compilation issues in the current codebase, attempting to run these examples directly may fail.

To use these examples effectively:

1. Review the code to understand the architecture and patterns
2. Copy relevant sections into your own projects
3. Use MemoryStorage implementation rather than RocksDB or PostgreSQL until the storage issues are resolved

## Known Issues

The main codebase currently has compilation errors in:

1. RocksDB implementation - Functions incorrectly defined in a struct definition 
2. Storage trait implementations - Missing required method implementations
3. PostgreSQL implementation - SQL query errors due to missing database connection

The examples have been designed to work around these issues by using the MemoryStorage implementation which is fully functional. 