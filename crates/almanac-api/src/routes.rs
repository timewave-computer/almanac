use crate::{
    handlers::{
        create_api_key, delete_api_key, get_event, get_latest_block, list_api_keys,
        list_chain_events, list_events,
        idl::{get_anchor_idl, list_anchor_idls},
        semantics::{
            delete_semantic_interpretation, get_semantic_conflicts, get_semantic_interpretations,
            resolve_semantic_conflict, submit_semantic_interpretation,
            update_semantic_interpretation,
        },
        subscriptions::{
            add_accounts, generate_inclusion_proof, get_account_status, get_subscription_stats,
            get_touched_accounts, list_subscribed_accounts, remove_accounts,
        },
        traverse::{
            generate_traverse_request, generate_witness_for_slot, get_traverse_request_status,
            list_traverse_requests, submit_semantic_context, verify_traverse_witness,
        },
        trust::{
            check_trust, get_trust_graph, get_trust_policy, get_trust_relationships, grant_trust,
            revoke_trust, update_trust_policy,
        },
        update_api_key,
    },
    health::{health_check, liveness, readiness},
    middleware::auth::AuthState,
    middleware::{add_headers, log_request},
    openapi::{redoc, swagger_ui, ApiDoc},
    rate_limit::create_rate_limiter,
    state::AppState,
};
use axum::{
    middleware::{self},
    routing::{delete, get, patch, post, put},
    Router,
};
use std::sync::Arc;
use utoipa::OpenApi;

/// Create AuthState from AppState
fn create_auth_state(app_state: &AppState) -> Option<AuthState> {
    match (&app_state.auth_service, &app_state.permission_store) {
        (Some(auth_service), Some(permission_store)) => Some(AuthState {
            auth_service: auth_service.clone()
                as Arc<dyn crate::types::traits::AuthenticationService>,
            permission_store: permission_store.clone(),
        }),
        _ => None,
    }
}

/// Create the main router
pub fn create_router(mut state: AppState) -> Router {
    // Create rate limiter with cleanup task
    state.rate_limiter = create_rate_limiter(100);

    // Health check routes (no middleware)
    let health_routes = Router::new()
        .route("/health", get(health_check))
        .route("/liveness", get(liveness))
        .route("/readiness", get(readiness));

    // Documentation routes (legacy endpoints)
    let doc_routes = Router::new()
        .route("/docs", get(swagger_ui))
        .route("/redoc", get(redoc));

    // Admin routes (require authentication)
    let admin_routes = Router::new()
        .route("/keys", post(create_api_key))
        .route("/keys", get(list_api_keys))
        .route("/keys/:key", delete(delete_api_key))
        .route("/keys/:key", patch(update_api_key));
    // .layer(from_fn_with_state(state.key_store.clone(), legacy_auth_middleware));

    // Trust management routes
    let trust_routes = Router::new()
        .route("/grant", post(grant_trust))
        .route("/revoke", post(revoke_trust))
        .route("/policy/:developer_id", get(get_trust_policy))
        .route("/policy/:developer_id", put(update_trust_policy))
        .route("/relationships", get(get_trust_relationships))
        .route("/check/:trustor/:trustee", get(check_trust))
        .route("/graph/:developer_id", get(get_trust_graph));

    // Contract management routes
    let contract_routes = Router::new()
        // Semantic endpoints
        .route(
            "/:address/semantics/:field",
            post(submit_semantic_interpretation),
        )
        .route(
            "/:address/semantics/:field",
            get(get_semantic_interpretations),
        )
        .route(
            "/:address/semantics/:field",
            put(update_semantic_interpretation),
        )
        .route(
            "/:address/semantics/:field",
            delete(delete_semantic_interpretation),
        )
        .route(
            "/:address/semantics/:field/resolve",
            post(resolve_semantic_conflict),
        );

    // Semantic conflicts route (global view)
    let semantics_routes =
        Router::new().route("/conflicts/:chain/:contract", get(get_semantic_conflicts));

    // Traverse integration routes
    let traverse_routes = Router::new()
        .route("/request/:chain/:contract", post(generate_traverse_request))
        .route("/verify", post(verify_traverse_witness))
        .route("/request/:request_id", get(get_traverse_request_status))
        .route("/requests/:chain/:contract", get(list_traverse_requests))
        .route(
            "/witness/:chain/:contract/:slot",
            get(generate_witness_for_slot),
        )
        .route("/context/:request_id", post(submit_semantic_context));

    // Account subscription routes
    let subscription_routes = Router::new()
        .route("/accounts", post(add_accounts))
        .route("/accounts", delete(remove_accounts))
        .route("/accounts/status", get(get_account_status))
        .route("/accounts", get(list_subscribed_accounts))
        .route(
            "/accounts/:chain_id/:address/proof",
            get(generate_inclusion_proof),
        )
        .route("/stats", get(get_subscription_stats))
        .route("/touched", get(get_touched_accounts));

    // API v1 routes (optional authentication for rate limiting)
    let api_v1_routes = Router::new()
        .route("/events", get(list_events))
        .route("/events/:id", get(get_event))
        .route("/chains/:chain/events", get(list_chain_events))
        .route("/chains/:chain/latest-block", get(get_latest_block))
        // Anchor IDL routes
        .route("/anchor-idl", get(list_anchor_idls))
        .route("/anchor-idl/:program_id", get(get_anchor_idl))
        // New trust and contract routes
        .nest("/trust", trust_routes.with_state(state.clone()))
        .nest(
            "/chains/:chain/contracts",
            contract_routes.with_state(state.clone().into()),
        )
        .nest(
            "/semantics",
            semantics_routes.with_state(state.clone().into()),
        )
        .nest(
            "/traverse",
            traverse_routes.with_state(state.clone().into()),
        )
        .nest(
            "/subscriptions",
            subscription_routes.with_state(state.clone()),
        );
    // .layer(from_fn_with_state(state.rate_limiter.clone(), rate_limit_middleware));

    // Apply new auth middleware if auth service is configured, otherwise use legacy
    if let Some(_auth_state) = create_auth_state(&state) {
        // For now, continue using legacy auth until we properly integrate the new system
        // api_v1_routes = api_v1_routes.layer(from_fn_with_state(state.key_store.clone(), legacy_optional_auth_middleware));
        // TODO: Switch to new auth middleware when fully integrated
        // api_v1_routes = api_v1_routes.layer(from_fn_with_state(auth_state, auth_middleware));
    } else {
        // api_v1_routes = api_v1_routes.layer(from_fn_with_state(state.key_store.clone(), legacy_optional_auth_middleware));
    }

    // Main router with SwaggerUI at root for proper OpenAPI serving
    Router::new()
        .merge(health_routes)
        .merge(
            utoipa_swagger_ui::SwaggerUi::new("/swagger-ui")
                .url("/openapi.json", ApiDoc::openapi()),
        )
        .nest("/api", doc_routes)
        .nest("/admin", admin_routes)
        .nest("/api/v1", api_v1_routes)
        // .layer(body_limit(1024 * 1024)) // 1MB request size limit
        // .layer(timeout(30)) // 30 second timeout
        .layer(middleware::from_fn(add_headers))
        // Remove handle_error - it's not a middleware function
        .layer(middleware::from_fn(log_request))
        .with_state(state)
}

/// Create a test router with minimal middleware
pub fn create_test_router(state: AppState) -> Router {
    // Admin routes (require authentication)
    let admin_routes = Router::new()
        .route("/keys", post(create_api_key))
        .route("/keys", get(list_api_keys))
        .route("/keys/:key", delete(delete_api_key))
        .route("/keys/:key", patch(update_api_key));
    // .layer(from_fn_with_state(state.key_store.clone(), legacy_auth_middleware));

    // Trust management routes
    let trust_routes = Router::new()
        .route("/grant", post(grant_trust))
        .route("/revoke", post(revoke_trust))
        .route("/policy/:developer_id", get(get_trust_policy))
        .route("/policy/:developer_id", put(update_trust_policy))
        .route("/relationships", get(get_trust_relationships))
        .route("/check/:trustor/:trustee", get(check_trust))
        .route("/graph/:developer_id", get(get_trust_graph));

    // Contract management routes
    let contract_routes = Router::new()
        // Semantic endpoints
        .route(
            "/:address/semantics/:field",
            post(submit_semantic_interpretation),
        )
        .route(
            "/:address/semantics/:field",
            get(get_semantic_interpretations),
        )
        .route(
            "/:address/semantics/:field",
            put(update_semantic_interpretation),
        )
        .route(
            "/:address/semantics/:field",
            delete(delete_semantic_interpretation),
        )
        .route(
            "/:address/semantics/:field/resolve",
            post(resolve_semantic_conflict),
        );

    // Semantic conflicts route (global view)
    let semantics_routes =
        Router::new().route("/conflicts/:chain/:contract", get(get_semantic_conflicts));

    // Traverse integration routes
    let traverse_routes = Router::new()
        .route("/request/:chain/:contract", post(generate_traverse_request))
        .route("/verify", post(verify_traverse_witness))
        .route("/request/:request_id", get(get_traverse_request_status))
        .route("/requests/:chain/:contract", get(list_traverse_requests))
        .route(
            "/witness/:chain/:contract/:slot",
            get(generate_witness_for_slot),
        )
        .route("/context/:request_id", post(submit_semantic_context));

    // Account subscription routes
    let subscription_routes = Router::new()
        .route("/accounts", post(add_accounts))
        .route("/accounts", delete(remove_accounts))
        .route("/accounts/status", get(get_account_status))
        .route("/accounts", get(list_subscribed_accounts))
        .route(
            "/accounts/:chain_id/:address/proof",
            get(generate_inclusion_proof),
        )
        .route("/stats", get(get_subscription_stats))
        .route("/touched", get(get_touched_accounts));

    // API routes with rate limiting
    let api_routes = Router::new()
        .route("/events", get(list_events))
        .route("/events/:id", get(get_event))
        .route("/chains/:chain/events", get(list_chain_events))
        .route("/chains/:chain/latest-block", get(get_latest_block))
        // Anchor IDL routes
        .route("/anchor-idl", get(list_anchor_idls))
        .route("/anchor-idl/:program_id", get(get_anchor_idl))
        // New trust and contract routes
        .nest("/trust", trust_routes.with_state(state.clone()))
        .nest(
            "/chains/:chain/contracts",
            contract_routes.with_state(state.clone().into()),
        )
        .nest(
            "/semantics",
            semantics_routes.with_state(state.clone().into()),
        )
        .nest(
            "/traverse",
            traverse_routes.with_state(state.clone().into()),
        )
        .nest(
            "/subscriptions",
            subscription_routes.with_state(state.clone()),
        );
    // .layer(from_fn_with_state(state.rate_limiter.clone(), rate_limit_middleware))
    // .layer(from_fn_with_state(state.key_store.clone(), legacy_optional_auth_middleware));

    // Documentation routes (legacy endpoints)
    let doc_routes = Router::new()
        .route("/docs", get(swagger_ui))
        .route("/redoc", get(redoc));

    Router::new()
        .route("/health", get(health_check))
        .nest("/api", doc_routes)
        .nest("/admin", admin_routes)
        .nest("/api/v1", api_routes)
        // .layer(body_limit(10 * 1024 * 1024)) // 10MB for tests
        .with_state(state)
}
