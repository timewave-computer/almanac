# Contract Indexer Implementation

## Overview

The Almanac indexer includes a comprehensive set of contract indexers for tracking Valence protocol contracts across Ethereum and Cosmos chains. These indexers monitor contract events, update state, and provide query capabilities for Valence Accounts, Processors, Authorizations, and Libraries.

## Contract Indexer Architecture

Each contract indexer follows a common architecture with contract-specific implementations:

```
┌─────────────────────────────────────────────────┐
│              Contract Indexer                   │
├─────────────────────────────────────────────────┤
│ ┌───────────────┐ ┌──────────────┐ ┌──────────┐ │
│ │ Event         │ │ State        │ │ Query    │ │
│ │ Processor     │ │ Manager      │ │ Engine   │ │
│ └───────────────┘ └──────────────┘ └──────────┘ │
│                                                 │
│ ┌───────────────┐ ┌──────────────┐ ┌──────────┐ │
│ │ Data          │ │ Storage      │ │ API      │ │
│ │ Models        │ │ Interface    │ │ Provider │ │
│ └───────────────┘ └──────────────┘ └──────────┘ │
└─────────────────────────────────────────────────┘
```

## Implemented Contract Indexers

### 1. Account Indexer

The Account Indexer tracks Valence accounts, including creation, ownership, library approvals, and execution.

#### Data Models

```rust
pub struct Account {
    pub id: Uuid,
    pub chain_id: String,
    pub contract_address: String,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub created_block: u64,
    pub created_tx: String,
    pub metadata: Option<serde_json::Value>,
}

pub struct AccountLibraryApproval {
    pub id: Uuid,
    pub account_id: Uuid,
    pub library_id: Uuid,
    pub approved_at: DateTime<Utc>,
    pub approved_block: u64,
    pub approved_tx: String,
    pub version: Option<String>,
    pub is_active: bool,
}

pub struct AccountExecution {
    pub id: Uuid,
    pub account_id: Uuid,
    pub library_id: Uuid,
    pub executor: String,
    pub executed_at: DateTime<Utc>,
    pub executed_block: u64,
    pub executed_tx: String,
    pub success: bool,
    pub error_message: Option<String>,
    pub gas_used: Option<u64>,
    pub return_data: Option<Vec<u8>>,
}
```

#### Database Schema

```sql
-- Accounts table
CREATE TABLE accounts (
    id UUID PRIMARY KEY,
    chain_id TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    owner TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_block BIGINT NOT NULL,
    created_tx TEXT NOT NULL,
    metadata JSONB,
    UNIQUE(chain_id, contract_address)
);

-- Account library approvals
CREATE TABLE account_library_approvals (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES accounts(id),
    library_id UUID NOT NULL,
    approved_at TIMESTAMP WITH TIME ZONE NOT NULL,
    approved_block BIGINT NOT NULL,
    approved_tx TEXT NOT NULL,
    version TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    UNIQUE(account_id, library_id, version)
);

-- Account executions
CREATE TABLE account_executions (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES accounts(id),
    library_id UUID NOT NULL,
    executor TEXT NOT NULL,
    executed_at TIMESTAMP WITH TIME ZONE NOT NULL,
    executed_block BIGINT NOT NULL,
    executed_tx TEXT NOT NULL,
    success BOOLEAN NOT NULL,
    error_message TEXT,
    gas_used BIGINT,
    return_data BYTEA
);

-- Indexes
CREATE INDEX idx_accounts_owner ON accounts(owner);
CREATE INDEX idx_account_library_approvals_library ON account_library_approvals(library_id);
CREATE INDEX idx_account_executions_library ON account_executions(library_id);
CREATE INDEX idx_account_executions_executor ON account_executions(executor);
```

#### Event Processing

The Account Indexer processes the following events:

1. **AccountCreated**: Triggered when a new account is created
2. **OwnershipTransferred**: Triggered when account ownership changes
3. **LibraryApproved**: Triggered when a library is approved for use
4. **LibraryRevoked**: Triggered when a library approval is revoked
5. **Executed**: Triggered when the account executes a library function

```rust
pub async fn process_event(&self, chain_id: &str, event: &Event) -> Result<()> {
    match event.event_type.as_str() {
        "AccountCreated" => self.process_account_created(chain_id, event).await,
        "OwnershipTransferred" => self.process_ownership_transferred(chain_id, event).await,
        "LibraryApproved" => self.process_library_approved(chain_id, event).await,
        "LibraryRevoked" => self.process_library_revoked(chain_id, event).await,
        "Executed" => self.process_executed(chain_id, event).await,
        _ => Ok(()),
    }
}
```

### 2. Processor Indexer

The Processor Indexer tracks Valence processors, including instantiation, configuration, and cross-chain message processing.

#### Data Models

```rust
pub struct Processor {
    pub id: Uuid,
    pub chain_id: String,
    pub contract_address: String,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub created_block: u64,
    pub created_tx: String,
    pub config: ProcessorConfig,
}

pub struct ProcessorConfig {
    pub gateway_address: String,
    pub target_chains: Vec<String>,
    pub allowlist: Vec<String>,
    pub denylist: Vec<String>,
    pub default_policy: PolicyType,
}

pub struct ProcessorMessage {
    pub id: Uuid,
    pub processor_id: Uuid,
    pub source_chain_id: String,
    pub target_chain_id: String,
    pub message_hash: String,
    pub sender: String,
    pub recipient: String,
    pub payload: Vec<u8>,
    pub nonce: u64,
    pub status: MessageStatus,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub processed_block: Option<u64>,
    pub processed_tx: Option<String>,
    pub success: Option<bool>,
    pub error_message: Option<String>,
}
```

#### Database Schema

```sql
-- Processors table
CREATE TABLE processors (
    id UUID PRIMARY KEY,
    chain_id TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    owner TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_block BIGINT NOT NULL,
    created_tx TEXT NOT NULL,
    gateway_address TEXT NOT NULL,
    target_chains TEXT[] NOT NULL,
    allowlist TEXT[],
    denylist TEXT[],
    default_policy TEXT NOT NULL,
    UNIQUE(chain_id, contract_address)
);

-- Processor messages
CREATE TABLE processor_messages (
    id UUID PRIMARY KEY,
    processor_id UUID NOT NULL REFERENCES processors(id),
    source_chain_id TEXT NOT NULL,
    target_chain_id TEXT NOT NULL,
    message_hash TEXT NOT NULL,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    payload BYTEA NOT NULL,
    nonce BIGINT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    processed_at TIMESTAMP WITH TIME ZONE,
    processed_block BIGINT,
    processed_tx TEXT,
    success BOOLEAN,
    error_message TEXT,
    UNIQUE(processor_id, message_hash)
);

-- Indexes
CREATE INDEX idx_processors_owner ON processors(owner);
CREATE INDEX idx_processor_messages_status ON processor_messages(status);
CREATE INDEX idx_processor_messages_sender ON processor_messages(sender);
CREATE INDEX idx_processor_messages_recipient ON processor_messages(recipient);
```

#### Event Processing

The Processor Indexer processes the following events:

1. **ProcessorCreated**: Triggered when a new processor is created
2. **ProcessorConfigUpdated**: Triggered when processor configuration changes
3. **MessageReceived**: Triggered when a cross-chain message is received
4. **MessageProcessed**: Triggered when a message is processed
5. **MessageFailed**: Triggered when message processing fails

```rust
pub async fn process_event(&self, chain_id: &str, event: &Event) -> Result<()> {
    match event.event_type.as_str() {
        "ProcessorCreated" => self.process_processor_created(chain_id, event).await,
        "ProcessorConfigUpdated" => self.process_processor_config_updated(chain_id, event).await,
        "MessageReceived" => self.process_message_received(chain_id, event).await,
        "MessageProcessed" => self.process_message_processed(chain_id, event).await,
        "MessageFailed" => self.process_message_failed(chain_id, event).await,
        _ => Ok(()),
    }
}
```

### 3. Authorization Indexer

The Authorization Indexer tracks Valence authorization contracts, including policy management, permission grants/revocations, and requests.

#### Data Models

```rust
pub struct AuthorizationPolicy {
    pub id: Uuid,
    pub chain_id: String,
    pub contract_address: String,
    pub policy_id: String,
    pub owner: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub created_block: u64,
    pub created_tx: String,
    pub policy_type: PolicyType,
    pub policy_data: serde_json::Value,
}

pub struct Permission {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub grantee: String,
    pub resource: String,
    pub permission_type: PermissionType,
    pub is_active: bool,
    pub granted_at: DateTime<Utc>,
    pub granted_block: u64,
    pub granted_tx: String,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_block: Option<u64>,
    pub revoked_tx: Option<String>,
}

pub struct AuthorizationRequest {
    pub id: Uuid,
    pub policy_id: Uuid,
    pub requestor: String,
    pub resource: String,
    pub action: String,
    pub requested_at: DateTime<Utc>,
    pub requested_block: u64,
    pub requested_tx: String,
    pub status: RequestStatus,
    pub decided_at: Option<DateTime<Utc>>,
    pub decided_block: Option<u64>,
    pub decided_tx: Option<String>,
    pub decision: Option<bool>,
    pub decision_reason: Option<String>,
}
```

#### Database Schema

```sql
-- Authorization policies
CREATE TABLE authorization_policies (
    id UUID PRIMARY KEY,
    chain_id TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    policy_id TEXT NOT NULL,
    owner TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_block BIGINT NOT NULL,
    created_tx TEXT NOT NULL,
    policy_type TEXT NOT NULL,
    policy_data JSONB NOT NULL,
    UNIQUE(chain_id, contract_address, policy_id)
);

-- Permissions
CREATE TABLE permissions (
    id UUID PRIMARY KEY,
    policy_id UUID NOT NULL REFERENCES authorization_policies(id),
    grantee TEXT NOT NULL,
    resource TEXT NOT NULL,
    permission_type TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    granted_at TIMESTAMP WITH TIME ZONE NOT NULL,
    granted_block BIGINT NOT NULL,
    granted_tx TEXT NOT NULL,
    revoked_at TIMESTAMP WITH TIME ZONE,
    revoked_block BIGINT,
    revoked_tx TEXT,
    UNIQUE(policy_id, grantee, resource, permission_type)
);

-- Authorization requests
CREATE TABLE authorization_requests (
    id UUID PRIMARY KEY,
    policy_id UUID NOT NULL REFERENCES authorization_policies(id),
    requestor TEXT NOT NULL,
    resource TEXT NOT NULL,
    action TEXT NOT NULL,
    requested_at TIMESTAMP WITH TIME ZONE NOT NULL,
    requested_block BIGINT NOT NULL,
    requested_tx TEXT NOT NULL,
    status TEXT NOT NULL,
    decided_at TIMESTAMP WITH TIME ZONE,
    decided_block BIGINT,
    decided_tx TEXT,
    decision BOOLEAN,
    decision_reason TEXT
);

-- Indexes
CREATE INDEX idx_authorization_policies_owner ON authorization_policies(owner);
CREATE INDEX idx_permissions_grantee ON permissions(grantee);
CREATE INDEX idx_permissions_resource ON permissions(resource);
CREATE INDEX idx_authorization_requests_status ON authorization_requests(status);
CREATE INDEX idx_authorization_requests_requestor ON authorization_requests(requestor);
```

#### Event Processing

The Authorization Indexer processes the following events:

1. **PolicyCreated**: Triggered when a new policy is created
2. **PolicyUpdated**: Triggered when a policy is updated
3. **PolicyActivated**: Triggered when a policy is activated
4. **PolicyDeactivated**: Triggered when a policy is deactivated
5. **PermissionGranted**: Triggered when a permission is granted
6. **PermissionRevoked**: Triggered when a permission is revoked
7. **AuthorizationRequested**: Triggered when authorization is requested
8. **AuthorizationDecided**: Triggered when an authorization decision is made

```rust
pub async fn process_event(&self, chain_id: &str, event: &Event) -> Result<()> {
    match event.event_type.as_str() {
        "PolicyCreated" => self.process_policy_created(chain_id, event).await,
        "PolicyUpdated" => self.process_policy_updated(chain_id, event).await,
        "PolicyActivated" => self.process_policy_activated(chain_id, event).await,
        "PolicyDeactivated" => self.process_policy_deactivated(chain_id, event).await,
        "PermissionGranted" => self.process_permission_granted(chain_id, event).await,
        "PermissionRevoked" => self.process_permission_revoked(chain_id, event).await,
        "AuthorizationRequested" => self.process_authorization_requested(chain_id, event).await,
        "AuthorizationDecided" => self.process_authorization_decided(chain_id, event).await,
        _ => Ok(()),
    }
}
```

### 4. Library Indexer

The Library Indexer tracks Valence libraries, including deployments, versions, approvals, and usage.

#### Data Models

```rust
pub struct Library {
    pub id: Uuid,
    pub chain_id: String,
    pub contract_address: String,
    pub name: String,
    pub description: Option<String>,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub created_block: u64,
    pub created_tx: String,
}

pub struct LibraryVersion {
    pub id: Uuid,
    pub library_id: Uuid,
    pub version: String,
    pub code_hash: String,
    pub published_at: DateTime<Utc>,
    pub published_block: u64,
    pub published_tx: String,
    pub is_active: bool,
    pub metadata: Option<serde_json::Value>,
}

pub struct LibraryUsage {
    pub id: Uuid,
    pub library_id: Uuid,
    pub version_id: Uuid,
    pub account_id: Uuid,
    pub used_at: DateTime<Utc>,
    pub used_block: u64,
    pub used_tx: String,
    pub function_name: String,
    pub success: bool,
    pub error_message: Option<String>,
}
```

#### Database Schema

```sql
-- Libraries table
CREATE TABLE libraries (
    id UUID PRIMARY KEY,
    chain_id TEXT NOT NULL,
    contract_address TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    owner TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_block BIGINT NOT NULL,
    created_tx TEXT NOT NULL,
    UNIQUE(chain_id, contract_address)
);

-- Library versions
CREATE TABLE library_versions (
    id UUID PRIMARY KEY,
    library_id UUID NOT NULL REFERENCES libraries(id),
    version TEXT NOT NULL,
    code_hash TEXT NOT NULL,
    published_at TIMESTAMP WITH TIME ZONE NOT NULL,
    published_block BIGINT NOT NULL,
    published_tx TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT FALSE,
    metadata JSONB,
    UNIQUE(library_id, version)
);

-- Library usage
CREATE TABLE library_usage (
    id UUID PRIMARY KEY,
    library_id UUID NOT NULL REFERENCES libraries(id),
    version_id UUID NOT NULL REFERENCES library_versions(id),
    account_id UUID NOT NULL,
    used_at TIMESTAMP WITH TIME ZONE NOT NULL,
    used_block BIGINT NOT NULL,
    used_tx TEXT NOT NULL,
    function_name TEXT NOT NULL,
    success BOOLEAN NOT NULL,
    error_message TEXT
);

-- Indexes
CREATE INDEX idx_libraries_owner ON libraries(owner);
CREATE INDEX idx_libraries_name ON libraries(name);
CREATE INDEX idx_library_versions_active ON library_versions(library_id, is_active);
CREATE INDEX idx_library_usage_account ON library_usage(account_id);
CREATE INDEX idx_library_usage_function ON library_usage(function_name);
```

#### Event Processing

The Library Indexer processes the following events:

1. **LibraryCreated**: Triggered when a new library is created
2. **VersionPublished**: Triggered when a new version is published
3. **VersionActivated**: Triggered when a version is activated
4. **VersionDeactivated**: Triggered when a version is deactivated
5. **LibraryUsed**: Triggered when a library function is called

```rust
pub async fn process_event(&self, chain_id: &str, event: &Event) -> Result<()> {
    match event.event_type.as_str() {
        "LibraryCreated" => self.process_library_created(chain_id, event).await,
        "VersionPublished" => self.process_version_published(chain_id, event).await,
        "VersionActivated" => self.process_version_activated(chain_id, event).await,
        "VersionDeactivated" => self.process_version_deactivated(chain_id, event).await,
        "LibraryUsed" => self.process_library_used(chain_id, event).await,
        _ => Ok(()),
    }
}
```

## Integration with Chain Adapters

Contract indexers integrate with the Ethereum and Cosmos chain adapters to receive and process contract events:

```rust
pub struct ContractIndexerManager {
    account_indexer: Arc<AccountIndexer>,
    processor_indexer: Arc<ProcessorIndexer>,
    authorization_indexer: Arc<AuthorizationIndexer>,
    library_indexer: Arc<LibraryIndexer>,
}

impl ContractIndexerManager {
    pub async fn process_ethereum_events(&self, block: &Block, events: &[Event]) -> Result<()> {
        for event in events {
            if self.is_account_event(event) {
                self.account_indexer.process_event("ethereum", event).await?;
            } else if self.is_processor_event(event) {
                self.processor_indexer.process_event("ethereum", event).await?;
            } else if self.is_authorization_event(event) {
                self.authorization_indexer.process_event("ethereum", event).await?;
            } else if self.is_library_event(event) {
                self.library_indexer.process_event("ethereum", event).await?;
            }
        }
        
        Ok(())
    }
    
    pub async fn process_cosmos_events(&self, block: &Block, events: &[Event]) -> Result<()> {
        for event in events {
            if self.is_account_event(event) {
                self.account_indexer.process_event("cosmos", event).await?;
            } else if self.is_processor_event(event) {
                self.processor_indexer.process_event("cosmos", event).await?;
            } else if self.is_authorization_event(event) {
                self.authorization_indexer.process_event("cosmos", event).await?;
            } else if self.is_library_event(event) {
                self.library_indexer.process_event("cosmos", event).await?;
            }
        }
        
        Ok(())
    }
    
    // Helper methods to identify event types
    fn is_account_event(&self, event: &Event) -> bool {
        // Implementation to identify account events
    }
    
    fn is_processor_event(&self, event: &Event) -> bool {
        // Implementation to identify processor events
    }
    
    fn is_authorization_event(&self, event: &Event) -> bool {
        // Implementation to identify authorization events
    }
    
    fn is_library_event(&self, event: &Event) -> bool {
        // Implementation to identify library events
    }
}
```

## Query Capabilities

Each contract indexer provides specialized query capabilities:

### Account Queries

```rust
impl AccountIndexer {
    pub async fn get_account_by_id(&self, id: &Uuid) -> Result<Option<Account>> {
        // Implementation to get account by ID
    }
    
    pub async fn get_account_by_address(&self, chain_id: &str, address: &str) -> Result<Option<Account>> {
        // Implementation to get account by address
    }
    
    pub async fn get_accounts_by_owner(&self, owner: &str) -> Result<Vec<Account>> {
        // Implementation to get accounts by owner
    }
    
    pub async fn get_account_library_approvals(&self, account_id: &Uuid) -> Result<Vec<AccountLibraryApproval>> {
        // Implementation to get library approvals for an account
    }
    
    pub async fn get_account_executions(&self, account_id: &Uuid, limit: u32, offset: u32) -> Result<Vec<AccountExecution>> {
        // Implementation to get executions for an account
    }
}
```

### Processor Queries

```rust
impl ProcessorIndexer {
    pub async fn get_processor_by_id(&self, id: &Uuid) -> Result<Option<Processor>> {
        // Implementation to get processor by ID
    }
    
    pub async fn get_processor_by_address(&self, chain_id: &str, address: &str) -> Result<Option<Processor>> {
        // Implementation to get processor by address
    }
    
    pub async fn get_processors_by_owner(&self, owner: &str) -> Result<Vec<Processor>> {
        // Implementation to get processors by owner
    }
    
    pub async fn get_processor_messages(&self, processor_id: &Uuid, status: Option<MessageStatus>, limit: u32, offset: u32) -> Result<Vec<ProcessorMessage>> {
        // Implementation to get messages for a processor
    }
    
    pub async fn get_message_by_hash(&self, processor_id: &Uuid, message_hash: &str) -> Result<Option<ProcessorMessage>> {
        // Implementation to get message by hash
    }
}
```

### Authorization Queries

```rust
impl AuthorizationIndexer {
    pub async fn get_policy_by_id(&self, id: &Uuid) -> Result<Option<AuthorizationPolicy>> {
        // Implementation to get policy by ID
    }
    
    pub async fn get_policies_by_owner(&self, owner: &str) -> Result<Vec<AuthorizationPolicy>> {
        // Implementation to get policies by owner
    }
    
    pub async fn get_permissions_by_policy(&self, policy_id: &Uuid) -> Result<Vec<Permission>> {
        // Implementation to get permissions for a policy
    }
    
    pub async fn get_permissions_by_grantee(&self, grantee: &str) -> Result<Vec<Permission>> {
        // Implementation to get permissions for a grantee
    }
    
    pub async fn get_authorization_requests(&self, policy_id: &Uuid, status: Option<RequestStatus>, limit: u32, offset: u32) -> Result<Vec<AuthorizationRequest>> {
        // Implementation to get requests for a policy
    }
}
```

### Library Queries

```rust
impl LibraryIndexer {
    pub async fn get_library_by_id(&self, id: &Uuid) -> Result<Option<Library>> {
        // Implementation to get library by ID
    }
    
    pub async fn get_library_by_address(&self, chain_id: &str, address: &str) -> Result<Option<Library>> {
        // Implementation to get library by address
    }
    
    pub async fn get_libraries_by_owner(&self, owner: &str) -> Result<Vec<Library>> {
        // Implementation to get libraries by owner
    }
    
    pub async fn get_library_versions(&self, library_id: &Uuid) -> Result<Vec<LibraryVersion>> {
        // Implementation to get versions for a library
    }
    
    pub async fn get_active_library_version(&self, library_id: &Uuid) -> Result<Option<LibraryVersion>> {
        // Implementation to get active version for a library
    }
    
    pub async fn get_library_usage(&self, library_id: &Uuid, limit: u32, offset: u32) -> Result<Vec<LibraryUsage>> {
        // Implementation to get usage for a library
    }
}
```

## API Integration

The contract indexers expose their data through GraphQL and REST APIs:

### GraphQL Schema

```graphql
type Account {
  id: ID!
  chainId: String!
  contractAddress: String!
  owner: String!
  createdAt: DateTime!
  createdBlock: Int!
  createdTx: String!
  approvedLibraries: [AccountLibraryApproval!]!
  executions: [AccountExecution!]!
}

type Processor {
  id: ID!
  chainId: String!
  contractAddress: String!
  owner: String!
  createdAt: DateTime!
  gatewayAddress: String!
  targetChains: [String!]!
  allowlist: [String!]
  denylist: [String!]
  defaultPolicy: String!
  messages: [ProcessorMessage!]!
}

type AuthorizationPolicy {
  id: ID!
  chainId: String!
  contractAddress: String!
  policyId: String!
  owner: String!
  isActive: Boolean!
  createdAt: DateTime!
  policyType: String!
  policyData: JSONObject!
  permissions: [Permission!]!
  requests: [AuthorizationRequest!]!
}

type Library {
  id: ID!
  chainId: String!
  contractAddress: String!
  name: String!
  description: String
  owner: String!
  createdAt: DateTime!
  versions: [LibraryVersion!]!
  activeVersion: LibraryVersion
  usage: [LibraryUsage!]!
}

# Query root type
type Query {
  # Account queries
  account(id: ID!): Account
  accountByAddress(chainId: String!, address: String!): Account
  accountsByOwner(owner: String!): [Account!]!
  
  # Processor queries
  processor(id: ID!): Processor
  processorByAddress(chainId: String!, address: String!): Processor
  processorsByOwner(owner: String!): [Processor!]!
  
  # Authorization queries
  authorizationPolicy(id: ID!): AuthorizationPolicy
  authorizationPoliciesByOwner(owner: String!): [AuthorizationPolicy!]!
  permissionsByGrantee(grantee: String!): [Permission!]!
  
  # Library queries
  library(id: ID!): Library
  libraryByAddress(chainId: String!, address: String!): Library
  librariesByOwner(owner: String!): [Library!]!
}
```

## Testing

Each contract indexer is thoroughly tested with specific test cases:

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    // Test account creation and retrieval
    #[tokio::test]
    async fn test_account_creation() {
        // Implementation to test account creation
    }
    
    // Test ownership transfer
    #[tokio::test]
    async fn test_ownership_transfer() {
        // Implementation to test ownership transfer
    }
    
    // Test library approval and revocation
    #[tokio::test]
    async fn test_library_approval() {
        // Implementation to test library approval
    }
    
    // Test account execution
    #[tokio::test]
    async fn test_account_execution() {
        // Implementation to test account execution
    }
    
    // Similar test structures for other indexers
}
```

## Performance Considerations

1. **Batch Processing**: Events are processed in batches for efficiency
2. **Optimized Database Queries**: Strategic indexing for common query patterns
3. **Caching**: Frequently accessed entities are cached in memory
4. **Parallel Processing**: Independent events are processed in parallel

## Future Enhancements

1. **History Tracking**: Enhanced historical state tracking for all entities
2. **Versioned References**: Support for contract upgrades and versioning
3. **Analytics Extensions**: Advanced analytics for contract usage patterns
4. **Graph Relationship Queries**: Graph-based queries for complex relationships between entities 