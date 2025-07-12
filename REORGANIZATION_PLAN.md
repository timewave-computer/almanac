# Almanac Repository Reorganization Plan

## Overview

This document outlines the reorganization of the Almanac repository to create more cohesive submodules with clear separation of concerns.

## Current Issues

1. **Monolithic Core**: The `core` crate contains 30+ modules with mixed responsibilities
2. **Scattered Distributed Logic**: Distributed functionality spread across multiple crates
3. **Mixed API Styles**: REST, GraphQL, and WebSocket APIs mixed in one crate
4. **Custom Storage Implementation**: Custom RocksDB FFI and storage proof implementation
5. **Duplicated Codegen**: Similar code generation logic in ethereum and cosmos crates

## New Structure

```
almanac/
├── crates/
│   ├── core/                    # Core types, traits, and errors only
│   ├── distributed/             # All distributed computing functionality
│   │   ├── src/
│   │   │   ├── cache.rs
│   │   │   ├── coordination.rs
│   │   │   ├── correlation.rs
│   │   │   ├── event_service.rs
│   │   │   ├── gateway.rs
│   │   │   ├── monitoring.rs
│   │   │   ├── query_planner.rs
│   │   │   └── fdb/            # FoundationDB implementations
│   │   └── tests/
│   ├── pipeline/               # Data processing pipeline
│   │   └── src/
│   │       ├── batch_processor.rs
│   │       ├── event_ordering.rs
│   │       ├── finality_processor.rs
│   │       ├── ingestion_queue.rs
│   │       ├── pipeline.rs
│   │       ├── reorg_handler.rs
│   │       └── sync_tracker.rs
│   ├── observability/          # Monitoring, metrics, and logging
│   │   └── src/
│   │       ├── health.rs
│   │       ├── logging.rs
│   │       ├── metrics.rs
│   │       ├── observability.rs
│   │       └── prometheus_exporter.rs
│   ├── validation/             # Data validation
│   │   └── src/
│   │       └── validation.rs
│   ├── storage-core/           # Storage traits and common logic
│   ├── storage-postgres/       # PostgreSQL implementation
│   ├── storage-rocks/          # RocksDB implementation
│   ├── storage-memory/         # In-memory implementation
│   ├── storage-proof/          # Storage proofs using traverse
│   ├── api-core/              # Shared API logic and auth
│   ├── api-rest/              # REST API endpoints
│   ├── api-graphql/           # GraphQL API
│   ├── api-streaming/         # WebSocket and streaming APIs
│   ├── cross-chain/           # Cross-chain functionality
│   ├── data-services/         # Data processing services
│   ├── codegen/               # Unified code generation
│   ├── ethereum/              # Ethereum-specific logic
│   ├── cosmos/                # Cosmos-specific logic
│   ├── causality/             # Already well-organized
│   ├── query/                 # Query execution
│   └── testing/               # Consolidated testing
│       ├── benchmarks/
│       ├── integration/
│       ├── e2e/
│       └── fixtures/
```

## Key Changes

### 1. Traverse Integration
- Remove custom storage proof implementation
- Integrate `traverse` library from https://github.com/timewave-computer/traverse
- Create new `storage-proof` crate using traverse

### 2. Distributed Module Consolidation
- Move all distributed functionality to `distributed` crate
- Clear separation between local and distributed operations
- FDB-specific code in submodule

### 3. API Modularization
- Split APIs by protocol (REST, GraphQL, Streaming)
- Shared logic in `api-core`
- Each API type can be deployed independently

### 4. Storage Abstraction
- Clean separation between trait definitions and implementations
- Each storage backend in its own crate
- Remove custom RocksDB FFI

### 5. Pipeline Extraction
- All data processing pipeline logic in dedicated crate
- Clear data flow through the system
- Easier to test and modify

## Migration Steps

1. **Run Scripts** (in order):
   ```bash
   chmod +x scripts/*.sh
   ./scripts/remove_custom_storage.sh
   ./scripts/reorganize_repo.sh
   ./scripts/move_files.sh
   ./scripts/update_imports.sh
   ```

2. **Fix Compilation**:
   ```bash
   cargo check --all-features
   # Fix any errors
   ```

3. **Run Tests**:
   ```bash
   cargo test --all-features
   ```

4. **Update Documentation**:
   - Update README with new structure
   - Update contribution guidelines
   - Update API documentation

## Benefits

1. **Modularity**: Each crate has a single, well-defined purpose
2. **Maintainability**: Easier to find and modify related code
3. **Scalability**: New features as separate crates
4. **Performance**: Better incremental compilation
5. **Flexibility**: Mix and match components as needed
6. **Clarity**: Clear dependency graph

## Dependency Graph

```
                    ┌─────────────┐
                    │    core     │
                    └──────┬──────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼────┐      ┌─────▼──────┐    ┌─────▼──────┐
   │pipeline │      │observability    │validation  │
   └─────────┘      └────────────┘    └────────────┘
        │                  │                  │
        └──────────────────┼──────────────────┘
                           │
                    ┌──────▼──────┐
                    │storage-core │
                    └──────┬──────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼────┐      ┌─────▼──────┐    ┌─────▼──────┐
   │postgres │      │   rocks    │    │  memory    │
   └─────────┘      └────────────┘    └────────────┘
                           │
                    ┌──────▼──────┐
                    │distributed  │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │  api-core   │
                    └──────┬──────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
   ┌────▼────┐      ┌─────▼──────┐    ┌─────▼──────┐
   │api-rest │      │api-graphql │    │api-streaming
   └─────────┘      └────────────┘    └────────────┘
```

## Timeline

- **Phase 1** (Day 1): Remove custom storage, create structure
- **Phase 2** (Day 2): Move files, update imports
- **Phase 3** (Day 3): Fix compilation errors
- **Phase 4** (Day 4): Run and fix tests
- **Phase 5** (Day 5): Update documentation

## Notes

- All scripts are idempotent and can be run multiple times
- Create a backup before starting (`cp -r . ../almanac-backup`)
- The reorganization maintains all functionality
- No breaking changes to external APIs