# Infrastructure Reorganization Update

## Current Project Structure

The Almanac project has been reorganized into a hierarchical modular architecture that provides better separation of concerns, improved maintainability, and clearer development workflows.

### Complete Project Structure

```
almanac/
├── crates/
│   ├── infrastructure/          # Core infrastructure layer
│   │   ├── core/               # Essential types and traits only
│   │   ├── storage/            # Storage abstractions (PostgreSQL + RocksDB)

│   │   ├── coordination/       # Distributed coordination services
│   │   ├── query/              # Query planning and execution
│   │   ├── monitoring/         # Monitoring and observability
│   │   ├── caching/            # Distributed caching layer
│   │   └── chains/             # Shared chain logic
│   ├── services/               # Service implementations
│   │   ├── api/                # API service (HTTP, WebSocket)
│   │   ├── query/              # Query service
│   │   └── indexing/           # Indexing service
│   ├── chains/                 # Chain-specific code
│   │   ├── cosmos/             # Cosmos integration with codegen
│   │   └── ethereum/           # Ethereum integration with codegen
│   ├── development/            # Development tools and testing
│   │   ├── benchmarks/         # Performance testing
│   │   ├── testing/            # Comprehensive testing framework
│   │   └── tools/              # CLI tools and utilities
│   ├── api-core/               # Core API types and traits
│   └── api-http/               # HTTP API server components
├── e2e/                        # End-to-end integration tests
├── docs/                       # Documentation
├── examples/                   # Example integrations
├── scripts/                    # Development and deployment scripts
├── simulation/                 # Blockchain simulation tools
├── nix/                        # Nix configuration and modules
└── config/                     # Configuration files
```

### Crate Organization

#### Infrastructure Layer (`crates/infrastructure/`)
Core system components that provide foundational services:

- **`almanac-infrastructure-core`**: Essential types, traits, error handling, and cross-chain coordination
- **`almanac-infrastructure-storage`**: Dual storage backends (PostgreSQL + RocksDB) with unified trait interface

- **`almanac-infrastructure-coordination`**: Distributed coordination services for multi-node deployments
- **`almanac-infrastructure-query`**: Query planning and execution engine
- **`almanac-infrastructure-monitoring`**: Monitoring and observability infrastructure
- **`almanac-infrastructure-caching`**: Distributed caching layer with compression
- **`almanac-infrastructure-chains`**: Shared chain-specific logic and abstractions

#### Services Layer (`crates/services/`)
User-facing services built on the infrastructure layer:

- **`indexer-api`**: Main API service providing HTTP REST and WebSocket endpoints
- **`almanac-services-query`**: Query optimization and aggregation service
- **`almanac-services-indexing`**: Event indexing and processing service

#### Chains Layer (`crates/chains/`)
Blockchain-specific integrations:

- **`indexer-ethereum`**: Ethereum/EVM chain integration with valence-domain-clients
- **`indexer-cosmos`**: Cosmos SDK chain integration with valence-domain-clients

#### Development Layer (`crates/development/`)
Development and testing tools:

- **`indexer-benchmarks`**: Performance testing and load testing infrastructure
- **`indexer-testing`**: Comprehensive testing framework with chaos engineering
- **`indexer-tools`**: Development utilities and CLI tools

#### API Layer (`crates/api-*/`)
Specialized API components:

- **`almanac-api-core`**: Core API types and traits
- **`almanac-api-http`**: HTTP API server components

## Key Benefits

### 1. **Improved Maintainability**
- Clear separation of concerns with well-defined module boundaries
- Easier to understand and modify individual components
- Reduced coupling between different system layers

### 2. **Enhanced Testability**
- Each module can be tested independently
- Infrastructure components can be mocked for service testing
- Better isolation of test failures

### 3. **Better Developer Experience**
- Cleaner import paths and dependencies
- Easier to navigate and understand the codebase
- Consistent naming conventions with `almanac-` prefix

### 4. **Scalability**
- Infrastructure layer can be scaled independently
- Services can be deployed separately
- Clear boundaries enable horizontal scaling

### 5. **Flexibility**
- Easy to add new chains without affecting core infrastructure
- Services can be composed differently for different deployment scenarios
- Infrastructure components can be reused across different services

## Migration Status

### Completed Changes

1. **Crate Reorganization**: All crates have been moved to their appropriate layers
2. **Import Updates**: All internal imports have been updated to use new crate names
3. **Workspace Configuration**: `Cargo.toml` workspace configuration updated
4. **Build System**: Nix flake updated to reflect new structure
5. **Documentation**: Core documentation updated to reflect new organization

### Current Crate Mapping

| Old Name | New Name | Location |
|----------|----------|----------|
| `indexer-core` | `almanac-infrastructure-core` | `crates/infrastructure/core` |
| `indexer-storage` | `almanac-infrastructure-storage` | `crates/infrastructure/storage` |

| `indexer-api` | `indexer-api` | `crates/services/api` |
| `indexer-ethereum` | `indexer-ethereum` | `crates/chains/ethereum` |
| `indexer-cosmos` | `indexer-cosmos` | `crates/chains/cosmos` |
| `indexer-tools` | `indexer-tools` | `crates/development/tools` |
| `indexer-benchmarks` | `indexer-benchmarks` | `crates/development/benchmarks` |

### Development Workflow

The new structure supports several development workflows:

1. **Infrastructure Development**: Focus on core components in `crates/infrastructure/`
2. **Service Development**: Build services in `crates/services/` using infrastructure
3. **Chain Integration**: Add new chains in `crates/chains/`
4. **API Development**: Extend APIs in `crates/api-*/` and `crates/services/api/`

## Usage Examples

### Working with Infrastructure

```rust
// Core types and traits
use almanac_infrastructure_core::{Error, Result, Event};

// Storage layer
use almanac_infrastructure_storage::{Storage, create_postgres_storage};


```

### Building Services

```rust
// API service
use indexer_api::{ApiServer, ApiConfig};

// Chain clients
use indexer_ethereum::EthereumClient;
use indexer_cosmos::CosmosClientWrapper;
```

### Development Tools

```rust
// Testing framework
use indexer_testing::{TestRunner, ChaosTestRunner};

// Benchmarking
use indexer_benchmarks::{BenchmarkRunner, LoadTestRunner};
```

## Future Enhancements

The hierarchical structure enables several future enhancements:

1. **Microservices Architecture**: Services can be deployed independently
2. **Plugin System**: New chains can be added as plugins
3. **Distributed Deployment**: Infrastructure components can be distributed across nodes
4. **Configuration Management**: Centralized configuration for all components
5. **Monitoring Integration**: Unified monitoring across all layers

## Build and Development

The reorganization maintains compatibility with existing workflows:

```bash
# Build all components
cargo build

# Build specific layers
cargo build -p almanac-infrastructure-core
cargo build -p indexer-api

# Run tests
cargo test

# Development environment
nix develop
```

## Conclusion

The infrastructure reorganization provides a solid foundation for the Almanac project's continued growth and development. The hierarchical structure improves maintainability, testability, and developer experience while enabling future enhancements and scaling capabilities.

The migration is complete and the workspace builds successfully with the new structure, providing a clear path forward for development and deployment.