# Cross-Chain Message Tracking

## Overview

The Almanac indexer provides a comprehensive system for tracking cross-chain messages across Ethereum and Cosmos chains. This functionality is essential for monitoring Valence protocol operations that span multiple blockchains, ensuring messages are properly tracked from origination through delivery and execution.

## Message Lifecycle

A cross-chain message travels through several stages during its lifecycle:

```
┌────────────────┐     ┌────────────────┐     ┌────────────────┐     ┌────────────────┐
│  Origination   │────▶│   In Transit   │────▶│    Delivery    │────▶│   Execution    │
└────────────────┘     └────────────────┘     └────────────────┘     └────────────────┘
        │                                                                     │
        │                                                                     │
        └─────────────────────────┬─────────────────────────────────┬────────┘
                                  │                                 │
                           ┌─────────────┐                   ┌─────────────┐
                           │  Timeout    │                   │    Error    │
                           └─────────────┘                   └─────────────┘
```

### Message States

1. **Originated**: A message has been created and sent from the source chain.
2. **In Transit**: The message has been acknowledged by the cross-chain infrastructure but has not yet been delivered.
3. **Delivered**: The message has been delivered to the target chain but not yet executed.
4. **Executed**: The message has been successfully processed on the target chain.
5. **Failed**: The message execution has failed on the target chain.
6. **Timed Out**: The message delivery or execution did not complete within the specified timeframe.

## Data Model

### Cross-Chain Message

```rust
pub struct CrossChainMessage {
    // Unique identifier for the message
    pub id: Uuid,
    
    // Chain identifiers
    pub source_chain_id: String,
    pub target_chain_id: String,
    
    // Block information
    pub source_block_number: u64,
    pub source_block_hash: String,
    pub target_block_number: Option<u64>,
    pub target_block_hash: Option<String>,
    
    // Transaction information
    pub source_tx_hash: String,
    pub target_tx_hash: Option<String>,
    
    // Timestamp information
    pub created_at: DateTime<Utc>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub executed_at: Option<DateTime<Utc>>,
    
    // Message details
    pub message_type: String,
    pub sender: String,
    pub recipient: String,
    pub payload: Vec<u8>,
    
    // Status tracking
    pub status: MessageStatus,
    pub retry_count: u32,
    pub execution_result: Option<MessageExecutionResult>,
}

pub enum MessageStatus {
    Originated,
    InTransit,
    Delivered,
    Executed,
    Failed,
    TimedOut,
}

pub struct MessageExecutionResult {
    pub success: bool,
    pub return_data: Option<Vec<u8>>,
    pub error_message: Option<String>,
    pub gas_used: Option<u64>,
}
```

### Database Schema

```sql
-- Cross-chain messages table
CREATE TABLE cross_chain_messages (
    id UUID PRIMARY KEY,
    source_chain_id TEXT NOT NULL,
    target_chain_id TEXT NOT NULL,
    source_block_number BIGINT NOT NULL,
    source_block_hash TEXT NOT NULL,
    target_block_number BIGINT,
    target_block_hash TEXT,
    source_tx_hash TEXT NOT NULL,
    target_tx_hash TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    delivered_at TIMESTAMP WITH TIME ZONE,
    executed_at TIMESTAMP WITH TIME ZONE,
    message_type TEXT NOT NULL,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    payload BYTEA NOT NULL,
    status TEXT NOT NULL,
    retry_count INTEGER NOT NULL DEFAULT 0,
    execution_success BOOLEAN,
    return_data BYTEA,
    error_message TEXT,
    gas_used BIGINT
);

-- Indexes for efficient querying
CREATE INDEX idx_cross_chain_messages_source ON cross_chain_messages(source_chain_id, source_block_number);
CREATE INDEX idx_cross_chain_messages_target ON cross_chain_messages(target_chain_id, target_block_number);
CREATE INDEX idx_cross_chain_messages_status ON cross_chain_messages(status);
CREATE INDEX idx_cross_chain_messages_created_at ON cross_chain_messages(created_at);
CREATE INDEX idx_cross_chain_messages_sender ON cross_chain_messages(sender);
CREATE INDEX idx_cross_chain_messages_recipient ON cross_chain_messages(recipient);
```

## Implementation Components

### 1. Message Detection

The indexer detects cross-chain messages by monitoring specific events on supported chains:

#### Ethereum Message Detection

```rust
pub async fn detect_ethereum_messages(&self, block: &Block, transactions: &[Transaction]) -> Result<Vec<CrossChainMessage>> {
    let mut messages = Vec::new();
    
    for tx in transactions {
        // Check for processor send events
        if let Some(logs) = &tx.logs {
            for log in logs {
                if log.address == self.processor_address && 
                   log.topics[0] == keccak256("MessageSent(address,bytes32,bytes)") 
                {
                    // Parse event data to create cross-chain message
                    let message = self.parse_ethereum_message_sent_event(block, tx, log)?;
                    messages.push(message);
                }
            }
        }
    }
    
    Ok(messages)
}
```

#### Cosmos Message Detection

```rust
pub async fn detect_cosmos_messages(&self, block: &Block, txs: &[Transaction]) -> Result<Vec<CrossChainMessage>> {
    let mut messages = Vec::new();
    
    for tx in txs {
        for event in &tx.events {
            if event.type_str == "wasm" {
                let contract = event.attributes
                    .iter()
                    .find(|attr| attr.key == "_contract_address")
                    .map(|attr| attr.value.clone());
                
                // Check if this is a processor contract
                if let Some(contract_addr) = contract {
                    if self.is_processor_contract(&contract_addr).await? {
                        if let Some(msg) = self.parse_cosmos_message_event(block, tx, event)? {
                            messages.push(msg);
                        }
                    }
                }
            }
        }
    }
    
    Ok(messages)
}
```

### 2. Message Tracking

The indexer tracks message state transitions as they move through the cross-chain process:

```rust
pub async fn update_message_status(&self, message_id: &Uuid, new_status: MessageStatus) -> Result<()> {
    let now = Utc::now();
    
    // Update the message status in PostgreSQL
    sqlx::query!(
        r#"
        UPDATE cross_chain_messages
        SET status = $1,
            delivered_at = CASE WHEN $1 = 'Delivered' THEN $2 ELSE delivered_at END,
            executed_at = CASE WHEN $1 = 'Executed' THEN $2 ELSE executed_at END
        WHERE id = $3
        "#,
        new_status.to_string(),
        now,
        message_id
    )
    .execute(&self.pg_pool)
    .await?;
    
    // Update the message status in RocksDB for quick lookups
    let key = format!("message:status:{}", message_id);
    self.rocks_db.put(key.as_bytes(), new_status.to_string().as_bytes())?;
    
    Ok(())
}
```

### 3. Message Correlation

One of the most important aspects of cross-chain message tracking is correlating messages across chains:

```rust
pub async fn correlate_messages(&self) -> Result<()> {
    // Get all messages in transit without target transaction hash
    let in_transit_messages = sqlx::query_as!(
        CrossChainMessageRow,
        r#"
        SELECT * FROM cross_chain_messages 
        WHERE status = 'InTransit' OR (status = 'Delivered' AND target_tx_hash IS NULL)
        "#
    )
    .fetch_all(&self.pg_pool)
    .await?;
    
    for message in in_transit_messages {
        // Look for corresponding delivery events on target chain
        let target_chain = &message.target_chain_id;
        let delivery_events = self.find_delivery_events(target_chain, &message.id).await?;
        
        if let Some(delivery) = delivery_events.first() {
            // Update message with delivery information
            self.correlate_message_delivery(&message.id, delivery).await?;
        }
    }
    
    Ok(())
}

pub async fn find_delivery_events(&self, chain_id: &str, message_id: &Uuid) -> Result<Vec<MessageDeliveryEvent>> {
    // Implementation depends on chain-specific ways to identify message deliveries
    match chain_id {
        "ethereum" => self.find_ethereum_delivery_events(message_id).await,
        "cosmos" => self.find_cosmos_delivery_events(message_id).await,
        _ => Err(Error::UnsupportedChain(chain_id.to_string())),
    }
}
```

## API Endpoints

The indexer provides APIs for querying cross-chain message state:

### GraphQL API

```graphql
type CrossChainMessage {
  id: ID!
  sourceChainId: String!
  targetChainId: String!
  sourceBlockNumber: Int!
  targetBlockNumber: Int
  sourceTxHash: String!
  targetTxHash: String
  createdAt: DateTime!
  deliveredAt: DateTime
  executedAt: DateTime
  messageType: String!
  sender: String!
  recipient: String!
  payload: String!
  status: MessageStatus!
  executionSuccess: Boolean
  errorMessage: String
}

enum MessageStatus {
  ORIGINATED
  IN_TRANSIT
  DELIVERED
  EXECUTED
  FAILED
  TIMED_OUT
}

type Query {
  # Get a message by ID
  crossChainMessage(id: ID!): CrossChainMessage
  
  # Get messages based on filters
  crossChainMessages(
    sourceChainId: String
    targetChainId: String
    sender: String
    recipient: String
    status: MessageStatus
    fromDate: DateTime
    toDate: DateTime
    limit: Int = 10
    offset: Int = 0
  ): [CrossChainMessage!]!
  
  # Get message count by status
  messageCountByStatus: [StatusCount!]!
}

type StatusCount {
  status: MessageStatus!
  count: Int!
}
```

### REST API

```
GET /api/messages/:id
GET /api/messages?sourceChain=:chainId&status=:status
GET /api/messages/stats
```

## Visualization

The indexer provides message flow visualization:

```
┌─────────────┐                                     ┌─────────────┐
│             │                                     │             │
│  Ethereum   │                                     │   Cosmos    │
│             │                                     │             │
└─────┬───────┘                                     └───────┬─────┘
      │                                                     │
      │ MessageSent                                         │
      │ event                                               │
      │                                                     │
      │                   ┌─────────────┐                   │
      └──────────────────►             │                   │
                         │  Transport   │                   │
                         │             │                   │
                         └──────┬──────┘                   │
                                │                          │
                                │                          │
                                │                          │
                                │                          │
                                │                          │
                                │      MessageReceived     │
                                └─────────────────────────►│
                                                           │
                                                           │
                                                           │
                                       ExecuteMessage      │
                                           event           │
                                                           │
```

## Testing

The message tracking system is tested with various scenarios:

1. **Happy Path**: Messages successfully transit from origin to execution
2. **Failure Scenarios**: Messages fail during various stages
3. **Timeout Scenarios**: Messages time out during transit or execution
4. **Recovery Scenarios**: Messages recover after temporary failures

### Test Suite Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_message_origination() {
        // Test message creation and detection
    }
    
    #[tokio::test]
    async fn test_message_delivery() {
        // Test message delivery detection and status update
    }
    
    #[tokio::test]
    async fn test_message_execution() {
        // Test message execution detection and status update
    }
    
    #[tokio::test]
    async fn test_message_failure() {
        // Test failure handling
    }
    
    #[tokio::test]
    async fn test_message_correlation() {
        // Test linking messages across chains
    }
}
```

## Integration with Valence Contracts

The cross-chain message tracking system is tightly integrated with Valence protocol contracts:

1. **Processor Contract**: Monitors message sending and receiving events
2. **Account Contract**: Tracks execution requests spanning multiple chains
3. **Library Contract**: Monitors cross-chain library usage

### Processor Contract Integration

```rust
pub struct ProcessorMonitor {
    ethereum_processor_address: Address,
    cosmos_processor_contracts: Vec<String>,
    message_tracker: Arc<MessageTracker>,
}

impl ProcessorMonitor {
    pub async fn process_ethereum_block(&self, block: &Block, txs: &[Transaction]) -> Result<()> {
        // Extract messages from Ethereum processor events
        let messages = self.extract_ethereum_messages(block, txs).await?;
        
        // Track new messages
        for message in messages {
            self.message_tracker.track_new_message(message).await?;
        }
        
        Ok(())
    }
    
    pub async fn process_cosmos_block(&self, block: &Block, txs: &[Transaction]) -> Result<()> {
        // Extract messages from Cosmos processor events
        let messages = self.extract_cosmos_messages(block, txs).await?;
        
        // Track new messages
        for message in messages {
            self.message_tracker.track_new_message(message).await?;
        }
        
        Ok(())
    }
}
```

## Performance Considerations

1. **Efficient Event Filtering**: Filtering events at the chain adapter level
2. **Optimized Database Schema**: Indexes for common query patterns
3. **Caching Layer**: Frequently accessed message status cached in memory
4. **Background Processing**: Correlation jobs run in background workers

## Future Enhancements

1. **Multi-hop Tracking**: Track messages that traverse more than two chains
2. **Merkle Proofs**: Include merkle proofs with messages for verification
3. **Analytics Engine**: Statistical analysis of cross-chain message flows
4. **Latency Monitoring**: Track and alert on unusual message delays 