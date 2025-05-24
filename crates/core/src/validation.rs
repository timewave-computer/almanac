/// Chain-specific data validation functionality
use std::collections::HashMap;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use regex::Regex;

use crate::event::Event;
use crate::{Result, Error};

/// Validation configuration for a blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainValidationConfig {
    /// Chain name
    pub chain_name: String,
    
    /// Chain ID for verification
    pub chain_id: u64,
    
    /// Address format validation
    pub address_format: AddressFormat,
    
    /// Transaction hash format
    pub tx_hash_format: HashFormat,
    
    /// Block hash format
    pub block_hash_format: HashFormat,
    
    /// Block number constraints
    pub block_constraints: BlockConstraints,
    
    /// Event type validation rules
    pub event_type_rules: Vec<EventTypeRule>,
    
    /// Custom validation rules
    pub custom_rules: Vec<CustomValidationRule>,
    
    /// Whether to enforce strict validation
    pub strict_mode: bool,
    
    /// Maximum allowed timestamp deviation (seconds)
    pub max_timestamp_deviation: u64,
}

impl Default for ChainValidationConfig {
    fn default() -> Self {
        Self {
            chain_name: "unknown".to_string(),
            chain_id: 0,
            address_format: AddressFormat::Ethereum,
            tx_hash_format: HashFormat::Keccak256,
            block_hash_format: HashFormat::Keccak256,
            block_constraints: BlockConstraints::default(),
            event_type_rules: Vec::new(),
            custom_rules: Vec::new(),
            strict_mode: false,
            max_timestamp_deviation: 300, // 5 minutes
        }
    }
}

/// Address format types for different blockchains
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AddressFormat {
    /// Ethereum-style addresses (0x + 40 hex chars)
    Ethereum,
    
    /// Bitcoin-style addresses (various formats)
    Bitcoin,
    
    /// Cosmos-style addresses (bech32 encoded)
    Cosmos { prefix: String },
    
    /// Solana-style addresses (base58 encoded, 32 bytes)
    Solana,
    
    /// Custom format with regex pattern
    Custom { pattern: String },
}

/// Hash format types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HashFormat {
    /// Keccak256 hash (0x + 64 hex chars)
    Keccak256,
    
    /// SHA256 hash (64 hex chars)
    Sha256,
    
    /// Blake2b hash (64 hex chars)
    Blake2b,
    
    /// Custom hash format with regex
    Custom { pattern: String },
}

/// Block number constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockConstraints {
    /// Minimum valid block number
    pub min_block: u64,
    
    /// Maximum valid block number (0 = no limit)
    pub max_block: u64,
    
    /// Expected block time in seconds
    pub expected_block_time: u64,
    
    /// Maximum block time deviation allowed
    pub max_block_time_deviation: u64,
}

impl Default for BlockConstraints {
    fn default() -> Self {
        Self {
            min_block: 0,
            max_block: 0, // No limit
            expected_block_time: 12, // Ethereum default
            max_block_time_deviation: 60, // 1 minute
        }
    }
}

/// Event type validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTypeRule {
    /// Event type pattern (regex)
    pub event_type_pattern: String,
    
    /// Required fields for this event type
    pub required_fields: Vec<String>,
    
    /// Optional fields
    pub optional_fields: Vec<String>,
    
    /// Field validation rules
    pub field_validations: HashMap<String, FieldValidation>,
    
    /// Whether this event type is deprecated
    pub deprecated: bool,
}

/// Field validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValidation {
    /// Field data type
    pub field_type: FieldType,
    
    /// Validation pattern (regex)
    pub pattern: Option<String>,
    
    /// Minimum value (for numeric types)
    pub min_value: Option<f64>,
    
    /// Maximum value (for numeric types)
    pub max_value: Option<f64>,
    
    /// Allowed enum values
    pub allowed_values: Option<Vec<String>>,
    
    /// Whether field is required
    pub required: bool,
}

/// Field data types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FieldType {
    String,
    Number,
    Boolean,
    Address,
    Hash,
    Timestamp,
    Amount,
    Custom(String),
}

/// Custom validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomValidationRule {
    /// Rule name
    pub name: String,
    
    /// Rule description
    pub description: String,
    
    /// Fields this rule applies to
    pub applies_to: Vec<String>,
    
    /// Validation expression (simplified)
    pub expression: String,
    
    /// Error message if validation fails
    pub error_message: String,
    
    /// Rule severity
    pub severity: ValidationSeverity,
}

/// Validation severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub is_valid: bool,
    
    /// List of validation errors
    pub errors: Vec<ValidationError>,
    
    /// List of validation warnings
    pub warnings: Vec<ValidationWarning>,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error code
    pub code: String,
    
    /// Error message
    pub message: String,
    
    /// Field that caused the error
    pub field: Option<String>,
    
    /// Expected value or format
    pub expected: Option<String>,
    
    /// Actual value received
    pub actual: Option<String>,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning code
    pub code: String,
    
    /// Warning message
    pub message: String,
    
    /// Field that caused the warning
    pub field: Option<String>,
}

/// Chain-specific validator trait
#[async_trait]
pub trait ChainValidator: Send + Sync {
    /// Validate an event according to chain rules
    async fn validate_event(
        &self,
        event: &dyn Event,
        config: &ChainValidationConfig,
    ) -> Result<ValidationResult>;
    
    /// Validate multiple events in batch
    async fn validate_events(
        &self,
        events: Vec<&dyn Event>,
        config: &ChainValidationConfig,
    ) -> Result<Vec<ValidationResult>>;
    
    /// Validate chain-specific data format
    async fn validate_chain_data(
        &self,
        data: &serde_json::Value,
        config: &ChainValidationConfig,
    ) -> Result<ValidationResult>;
}

/// Default chain validator implementation
pub struct DefaultChainValidator {
    /// Cached regex patterns for performance
    regex_cache: HashMap<String, Regex>,
}

impl DefaultChainValidator {
    pub fn new() -> Self {
        Self {
            regex_cache: HashMap::new(),
        }
    }
    
    /// Get or compile regex pattern
    fn get_regex(&mut self, pattern: &str) -> Result<&Regex> {
        if !self.regex_cache.contains_key(pattern) {
            let regex = Regex::new(pattern)
                .map_err(|e| Error::InvalidData(format!("Invalid regex pattern '{}': {}", pattern, e)))?;
            self.regex_cache.insert(pattern.to_string(), regex);
        }
        Ok(self.regex_cache.get(pattern).unwrap())
    }
    
    /// Validate address format
    fn validate_address(&mut self, address: &str, format: &AddressFormat) -> Result<bool> {
        match format {
            AddressFormat::Ethereum => {
                let pattern = r"^0x[a-fA-F0-9]{40}$";
                let regex = self.get_regex(pattern)?;
                Ok(regex.is_match(address))
            }
            AddressFormat::Bitcoin => {
                // Simplified Bitcoin address validation (supports multiple formats)
                let patterns = vec![
                    r"^[13][a-km-zA-HJ-NP-Z1-9]{25,34}$", // Legacy
                    r"^bc1[a-z0-9]{39,59}$", // Bech32
                ];
                for pattern in patterns {
                    let regex = self.get_regex(pattern)?;
                    if regex.is_match(address) {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            AddressFormat::Cosmos { prefix } => {
                let pattern = format!(r"^{}[a-z0-9]{{38,58}}$", prefix);
                let regex = self.get_regex(&pattern)?;
                Ok(regex.is_match(address))
            }
            AddressFormat::Solana => {
                // Solana addresses are base58 encoded, 32 bytes (44 chars)
                let pattern = r"^[1-9A-HJ-NP-Za-km-z]{43,44}$";
                let regex = self.get_regex(pattern)?;
                Ok(regex.is_match(address))
            }
            AddressFormat::Custom { pattern } => {
                let regex = self.get_regex(pattern)?;
                Ok(regex.is_match(address))
            }
        }
    }
    
    /// Validate hash format
    fn validate_hash(&mut self, hash: &str, format: &HashFormat) -> Result<bool> {
        match format {
            HashFormat::Keccak256 => {
                let pattern = r"^0x[a-fA-F0-9]{64}$";
                let regex = self.get_regex(pattern)?;
                Ok(regex.is_match(hash))
            }
            HashFormat::Sha256 | HashFormat::Blake2b => {
                let pattern = r"^[a-fA-F0-9]{64}$";
                let regex = self.get_regex(pattern)?;
                Ok(regex.is_match(hash))
            }
            HashFormat::Custom { pattern } => {
                let regex = self.get_regex(pattern)?;
                Ok(regex.is_match(hash))
            }
        }
    }
    
    /// Validate field value according to validation rule
    fn validate_field_value(
        &mut self,
        field_name: &str,
        value: &serde_json::Value,
        validation: &FieldValidation,
    ) -> Result<Vec<ValidationError>> {
        let mut errors = Vec::new();
        
        // Check if required field is present
        if validation.required && value.is_null() {
            errors.push(ValidationError {
                code: "REQUIRED_FIELD_MISSING".to_string(),
                message: format!("Required field '{}' is missing", field_name),
                field: Some(field_name.to_string()),
                expected: Some("non-null value".to_string()),
                actual: Some("null".to_string()),
            });
            return Ok(errors);
        }
        
        if value.is_null() {
            return Ok(errors); // Optional field is null, that's fine
        }
        
        // Validate field type
        match &validation.field_type {
            FieldType::String => {
                if !value.is_string() {
                    errors.push(ValidationError {
                        code: "INVALID_TYPE".to_string(),
                        message: format!("Field '{}' must be a string", field_name),
                        field: Some(field_name.to_string()),
                        expected: Some("string".to_string()),
                        actual: Some(format!("{:?}", value)),
                    });
                }
            }
            FieldType::Number => {
                if !value.is_number() {
                    errors.push(ValidationError {
                        code: "INVALID_TYPE".to_string(),
                        message: format!("Field '{}' must be a number", field_name),
                        field: Some(field_name.to_string()),
                        expected: Some("number".to_string()),
                        actual: Some(format!("{:?}", value)),
                    });
                }
            }
            FieldType::Boolean => {
                if !value.is_boolean() {
                    errors.push(ValidationError {
                        code: "INVALID_TYPE".to_string(),
                        message: format!("Field '{}' must be a boolean", field_name),
                        field: Some(field_name.to_string()),
                        expected: Some("boolean".to_string()),
                        actual: Some(format!("{:?}", value)),
                    });
                }
            }
            FieldType::Address => {
                if let Some(addr_str) = value.as_str() {
                    // Address validation would need format info - simplified here
                    if addr_str.is_empty() {
                        errors.push(ValidationError {
                            code: "INVALID_ADDRESS".to_string(),
                            message: format!("Field '{}' cannot be an empty address", field_name),
                            field: Some(field_name.to_string()),
                            expected: Some("valid address".to_string()),
                            actual: Some(addr_str.to_string()),
                        });
                    }
                }
            }
            FieldType::Hash => {
                if let Some(hash_str) = value.as_str() {
                    if hash_str.is_empty() {
                        errors.push(ValidationError {
                            code: "INVALID_HASH".to_string(),
                            message: format!("Field '{}' cannot be an empty hash", field_name),
                            field: Some(field_name.to_string()),
                            expected: Some("valid hash".to_string()),
                            actual: Some(hash_str.to_string()),
                        });
                    }
                }
            }
            FieldType::Timestamp => {
                if let Some(ts) = value.as_u64() {
                    let now = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    
                    // Basic sanity check - timestamp shouldn't be too far in future
                    if ts > now + 86400 { // 1 day in future
                        errors.push(ValidationError {
                            code: "INVALID_TIMESTAMP".to_string(),
                            message: format!("Field '{}' timestamp is too far in the future", field_name),
                            field: Some(field_name.to_string()),
                            expected: Some("reasonable timestamp".to_string()),
                            actual: Some(ts.to_string()),
                        });
                    }
                }
            }
            FieldType::Amount => {
                if let Some(amount) = value.as_f64() {
                    if amount < 0.0 {
                        errors.push(ValidationError {
                            code: "INVALID_AMOUNT".to_string(),
                            message: format!("Field '{}' amount cannot be negative", field_name),
                            field: Some(field_name.to_string()),
                            expected: Some("non-negative amount".to_string()),
                            actual: Some(amount.to_string()),
                        });
                    }
                }
            }
            FieldType::Custom(_) => {
                // Custom validation would be implemented based on requirements
            }
        }
        
        // Validate pattern if specified
        if let Some(pattern) = &validation.pattern {
            if let Some(str_value) = value.as_str() {
                let regex = self.get_regex(pattern)?;
                if !regex.is_match(str_value) {
                    errors.push(ValidationError {
                        code: "PATTERN_MISMATCH".to_string(),
                        message: format!("Field '{}' does not match required pattern", field_name),
                        field: Some(field_name.to_string()),
                        expected: Some(pattern.clone()),
                        actual: Some(str_value.to_string()),
                    });
                }
            }
        }
        
        // Validate numeric range
        if let Some(num_value) = value.as_f64() {
            if let Some(min_val) = validation.min_value {
                if num_value < min_val {
                    errors.push(ValidationError {
                        code: "VALUE_TOO_LOW".to_string(),
                        message: format!("Field '{}' value is below minimum", field_name),
                        field: Some(field_name.to_string()),
                        expected: Some(format!(">= {}", min_val)),
                        actual: Some(num_value.to_string()),
                    });
                }
            }
            
            if let Some(max_val) = validation.max_value {
                if num_value > max_val {
                    errors.push(ValidationError {
                        code: "VALUE_TOO_HIGH".to_string(),
                        message: format!("Field '{}' value is above maximum", field_name),
                        field: Some(field_name.to_string()),
                        expected: Some(format!("<= {}", max_val)),
                        actual: Some(num_value.to_string()),
                    });
                }
            }
        }
        
        // Validate allowed values
        if let Some(allowed) = &validation.allowed_values {
            if let Some(str_value) = value.as_str() {
                if !allowed.contains(&str_value.to_string()) {
                    errors.push(ValidationError {
                        code: "INVALID_VALUE".to_string(),
                        message: format!("Field '{}' has invalid value", field_name),
                        field: Some(field_name.to_string()),
                        expected: Some(format!("one of: {:?}", allowed)),
                        actual: Some(str_value.to_string()),
                    });
                }
            }
        }
        
        Ok(errors)
    }
    
    /// Validate event type according to rules
    fn validate_event_type(
        &mut self,
        event_type: &str,
        event_data: &serde_json::Value,
        rules: &[EventTypeRule],
    ) -> Result<Vec<ValidationError>> {
        let mut errors = Vec::new();
        
        // Find matching rule
        let matching_rule = rules.iter().find(|rule| {
            if let Ok(regex) = Regex::new(&rule.event_type_pattern) {
                regex.is_match(event_type)
            } else {
                false
            }
        });
        
        if let Some(rule) = matching_rule {
            if rule.deprecated {
                // This would be a warning in practice
                errors.push(ValidationError {
                    code: "DEPRECATED_EVENT_TYPE".to_string(),
                    message: format!("Event type '{}' is deprecated", event_type),
                    field: Some("event_type".to_string()),
                    expected: Some("non-deprecated event type".to_string()),
                    actual: Some(event_type.to_string()),
                });
            }
            
            // Check required fields
            if let Some(obj) = event_data.as_object() {
                for required_field in &rule.required_fields {
                    if !obj.contains_key(required_field) {
                        errors.push(ValidationError {
                            code: "MISSING_REQUIRED_FIELD".to_string(),
                            message: format!("Required field '{}' missing for event type '{}'", required_field, event_type),
                            field: Some(required_field.clone()),
                            expected: Some("field present".to_string()),
                            actual: Some("field missing".to_string()),
                        });
                    }
                }
                
                // Validate individual fields
                for (field_name, field_validation) in &rule.field_validations {
                    let field_value = obj.get(field_name).unwrap_or(&serde_json::Value::Null);
                    let field_errors = self.validate_field_value(field_name, field_value, field_validation)?;
                    errors.extend(field_errors);
                }
            }
        }
        
        Ok(errors)
    }
}

#[async_trait]
impl ChainValidator for DefaultChainValidator {
    async fn validate_event(
        &self,
        event: &dyn Event,
        config: &ChainValidationConfig,
    ) -> Result<ValidationResult> {
        let mut validator = self.clone();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut metadata = HashMap::new();
        
        // Validate basic event properties
        
        // Chain validation
        if event.chain() != config.chain_name {
            if config.strict_mode {
                errors.push(ValidationError {
                    code: "CHAIN_MISMATCH".to_string(),
                    message: format!("Event chain '{}' does not match expected '{}'", event.chain(), config.chain_name),
                    field: Some("chain".to_string()),
                    expected: Some(config.chain_name.clone()),
                    actual: Some(event.chain().to_string()),
                });
            } else {
                warnings.push(ValidationWarning {
                    code: "CHAIN_MISMATCH".to_string(),
                    message: format!("Event chain '{}' does not match expected '{}'", event.chain(), config.chain_name),
                    field: Some("chain".to_string()),
                });
            }
        }
        
        // Block number validation
        if config.block_constraints.max_block > 0 && event.block_number() > config.block_constraints.max_block {
            errors.push(ValidationError {
                code: "BLOCK_NUMBER_TOO_HIGH".to_string(),
                message: format!("Block number {} exceeds maximum {}", event.block_number(), config.block_constraints.max_block),
                field: Some("block_number".to_string()),
                expected: Some(format!("<= {}", config.block_constraints.max_block)),
                actual: Some(event.block_number().to_string()),
            });
        }
        
        if event.block_number() < config.block_constraints.min_block {
            errors.push(ValidationError {
                code: "BLOCK_NUMBER_TOO_LOW".to_string(),
                message: format!("Block number {} below minimum {}", event.block_number(), config.block_constraints.min_block),
                field: Some("block_number".to_string()),
                expected: Some(format!(">= {}", config.block_constraints.min_block)),
                actual: Some(event.block_number().to_string()),
            });
        }
        
        // Transaction hash validation
        let tx_hash_valid = validator.validate_hash(event.tx_hash(), &config.tx_hash_format)?;
        if !tx_hash_valid {
            errors.push(ValidationError {
                code: "INVALID_TX_HASH_FORMAT".to_string(),
                message: "Transaction hash format is invalid".to_string(),
                field: Some("tx_hash".to_string()),
                expected: Some(format!("{:?}", config.tx_hash_format)),
                actual: Some(event.tx_hash().to_string()),
            });
        }
        
        // Block hash validation
        let block_hash_valid = validator.validate_hash(event.block_hash(), &config.block_hash_format)?;
        if !block_hash_valid {
            errors.push(ValidationError {
                code: "INVALID_BLOCK_HASH_FORMAT".to_string(),
                message: "Block hash format is invalid".to_string(),
                field: Some("block_hash".to_string()),
                expected: Some(format!("{:?}", config.block_hash_format)),
                actual: Some(event.block_hash().to_string()),
            });
        }
        
        // Timestamp validation
        let now = SystemTime::now();
        let event_time = event.timestamp();
        let time_diff = if now > event_time {
            now.duration_since(event_time).unwrap_or_default().as_secs()
        } else {
            event_time.duration_since(now).unwrap_or_default().as_secs()
        };
        
        if time_diff > config.max_timestamp_deviation {
            if config.strict_mode {
                errors.push(ValidationError {
                    code: "TIMESTAMP_DEVIATION".to_string(),
                    message: format!("Event timestamp deviates too much from current time ({} seconds)", time_diff),
                    field: Some("timestamp".to_string()),
                    expected: Some(format!("within {} seconds of now", config.max_timestamp_deviation)),
                    actual: Some(format!("{} seconds deviation", time_diff)),
                });
            } else {
                warnings.push(ValidationWarning {
                    code: "TIMESTAMP_DEVIATION".to_string(),
                    message: format!("Event timestamp deviates from current time ({} seconds)", time_diff),
                    field: Some("timestamp".to_string()),
                });
            }
        }
        
        // Parse and validate event data
        if let Ok(raw_str) = String::from_utf8(event.raw_data().to_vec()) {
            if let Ok(event_data) = serde_json::from_str::<serde_json::Value>(&raw_str) {
                // Validate event type according to rules
                let event_type_errors = validator.validate_event_type(
                    event.event_type(),
                    &event_data,
                    &config.event_type_rules,
                )?;
                errors.extend(event_type_errors);
                
                metadata.insert("data_parsed".to_string(), "true".to_string());
            } else {
                if config.strict_mode {
                    errors.push(ValidationError {
                        code: "INVALID_JSON".to_string(),
                        message: "Event raw data is not valid JSON".to_string(),
                        field: Some("raw_data".to_string()),
                        expected: Some("valid JSON".to_string()),
                        actual: Some("invalid JSON".to_string()),
                    });
                }
                metadata.insert("data_parsed".to_string(), "false".to_string());
            }
        }
        
        // Add validation metadata
        metadata.insert("validator".to_string(), "DefaultChainValidator".to_string());
        metadata.insert("config_chain".to_string(), config.chain_name.clone());
        metadata.insert("strict_mode".to_string(), config.strict_mode.to_string());
        
        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            metadata,
        })
    }
    
    async fn validate_events(
        &self,
        events: Vec<&dyn Event>,
        config: &ChainValidationConfig,
    ) -> Result<Vec<ValidationResult>> {
        let mut results = Vec::new();
        
        for event in events {
            let result = self.validate_event(event, config).await?;
            results.push(result);
        }
        
        Ok(results)
    }
    
    async fn validate_chain_data(
        &self,
        data: &serde_json::Value,
        config: &ChainValidationConfig,
    ) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut metadata = HashMap::new();
        
        // Validate chain-specific data structure
        if let Some(obj) = data.as_object() {
            // Example: validate chain ID if present
            if let Some(chain_id_value) = obj.get("chain_id") {
                if let Some(chain_id) = chain_id_value.as_u64() {
                    if chain_id != config.chain_id {
                        errors.push(ValidationError {
                            code: "CHAIN_ID_MISMATCH".to_string(),
                            message: format!("Chain ID {} does not match expected {}", chain_id, config.chain_id),
                            field: Some("chain_id".to_string()),
                            expected: Some(config.chain_id.to_string()),
                            actual: Some(chain_id.to_string()),
                        });
                    }
                }
            }
            
            metadata.insert("fields_count".to_string(), obj.len().to_string());
        }
        
        metadata.insert("data_type".to_string(), format!("{:?}", data));
        
        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            metadata,
        })
    }
}

impl Clone for DefaultChainValidator {
    fn clone(&self) -> Self {
        Self {
            regex_cache: HashMap::new(), // Don't clone regex cache for simplicity
        }
    }
}

/// Chain validation manager for multiple chains
pub struct ChainValidationManager {
    validators: HashMap<String, Box<dyn ChainValidator>>,
    configs: HashMap<String, ChainValidationConfig>,
}

impl ChainValidationManager {
    /// Create a new validation manager
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            configs: HashMap::new(),
        }
    }
    
    /// Register a validator for a specific chain
    pub fn register_chain(
        &mut self,
        chain_name: String,
        config: ChainValidationConfig,
        validator: Box<dyn ChainValidator>,
    ) {
        self.configs.insert(chain_name.clone(), config);
        self.validators.insert(chain_name, validator);
    }
    
    /// Validate an event using the appropriate chain validator
    pub async fn validate_event(&self, event: &dyn Event) -> Result<ValidationResult> {
        let chain = event.chain();
        
        if let (Some(validator), Some(config)) = (
            self.validators.get(chain),
            self.configs.get(chain)
        ) {
            validator.validate_event(event, config).await
        } else {
            Err(Error::Generic(format!("No validator registered for chain: {}", chain)))
        }
    }
    
    /// Get validation config for a chain
    pub fn get_config(&self, chain: &str) -> Option<&ChainValidationConfig> {
        self.configs.get(chain)
    }
    
    /// List registered chains
    pub fn list_chains(&self) -> Vec<String> {
        self.configs.keys().cloned().collect()
    }
}

/// Predefined validation configs for common chains
pub struct PredefinedConfigs;

impl PredefinedConfigs {
    /// Get Ethereum validation config
    pub fn ethereum() -> ChainValidationConfig {
        ChainValidationConfig {
            chain_name: "ethereum".to_string(),
            chain_id: 1,
            address_format: AddressFormat::Ethereum,
            tx_hash_format: HashFormat::Keccak256,
            block_hash_format: HashFormat::Keccak256,
            block_constraints: BlockConstraints {
                min_block: 0,
                max_block: 0,
                expected_block_time: 12,
                max_block_time_deviation: 60,
            },
            event_type_rules: vec![
                EventTypeRule {
                    event_type_pattern: r"^Transfer$".to_string(),
                    required_fields: vec!["from".to_string(), "to".to_string(), "value".to_string()],
                    optional_fields: vec!["token".to_string()],
                    field_validations: HashMap::from([
                        ("from".to_string(), FieldValidation {
                            field_type: FieldType::Address,
                            pattern: None,
                            min_value: None,
                            max_value: None,
                            allowed_values: None,
                            required: true,
                        }),
                        ("to".to_string(), FieldValidation {
                            field_type: FieldType::Address,
                            pattern: None,
                            min_value: None,
                            max_value: None,
                            allowed_values: None,
                            required: true,
                        }),
                        ("value".to_string(), FieldValidation {
                            field_type: FieldType::Amount,
                            pattern: None,
                            min_value: Some(0.0),
                            max_value: None,
                            allowed_values: None,
                            required: true,
                        }),
                    ]),
                    deprecated: false,
                },
            ],
            custom_rules: Vec::new(),
            strict_mode: false,
            max_timestamp_deviation: 300,
        }
    }
    
    /// Get Polygon validation config
    pub fn polygon() -> ChainValidationConfig {
        let mut config = Self::ethereum();
        config.chain_name = "polygon".to_string();
        config.chain_id = 137;
        config.block_constraints.expected_block_time = 2;
        config
    }
    
    /// Get Cosmos validation config
    pub fn cosmos() -> ChainValidationConfig {
        ChainValidationConfig {
            chain_name: "cosmos".to_string(),
            chain_id: 1,
            address_format: AddressFormat::Cosmos { prefix: "cosmos".to_string() },
            tx_hash_format: HashFormat::Sha256,
            block_hash_format: HashFormat::Sha256,
            block_constraints: BlockConstraints {
                min_block: 0,
                max_block: 0,
                expected_block_time: 6,
                max_block_time_deviation: 30,
            },
            event_type_rules: Vec::new(),
            custom_rules: Vec::new(),
            strict_mode: false,
            max_timestamp_deviation: 300,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{UNIX_EPOCH, Duration};
    
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
    
    impl Event for TestEvent {
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
    
    fn create_test_event(
        chain: &str,
        event_type: &str,
        tx_hash: &str,
        block_hash: &str,
        raw_data: &str,
    ) -> TestEvent {
        TestEvent {
            id: "test_event".to_string(),
            chain: chain.to_string(),
            block_number: 100,
            block_hash: block_hash.to_string(),
            tx_hash: tx_hash.to_string(),
            timestamp: SystemTime::now(),
            event_type: event_type.to_string(),
            raw_data: raw_data.as_bytes().to_vec(),
        }
    }
    
    #[tokio::test]
    async fn test_ethereum_validation() {
        let validator = DefaultChainValidator::new();
        let config = PredefinedConfigs::ethereum();
        
        let valid_event = create_test_event(
            "ethereum",
            "Transfer",
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            r#"{"from": "0x1234567890123456789012345678901234567890", "to": "0x0987654321098765432109876543210987654321", "value": "1000"}"#,
        );
        
        let result = validator.validate_event(&valid_event, &config).await.unwrap();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }
    
    #[tokio::test]
    async fn test_invalid_hash_format() {
        let validator = DefaultChainValidator::new();
        let config = PredefinedConfigs::ethereum();
        
        let invalid_event = create_test_event(
            "ethereum",
            "Transfer",
            "invalid_hash", // Invalid hash format
            "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            r#"{"from": "0x1234567890123456789012345678901234567890", "to": "0x0987654321098765432109876543210987654321", "value": "1000"}"#,
        );
        
        let result = validator.validate_event(&invalid_event, &config).await.unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert_eq!(result.errors[0].code, "INVALID_TX_HASH_FORMAT");
    }
    
    #[tokio::test]
    async fn test_missing_required_field() {
        let validator = DefaultChainValidator::new();
        let config = PredefinedConfigs::ethereum();
        
        let invalid_event = create_test_event(
            "ethereum",
            "Transfer",
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            r#"{"from": "0x1234567890123456789012345678901234567890", "value": "1000"}"#, // Missing "to" field
        );
        
        let result = validator.validate_event(&invalid_event, &config).await.unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert_eq!(result.errors[0].code, "MISSING_REQUIRED_FIELD");
    }
    
    #[test]
    fn test_address_validation() {
        let mut validator = DefaultChainValidator::new();
        
        // Test Ethereum address
        let eth_format = AddressFormat::Ethereum;
        assert!(validator.validate_address("0x1234567890123456789012345678901234567890", &eth_format).unwrap());
        assert!(!validator.validate_address("invalid_address", &eth_format).unwrap());
        
        // Test Cosmos address
        let cosmos_format = AddressFormat::Cosmos { prefix: "cosmos".to_string() };
        assert!(validator.validate_address("cosmos1234567890123456789012345678901234567890", &cosmos_format).unwrap());
        assert!(!validator.validate_address("invalid1234567890123456789012345678901234567890", &cosmos_format).unwrap());
    }
    
    #[tokio::test]
    async fn test_validation_manager() {
        let mut manager = ChainValidationManager::new();
        
        // Register Ethereum validator
        manager.register_chain(
            "ethereum".to_string(),
            PredefinedConfigs::ethereum(),
            Box::new(DefaultChainValidator::new()),
        );
        
        let event = create_test_event(
            "ethereum",
            "Transfer",
            "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            r#"{"from": "0x1234567890123456789012345678901234567890", "to": "0x0987654321098765432109876543210987654321", "value": "1000"}"#,
        );
        
        let result = manager.validate_event(&event).await.unwrap();
        assert!(result.is_valid);
        
        // Test chains listing
        let chains = manager.list_chains();
        assert!(chains.contains(&"ethereum".to_string()));
    }
    
    #[test]
    fn test_predefined_configs() {
        let eth_config = PredefinedConfigs::ethereum();
        assert_eq!(eth_config.chain_name, "ethereum");
        assert_eq!(eth_config.chain_id, 1);
        assert_eq!(eth_config.address_format, AddressFormat::Ethereum);
        
        let polygon_config = PredefinedConfigs::polygon();
        assert_eq!(polygon_config.chain_name, "polygon");
        assert_eq!(polygon_config.chain_id, 137);
        assert_eq!(polygon_config.block_constraints.expected_block_time, 2);
        
        let cosmos_config = PredefinedConfigs::cosmos();
        assert_eq!(cosmos_config.chain_name, "cosmos");
        assert_eq!(cosmos_config.address_format, AddressFormat::Cosmos { prefix: "cosmos".to_string() });
    }
} 