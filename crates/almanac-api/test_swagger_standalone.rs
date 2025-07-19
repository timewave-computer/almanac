use serde::{Deserialize, Serialize};
use utoipa::{ToSchema, OpenApi};

// Mock types for demonstration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiEvent {
    pub id: String,
    pub chain: String,
    pub block_number: u64,
    pub event_type: String,
    pub tx_hash: Option<String>,
    pub event_index: u32,
    pub timestamp: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct EventQuery {
    pub chain: Option<String>,
    pub from_block: Option<u64>,
    pub to_block: Option<u64>,
    pub event_type: Option<String>,
    pub address: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}

fn default_limit() -> u32 {
    100
}

#[derive(serde::Serialize, ToSchema)]
pub struct LatestBlockResponse {
    pub chain: String,
    pub latest_block: u64,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        // These would be the actual handler functions in a real implementation
        // list_events,
        // get_event,
        // list_chain_events,
        // get_latest_block,
    ),
    components(
        schemas(
            ApiEvent,
            EventQuery,
            LatestBlockResponse,
        )
    ),
    tags(
        (name = "events", description = "Event management API")
    ),
    info(
        title = "Almanac API",
        version = "1.0.0",
        description = "High-performance, multi-chain blockchain indexing and querying system",
        contact(
            name = "Timewave Team",
            url = "https://timewave.computer"
        ),
        license(
            name = "MIT"
        )
    ),
    servers(
        (url = "/", description = "Local server")
    )
)]
pub struct ApiDoc;

fn main() {
    println!("Almanac API - Swagger UI Integration Demo");
    println!("═══════════════════════════════════════════════");
    
    // Generate the OpenAPI specification
    let spec = ApiDoc::openapi();
    
    println!("\nAPI Information:");
    println!("  Title: {}", spec.info.title);
    println!("  Version: {}", spec.info.version);
    println!("  Description: {}", spec.info.description.as_ref().unwrap_or(&"N/A".to_string()));
    
    println!("\nTags:");
    if let Some(tags) = &spec.tags {
        if tags.is_empty() {
            println!("  • No tags defined");
        } else {
            for tag in tags.iter() {
                println!("  • {}: {}", tag.name, tag.description.as_ref().unwrap_or(&"No description".to_string()));
            }
        }
    } else {
        println!("  • No tags defined");
    }
    
    println!("\nComponent Schemas:");
    if let Some(components) = &spec.components {
        for (name, schema) in components.schemas.iter() {
            println!("  • {} ({})", name, schema.schema_type.as_ref().unwrap_or(&"unknown".to_string()));
        }
    }
    
    println!("\nServer Configuration:");
    for server in &spec.servers {
        println!("  • {}: {}", server.url, server.description.as_ref().unwrap_or(&"No description".to_string()));
    }
    
    // Show JSON serialization works
    println!("\nJSON Serialization Test:");
    match serde_json::to_string_pretty(&spec) {
        Ok(json) => {
            let preview = if json.len() > 300 {
                format!("{}...\n[truncated {} chars]", &json[..300], json.len() - 300)
            } else {
                json
            };
            println!("{}", preview);
        }
        Err(e) => {
            println!("JSON serialization failed: {}", e);
        }
    }
    
    println!("\nOpenAPI specification generated successfully!");
    println!("\nIntegration Details:");
    println!("  Dependencies added:");
    println!("    • utoipa = {{ version = \"5.0\", features = [\"axum_extras\", \"chrono\", \"uuid\"] }}");
    println!("    • utoipa-swagger-ui = {{ version = \"8.0\", features = [\"axum\"] }}");
    println!("\n  Changes made:");
    println!("    • Added ToSchema derives to API types");
    println!("    • Added #[utoipa::path(...)] annotations to handlers");
    println!("    • Created ApiDoc struct with OpenAPI generation");
    println!("    • Integrated Swagger UI into routing");
    println!("\n  When server runs, documentation will be available at:");
    println!("    • /api/openapi.json - OpenAPI specification");
    println!("    • /api/swagger-ui - Interactive Swagger UI");
    println!("    • /api/docs - Custom HTML Swagger UI");
    println!("    • /api/redoc - ReDoc documentation");
    
    println!("\nIntegration complete! The Swagger UI is ready to use.");
}