use crate::handlers::events::{EventQuery, LatestBlockResponse};
use axum::{
    response::{Html, IntoResponse},
    Json,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::events::list_events,
        crate::handlers::events::get_event,
        crate::handlers::events::list_chain_events,
        crate::handlers::events::get_latest_block,
        crate::handlers::idl::get_anchor_idl,
        crate::handlers::idl::list_anchor_idls,
    ),
    components(
        schemas(
            crate::types::event::ApiEvent,
            EventQuery,
            LatestBlockResponse,
            crate::handlers::idl::IdlListResponse,
        )
    ),
    tags(
        (name = "events", description = "Event management API"),
        (name = "anchor-idl", description = "Anchor IDL management API")
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

/// Serve the OpenAPI specification
pub async fn openapi_spec() -> impl IntoResponse {
    Json(ApiDoc::openapi())
}

/// Serve Swagger UI HTML
pub async fn swagger_ui() -> impl IntoResponse {
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Almanac API Documentation</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
        window.onload = () => {
            window.ui = SwaggerUIBundle({
                url: '/api/openapi.json',
                dom_id: '#swagger-ui',
                deepLinking: true,
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIBundle.SwaggerUIStandalonePreset
                ],
                layout: "BaseLayout"
            });
        };
    </script>
</body>
</html>
    "#,
    )
}

/// Serve ReDoc HTML
pub async fn redoc() -> impl IntoResponse {
    Html(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Almanac API Documentation</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body { margin: 0; padding: 0; }
    </style>
</head>
<body>
    <redoc spec-url='/api/openapi.json'></redoc>
    <script src="https://cdn.jsdelivr.net/npm/redoc@2/bundles/redoc.standalone.js"></script>
</body>
</html>
    "#,
    )
}
