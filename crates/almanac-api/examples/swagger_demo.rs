use almanac_api::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
    // Generate the OpenAPI specification
    let spec = ApiDoc::openapi();

    println!("Almanac API OpenAPI Specification Generated!");
    println!("═══════════════════════════════════════════");

    // Print basic info
    println!("API Info:");
    println!("  Title: {}", spec.info.title);
    println!("  Version: {}", spec.info.version);
    println!(
        "  Description: {}",
        spec.info
            .description
            .as_ref()
            .unwrap_or(&"No description".to_string())
    );

    // Print paths
    println!("\nAvailable Endpoints:");
    for (path, _) in spec.paths.paths.iter() {
        println!("  • {}", path);
    }

    // Print schemas
    println!("\nAvailable Schemas:");
    if let Some(components) = &spec.components {
        for (name, _) in components.schemas.iter() {
            println!("  • {}", name);
        }
    }

    // Print tags
    println!("\nTags:");
    if let Some(tags) = &spec.tags {
        if tags.is_empty() {
            println!("  • No tags defined");
        } else {
            for tag in tags.iter() {
                println!(
                    "  • {}: {}",
                    tag.name,
                    tag.description
                        .as_ref()
                        .unwrap_or(&"No description".to_string())
                );
            }
        }
    } else {
        println!("  • No tags defined");
    }

    // Generate JSON output
    println!("\nJSON Output (first 500 chars):");
    let json = serde_json::to_string_pretty(&spec).unwrap();
    let preview = if json.len() > 500 {
        format!("{}...", &json[..500])
    } else {
        json
    };
    println!("{}", preview);

    println!("\nOpenAPI spec generation successful!");
    println!("To view the Swagger UI, start the server and visit:");
    println!("   http://localhost:8080/api/swagger-ui");
    println!("   http://localhost:8080/api/docs");
    println!("   http://localhost:8080/api/redoc");
}
