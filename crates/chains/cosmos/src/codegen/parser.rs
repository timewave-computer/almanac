//! CosmWasm message schema parser
//! 
//! Parses CosmWasm *_msg.json files to extract contract interface definitions.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use indexer_core::Result;

/// Parsed CosmWasm contract schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmWasmSchema {
    /// Contract instantiate message schema
    pub instantiate_msg: Option<MessageSchema>,
    /// Contract execute message schema
    pub execute_msg: Option<MessageSchema>,
    /// Contract query message schema  
    pub query_msg: Option<MessageSchema>,
    /// Contract migrate message schema
    pub migrate_msg: Option<MessageSchema>,
    /// Contract event definitions
    pub events: Vec<EventSchema>,
    /// Custom type definitions
    pub definitions: HashMap<String, TypeDefinition>,
}

/// Message schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSchema {
    /// Schema title
    pub title: Option<String>,
    /// Schema description
    pub description: Option<String>,
    /// Message variants (for enums) or properties (for structs)
    pub variants: Vec<MessageVariant>,
    /// Whether this is an enum or struct
    pub is_enum: bool,
}

/// Message variant definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageVariant {
    /// Variant name
    pub name: String,
    /// Variant description
    pub description: Option<String>,
    /// Variant properties/fields
    pub properties: Vec<PropertySchema>,
    /// Required fields
    pub required: Vec<String>,
}

/// Property schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    /// Property name
    pub name: String,
    /// Property type
    pub type_info: TypeInfo,
    /// Property description
    pub description: Option<String>,
    /// Whether property is required
    pub required: bool,
}

/// Type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    /// Base type (string, number, boolean, object, array)
    pub base_type: String,
    /// Reference to a defined type
    pub reference: Option<String>,
    /// Array item type (for arrays)
    pub items: Option<Box<TypeInfo>>,
    /// Enum values (for string enums)
    pub enum_values: Option<Vec<String>>,
}

/// Custom type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinition {
    /// Type name
    pub name: String,
    /// Type description
    pub description: Option<String>,
    /// Type properties (for objects)
    pub properties: Vec<PropertySchema>,
    /// Whether this is an enum
    pub is_enum: bool,
    /// Enum variants (if is_enum is true)
    pub variants: Vec<String>,
}

/// Event schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSchema {
    /// Event name
    pub name: String,
    /// Event description
    pub description: Option<String>,
    /// Event attributes
    pub attributes: Vec<PropertySchema>,
}

/// CosmWasm message parser
pub struct CosmWasmMsgParser;

impl CosmWasmMsgParser {
    /// Create a new parser instance
    pub fn new() -> Self {
        Self
    }

    /// Parse a CosmWasm message schema file
    pub fn parse_file(&self, file_path: &str) -> Result<CosmWasmSchema> {
        let content = std::fs::read_to_string(file_path)?;
        self.parse_content(&content)
    }

    /// Parse CosmWasm message schema from JSON content
    pub fn parse_content(&self, content: &str) -> Result<CosmWasmSchema> {
        let value: Value = serde_json::from_str(content)?;
        self.parse_schema(&value)
    }

    /// Parse schema from JSON value
    fn parse_schema(&self, value: &Value) -> Result<CosmWasmSchema> {
        let mut schema = CosmWasmSchema {
            instantiate_msg: None,
            execute_msg: None,
            query_msg: None,
            migrate_msg: None,
            events: Vec::new(),
            definitions: HashMap::new(),
        };

        // Parse definitions first
        if let Some(definitions) = value.get("definitions").and_then(|v| v.as_object()) {
            for (name, def) in definitions {
                if let Ok(type_def) = self.parse_type_definition(name, def) {
                    schema.definitions.insert(name.clone(), type_def);
                }
            }
        }

        // Check if this is a single message schema or a combined schema file
        if let Some(schema_type) = value.get("title").and_then(|v| v.as_str()) {
            // Single message schema file
            match schema_type {
                "InstantiateMsg" => {
                    schema.instantiate_msg = Some(self.parse_message_schema(value)?);
                }
                "ExecuteMsg" => {
                    schema.execute_msg = Some(self.parse_message_schema(value)?);
                }
                "QueryMsg" => {
                    schema.query_msg = Some(self.parse_message_schema(value)?);
                }
                "MigrateMsg" => {
                    schema.migrate_msg = Some(self.parse_message_schema(value)?);
                }
                _ => {
                    // Unknown schema type, try to parse as generic message
                    if let Ok(msg_schema) = self.parse_message_schema(value) {
                        // Try to infer the type based on content
                        if schema_type.to_lowercase().contains("instantiate") {
                            schema.instantiate_msg = Some(msg_schema);
                        } else if schema_type.to_lowercase().contains("execute") {
                            schema.execute_msg = Some(msg_schema);
                        } else if schema_type.to_lowercase().contains("query") {
                            schema.query_msg = Some(msg_schema);
                        } else if schema_type.to_lowercase().contains("migrate") {
                            schema.migrate_msg = Some(msg_schema);
                        }
                    }
                }
            }
        } else {
            // Combined schema file - look for specific message schemas
            if let Some(inst_msg) = value.get("instantiate").or_else(|| value.get("InstantiateMsg")) {
                schema.instantiate_msg = Some(self.parse_message_schema(inst_msg)?);
            }

            if let Some(exec_msg) = value.get("execute").or_else(|| value.get("ExecuteMsg")) {
                schema.execute_msg = Some(self.parse_message_schema(exec_msg)?);
            }

            if let Some(query_msg) = value.get("query").or_else(|| value.get("QueryMsg")) {
                schema.query_msg = Some(self.parse_message_schema(query_msg)?);
            }

            if let Some(migrate_msg) = value.get("migrate").or_else(|| value.get("MigrateMsg")) {
                schema.migrate_msg = Some(self.parse_message_schema(migrate_msg)?);
            }
        }

        // Parse events if present
        if let Some(events) = value.get("events").and_then(|v| v.as_array()) {
            for event_value in events {
                if let Ok(event_schema) = self.parse_event_schema(event_value) {
                    schema.events.push(event_schema);
                }
            }
        }

        Ok(schema)
    }

    /// Parse a message schema
    fn parse_message_schema(&self, value: &Value) -> Result<MessageSchema> {
        let title = value.get("title").and_then(|v| v.as_str()).map(String::from);
        let description = value.get("description").and_then(|v| v.as_str()).map(String::from);

        let mut variants = Vec::new();
        let mut is_enum = false;

        // Check if this is an enum (oneOf) or struct (properties)
        if let Some(one_of) = value.get("oneOf").and_then(|v| v.as_array()) {
            is_enum = true;
            for variant_value in one_of {
                variants.push(self.parse_message_variant(variant_value)?);
            }
        } else if let Some(properties) = value.get("properties").and_then(|v| v.as_object()) {
            // This is a struct-like message
            let required: Vec<&str> = value.get("required")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            let mut props = Vec::new();
            for (prop_name, prop_value) in properties {
                props.push(PropertySchema {
                    name: prop_name.clone(),
                    type_info: self.parse_type_info(prop_value)?,
                    description: prop_value.get("description").and_then(|v| v.as_str()).map(String::from),
                    required: required.contains(&prop_name.as_str()),
                });
            }

            variants.push(MessageVariant {
                name: title.clone().unwrap_or_else(|| "Message".to_string()),
                description: description.clone(),
                properties: props,
                required: required.iter().map(|s| s.to_string()).collect(),
            });
        }

        Ok(MessageSchema {
            title,
            description,
            variants,
            is_enum,
        })
    }

    /// Parse a message variant
    fn parse_message_variant(&self, value: &Value) -> Result<MessageVariant> {
        let title = value.get("title").and_then(|v| v.as_str()).map(String::from);
        let description = value.get("description").and_then(|v| v.as_str()).map(String::from);
        
        let name = title.unwrap_or_else(|| "Variant".to_string());
        let mut properties = Vec::new();

        if let Some(props) = value.get("properties").and_then(|v| v.as_object()) {
            let required: Vec<&str> = value.get("required")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            for (prop_name, prop_value) in props {
                properties.push(PropertySchema {
                    name: prop_name.clone(),
                    type_info: self.parse_type_info(prop_value)?,
                    description: prop_value.get("description").and_then(|v| v.as_str()).map(String::from),
                    required: required.contains(&prop_name.as_str()),
                });
            }
        }

        Ok(MessageVariant {
            name,
            description,
            properties,
            required: value.get("required")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<String>>())
                .unwrap_or_default(),
        })
    }

    /// Parse type information
    fn parse_type_info(&self, value: &Value) -> Result<TypeInfo> {
        Self::parse_type_info_static(value)
    }

    /// Parse type information (static implementation)
    fn parse_type_info_static(value: &Value) -> Result<TypeInfo> {
        let base_type = value.get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("object")
            .to_string();

        let reference = value.get("$ref")
            .and_then(|v| v.as_str())
            .map(|s| s.trim_start_matches("#/definitions/").to_string());

        let items = if base_type == "array" {
            value.get("items")
                .map(Self::parse_type_info_static)
                .transpose()?
                .map(Box::new)
        } else {
            None
        };

        let enum_values = value.get("enum")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

        Ok(TypeInfo {
            base_type,
            reference,
            items,
            enum_values,
        })
    }

    /// Parse a type definition
    fn parse_type_definition(&self, name: &str, value: &Value) -> Result<TypeDefinition> {
        let description = value.get("description").and_then(|v| v.as_str()).map(String::from);
        let mut properties = Vec::new();
        let mut is_enum = false;
        let mut variants = Vec::new();

        if let Some(props) = value.get("properties").and_then(|v| v.as_object()) {
            let required: Vec<&str> = value.get("required")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            for (prop_name, prop_value) in props {
                properties.push(PropertySchema {
                    name: prop_name.clone(),
                    type_info: Self::parse_type_info_static(prop_value)?,
                    description: prop_value.get("description").and_then(|v| v.as_str()).map(String::from),
                    required: required.contains(&prop_name.as_str()),
                });
            }
        } else if let Some(enum_vals) = value.get("enum").and_then(|v| v.as_array()) {
            is_enum = true;
            variants = enum_vals.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        }

        Ok(TypeDefinition {
            name: name.to_string(),
            description,
            properties,
            is_enum,
            variants,
        })
    }

    /// Parse an event schema
    fn parse_event_schema(&self, value: &Value) -> Result<EventSchema> {
        let name = value.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| indexer_core::Error::Config("Event must have a name".to_string()))?
            .to_string();

        let description = value.get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        let mut attributes = Vec::new();

        if let Some(attrs) = value.get("attributes").and_then(|v| v.as_array()) {
            for attr_value in attrs {
                if let Some(attr_name) = attr_value.get("name").and_then(|v| v.as_str()) {
                    attributes.push(PropertySchema {
                        name: attr_name.to_string(),
                        type_info: Self::parse_type_info_static(attr_value)?,
                        description: attr_value.get("description").and_then(|v| v.as_str()).map(String::from),
                        required: attr_value.get("required").and_then(|v| v.as_bool()).unwrap_or(false),
                    });
                }
            }
        } else if let Some(props) = value.get("properties").and_then(|v| v.as_object()) {
            // Alternative format where event attributes are defined as properties
            let required: Vec<&str> = value.get("required")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            for (prop_name, prop_value) in props {
                attributes.push(PropertySchema {
                    name: prop_name.clone(),
                    type_info: Self::parse_type_info_static(prop_value)?,
                    description: prop_value.get("description").and_then(|v| v.as_str()).map(String::from),
                    required: required.contains(&prop_name.as_str()),
                });
            }
        }

        Ok(EventSchema {
            name,
            description,
            attributes,
        })
    }

    /// Generate comprehensive type definitions from schema
    pub fn generate_type_definitions(&self, schema: &CosmWasmSchema) -> Vec<String> {
        let mut types = Vec::new();

        // Generate types from message schemas
        if let Some(msg) = &schema.instantiate_msg {
            types.extend(Self::extract_types_from_message("InstantiateMsg", msg));
        }

        if let Some(msg) = &schema.execute_msg {
            types.extend(Self::extract_types_from_message("ExecuteMsg", msg));
        }

        if let Some(msg) = &schema.query_msg {
            types.extend(Self::extract_types_from_message("QueryMsg", msg));
        }

        if let Some(msg) = &schema.migrate_msg {
            types.extend(Self::extract_types_from_message("MigrateMsg", msg));
        }

        // Add types from definitions
        for name in schema.definitions.keys() {
            types.push(name.clone());
        }

        // Remove duplicates
        types.sort();
        types.dedup();
        types
    }

    /// Extract type names from a message schema
    fn extract_types_from_message(msg_type: &str, msg: &MessageSchema) -> Vec<String> {
        let mut types = vec![msg_type.to_string()];

        for variant in &msg.variants {
            // Add variant name as a potential type
            if msg.is_enum {
                types.push(format!("{}::{}", msg_type, variant.name));
            }

            for property in &variant.properties {
                types.extend(Self::extract_types_from_type_info(&property.type_info));
            }
        }

        types
    }

    /// Recursively extract type names from TypeInfo
    fn extract_types_from_type_info(type_info: &TypeInfo) -> Vec<String> {
        let mut types = Vec::new();

        // Add reference types
        if let Some(ref reference) = type_info.reference {
            types.push(reference.clone());
        }

        // Handle array item types
        if let Some(ref items) = type_info.items {
            types.extend(Self::extract_types_from_type_info(items));
        }

        types
    }

    /// Check if the schema contains nested types and enums
    pub fn has_nested_types(&self, schema: &CosmWasmSchema) -> bool {
        // Check if any message has complex nested structures
        if let Some(msg) = &schema.execute_msg {
            if Self::message_has_nested_types(msg) {
                return true;
            }
        }

        if let Some(msg) = &schema.query_msg {
            if Self::message_has_nested_types(msg) {
                return true;
            }
        }

        if let Some(msg) = &schema.instantiate_msg {
            if Self::message_has_nested_types(msg) {
                return true;
            }
        }

        if let Some(msg) = &schema.migrate_msg {
            if Self::message_has_nested_types(msg) {
                return true;
            }
        }

        // Check definitions for complex types
        for def in schema.definitions.values() {
            if def.is_enum || !def.properties.is_empty() {
                return true;
            }
        }

        false
    }

    /// Check if a message has nested types
    fn message_has_nested_types(msg: &MessageSchema) -> bool {
        for variant in &msg.variants {
            for property in &variant.properties {
                if Self::type_info_is_complex(&property.type_info) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a TypeInfo represents a complex/nested type
    fn type_info_is_complex(type_info: &TypeInfo) -> bool {
        // Arrays are complex
        if type_info.items.is_some() {
            return true;
        }

        // References to custom types are complex
        if type_info.reference.is_some() {
            return true;
        }

        // Enums are complex
        if type_info.enum_values.is_some() {
            return true;
        }

        // Object types (not basic primitives) are complex
        if type_info.base_type == "object" {
            return true;
        }

        false
    }
}

impl Default for CosmWasmMsgParser {
    fn default() -> Self {
        Self::new()
    }
} 