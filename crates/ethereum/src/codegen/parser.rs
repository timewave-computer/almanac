//! Ethereum ABI parser
//! 
//! Parses Ethereum contract ABI JSON files to extract contract interface definitions.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use indexer_core::Result;

/// Parsed Ethereum contract ABI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthereumAbi {
    /// Contract constructor
    pub constructor: Option<AbiFunction>,
    /// Contract functions (view/pure and state-changing)
    pub functions: Vec<AbiFunction>,
    /// Contract events
    pub events: Vec<AbiEvent>,
    /// Contract errors (if present)
    pub errors: Vec<AbiError>,
    /// Raw ABI for reference
    pub raw_abi: Value,
}

/// ABI function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbiFunction {
    /// Function name
    pub name: String,
    /// Function type (function, constructor, fallback, receive)
    pub function_type: String,
    /// Function inputs
    pub inputs: Vec<AbiParameter>,
    /// Function outputs
    pub outputs: Vec<AbiParameter>,
    /// State mutability (pure, view, nonpayable, payable)
    pub state_mutability: String,
    /// Whether function is payable
    pub payable: bool,
    /// Whether function is constant (view/pure)
    pub constant: bool,
    /// Function signature (4-byte selector)
    pub signature: Option<String>,
}

/// ABI event definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbiEvent {
    /// Event name
    pub name: String,
    /// Event inputs
    pub inputs: Vec<AbiParameter>,
    /// Whether event is anonymous
    pub anonymous: bool,
    /// Event signature hash
    pub signature: Option<String>,
}

/// ABI error definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbiError {
    /// Error name
    pub name: String,
    /// Error inputs
    pub inputs: Vec<AbiParameter>,
}

/// ABI parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbiParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type (e.g., uint256, address, string)
    pub param_type: String,
    /// Internal type (for structs and custom types)
    pub internal_type: Option<String>,
    /// Components (for tuples and structs)
    pub components: Option<Vec<AbiParameter>>,
    /// Whether parameter is indexed (for events)
    pub indexed: bool,
}

/// Ethereum ABI parser
pub struct AbiParser;

impl AbiParser {
    /// Create a new parser instance
    pub fn new() -> Self {
        Self
    }

    /// Parse an Ethereum ABI file
    pub fn parse_file(&self, file_path: &str) -> Result<EthereumAbi> {
        let content = std::fs::read_to_string(file_path)?;
        self.parse_content(&content)
    }

    /// Parse Ethereum ABI from JSON content
    pub fn parse_content(&self, content: &str) -> Result<EthereumAbi> {
        let value: Value = serde_json::from_str(content)?;
        self.parse_abi(&value)
    }

    /// Parse ABI from JSON value
    fn parse_abi(&self, value: &Value) -> Result<EthereumAbi> {
        let abi_array = value.as_array()
            .ok_or_else(|| indexer_core::Error::Config("ABI must be an array".to_string()))?;

        let mut constructor = None;
        let mut functions = Vec::new();
        let mut events = Vec::new();
        let mut errors = Vec::new();

        for item in abi_array {
            let item_type = item.get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("function");

            match item_type {
                "constructor" => {
                    constructor = Some(self.parse_function(item, "constructor")?);
                }
                "function" => {
                    functions.push(self.parse_function(item, "function")?);
                }
                "event" => {
                    events.push(self.parse_event(item)?);
                }
                "error" => {
                    errors.push(self.parse_error(item)?);
                }
                "fallback" | "receive" => {
                    functions.push(self.parse_function(item, item_type)?);
                }
                _ => {
                    // Unknown type, skip
                }
            }
        }

        Ok(EthereumAbi {
            constructor,
            functions,
            events,
            errors,
            raw_abi: value.clone(),
        })
    }

    /// Parse a function from ABI
    fn parse_function(&self, value: &Value, function_type: &str) -> Result<AbiFunction> {
        let name = value.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(function_type)
            .to_string();

        let inputs = value.get("inputs")
            .and_then(|v| v.as_array())
            .map(|arr| self.parse_parameters(arr))
            .transpose()?
            .unwrap_or_default();

        let outputs = value.get("outputs")
            .and_then(|v| v.as_array())
            .map(|arr| self.parse_parameters(arr))
            .transpose()?
            .unwrap_or_default();

        let state_mutability = value.get("stateMutability")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| {
                // Legacy support
                if value.get("constant").and_then(|v| v.as_bool()).unwrap_or(false) {
                    "view"
                } else if value.get("payable").and_then(|v| v.as_bool()).unwrap_or(false) {
                    "payable"
                } else {
                    "nonpayable"
                }
            })
            .to_string();

        let payable = state_mutability == "payable";
        let constant = state_mutability == "view" || state_mutability == "pure";

        // Generate function signature
        let signature = if function_type == "function" {
            Some(self.generate_function_signature(&name, &inputs))
        } else {
            None
        };

        Ok(AbiFunction {
            name,
            function_type: function_type.to_string(),
            inputs,
            outputs,
            state_mutability,
            payable,
            constant,
            signature,
        })
    }

    /// Parse an event from ABI
    fn parse_event(&self, value: &Value) -> Result<AbiEvent> {
        let name = value.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| indexer_core::Error::Config("Event must have a name".to_string()))?
            .to_string();

        let inputs = value.get("inputs")
            .and_then(|v| v.as_array())
            .map(|arr| self.parse_event_parameters(arr))
            .transpose()?
            .unwrap_or_default();

        let anonymous = value.get("anonymous")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let signature = Some(self.generate_event_signature(&name, &inputs));

        Ok(AbiEvent {
            name,
            inputs,
            anonymous,
            signature,
        })
    }

    /// Parse an error from ABI
    fn parse_error(&self, value: &Value) -> Result<AbiError> {
        let name = value.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| indexer_core::Error::Config("Error must have a name".to_string()))?
            .to_string();

        let inputs = value.get("inputs")
            .and_then(|v| v.as_array())
            .map(|arr| self.parse_parameters(arr))
            .transpose()?
            .unwrap_or_default();

        Ok(AbiError { name, inputs })
    }

    /// Parse function/error parameters
    fn parse_parameters(&self, array: &[Value]) -> Result<Vec<AbiParameter>> {
        Self::parse_parameters_static(array)
    }

    /// Parse function/error parameters (static implementation)
    fn parse_parameters_static(array: &[Value]) -> Result<Vec<AbiParameter>> {
        let mut parameters = Vec::new();

        for param in array {
            let name = param.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let param_type = param.get("type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| indexer_core::Error::Config("Parameter must have a type".to_string()))?
                .to_string();

            let internal_type = param.get("internalType")
                .and_then(|v| v.as_str())
                .map(String::from);

            let components = if param_type.starts_with("tuple") {
                param.get("components")
                    .and_then(|v| v.as_array())
                    .map(|arr| Self::parse_parameters_static(arr))
                    .transpose()?
            } else {
                None
            };

            parameters.push(AbiParameter {
                name,
                param_type,
                internal_type,
                components,
                indexed: false, // Will be set in parse_event_parameters
            });
        }

        Ok(parameters)
    }

    /// Parse event parameters (with indexed flag)
    fn parse_event_parameters(&self, array: &[Value]) -> Result<Vec<AbiParameter>> {
        let mut parameters = Vec::new();

        for param in array {
            let name = param.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let param_type = param.get("type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| indexer_core::Error::Config("Parameter must have a type".to_string()))?
                .to_string();

            let internal_type = param.get("internalType")
                .and_then(|v| v.as_str())
                .map(String::from);

            let indexed = param.get("indexed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let components = if param_type.starts_with("tuple") {
                param.get("components")
                    .and_then(|v| v.as_array())
                    .map(|arr| Self::parse_parameters_static(arr))
                    .transpose()?
            } else {
                None
            };

            parameters.push(AbiParameter {
                name,
                param_type,
                internal_type,
                components,
                indexed,
            });
        }

        Ok(parameters)
    }

    /// Generate function signature (4-byte selector)
    fn generate_function_signature(&self, name: &str, inputs: &[AbiParameter]) -> String {
        let types: Vec<String> = inputs.iter()
            .map(|param| Self::canonical_type(&param.param_type))
            .collect();
        
        let signature_string = format!("{}({})", name, types.join(","));
        
        // Use keccak256 to generate the selector
        use sha3::{Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(signature_string.as_bytes());
        let hash = hasher.finalize();
        format!("0x{}", hex::encode(&hash[..4]))
    }

    /// Generate event signature hash
    fn generate_event_signature(&self, name: &str, inputs: &[AbiParameter]) -> String {
        let types: Vec<String> = inputs.iter()
            .map(|param| Self::canonical_type(&param.param_type))
            .collect();
        
        let signature_string = format!("{}({})", name, types.join(","));
        
        // Use keccak256 to generate the event topic
        use sha3::{Digest, Keccak256};
        let mut hasher = Keccak256::new();
        hasher.update(signature_string.as_bytes());
        let hash = hasher.finalize();
        format!("0x{}", hex::encode(hash))
    }

    /// Convert parameter type to canonical form
    fn canonical_type(param_type: &str) -> String {
        // Handle array types
        if let Some(base_type) = param_type.strip_suffix("[]") {
            return format!("{}[]", Self::canonical_type(base_type));
        }

        // Handle fixed-size arrays
        if let Some(bracket_pos) = param_type.rfind('[') {
            let base_type = &param_type[..bracket_pos];
            let array_part = &param_type[bracket_pos..];
            return format!("{}{}", Self::canonical_type(base_type), array_part);
        }

        // Handle tuple types
        if param_type.starts_with("tuple") {
            return param_type.to_string(); // Tuples need special handling in real implementation
        }

        // Basic types - return as-is (uint256, address, bool, etc.)
        param_type.to_string()
    }

    /// Get human-readable function signature
    pub fn get_function_signature(&self, function: &AbiFunction) -> String {
        let input_types: Vec<String> = function.inputs.iter()
            .map(|param| {
                if param.name.is_empty() {
                    param.param_type.clone()
                } else {
                    format!("{} {}", param.param_type, param.name)
                }
            })
            .collect();

        let output_types: Vec<String> = function.outputs.iter()
            .map(|param| param.param_type.clone())
            .collect();

        let outputs_part = if output_types.is_empty() {
            String::new()
        } else {
            format!(" returns ({})", output_types.join(", "))
        };

        format!("function {}({}){}", function.name, input_types.join(", "), outputs_part)
    }
}

impl Default for AbiParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for working with parsed ABIs
impl EthereumAbi {
    /// Get view functions (read-only)
    pub fn get_view_functions(&self) -> Vec<&AbiFunction> {
        self.functions.iter()
            .filter(|f| f.state_mutability == "view" || f.state_mutability == "pure")
            .collect()
    }

    /// Get transaction functions (state-changing)
    pub fn get_transaction_functions(&self) -> Vec<&AbiFunction> {
        self.functions.iter()
            .filter(|f| f.state_mutability == "nonpayable" || f.state_mutability == "payable")
            .collect()
    }

    /// Get payable functions
    pub fn get_payable_functions(&self) -> Vec<&AbiFunction> {
        self.functions.iter()
            .filter(|f| f.state_mutability == "payable")
            .collect()
    }

    /// Extract all parameter types from ABI functions
    pub fn extract_custom_types(&self) -> Vec<String> {
        let mut types = std::collections::HashSet::new();

        // Collect from function inputs and outputs
        for function in &self.functions {
            for param in &function.inputs {
                Self::collect_types_from_parameter(param, &mut types);
            }
            for param in &function.outputs {
                Self::collect_types_from_parameter(param, &mut types);
            }
        }

        // Collect from constructor
        if let Some(constructor) = &self.constructor {
            for param in &constructor.inputs {
                Self::collect_types_from_parameter(param, &mut types);
            }
        }

        // Collect from events
        for event in &self.events {
            for param in &event.inputs {
                Self::collect_types_from_parameter(param, &mut types);
            }
        }

        types.into_iter().collect()
    }

    /// Helper method to collect types from a parameter recursively
    fn collect_types_from_parameter(param: &AbiParameter, types: &mut std::collections::HashSet<String>) {
        // Add the parameter type
        types.insert(param.param_type.clone());

        // Add internal type if it's different and represents a custom type
        if let Some(ref internal_type) = param.internal_type {
            if internal_type != &param.param_type && internal_type.contains("struct") {
                types.insert(internal_type.clone());
            }
        }

        // Recursively process components for tuples/structs
        if let Some(ref components) = param.components {
            for component in components {
                Self::collect_types_from_parameter(component, types);
            }
        }
    }
} 