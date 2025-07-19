use crate::openapi::ApiDoc;
use utoipa::OpenApi;

#[test]
fn test_openapi_generation() {
    // Test that we can generate the OpenAPI spec
    let spec = ApiDoc::openapi();
    
    // Verify basic structure
    assert_eq!(spec.info.title, "Almanac API");
    assert_eq!(spec.info.version, "1.0.0");
    
    // Verify we have the events endpoints
    assert!(spec.paths.paths.contains_key("/api/v1/events"));
    assert!(spec.paths.paths.contains_key("/api/v1/events/{id}"));
    assert!(spec.paths.paths.contains_key("/api/v1/chains/{chain}/events"));
    assert!(spec.paths.paths.contains_key("/api/v1/chains/{chain}/latest-block"));
    
    // Verify we have the schemas
    assert!(spec.components.as_ref().unwrap().schemas.contains_key("ApiEvent"));
    assert!(spec.components.as_ref().unwrap().schemas.contains_key("EventQuery"));
    assert!(spec.components.as_ref().unwrap().schemas.contains_key("LatestBlockResponse"));
    
    // Ensure we can serialize to JSON
    let json = serde_json::to_string_pretty(&spec).unwrap();
    assert!(json.contains("\"title\": \"Almanac API\""));
    
    println!("OpenAPI spec generated successfully!");
}