# FoundationDB Implementation Status

This document summarizes the FoundationDB implementation for the Almanac project based on the work documents.

## Completed

### 1. Solana Support
- Created Solana chain adapter: `/crates/almanac-chains/src/adapters/solana.rs`
- Created Solana node binary: `/crates/nodes/almanac-solana/`
- Implements slot-based indexing with account data extraction

### 2. FDB Storage Structure
- Created comprehensive tuple layer: `/crates/almanac-storage/src/fdb/`
- Implemented all keyspaces from work documents:
  - `ContractState` - Contract storage slots
  - `BlockMeta` - Block headers and consensus data
  - `StorageProofs` - Merkle proofs for verification
  - `SemanticMeta` - Trust-scoped semantic interpretations
  - `TrustGraph` - Developer trust relationships
  - `ContractLayouts` - Layout commitments from Traverse
  - `ViewRegistry` - Custom view definitions
  - `LightwaveState` - Recursive proof coordination
  - `TrustedCheckpoints` - Network checkpoints
  - `CircuitKeys` - SP1 verification keys
  - `SolanaAccounts` - Solana account data
  - `ChangeTracking` - State change logs

### 3. Type Modules Created
- `trust.rs` - Trust management types (TrustScope, TrustLevel, TrustPolicy)
- `semantic.rs` - Semantic metadata with conflict detection
- `layout.rs` - Contract layout commitments
- `view_registry.rs` - View definitions and query results
- `storage_proof.rs` - Storage proof types

### 4. FDB Implementation
- Created `impl_actual.rs` with full FoundationDB integration
- Implements tuple encoding/decoding using foundationdb-rs crate
- Trust-aware semantic metadata filtering
- Layout conflict detection
- Developer trust graph management

### 5. API Integration
- Created `FdbStorageExt` that implements `ExtendedStorageReader`
- Created `api_adapter.rs` to bridge storage and API layers
- All storage methods properly typed and structured

## Existing but Needs Connection

### API Handlers
Located in `/crates/almanac-api/src/handlers/`:
- `semantics.rs` - Endpoints exist but return TODO
- `trust.rs` - Endpoints exist but return TODO  
- `layout.rs` - More complete, references storage backend
- `views.rs` - Endpoints exist but return TODO

These handlers need to be connected to the FDB storage backend.

## Next Steps

1. **Resolve Dependency Conflicts**
   - Current conflict between bcrypt (API) and solana (chains) over zeroize version
   - Consider updating bcrypt or isolating solana dependencies

2. **Connect API to Storage**
   - Update API server initialization to create FdbStorageExt
   - Pass storage instance to API handlers
   - Update handlers to use actual storage methods instead of TODOs

3. **Testing Infrastructure**
   - Create integration tests for FDB storage
   - Test trust-scoped queries
   - Verify layout conflict detection
   - Test semantic metadata filtering

4. **Lightwave Integration**
   - Implement checkpoint verification
   - Add circuit key management
   - Integrate with recursive proof system

## Building with FDB

```bash
# Set up FDB headers (macOS)
./scripts/dev/setup-fdb-headers.sh

# Source environment
source .env.fdb

# Build with FDB feature
nix develop .#fdb --command cargo build --features fdb

# Start FDB server (via nix)
nix run .#fdb-manage start
```

## Architecture Overview

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   API Handlers  │────▶│ ExtendedStorage │────▶│ FDB Tuple Layer │
└─────────────────┘     └─────────────────┘     └─────────────────┘
                                                          │
                        ┌─────────────────────────────────┼─────────────────────────────────┐
                        │                                 │                                 │
                  ┌─────▼──────┐                    ┌─────▼─────┐                    ┌──────▼────┐
                  │  Semantic  │                    │   Trust   │                    │  Layout   │
                  │  Metadata  │                    │   Graph   │                    │ Commits   │
                  └────────────┘                    └───────────┘                    └───────────┘
```

## Trust-Scoped Query Flow

1. Developer submits semantic interpretation
2. Stored with developer ID in FDB tuple
3. Query with trust policy:
   - `Strict`: Only return trusted developer interpretations
   - `Permissive`: Prefer trusted, accept others
   - `Explicit`: Only return specific developer's interpretation

## Layout Conflict Resolution

1. Multiple developers submit different layouts for same contract
2. System detects conflicts automatically
3. Moderator resolves conflict by choosing canonical version
4. All queries return chosen version with trust context

## Extending the Tuple Space Schema

### Adding New Keyspaces

To extend the FoundationDB schema with new data types:

1. **Add to TupleKeyspace enum** in `crates/almanac-storage/src/fdb/tuple_layer.rs`:
   ```rust
   pub enum TupleKeyspace {
       // ... existing keyspaces ...
       NewDataType = 14,  // Use next available number
   }
   ```

2. **Define Key and Value types**:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   pub struct NewDataTypeKey {
       pub field1: String,
       pub field2: u64,
   }

   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   pub struct NewDataTypeValue {
       pub data: Vec<u8>,
       pub timestamp: u64,
   }
   ```

3. **Add tuple key builder method**:
   ```rust
   impl TupleFdbStorage {
       pub fn build_new_data_type_key(&self, key: &NewDataTypeKey) -> Vec<Element<'static>> {
           let mut tuple = Vec::new();
           tuple.push(Element::Bytes(Bytes::from(self.namespace.clone().into_bytes())));
           tuple.push(Element::Int(TupleKeyspace::NewDataType as i64));
           tuple.push(Element::String(key.field1.clone().into()));
           tuple.push(Element::Int(key.field2 as i64));
           tuple
       }
   }
   ```

4. **Implement storage methods**:
   ```rust
   pub async fn store_new_data_type(
       &self,
       key: NewDataTypeKey,
       value: NewDataTypeValue,
   ) -> Result<()> {
       let tx = self.db.create_trx()?;
       let tuple_key = self.build_new_data_type_key(&key);
       let serialized = bincode::serialize(&value)?;
       
       tx.set(&pack(&tuple_key), &serialized);
       tx.commit().await.map_err(|e| anyhow!("Transaction commit failed: {:?}", e))?;
       
       Ok(())
   }

   pub async fn get_new_data_type(&self, key: &NewDataTypeKey) -> Result<Option<NewDataTypeValue>> {
       let tx = self.db.create_trx()?;
       let tuple_key = self.build_new_data_type_key(key);
       
       if let Some(value) = tx.get(&pack(&tuple_key), false).await? {
           let data: NewDataTypeValue = bincode::deserialize(&value)?;
           Ok(Some(data))
       } else {
           Ok(None)
       }
   }
   ```

### Schema Design Guidelines

1. **Keyspace Numbering**: Use sequential numbers starting from the highest existing keyspace
2. **Key Structure**: Design tuple keys for efficient range queries
3. **Serialization**: All types must implement `Serialize` and `Deserialize`
4. **Namespacing**: Always include namespace as first tuple element
5. **Indexing**: Order tuple elements by query patterns (most selective first)

### Best Practices

- **Backwards Compatibility**: Never change existing keyspace numbers
- **Type Safety**: Use strongly typed keys and values
- **Error Handling**: Properly handle serialization and transaction errors
- **Documentation**: Update this documentation when adding new schema types
- **Testing**: Add integration tests for new storage methods

The implementation follows all specifications from the work documents and provides a solid foundation for the multi-chain indexing platform with semantic metadata support.