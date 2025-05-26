use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use regex::Regex;
use std::time::SystemTime;

/// Chain identifier
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ChainId(pub String);

impl From<&str> for ChainId {
    fn from(s: &str) -> Self {
        ChainId(s.to_string())
    }
}

impl From<String> for ChainId {
    fn from(s: String) -> Self {
        ChainId(s)
    }
}

/// Text search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSearchConfig {
    /// Search mode
    pub mode: TextSearchMode,
    
    /// Fields to search in
    pub fields: Option<Vec<String>>,
    
    /// Case sensitivity
    pub case_sensitive: bool,
    
    /// Use stemming
    pub use_stemming: bool,
    
    /// Minimum match score (0.0 to 1.0)
    pub min_score: Option<f32>,
    
    /// Maximum results to return
    pub max_results: Option<usize>,
}

/// Text search modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextSearchMode {
    /// Simple substring search
    Contains,
    
    /// Full-text search with ranking
    FullText,
    
    /// Fuzzy search with edit distance
    Fuzzy { max_distance: u32 },
    
    /// Regular expression search
    Regex,
    
    /// Phrase search (exact phrase)
    Phrase,
    
    /// Boolean search (AND, OR, NOT operators)
    Boolean,
}

impl Default for TextSearchConfig {
    fn default() -> Self {
        Self {
            mode: TextSearchMode::Contains,
            fields: None,
            case_sensitive: false,
            use_stemming: false,
            min_score: None,
            max_results: None,
        }
    }
}

/// Filter for querying events with advanced capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Multiple chain IDs to filter (OR operation)
    pub chain_ids: Option<Vec<ChainId>>,
    
    /// Chain string (used in simpler interfaces)
    pub chain: Option<String>,
    
    /// Multiple chains (OR operation)
    pub chains: Option<Vec<String>>,
    
    /// Block number range
    pub block_range: Option<(u64, u64)>,
    
    /// Multiple block ranges (OR operation)
    pub block_ranges: Option<Vec<(u64, u64)>>,
    
    /// Time range in seconds since UNIX epoch
    pub time_range: Option<(u64, u64)>,
    
    /// Multiple time ranges (OR operation)
    pub time_ranges: Option<Vec<(u64, u64)>>,
    
    /// Event types to include (OR operation)
    pub event_types: Option<Vec<String>>,
    
    /// Event types to exclude
    pub exclude_event_types: Option<Vec<String>>,
    
    /// Contract addresses to include (OR operation)
    pub addresses: Option<Vec<String>>,
    
    /// Contract addresses to exclude
    pub exclude_addresses: Option<Vec<String>>,
    
    /// Transaction hashes to include (OR operation)
    pub tx_hashes: Option<Vec<String>>,
    
    /// Block hashes to include (OR operation)
    pub block_hashes: Option<Vec<String>>,
    
    /// Additional filters as key-value pairs (AND operation)
    pub custom_filters: HashMap<String, String>,
    
    /// Advanced attribute filters with operators
    pub attribute_filters: Option<Vec<AttributeFilter>>,
    
    /// Text search query (if full-text search is available)
    pub text_query: Option<String>,
    
    /// Text search configuration
    pub text_search_config: Option<TextSearchConfig>,
    
    /// Sort order
    pub sort_by: Option<SortField>,
    
    /// Sort direction
    pub sort_direction: Option<SortDirection>,
    
    /// Maximum number of events to return
    pub limit: Option<usize>,
    
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Advanced attribute filter with operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeFilter {
    /// Attribute name/key
    pub key: String,
    
    /// Filter operator
    pub operator: FilterOperator,
    
    /// Value to compare against
    pub value: FilterValue,
}

/// Filter operators for advanced filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    /// Exact match
    Equals,
    
    /// Not equal
    NotEquals,
    
    /// Contains substring
    Contains,
    
    /// Does not contain substring
    NotContains,
    
    /// Starts with
    StartsWith,
    
    /// Ends with
    EndsWith,
    
    /// Greater than (numeric)
    GreaterThan,
    
    /// Greater than or equal (numeric)
    GreaterThanOrEqual,
    
    /// Less than (numeric)
    LessThan,
    
    /// Less than or equal (numeric)
    LessThanOrEqual,
    
    /// In list (OR operation)
    In,
    
    /// Not in list
    NotIn,
    
    /// Regular expression match
    Regex,
    
    /// Field exists
    Exists,
    
    /// Field does not exist
    NotExists,
}

/// Filter value types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterValue {
    String(String),
    Number(f64),
    Boolean(bool),
    List(Vec<String>),
}

/// Fields available for sorting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortField {
    /// Sort by block number
    BlockNumber,
    
    /// Sort by timestamp
    Timestamp,
    
    /// Sort by event type
    EventType,
    
    /// Sort by chain
    Chain,
    
    /// Sort by transaction hash
    TxHash,
    
    /// Sort by custom attribute
    Attribute(String),
}

/// Sort direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl EventFilter {
    /// Create a new empty event filter
    pub fn new() -> Self {
        Self {
            chain_ids: None,
            chain: None,
            chains: None,
            block_range: None,
            block_ranges: None,
            time_range: None,
            time_ranges: None,
            event_types: None,
            exclude_event_types: None,
            addresses: None,
            exclude_addresses: None,
            tx_hashes: None,
            block_hashes: None,
            custom_filters: HashMap::new(),
            attribute_filters: None,
            text_query: None,
            text_search_config: None,
            sort_by: None,
            sort_direction: None,
            limit: None,
            offset: None,
        }
    }
    
    /// Add a chain filter
    pub fn with_chain(mut self, chain: String) -> Self {
        self.chain = Some(chain);
        self
    }
    
    /// Add multiple chains filter
    pub fn with_chains(mut self, chains: Vec<String>) -> Self {
        self.chains = Some(chains);
        self
    }
    
    /// Add block range filter
    pub fn with_block_range(mut self, from: u64, to: u64) -> Self {
        self.block_range = Some((from, to));
        self
    }
    
    /// Add time range filter
    pub fn with_time_range(mut self, from: u64, to: u64) -> Self {
        self.time_range = Some((from, to));
        self
    }
    
    /// Add event type filter
    pub fn with_event_types(mut self, event_types: Vec<String>) -> Self {
        self.event_types = Some(event_types);
        self
    }
    
    /// Add address filter
    pub fn with_addresses(mut self, addresses: Vec<String>) -> Self {
        self.addresses = Some(addresses);
        self
    }
    
    /// Add custom filter
    pub fn with_custom_filter(mut self, key: String, value: String) -> Self {
        self.custom_filters.insert(key, value);
        self
    }
    
    /// Add attribute filter
    pub fn with_attribute_filter(mut self, key: String, operator: FilterOperator, value: FilterValue) -> Self {
        let filter = AttributeFilter { key, operator, value };
        match self.attribute_filters {
            Some(ref mut filters) => filters.push(filter),
            None => self.attribute_filters = Some(vec![filter]),
        }
        self
    }
    
    /// Add text query
    pub fn with_text_query(mut self, query: String) -> Self {
        self.text_query = Some(query);
        self
    }
    
    /// Add text search with configuration
    pub fn with_text_search(mut self, query: String, config: TextSearchConfig) -> Self {
        self.text_query = Some(query);
        self.text_search_config = Some(config);
        self
    }
    
    /// Add sorting
    pub fn with_sort(mut self, field: SortField, direction: SortDirection) -> Self {
        self.sort_by = Some(field);
        self.sort_direction = Some(direction);
        self
    }
    
    /// Add pagination
    pub fn with_pagination(mut self, limit: usize, offset: usize) -> Self {
        self.limit = Some(limit);
        self.offset = Some(offset);
        self
    }
    
    /// Check if event matches this filter (for in-memory filtering)
    pub fn matches_event(&self, event: &dyn crate::event::Event) -> bool {
        // Check chain filters
        if let Some(ref chain) = self.chain {
            if event.chain() != chain {
                return false;
            }
        }
        
        if let Some(ref chains) = self.chains {
            if !chains.contains(&event.chain().to_string()) {
                return false;
            }
        }
        
        if let Some(ref chain_ids) = self.chain_ids {
            let event_chain_id = ChainId::from(event.chain());
            if !chain_ids.contains(&event_chain_id) {
                return false;
            }
        }
        
        // Check block range filters
        if let Some((min_block, max_block)) = self.block_range {
            if event.block_number() < min_block || event.block_number() > max_block {
                return false;
            }
        }
        
        if let Some(ref block_ranges) = self.block_ranges {
            let block_num = event.block_number();
            let in_range = block_ranges.iter().any(|(min, max)| block_num >= *min && block_num <= *max);
            if !in_range {
                return false;
            }
        }
        
        // Check time range filters
        let event_timestamp = event.timestamp()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        if let Some((min_time, max_time)) = self.time_range {
            if event_timestamp < min_time || event_timestamp > max_time {
                return false;
            }
        }
        
        if let Some(ref time_ranges) = self.time_ranges {
            let in_range = time_ranges.iter().any(|(min, max)| event_timestamp >= *min && event_timestamp <= *max);
            if !in_range {
                return false;
            }
        }
        
        // Check event type filters
        if let Some(ref event_types) = self.event_types {
            if !event_types.contains(&event.event_type().to_string()) {
                return false;
            }
        }
        
        if let Some(ref exclude_event_types) = self.exclude_event_types {
            if exclude_event_types.contains(&event.event_type().to_string()) {
                return false;
            }
        }
        
        // Check transaction hash filters
        if let Some(ref tx_hashes) = self.tx_hashes {
            if !tx_hashes.contains(&event.tx_hash().to_string()) {
                return false;
            }
        }
        
        // Check block hash filters
        if let Some(ref block_hashes) = self.block_hashes {
            if !block_hashes.contains(&event.block_hash().to_string()) {
                return false;
            }
        }
        
        // Check address filters
        if let Some(ref addresses) = self.addresses {
            // For now, check if any address appears in the event data
            let event_text = format!("{} {} {} {}", 
                event.id(), event.event_type(), event.tx_hash(), 
                String::from_utf8_lossy(event.raw_data()));
            let address_match = addresses.iter().any(|addr| event_text.contains(addr));
            if !address_match {
                return false;
            }
        }
        
        if let Some(ref exclude_addresses) = self.exclude_addresses {
            let event_text = format!("{} {} {} {}", 
                event.id(), event.event_type(), event.tx_hash(), 
                String::from_utf8_lossy(event.raw_data()));
            let address_match = exclude_addresses.iter().any(|addr| event_text.contains(addr));
            if address_match {
                return false;
            }
        }
        
        // Check text search
        if let Some(ref text_query) = self.text_query {
            let text_config = self.text_search_config.clone().unwrap_or_default();
            
            // Extract searchable text from event
            let searchable_text = format!("{} {} {} {} {} {}", 
                event.id(), 
                event.chain(), 
                event.event_type(), 
                event.tx_hash(), 
                event.block_hash(),
                String::from_utf8_lossy(event.raw_data())
            );
            
            // Perform the appropriate search based on mode
            let matches = match text_config.mode {
                TextSearchMode::Contains => {
                    let (search_query, search_text) = if text_config.case_sensitive {
                        (text_query.clone(), searchable_text)
                    } else {
                        (text_query.to_lowercase(), searchable_text.to_lowercase())
                    };
                    search_text.contains(&search_query)
                }
                TextSearchMode::FullText => {
                    // Simple full-text search - split query into words and check all exist
                    let search_words: Vec<&str> = text_query.split_whitespace().collect();
                    let search_text = if text_config.case_sensitive {
                        searchable_text
                    } else {
                        searchable_text.to_lowercase()
                    };
                    
                    search_words.iter().all(|word| {
                        let search_word = if text_config.case_sensitive {
                            word.to_string()
                        } else {
                            word.to_lowercase()
                        };
                        search_text.contains(&search_word)
                    })
                }
                TextSearchMode::Fuzzy { max_distance } => {
                    // Simple fuzzy search - check if any word is within edit distance
                    let search_words: Vec<&str> = searchable_text.split_whitespace().collect();
                    let search_text = if text_config.case_sensitive {
                        text_query.clone()
                    } else {
                        text_query.to_lowercase()
                    };
                    
                    search_words.iter().any(|word| {
                        let check_word = if text_config.case_sensitive {
                            word.to_string()
                        } else {
                            word.to_lowercase()
                        };
                        levenshtein_distance(&check_word, &search_text) <= max_distance
                    })
                }
                TextSearchMode::Regex => {
                    // Use regex for pattern matching
                    if let Ok(regex) = Regex::new(text_query) {
                        regex.is_match(&searchable_text)
                    } else {
                        false
                    }
                }
                TextSearchMode::Phrase => {
                    // Exact phrase search
                    let (search_phrase, search_text) = if text_config.case_sensitive {
                        (text_query.clone(), searchable_text)
                    } else {
                        (text_query.to_lowercase(), searchable_text.to_lowercase())
                    };
                    search_text.contains(&search_phrase)
                }
                TextSearchMode::Boolean => {
                    // Simple boolean search
                    let search_text = if text_config.case_sensitive {
                        searchable_text
                    } else {
                        searchable_text.to_lowercase()
                    };
                    let search_query = if text_config.case_sensitive {
                        text_query.clone()
                    } else {
                        text_query.to_lowercase()
                    };
                    
                    if search_query.contains(" and ") {
                        let terms: Vec<&str> = search_query.split(" and ").collect();
                        terms.iter().all(|term| search_text.contains(term.trim()))
                    } else if search_query.contains(" or ") {
                        let terms: Vec<&str> = search_query.split(" or ").collect();
                        terms.iter().any(|term| search_text.contains(term.trim()))
                    } else if search_query.starts_with("not ") {
                        let term = search_query.strip_prefix("not ").unwrap().trim();
                        !search_text.contains(term)
                    } else {
                        search_text.contains(&search_query)
                    }
                }
            };
            
            if !matches {
                return false;
            }
        }
        
        // TODO: Implement attribute filters with operators
        // This would require extending the Event trait to provide structured attribute access
        
        true
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> u32 {
    let len1 = s1.len();
    let len2 = s2.len();
    
    if len1 == 0 {
        return len2 as u32;
    }
    if len2 == 0 {
        return len1 as u32;
    }
    
    let mut matrix = vec![vec![0u32; len2 + 1]; len1 + 1];
    
    // Initialize first row and column
    for (i, row) in matrix.iter_mut().enumerate().take(len1 + 1) {
        row[0] = i as u32;
    }
    for j in 0..=len2 {
        matrix[0][j] = j as u32;
    }
    
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    
    // Fill the matrix
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };
            
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }
    
    matrix[len1][len2]
}

/// Configuration for an indexer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerConfig {
    /// Chain identifiers to index
    pub chains: Vec<ChainConfig>,
    
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// API configuration
    pub api: ApiConfig,
}

/// Configuration for a specific chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Chain identifier
    pub chain_id: ChainId,
    
    /// Chain type (e.g., "ethereum", "cosmos")
    pub chain_type: String,
    
    /// RPC endpoint URLs
    pub rpc_urls: Vec<String>,
    
    /// Starting block number
    pub start_block: Option<u64>,
    
    /// Chain-specific configuration parameters
    pub params: HashMap<String, String>,
}

/// Configuration for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage type (e.g., "rocksdb", "postgres")
    pub storage_type: String,
    
    /// Path or connection string
    pub connection: String,
    
    /// Storage-specific configuration parameters
    pub params: HashMap<String, String>,
}

/// Configuration for the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Host to bind to
    pub host: String,
    
    /// Port to listen on
    pub port: u16,
    
    /// Enable GraphQL API
    pub enable_graphql: bool,
    
    /// Enable REST API
    pub enable_rest: bool,
    
    /// Enable WebSocket subscriptions
    pub enable_websocket: bool,
    
    /// Additional API configuration parameters
    pub params: HashMap<String, String>,
}

/// Time period for aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimePeriod {
    Hour,
    Day,
    Week,
    Month,
    Year,
    Custom { seconds: u64 },
}

/// Aggregation function type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationFunction {
    Count,
    Sum { field: String },
    Average { field: String },
    Min { field: String },
    Max { field: String },
    Distinct { field: String },
}

/// Aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
    /// Time period for grouping
    pub time_period: TimePeriod,
    
    /// Aggregation functions to apply
    pub functions: Vec<AggregationFunction>,
    
    /// Additional grouping fields
    pub group_by: Option<Vec<String>>,
    
    /// Time range for aggregation
    pub time_range: Option<(SystemTime, SystemTime)>,
    
    /// Maximum number of buckets to return
    pub max_buckets: Option<usize>,
}

/// Result of an aggregation query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResult {
    /// Time bucket start
    pub time_bucket: SystemTime,
    
    /// Grouping field values
    pub group_values: HashMap<String, String>,
    
    /// Aggregation results
    pub aggregations: HashMap<String, AggregationValue>,
}

/// Value from aggregation calculation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AggregationValue {
    Count(u64),
    Sum(f64),
    Average(f64),
    Min(f64),
    Max(f64),
    Distinct(u64),
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            time_period: TimePeriod::Hour,
            functions: vec![AggregationFunction::Count],
            group_by: None,
            time_range: None,
            max_buckets: Some(100),
        }
    }
}

/// Event correlation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationConfig {
    /// Fields to correlate on
    pub correlation_fields: Vec<String>,
    
    /// Time window for correlation (in seconds)
    pub time_window: Option<u64>,
    
    /// Maximum distance between correlated events (in block numbers)
    pub max_block_distance: Option<u64>,
    
    /// Minimum number of events required for correlation
    pub min_events: Option<usize>,
    
    /// Chains to include in correlation
    pub chains: Option<Vec<String>>,
}

/// Event pattern definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPattern {
    /// Pattern name
    pub name: String,
    
    /// Sequence of event patterns to match
    pub sequence: Vec<EventPatternStep>,
    
    /// Time window for pattern matching (in seconds)
    pub time_window: Option<u64>,
    
    /// Whether pattern steps must be consecutive
    pub strict_order: bool,
    
    /// Maximum gap between pattern steps (in block numbers)
    pub max_gap: Option<u64>,
}

/// Single step in an event pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPatternStep {
    /// Event type to match
    pub event_type: Option<String>,
    
    /// Chain to match
    pub chain: Option<String>,
    
    /// Address to match
    pub address: Option<String>,
    
    /// Attributes that must match
    pub required_attributes: Option<HashMap<String, String>>,
    
    /// Optional step (pattern can continue without this step)
    pub optional: bool,
    
    /// Repeat this step (min, max) times
    pub repeat: Option<(usize, usize)>,
}

/// Result of event correlation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationResult {
    /// Correlation ID
    pub correlation_id: String,
    
    /// Events that are correlated
    pub events: Vec<String>, // Event IDs
    
    /// Field values that caused the correlation
    pub correlation_values: HashMap<String, String>,
    
    /// Time span of correlated events
    pub time_span: Option<(SystemTime, SystemTime)>,
    
    /// Block span of correlated events
    pub block_span: Option<(u64, u64)>,
    
    /// Chains involved in correlation
    pub chains: Vec<String>,
}

/// Result of pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatchResult {
    /// Pattern that was matched
    pub pattern_name: String,
    
    /// Events that matched the pattern (in order)
    pub matched_events: Vec<String>, // Event IDs
    
    /// Time when pattern started
    pub start_time: SystemTime,
    
    /// Time when pattern completed
    pub end_time: SystemTime,
    
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    
    /// Additional metadata about the match
    pub metadata: HashMap<String, String>,
}

impl Default for CorrelationConfig {
    fn default() -> Self {
        Self {
            correlation_fields: vec!["tx_hash".to_string()],
            time_window: Some(3600), // 1 hour
            max_block_distance: Some(100),
            min_events: Some(2),
            chains: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    // Mock event for testing
    #[derive(Debug, Clone)]
    struct TestEvent {
        id: String,
        chain: String,
        block_number: u64,
        block_hash: String,
        tx_hash: String,
        timestamp: SystemTime,
        event_type: String,
        raw_data: Vec<u8>,
    }
    
    impl crate::event::Event for TestEvent {
        fn id(&self) -> &str { &self.id }
        fn chain(&self) -> &str { &self.chain }
        fn block_number(&self) -> u64 { self.block_number }
        fn block_hash(&self) -> &str { &self.block_hash }
        fn tx_hash(&self) -> &str { &self.tx_hash }
        fn timestamp(&self) -> SystemTime { self.timestamp }
        fn event_type(&self) -> &str { &self.event_type }
        fn raw_data(&self) -> &[u8] { &self.raw_data }
        fn as_any(&self) -> &dyn std::any::Any { self }
    }
    
    #[test]
    fn test_event_filter_chain_matching() {
        let filter = EventFilter::new()
            .with_chain("ethereum".to_string());
            
        let event = TestEvent {
            id: "test".to_string(),
            chain: "ethereum".to_string(),
            block_number: 100,
            block_hash: "hash".to_string(),
            tx_hash: "tx".to_string(),
            timestamp: UNIX_EPOCH,
            event_type: "transfer".to_string(),
            raw_data: vec![],
        };
        
        assert!(filter.matches_event(&event));
        
        let wrong_chain_event = TestEvent {
            chain: "polygon".to_string(),
            ..event
        };
        
        assert!(!filter.matches_event(&wrong_chain_event));
    }
    
    #[test]
    fn test_event_filter_multiple_chains() {
        let filter = EventFilter::new()
            .with_chains(vec!["ethereum".to_string(), "polygon".to_string()]);
            
        let eth_event = TestEvent {
            id: "test".to_string(),
            chain: "ethereum".to_string(),
            block_number: 100,
            block_hash: "hash".to_string(),
            tx_hash: "tx".to_string(),
            timestamp: UNIX_EPOCH,
            event_type: "transfer".to_string(),
            raw_data: vec![],
        };
        
        let polygon_event = TestEvent {
            chain: "polygon".to_string(),
            ..eth_event.clone()
        };
        
        let cosmos_event = TestEvent {
            chain: "cosmos".to_string(),
            ..eth_event.clone()
        };
        
        assert!(filter.matches_event(&eth_event));
        assert!(filter.matches_event(&polygon_event));
        assert!(!filter.matches_event(&cosmos_event));
    }
    
    #[test]
    fn test_event_filter_block_ranges() {
        let filter = EventFilter::new()
            .with_block_range(100, 200);
            
        let event_in_range = TestEvent {
            id: "test".to_string(),
            chain: "ethereum".to_string(),
            block_number: 150,
            block_hash: "hash".to_string(),
            tx_hash: "tx".to_string(),
            timestamp: UNIX_EPOCH,
            event_type: "transfer".to_string(),
            raw_data: vec![],
        };
        
        let event_below_range = TestEvent {
            block_number: 50,
            ..event_in_range.clone()
        };
        
        let event_above_range = TestEvent {
            block_number: 250,
            ..event_in_range.clone()
        };
        
        assert!(filter.matches_event(&event_in_range));
        assert!(!filter.matches_event(&event_below_range));
        assert!(!filter.matches_event(&event_above_range));
    }
    
    #[test]
    fn test_event_filter_time_ranges() {
        let filter = EventFilter::new()
            .with_time_range(1000, 2000);
            
        let event_in_range = TestEvent {
            id: "test".to_string(),
            chain: "ethereum".to_string(),
            block_number: 100,
            block_hash: "hash".to_string(),
            tx_hash: "tx".to_string(),
            timestamp: UNIX_EPOCH + std::time::Duration::from_secs(1500),
            event_type: "transfer".to_string(),
            raw_data: vec![],
        };
        
        let event_below_range = TestEvent {
            timestamp: UNIX_EPOCH + std::time::Duration::from_secs(500),
            ..event_in_range.clone()
        };
        
        let event_above_range = TestEvent {
            timestamp: UNIX_EPOCH + std::time::Duration::from_secs(2500),
            ..event_in_range.clone()
        };
        
        assert!(filter.matches_event(&event_in_range));
        assert!(!filter.matches_event(&event_below_range));
        assert!(!filter.matches_event(&event_above_range));
    }
    
    #[test]
    fn test_event_filter_event_types() {
        let filter = EventFilter::new()
            .with_event_types(vec!["transfer".to_string(), "mint".to_string()]);
            
        let transfer_event = TestEvent {
            id: "test".to_string(),
            chain: "ethereum".to_string(),
            block_number: 100,
            block_hash: "hash".to_string(),
            tx_hash: "tx".to_string(),
            timestamp: UNIX_EPOCH,
            event_type: "transfer".to_string(),
            raw_data: vec![],
        };
        
        let mint_event = TestEvent {
            event_type: "mint".to_string(),
            ..transfer_event.clone()
        };
        
        let burn_event = TestEvent {
            event_type: "burn".to_string(),
            ..transfer_event.clone()
        };
        
        assert!(filter.matches_event(&transfer_event));
        assert!(filter.matches_event(&mint_event));
        assert!(!filter.matches_event(&burn_event));
    }
    
    #[test]
    fn test_event_filter_builder_pattern() {
        let filter = EventFilter::new()
            .with_chain("ethereum".to_string())
            .with_block_range(100, 200)
            .with_event_types(vec!["transfer".to_string()])
            .with_pagination(10, 0)
            .with_sort(SortField::BlockNumber, SortDirection::Descending);
            
        assert_eq!(filter.chain, Some("ethereum".to_string()));
        assert_eq!(filter.block_range, Some((100, 200)));
        assert_eq!(filter.event_types, Some(vec!["transfer".to_string()]));
        assert_eq!(filter.limit, Some(10));
        assert_eq!(filter.offset, Some(0));
        assert!(matches!(filter.sort_by, Some(SortField::BlockNumber)));
        assert!(matches!(filter.sort_direction, Some(SortDirection::Descending)));
    }
    
    #[test]
    fn test_text_search_contains() {
        let filter = EventFilter::new()
            .with_text_search("Alice".to_string(), TextSearchConfig {
                mode: TextSearchMode::Contains,
                case_sensitive: false,
                ..Default::default()
            });
            
        let event_with_alice = TestEvent {
            id: "test".to_string(),
            chain: "ethereum".to_string(),
            block_number: 100,
            block_hash: "hash".to_string(),
            tx_hash: "tx".to_string(),
            timestamp: UNIX_EPOCH,
            event_type: "transfer".to_string(),
            raw_data: "Alice sent tokens to Bob".as_bytes().to_vec(),
        };
        
        let event_without_alice = TestEvent {
            raw_data: "Charlie sent tokens to Dave".as_bytes().to_vec(),
            ..event_with_alice.clone()
        };
        
        assert!(filter.matches_event(&event_with_alice));
        assert!(!filter.matches_event(&event_without_alice));
    }
    
    #[test]
    fn test_text_search_case_sensitive() {
        let case_sensitive_filter = EventFilter::new()
            .with_text_search("Alice".to_string(), TextSearchConfig {
                mode: TextSearchMode::Contains,
                case_sensitive: true,
                ..Default::default()
            });
            
        let case_insensitive_filter = EventFilter::new()
            .with_text_search("Alice".to_string(), TextSearchConfig {
                mode: TextSearchMode::Contains,
                case_sensitive: false,
                ..Default::default()
            });
            
        let event = TestEvent {
            id: "test".to_string(),
            chain: "ethereum".to_string(),
            block_number: 100,
            block_hash: "hash".to_string(),
            tx_hash: "tx".to_string(),
            timestamp: UNIX_EPOCH,
            event_type: "transfer".to_string(),
            raw_data: "alice sent tokens".as_bytes().to_vec(),
        };
        
        assert!(!case_sensitive_filter.matches_event(&event)); // "Alice" != "alice"
        assert!(case_insensitive_filter.matches_event(&event)); // "Alice" == "alice" (case insensitive)
    }
    
    #[test]
    fn test_text_search_fuzzy() {
        let filter = EventFilter::new()
            .with_text_search("Alice".to_string(), TextSearchConfig {
                mode: TextSearchMode::Fuzzy { max_distance: 2 },
                case_sensitive: false,
                ..Default::default()
            });
            
        let event_exact = TestEvent {
            id: "test".to_string(),
            chain: "ethereum".to_string(),
            block_number: 100,
            block_hash: "hash".to_string(),
            tx_hash: "tx".to_string(),
            timestamp: UNIX_EPOCH,
            event_type: "transfer".to_string(),
            raw_data: "Alice sent tokens".as_bytes().to_vec(),
        };
        
        let event_fuzzy = TestEvent {
            raw_data: "Alicia sent tokens".as_bytes().to_vec(),
            ..event_exact.clone()
        };
        
        let event_no_match = TestEvent {
            raw_data: "Bob sent tokens".as_bytes().to_vec(),
            ..event_exact.clone()
        };
        
        assert!(filter.matches_event(&event_exact));
        assert!(filter.matches_event(&event_fuzzy)); // "Alicia" is within distance 2 of "Alice"
        assert!(!filter.matches_event(&event_no_match));
    }
    
    #[test]
    fn test_text_search_regex() {
        let filter = EventFilter::new()
            .with_text_search(r"0x[0-9a-f]+".to_string(), TextSearchConfig {
                mode: TextSearchMode::Regex,
                case_sensitive: false,
                ..Default::default()
            });
            
        let event_with_hex = TestEvent {
            id: "test".to_string(),
            chain: "ethereum".to_string(),
            block_number: 100,
            block_hash: "hash".to_string(),
            tx_hash: "tx".to_string(),
            timestamp: UNIX_EPOCH,
            event_type: "transfer".to_string(),
            raw_data: "Transaction hash: 0x123abc".as_bytes().to_vec(),
        };
        
        let event_without_hex = TestEvent {
            raw_data: "No hex here".as_bytes().to_vec(),
            ..event_with_hex.clone()
        };
        
        assert!(filter.matches_event(&event_with_hex));
        assert!(!filter.matches_event(&event_without_hex));
    }
    
    #[test]
    fn test_text_search_phrase() {
        let filter = EventFilter::new()
            .with_text_search("sent tokens".to_string(), TextSearchConfig {
                mode: TextSearchMode::Phrase,
                case_sensitive: false,
                ..Default::default()
            });
            
        let event_with_phrase = TestEvent {
            id: "test".to_string(),
            chain: "ethereum".to_string(),
            block_number: 100,
            block_hash: "hash".to_string(),
            tx_hash: "tx".to_string(),
            timestamp: UNIX_EPOCH,
            event_type: "transfer".to_string(),
            raw_data: "Alice sent tokens to Bob".as_bytes().to_vec(),
        };
        
        let event_without_phrase = TestEvent {
            raw_data: "Alice tokens sent to Bob".as_bytes().to_vec(), // Words not in sequence
            ..event_with_phrase.clone()
        };
        
        assert!(filter.matches_event(&event_with_phrase));
        assert!(!filter.matches_event(&event_without_phrase));
    }
    
    #[test]
    fn test_text_search_boolean() {
        let and_filter = EventFilter::new()
            .with_text_search("Alice and tokens".to_string(), TextSearchConfig {
                mode: TextSearchMode::Boolean,
                case_sensitive: false,
                ..Default::default()
            });
            
        let or_filter = EventFilter::new()
            .with_text_search("Alice or Charlie".to_string(), TextSearchConfig {
                mode: TextSearchMode::Boolean,
                case_sensitive: false,
                ..Default::default()
            });
            
        let not_filter = EventFilter::new()
            .with_text_search("not Bob".to_string(), TextSearchConfig {
                mode: TextSearchMode::Boolean,
                case_sensitive: false,
                ..Default::default()
            });
            
        let event_alice_tokens = TestEvent {
            id: "test".to_string(),
            chain: "ethereum".to_string(),
            block_number: 100,
            block_hash: "hash".to_string(),
            tx_hash: "tx".to_string(),
            timestamp: UNIX_EPOCH,
            event_type: "transfer".to_string(),
            raw_data: "Alice sent tokens".as_bytes().to_vec(),
        };
        
        let event_charlie_coins = TestEvent {
            raw_data: "Charlie sent coins".as_bytes().to_vec(),
            ..event_alice_tokens.clone()
        };
        
        let event_bob_tokens = TestEvent {
            raw_data: "Bob sent tokens".as_bytes().to_vec(),
            ..event_alice_tokens.clone()
        };
        
        // Test AND
        assert!(and_filter.matches_event(&event_alice_tokens)); // Has both "Alice" and "tokens"
        assert!(!and_filter.matches_event(&event_charlie_coins)); // Has neither
        
        // Test OR
        assert!(or_filter.matches_event(&event_alice_tokens)); // Has "Alice"
        assert!(or_filter.matches_event(&event_charlie_coins)); // Has "Charlie"
        assert!(!or_filter.matches_event(&event_bob_tokens)); // Has neither
        
        // Test NOT
        assert!(not_filter.matches_event(&event_alice_tokens)); // Doesn't have "Bob"
        assert!(!not_filter.matches_event(&event_bob_tokens)); // Has "Bob"
    }
    
    #[test]
    fn test_address_filters() {
        let include_filter = EventFilter::new()
            .with_addresses(vec!["0x123".to_string(), "0x456".to_string()]);
            
        let exclude_filter = EventFilter {
            exclude_addresses: Some(vec!["0x789".to_string()]),
            ..EventFilter::new()
        };
            
        let event_with_included = TestEvent {
            id: "test".to_string(),
            chain: "ethereum".to_string(),
            block_number: 100,
            block_hash: "hash".to_string(),
            tx_hash: "tx".to_string(),
            timestamp: UNIX_EPOCH,
            event_type: "transfer".to_string(),
            raw_data: "Transfer from 0x123 to 0x999".as_bytes().to_vec(),
        };
        
        let event_with_excluded = TestEvent {
            raw_data: "Transfer from 0x789 to 0x999".as_bytes().to_vec(),
            ..event_with_included.clone()
        };
        
        let event_with_neither = TestEvent {
            raw_data: "Transfer from 0xabc to 0xdef".as_bytes().to_vec(),
            ..event_with_included.clone()
        };
        
        // Test include filter
        assert!(include_filter.matches_event(&event_with_included));
        assert!(!include_filter.matches_event(&event_with_excluded));
        assert!(!include_filter.matches_event(&event_with_neither));
        
        // Test exclude filter
        assert!(exclude_filter.matches_event(&event_with_included));
        assert!(!exclude_filter.matches_event(&event_with_excluded));
        assert!(exclude_filter.matches_event(&event_with_neither));
    }
} 