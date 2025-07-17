// OpenAPI documentation generation for the REST API
//
// This module provides comprehensive OpenAPI 3.0 documentation for all REST endpoints,
// including schemas, examples, and interactive documentation

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use axum::{
    response::{Html, Json},
    routing::get,
    Router,
};
use crate::http::HttpState;

/// OpenAPI specification for the Almanac REST API
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: OpenApiInfo,
    pub servers: Vec<OpenApiServer>,
    pub paths: HashMap<String, HashMap<String, OpenApiOperation>>,
    pub components: OpenApiComponents,
}

/// API information
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiInfo {
    pub title: String,
    pub version: String,
    pub description: String,
    pub contact: Option<OpenApiContact>,
    pub license: Option<OpenApiLicense>,
}

/// Contact information
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiContact {
    pub name: String,
    pub url: Option<String>,
    pub email: Option<String>,
}

/// License information
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiLicense {
    pub name: String,
    pub url: Option<String>,
}

/// Server configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiServer {
    pub url: String,
    pub description: String,
}

/// Operation definition
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiOperation {
    pub summary: String,
    pub description: String,
    pub tags: Vec<String>,
    pub parameters: Option<Vec<OpenApiParameter>>,
    #[serde(rename = "requestBody")]
    pub request_body: Option<OpenApiRequestBody>,
    pub responses: HashMap<String, OpenApiResponse>,
    pub security: Option<Vec<HashMap<String, Vec<String>>>>,
}

/// Parameter definition
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiParameter {
    pub name: String,
    #[serde(rename = "in")]
    pub param_in: String,
    pub description: String,
    pub required: bool,
    pub schema: OpenApiSchema,
    pub example: Option<Value>,
}

/// Request body definition
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiRequestBody {
    pub description: String,
    pub required: bool,
    pub content: HashMap<String, OpenApiMediaType>,
}

/// Response definition
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiResponse {
    pub description: String,
    pub content: Option<HashMap<String, OpenApiMediaType>>,
    pub headers: Option<HashMap<String, OpenApiHeader>>,
}

/// Media type definition
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiMediaType {
    pub schema: OpenApiSchema,
    pub example: Option<Value>,
    pub examples: Option<HashMap<String, OpenApiExample>>,
}

/// Header definition
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiHeader {
    pub description: String,
    pub schema: OpenApiSchema,
}

/// Example definition
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiExample {
    pub summary: String,
    pub description: String,
    pub value: Value,
}

/// Schema definition
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiSchema {
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    pub format: Option<String>,
    pub description: Option<String>,
    pub properties: Option<HashMap<String, OpenApiSchema>>,
    pub required: Option<Vec<String>>,
    pub items: Option<Box<OpenApiSchema>>,
    #[serde(rename = "enum")]
    pub enum_values: Option<Vec<Value>>,
    pub example: Option<Value>,
    #[serde(rename = "$ref")]
    pub reference: Option<String>,
}

/// Components section
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiComponents {
    pub schemas: HashMap<String, OpenApiSchema>,
    #[serde(rename = "securitySchemes")]
    pub security_schemes: HashMap<String, OpenApiSecurityScheme>,
}

/// Security scheme definition
#[derive(Debug, Serialize, Deserialize)]
pub struct OpenApiSecurityScheme {
    #[serde(rename = "type")]
    pub scheme_type: String,
    pub description: String,
    pub name: Option<String>,
    #[serde(rename = "in")]
    pub scheme_in: Option<String>,
    pub scheme: Option<String>,
    #[serde(rename = "bearerFormat")]
    pub bearer_format: Option<String>,
}

/// Generate complete OpenAPI specification
pub fn generate_openapi_spec() -> OpenApiSpec {
    OpenApiSpec {
        openapi: "3.0.3".to_string(),
        info: OpenApiInfo {
            title: "Almanac Indexer API".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "Comprehensive REST API for the Almanac cross-chain indexer. Provides access to blockchain events, blocks, and chain status across multiple networks.".to_string(),
            contact: Some(OpenApiContact {
                name: "Almanac Team".to_string(),
                url: Some("https://github.com/timewave-lab/almanac".to_string()),
                email: None,
            }),
            license: Some(OpenApiLicense {
                name: "MIT".to_string(),
                url: Some("https://opensource.org/licenses/MIT".to_string()),
            }),
        },
        servers: vec![
            OpenApiServer {
                url: "https://api.almanac.com".to_string(),
                description: "Production server".to_string(),
            },
            OpenApiServer {
                url: "https://api-staging.almanac.com".to_string(),
                description: "Staging server".to_string(),
            },
            OpenApiServer {
                url: "http://localhost:8080".to_string(),
                description: "Local development server".to_string(),
            },
        ],
        paths: generate_paths(),
        components: generate_components(),
    }
}

/// Generate API paths
fn generate_paths() -> HashMap<String, HashMap<String, OpenApiOperation>> {
    let mut paths = HashMap::new();
    
    // Health endpoint
    paths.insert("/api/v1/health".to_string(), {
        let mut methods = HashMap::new();
        methods.insert("get".to_string(), OpenApiOperation {
            summary: "Health check".to_string(),
            description: "Returns the current health status of the API and indexer".to_string(),
            tags: vec!["Health".to_string()],
            parameters: None,
            request_body: None,
            responses: {
                let mut responses = HashMap::new();
                responses.insert("200".to_string(), OpenApiResponse {
                    description: "Health status retrieved successfully".to_string(),
                    content: Some({
                        let mut content = HashMap::new();
                        content.insert("application/json".to_string(), OpenApiMediaType {
                            schema: OpenApiSchema {
                                reference: Some("#/components/schemas/HealthResponse".to_string()),
                                schema_type: None,
                                format: None,
                                description: None,
                                properties: None,
                                required: None,
                                items: None,
                                enum_values: None,
                                example: None,
                            },
                            example: Some(json!({
                                "healthy": true,
                                "version": "0.1.0",
                                "uptime_seconds": 3600,
                                "latest_blocks": {
                                    "ethereum": {
                                        "block_number": 18500000,
                                        "block_hash": "0x1234567890abcdef",
                                        "timestamp": 1700000000,
                                        "finality_status": "confirmed"
                                    }
                                }
                            })),
                            examples: None,
                        });
                        content
                    }),
                    headers: None,
                });
                responses
            },
            security: None,
        });
        methods
    });
    
    // Events endpoints
    paths.insert("/api/v1/events/chain/{chain_id}".to_string(), {
        let mut methods = HashMap::new();
        methods.insert("get".to_string(), OpenApiOperation {
            summary: "Get events by chain".to_string(),
            description: "Retrieve events from a specific blockchain with optional filtering".to_string(),
            tags: vec!["Events".to_string()],
            parameters: Some(vec![
                OpenApiParameter {
                    name: "chain_id".to_string(),
                    param_in: "path".to_string(),
                    description: "Chain identifier (e.g., 'ethereum', 'osmosis')".to_string(),
                    required: true,
                    schema: OpenApiSchema {
                        schema_type: Some("string".to_string()),
                        format: None,
                        description: None,
                        properties: None,
                        required: None,
                        items: None,
                        enum_values: None,
                        example: Some(json!("ethereum")),
                        reference: None,
                    },
                    example: Some(json!("ethereum")),
                },
                OpenApiParameter {
                    name: "limit".to_string(),
                    param_in: "query".to_string(),
                    description: "Maximum number of events to return (1-1000)".to_string(),
                    required: false,
                    schema: OpenApiSchema {
                        schema_type: Some("integer".to_string()),
                        format: Some("int32".to_string()),
                        description: None,
                        properties: None,
                        required: None,
                        items: None,
                        enum_values: None,
                        example: Some(json!(100)),
                        reference: None,
                    },
                    example: Some(json!(100)),
                },
                OpenApiParameter {
                    name: "offset".to_string(),
                    param_in: "query".to_string(),
                    description: "Number of events to skip".to_string(),
                    required: false,
                    schema: OpenApiSchema {
                        schema_type: Some("integer".to_string()),
                        format: Some("int32".to_string()),
                        description: None,
                        properties: None,
                        required: None,
                        items: None,
                        enum_values: None,
                        example: Some(json!(0)),
                        reference: None,
                    },
                    example: Some(json!(0)),
                },
                OpenApiParameter {
                    name: "event_type".to_string(),
                    param_in: "query".to_string(),
                    description: "Filter by event type".to_string(),
                    required: false,
                    schema: OpenApiSchema {
                        schema_type: Some("string".to_string()),
                        format: None,
                        description: None,
                        properties: None,
                        required: None,
                        items: None,
                        enum_values: None,
                        example: Some(json!("Transfer")),
                        reference: None,
                    },
                    example: Some(json!("Transfer")),
                },
                OpenApiParameter {
                    name: "from_height".to_string(),
                    param_in: "query".to_string(),
                    description: "Filter events from this block height".to_string(),
                    required: false,
                    schema: OpenApiSchema {
                        schema_type: Some("integer".to_string()),
                        format: Some("int64".to_string()),
                        description: None,
                        properties: None,
                        required: None,
                        items: None,
                        enum_values: None,
                        example: Some(json!(18000000)),
                        reference: None,
                    },
                    example: Some(json!(18000000)),
                },
                OpenApiParameter {
                    name: "to_height".to_string(),
                    param_in: "query".to_string(),
                    description: "Filter events to this block height".to_string(),
                    required: false,
                    schema: OpenApiSchema {
                        schema_type: Some("integer".to_string()),
                        format: Some("int64".to_string()),
                        description: None,
                        properties: None,
                        required: None,
                        items: None,
                        enum_values: None,
                        example: Some(json!(18500000)),
                        reference: None,
                    },
                    example: Some(json!(18500000)),
                },
                OpenApiParameter {
                    name: "text_query".to_string(),
                    param_in: "query".to_string(),
                    description: "Full-text search query".to_string(),
                    required: false,
                    schema: OpenApiSchema {
                        schema_type: Some("string".to_string()),
                        format: None,
                        description: None,
                        properties: None,
                        required: None,
                        items: None,
                        enum_values: None,
                        example: Some(json!("USDC transfer")),
                        reference: None,
                    },
                    example: Some(json!("USDC transfer")),
                },
                OpenApiParameter {
                    name: "text_search_mode".to_string(),
                    param_in: "query".to_string(),
                    description: "Text search mode".to_string(),
                    required: false,
                    schema: OpenApiSchema {
                        schema_type: Some("string".to_string()),
                        format: None,
                        description: None,
                        properties: None,
                        required: None,
                        items: None,
                        enum_values: Some(vec![
                            json!("contains"),
                            json!("fulltext"),
                            json!("fuzzy"),
                            json!("regex"),
                            json!("phrase"),
                            json!("boolean"),
                        ]),
                        example: Some(json!("contains")),
                        reference: None,
                    },
                    example: Some(json!("contains")),
                },
            ]),
            request_body: None,
            responses: {
                let mut responses = HashMap::new();
                responses.insert("200".to_string(), OpenApiResponse {
                    description: "Events retrieved successfully".to_string(),
                    content: Some({
                        let mut content = HashMap::new();
                        content.insert("application/json".to_string(), OpenApiMediaType {
                            schema: OpenApiSchema {
                                reference: Some("#/components/schemas/EventsResponse".to_string()),
                                schema_type: None,
                                format: None,
                                description: None,
                                properties: None,
                                required: None,
                                items: None,
                                enum_values: None,
                                example: None,
                            },
                            example: Some(json!({
                                "events": [
                                    {
                                        "chain_id": "ethereum",
                                        "height": 18500000,
                                        "tx_hash": "0x1234567890abcdef",
                                        "index": 0,
                                        "address": "0xA0b86a33E6C17c3c94C72e1c3d1f42C76D3C2C4D",
                                        "event_type": "Transfer",
                                        "attributes": {
                                            "from": "0x1234567890abcdef",
                                            "to": "0xfedcba0987654321",
                                            "amount": "1000000000000000000"
                                        },
                                        "raw_data": "SGVsbG8gV29ybGQ=",
                                        "timestamp": 1700000000
                                    }
                                ],
                                "pagination": {
                                    "total": 1,
                                    "limit": 100,
                                    "offset": 0,
                                    "has_more": false
                                }
                            })),
                            examples: None,
                        });
                        content
                    }),
                    headers: Some({
                        let mut headers = HashMap::new();
                        headers.insert("X-RateLimit-Limit".to_string(), OpenApiHeader {
                            description: "Rate limit quota".to_string(),
                            schema: OpenApiSchema {
                                schema_type: Some("integer".to_string()),
                                format: None,
                                description: None,
                                properties: None,
                                required: None,
                                items: None,
                                enum_values: None,
                                example: Some(json!(1000)),
                                reference: None,
                            },
                        });
                        headers.insert("X-RateLimit-Remaining".to_string(), OpenApiHeader {
                            description: "Remaining rate limit quota".to_string(),
                            schema: OpenApiSchema {
                                schema_type: Some("integer".to_string()),
                                format: None,
                                description: None,
                                properties: None,
                                required: None,
                                items: None,
                                enum_values: None,
                                example: Some(json!(999)),
                                reference: None,
                            },
                        });
                        headers
                    }),
                });
                responses.insert("400".to_string(), OpenApiResponse {
                    description: "Bad request".to_string(),
                    content: Some({
                        let mut content = HashMap::new();
                        content.insert("application/json".to_string(), OpenApiMediaType {
                            schema: OpenApiSchema {
                                reference: Some("#/components/schemas/Error".to_string()),
                                schema_type: None,
                                format: None,
                                description: None,
                                properties: None,
                                required: None,
                                items: None,
                                enum_values: None,
                                example: None,
                            },
                            example: Some(json!({
                                "error": "Invalid chain_id parameter",
                                "status": 400
                            })),
                            examples: None,
                        });
                        content
                    }),
                    headers: None,
                });
                responses.insert("429".to_string(), OpenApiResponse {
                    description: "Rate limit exceeded".to_string(),
                    content: Some({
                        let mut content = HashMap::new();
                        content.insert("application/json".to_string(), OpenApiMediaType {
                            schema: OpenApiSchema {
                                reference: Some("#/components/schemas/Error".to_string()),
                                schema_type: None,
                                format: None,
                                description: None,
                                properties: None,
                                required: None,
                                items: None,
                                enum_values: None,
                                example: None,
                            },
                            example: Some(json!({
                                "error": "Rate limit exceeded",
                                "status": 429
                            })),
                            examples: None,
                        });
                        content
                    }),
                    headers: None,
                });
                responses.insert("500".to_string(), OpenApiResponse {
                    description: "Internal server error".to_string(),
                    content: Some({
                        let mut content = HashMap::new();
                        content.insert("application/json".to_string(), OpenApiMediaType {
                            schema: OpenApiSchema {
                                reference: Some("#/components/schemas/Error".to_string()),
                                schema_type: None,
                                format: None,
                                description: None,
                                properties: None,
                                required: None,
                                items: None,
                                enum_values: None,
                                example: None,
                            },
                            example: Some(json!({
                                "error": "Internal server error",
                                "status": 500
                            })),
                            examples: None,
                        });
                        content
                    }),
                    headers: None,
                });
                responses
            },
            security: Some(vec![{
                let mut security = HashMap::new();
                security.insert("ApiKeyAuth".to_string(), vec![]);
                security
            }]),
        });
        methods
    });
    
    // Add more endpoints here...
    
    paths
}

/// Generate components section
fn generate_components() -> OpenApiComponents {
    let mut schemas = HashMap::new();
    
    // HealthResponse schema
    schemas.insert("HealthResponse".to_string(), OpenApiSchema {
        schema_type: Some("object".to_string()),
        format: None,
        description: Some("Health check response".to_string()),
        properties: Some({
            let mut properties = HashMap::new();
            properties.insert("healthy".to_string(), OpenApiSchema {
                schema_type: Some("boolean".to_string()),
                format: None,
                description: Some("Whether the API is healthy".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!(true)),
                reference: None,
            });
            properties.insert("version".to_string(), OpenApiSchema {
                schema_type: Some("string".to_string()),
                format: None,
                description: Some("API version".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!("0.1.0")),
                reference: None,
            });
            properties.insert("uptime_seconds".to_string(), OpenApiSchema {
                schema_type: Some("integer".to_string()),
                format: Some("int64".to_string()),
                description: Some("Uptime in seconds".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!(3600)),
                reference: None,
            });
            properties.insert("latest_blocks".to_string(), OpenApiSchema {
                schema_type: Some("object".to_string()),
                format: None,
                description: Some("Latest block information by chain".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: None,
                reference: None,
            });
            properties
        }),
        required: Some(vec!["healthy".to_string(), "version".to_string(), "uptime_seconds".to_string()]),
        items: None,
        enum_values: None,
        example: None,
        reference: None,
    });
    
    // EventsResponse schema
    schemas.insert("EventsResponse".to_string(), OpenApiSchema {
        schema_type: Some("object".to_string()),
        format: None,
        description: Some("Events response with pagination".to_string()),
        properties: Some({
            let mut properties = HashMap::new();
            properties.insert("events".to_string(), OpenApiSchema {
                schema_type: Some("array".to_string()),
                format: None,
                description: Some("Array of events".to_string()),
                properties: None,
                required: None,
                items: Some(Box::new(OpenApiSchema {
                    reference: Some("#/components/schemas/EventResponse".to_string()),
                    schema_type: None,
                    format: None,
                    description: None,
                    properties: None,
                    required: None,
                    items: None,
                    enum_values: None,
                    example: None,
                })),
                enum_values: None,
                example: None,
                reference: None,
            });
            properties.insert("pagination".to_string(), OpenApiSchema {
                reference: Some("#/components/schemas/PaginationInfo".to_string()),
                schema_type: None,
                format: None,
                description: None,
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: None,
            });
            properties
        }),
        required: Some(vec!["events".to_string(), "pagination".to_string()]),
        items: None,
        enum_values: None,
        example: None,
        reference: None,
    });
    
    // EventResponse schema
    schemas.insert("EventResponse".to_string(), OpenApiSchema {
        schema_type: Some("object".to_string()),
        format: None,
        description: Some("Individual event response".to_string()),
        properties: Some({
            let mut properties = HashMap::new();
            properties.insert("chain_id".to_string(), OpenApiSchema {
                schema_type: Some("string".to_string()),
                format: None,
                description: Some("Chain identifier".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!("ethereum")),
                reference: None,
            });
            properties.insert("height".to_string(), OpenApiSchema {
                schema_type: Some("integer".to_string()),
                format: Some("int64".to_string()),
                description: Some("Block height".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!(18500000)),
                reference: None,
            });
            properties.insert("tx_hash".to_string(), OpenApiSchema {
                schema_type: Some("string".to_string()),
                format: None,
                description: Some("Transaction hash".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!("0x1234567890abcdef")),
                reference: None,
            });
            properties.insert("event_type".to_string(), OpenApiSchema {
                schema_type: Some("string".to_string()),
                format: None,
                description: Some("Event type".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!("Transfer")),
                reference: None,
            });
            properties.insert("attributes".to_string(), OpenApiSchema {
                schema_type: Some("object".to_string()),
                format: None,
                description: Some("Event attributes".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!({
                    "from": "0x1234567890abcdef",
                    "to": "0xfedcba0987654321",
                    "amount": "1000000000000000000"
                })),
                reference: None,
            });
            properties.insert("raw_data".to_string(), OpenApiSchema {
                schema_type: Some("string".to_string()),
                format: Some("base64".to_string()),
                description: Some("Base64 encoded raw event data".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!("SGVsbG8gV29ybGQ=")),
                reference: None,
            });
            properties.insert("timestamp".to_string(), OpenApiSchema {
                schema_type: Some("integer".to_string()),
                format: Some("int64".to_string()),
                description: Some("Unix timestamp".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!(1700000000)),
                reference: None,
            });
            properties
        }),
        required: Some(vec![
            "chain_id".to_string(),
            "height".to_string(),
            "tx_hash".to_string(),
            "event_type".to_string(),
            "attributes".to_string(),
            "raw_data".to_string(),
            "timestamp".to_string(),
        ]),
        items: None,
        enum_values: None,
        example: None,
        reference: None,
    });
    
    // PaginationInfo schema
    schemas.insert("PaginationInfo".to_string(), OpenApiSchema {
        schema_type: Some("object".to_string()),
        format: None,
        description: Some("Pagination information".to_string()),
        properties: Some({
            let mut properties = HashMap::new();
            properties.insert("total".to_string(), OpenApiSchema {
                schema_type: Some("integer".to_string()),
                format: Some("int64".to_string()),
                description: Some("Total number of items".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!(1000)),
                reference: None,
            });
            properties.insert("limit".to_string(), OpenApiSchema {
                schema_type: Some("integer".to_string()),
                format: Some("int32".to_string()),
                description: Some("Maximum number of items returned".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!(100)),
                reference: None,
            });
            properties.insert("offset".to_string(), OpenApiSchema {
                schema_type: Some("integer".to_string()),
                format: Some("int32".to_string()),
                description: Some("Number of items skipped".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!(0)),
                reference: None,
            });
            properties.insert("has_more".to_string(), OpenApiSchema {
                schema_type: Some("boolean".to_string()),
                format: None,
                description: Some("Whether there are more items available".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!(false)),
                reference: None,
            });
            properties
        }),
        required: Some(vec!["limit".to_string(), "offset".to_string(), "has_more".to_string()]),
        items: None,
        enum_values: None,
        example: None,
        reference: None,
    });
    
    // Error schema
    schemas.insert("Error".to_string(), OpenApiSchema {
        schema_type: Some("object".to_string()),
        format: None,
        description: Some("Error response".to_string()),
        properties: Some({
            let mut properties = HashMap::new();
            properties.insert("error".to_string(), OpenApiSchema {
                schema_type: Some("string".to_string()),
                format: None,
                description: Some("Error message".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!("Invalid parameter")),
                reference: None,
            });
            properties.insert("status".to_string(), OpenApiSchema {
                schema_type: Some("integer".to_string()),
                format: Some("int32".to_string()),
                description: Some("HTTP status code".to_string()),
                properties: None,
                required: None,
                items: None,
                enum_values: None,
                example: Some(json!(400)),
                reference: None,
            });
            properties
        }),
        required: Some(vec!["error".to_string(), "status".to_string()]),
        items: None,
        enum_values: None,
        example: None,
        reference: None,
    });
    
    // Security schemes
    let mut security_schemes = HashMap::new();
    security_schemes.insert("ApiKeyAuth".to_string(), OpenApiSecurityScheme {
        scheme_type: "apiKey".to_string(),
        description: "API key authentication".to_string(),
        name: Some("X-API-Key".to_string()),
        scheme_in: Some("header".to_string()),
        scheme: None,
        bearer_format: None,
    });
    security_schemes.insert("BearerAuth".to_string(), OpenApiSecurityScheme {
        scheme_type: "http".to_string(),
        description: "Bearer token authentication".to_string(),
        name: None,
        scheme_in: None,
        scheme: Some("bearer".to_string()),
        bearer_format: Some("JWT".to_string()),
    });
    
    OpenApiComponents {
        schemas,
        security_schemes,
    }
}

/// Serve OpenAPI specification as JSON
pub async fn serve_openapi_spec() -> Json<OpenApiSpec> {
    Json(generate_openapi_spec())
}

/// Serve Swagger UI
pub async fn serve_swagger_ui() -> Html<String> {
    Html(format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Almanac API Documentation</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui.css" />
    <style>
        html {{
            box-sizing: border-box;
            overflow: -moz-scrollbars-vertical;
            overflow-y: scroll;
        }}
        *, *:before, *:after {{
            box-sizing: inherit;
        }}
        body {{
            margin:0;
            background: #fafafa;
        }}
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@4.15.5/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {{
            const ui = SwaggerUIBundle({{
                url: '/api/v1/openapi.json',
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "StandaloneLayout"
            }});
        }};
    </script>
</body>
</html>
    "#))
}

/// Add OpenAPI routes to the router
pub fn add_openapi_routes(router: Router<HttpState>) -> Router<HttpState> {
    router
        .route("/api/v1/openapi.json", get(serve_openapi_spec))
        .route("/api/v1/docs", get(serve_swagger_ui))
} 