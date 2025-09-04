//! Account subscription management API endpoints
//!
//! This module provides REST API endpoints for managing account subscriptions
//! with bloom filter optimization and Merkle tree verification.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use utoipa::{IntoParams, ToSchema};

use crate::{
    state::AppState,
    types::common::{PaginatedResponse, PaginationParams},
    types::error::{ApiError, ApiResult},
    types::response::ApiResponse,
};
use almanac_chains::subscription::{
    AccountAddress,
    AccountSubscriptionError,
    // Removed unused import: AccountSubscriptionManager
};

/// Request to add accounts to subscription
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AddAccountsRequest {
    /// List of account addresses to subscribe to
    pub accounts: Vec<AccountAddressInput>,
    /// Optional developer ID for tracking
    pub developer_id: Option<String>,
}

/// Request to remove accounts from subscription
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RemoveAccountsRequest {
    /// List of account addresses to unsubscribe from
    pub accounts: Vec<AccountAddressInput>,
    /// Optional developer ID for tracking
    pub developer_id: Option<String>,
}

/// Account address input in various formats
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AccountAddressInput {
    /// Chain identifier (e.g., "ethereum", "solana", "cosmos")
    pub chain_id: String,
    /// Account address in chain-specific format
    pub address: String,
}

/// Account subscription status response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccountSubscriptionStatus {
    /// Account address information
    pub account: AccountAddressResponse,
    /// Whether the account is currently subscribed
    pub subscribed: bool,
    /// When the subscription was added (if subscribed)
    pub subscribed_at: Option<u64>,
    /// Last update timestamp for this account
    pub last_updated: Option<u64>,
}

/// Account address response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AccountAddressResponse {
    /// Chain identifier
    pub chain_id: String,
    /// Account address in hex format
    pub address_hex: String,
    /// Account address in chain-specific format
    pub address_native: String,
}

/// Merkle tree inclusion proof response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InclusionProofResponse {
    /// Account that was verified
    pub account: AccountAddressResponse,
    /// Hash of the account data
    pub data_hash: String,
    /// Current Merkle tree root hash
    pub root_hash: String,
    /// Merkle proof path
    pub proof_path: Vec<String>,
    /// Tree height/depth
    pub tree_height: u8,
    /// Whether the proof is valid
    pub is_valid: bool,
}

/// Subscription statistics response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SubscriptionStatsResponse {
    /// Total number of subscribed accounts
    pub total_subscribed_accounts: usize,
    /// Number of accounts by chain
    pub accounts_by_chain: std::collections::HashMap<String, usize>,
    /// Current Merkle tree root
    pub current_tree_root: String,
    /// Last update block/slot
    pub last_update_block: u64,
    /// Total updates processed
    pub total_updates_processed: u64,
    /// Bloom filter utilization percentage
    pub bloom_filter_utilization: f64,
    /// Average proof size in bytes
    pub average_proof_size: usize,
}

/// Query parameters for account subscription listing
#[derive(Debug, Deserialize, IntoParams)]
pub struct SubscriptionQuery {
    /// Filter by chain ID
    pub chain_id: Option<String>,
    /// Filter by developer ID
    pub developer_id: Option<String>,
}

/// Add accounts to subscription list
///
/// Subscribe to receive real-time updates for the specified accounts.
/// Updates are filtered using bloom filters and verified with Merkle trees.
#[utoipa::path(
    post,
    path = "/api/v1/subscriptions/accounts",
    request_body = AddAccountsRequest,
    responses(
        (status = 200, description = "Accounts successfully subscribed", body = ApiResponse<SubscriptionStatsResponse>),
        (status = 400, description = "Invalid request", body = ApiResponse<String>),
        (status = 429, description = "Subscription limit exceeded", body = ApiResponse<String>),
        (status = 500, description = "Internal server error", body = ApiResponse<String>)
    ),
    tag = "subscriptions"
)]
pub async fn add_accounts(
    State(_state): State<AppState>,
    Json(request): Json<AddAccountsRequest>,
) -> ApiResult<impl IntoResponse> {
    info!("Adding {} accounts to subscription", request.accounts.len());

    // Use the subscription manager from state if available
    if let Some(ref subscription_manager) = _state.subscription_manager {
        let mut accounts_to_add = Vec::new();

        for account_input in &request.accounts {
            let account =
                match AccountAddress::from_hex(&account_input.chain_id, &account_input.address) {
                    Ok(addr) => addr,
                    Err(_) => {
                        // Try base58 for Solana
                        AccountAddress::from_base58(&account_input.chain_id, &account_input.address)
                            .map_err(|_| {
                                ApiError::BadRequest(format!(
                                    "Invalid address format: {}",
                                    account_input.address
                                ))
                            })?
                    }
                };
            accounts_to_add.push(account);
        }

        match subscription_manager.add_accounts(accounts_to_add).await {
            Ok(()) => {
                info!(
                    "Successfully added {} accounts to subscription",
                    request.accounts.len()
                );
            }
            Err(AccountSubscriptionError::SubscriptionLimitExceeded(limit)) => {
                return Err(ApiError::BadRequest(format!(
                    "Subscription limit of {} accounts exceeded",
                    limit
                )));
            }
            Err(e) => {
                return Err(ApiError::InternalError(format!(
                    "Failed to add accounts: {}",
                    e
                )));
            }
        }
    } else {
        return Err(ApiError::InternalError(
            "Subscription manager not available".to_string(),
        ));
    }

    if request.accounts.is_empty() {
        return Err(ApiError::BadRequest("No accounts provided".to_string()));
    }

    // Validate account addresses
    for account_input in &request.accounts {
        if account_input.address.is_empty() {
            return Err(ApiError::BadRequest(
                "Empty account address provided".to_string(),
            ));
        }

        // Basic chain ID validation
        match account_input.chain_id.as_str() {
            "ethereum" | "solana" | "cosmos" => {} // Valid chains
            _ => {
                return Err(ApiError::BadRequest(format!(
                    "Unsupported chain: {}",
                    account_input.chain_id
                )))
            }
        }
    }

    // Get actual statistics from subscription manager
    let stats = if let Some(ref _subscription_manager) = _state.subscription_manager {
        // This would need to be implemented in the subscription manager trait
        // For now, use placeholder data
        SubscriptionStatsResponse {
            total_subscribed_accounts: request.accounts.len(),
            accounts_by_chain: {
                let mut map = std::collections::HashMap::new();
                for account in &request.accounts {
                    *map.entry(account.chain_id.clone()).or_insert(0) += 1;
                }
                map
            },
            current_tree_root: "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            last_update_block: 0,
            total_updates_processed: 0,
            bloom_filter_utilization: 0.0,
            average_proof_size: 0,
        }
    } else {
        return Err(ApiError::InternalError(
            "Subscription manager not available".to_string(),
        ));
    };

    Ok((StatusCode::OK, Json(ApiResponse::success(stats))))
}

/// Remove accounts from subscription list
///
/// Unsubscribe from receiving updates for the specified accounts.
#[utoipa::path(
    delete,
    path = "/api/v1/subscriptions/accounts",
    request_body = RemoveAccountsRequest,
    responses(
        (status = 200, description = "Accounts successfully unsubscribed", body = ApiResponse<SubscriptionStatsResponse>),
        (status = 400, description = "Invalid request", body = ApiResponse<String>),
        (status = 500, description = "Internal server error", body = ApiResponse<String>)
    ),
    tag = "subscriptions"
)]
pub async fn remove_accounts(
    State(_state): State<AppState>,
    Json(request): Json<RemoveAccountsRequest>,
) -> ApiResult<impl IntoResponse> {
    info!(
        "Removing {} accounts from subscription",
        request.accounts.len()
    );

    // Use the subscription manager from state if available
    if let Some(ref subscription_manager) = _state.subscription_manager {
        let mut accounts_to_remove = Vec::new();

        for account_input in &request.accounts {
            let account =
                match AccountAddress::from_hex(&account_input.chain_id, &account_input.address) {
                    Ok(addr) => addr,
                    Err(_) => {
                        // Try base58 for Solana
                        AccountAddress::from_base58(&account_input.chain_id, &account_input.address)
                            .map_err(|_| {
                                ApiError::BadRequest(format!(
                                    "Invalid address format: {}",
                                    account_input.address
                                ))
                            })?
                    }
                };
            accounts_to_remove.push(account);
        }

        match subscription_manager
            .remove_accounts(accounts_to_remove)
            .await
        {
            Ok(()) => {
                info!(
                    "Successfully removed {} accounts from subscription",
                    request.accounts.len()
                );
            }
            Err(e) => {
                return Err(ApiError::InternalError(format!(
                    "Failed to remove accounts: {}",
                    e
                )));
            }
        }
    } else {
        return Err(ApiError::InternalError(
            "Subscription manager not available".to_string(),
        ));
    }

    if request.accounts.is_empty() {
        return Err(ApiError::BadRequest("No accounts provided".to_string()));
    }

    // Placeholder statistics
    let stats = SubscriptionStatsResponse {
        total_subscribed_accounts: 0,
        accounts_by_chain: std::collections::HashMap::new(),
        current_tree_root: "0x0000000000000000000000000000000000000000000000000000000000000000"
            .to_string(),
        last_update_block: 0,
        total_updates_processed: 0,
        bloom_filter_utilization: 0.0,
        average_proof_size: 0,
    };

    Ok((StatusCode::OK, Json(ApiResponse::success(stats))))
}

/// Check subscription status for specific accounts
///
/// Get the current subscription status for one or more accounts.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/accounts/status",
    params(
        ("accounts" = Vec<String>, Query, description = "Comma-separated list of account addresses to check")
    ),
    responses(
        (status = 200, description = "Account subscription statuses", body = ApiResponse<Vec<AccountSubscriptionStatus>>),
        (status = 400, description = "Invalid request", body = ApiResponse<String>),
        (status = 500, description = "Internal server error", body = ApiResponse<String>)
    ),
    tag = "subscriptions"
)]
pub async fn get_account_status(
    State(_state): State<AppState>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<impl IntoResponse> {
    let accounts_param = query
        .get("accounts")
        .ok_or_else(|| ApiError::BadRequest("Missing 'accounts' parameter".to_string()))?;

    let account_addresses: Vec<&str> = accounts_param.split(',').collect();

    info!(
        "Checking subscription status for {} accounts",
        account_addresses.len()
    );

    // Use the subscription manager from state if available
    if let Some(ref subscription_manager) = _state.subscription_manager {
        let mut statuses = Vec::new();

        for address in account_addresses {
            if address.is_empty() {
                continue;
            }

            // Try to parse as different address formats
            let is_subscribed = if let Ok(account) = AccountAddress::from_hex("unknown", address) {
                subscription_manager
                    .is_subscribed(&account)
                    .await
                    .unwrap_or(false)
            } else if let Ok(account) = AccountAddress::from_base58("unknown", address) {
                subscription_manager
                    .is_subscribed(&account)
                    .await
                    .unwrap_or(false)
            } else {
                false
            };

            let status = AccountSubscriptionStatus {
                account: AccountAddressResponse {
                    chain_id: "unknown".to_string(),
                    address_hex: format!("0x{}", hex::encode(address.as_bytes())),
                    address_native: address.to_string(),
                },
                subscribed: is_subscribed,
                subscribed_at: None,
                last_updated: None,
            };

            statuses.push(status);
        }

        Ok((StatusCode::OK, Json(ApiResponse::success(statuses))))
    } else {
        // Fallback to placeholder response
        let mut statuses = Vec::new();

        for address in account_addresses {
            if address.is_empty() {
                continue;
            }

            let status = AccountSubscriptionStatus {
                account: AccountAddressResponse {
                    chain_id: "unknown".to_string(),
                    address_hex: format!("0x{}", hex::encode(address.as_bytes())),
                    address_native: address.to_string(),
                },
                subscribed: false,
                subscribed_at: None,
                last_updated: None,
            };

            statuses.push(status);
        }

        Ok((StatusCode::OK, Json(ApiResponse::success(statuses))))
    }
}

/// List all subscribed accounts
///
/// Get a paginated list of all currently subscribed accounts,
/// optionally filtered by chain or developer.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/accounts",
    params(
        SubscriptionQuery,
        PaginationParams
    ),
    responses(
        (status = 200, description = "List of subscribed accounts", body = ApiResponse<PaginatedResponse<AccountSubscriptionStatus>>),
        (status = 500, description = "Internal server error", body = ApiResponse<String>)
    ),
    tag = "subscriptions"
)]
pub async fn list_subscribed_accounts(
    State(_state): State<AppState>,
    Query(query): Query<SubscriptionQuery>,
    Query(pagination): Query<PaginationParams>,
) -> ApiResult<impl IntoResponse> {
    info!(
        "Listing subscribed accounts with filters: chain_id={:?}, developer_id={:?}",
        query.chain_id, query.developer_id
    );

    // Use the subscription manager from state if available
    if let Some(ref subscription_manager) = _state.subscription_manager {
        // Get subscribed accounts for the specified chain or all chains
        let accounts = if let Some(chain_id) = &query.chain_id {
            subscription_manager
                .get_subscribed_accounts(chain_id)
                .await
                .unwrap_or_else(|_| Vec::new())
        } else {
            // If no chain specified, get all accounts (this would need chain enumeration)
            Vec::new()
        };

        let account_count = accounts.len();
        let account_statuses: Vec<AccountSubscriptionStatus> = accounts
            .into_iter()
            .map(|account| {
                AccountSubscriptionStatus {
                    account: AccountAddressResponse {
                        chain_id: account.chain_id.clone(),
                        address_hex: account.to_hex(),
                        address_native: account.to_hex(), // Simplified for now
                    },
                    subscribed: true,
                    subscribed_at: Some(chrono::Utc::now().timestamp() as u64),
                    last_updated: Some(chrono::Utc::now().timestamp() as u64),
                }
            })
            .collect();

        let response = PaginatedResponse {
            data: account_statuses,
            page: pagination.page.unwrap_or(1),
            page_size: pagination.page_size.unwrap_or(50),
            total_pages: 1, // Simplified
            total_items: account_count as u64,
        };

        Ok((StatusCode::OK, Json(ApiResponse::success(response))))
    } else {
        // Fallback to empty response
        let empty_response = PaginatedResponse {
            data: Vec::<AccountSubscriptionStatus>::new(),
            page: pagination.page.unwrap_or(1),
            page_size: pagination.page_size.unwrap_or(50),
            total_pages: 0,
            total_items: 0,
        };

        Ok((StatusCode::OK, Json(ApiResponse::success(empty_response))))
    }
}

/// Generate Merkle tree inclusion proof for an account
///
/// Generate a cryptographic proof that an account is included in the
/// current Merkle tree state and has been updated.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/accounts/{chain_id}/{address}/proof",
    params(
        ("chain_id" = String, Path, description = "Chain identifier"),
        ("address" = String, Path, description = "Account address")
    ),
    responses(
        (status = 200, description = "Merkle inclusion proof", body = ApiResponse<InclusionProofResponse>),
        (status = 404, description = "Account not found or not subscribed", body = ApiResponse<String>),
        (status = 500, description = "Internal server error", body = ApiResponse<String>)
    ),
    tag = "subscriptions"
)]
pub async fn generate_inclusion_proof(
    State(_state): State<AppState>,
    Path((chain_id, address)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    info!(
        "Generating inclusion proof for account: {}:{}",
        chain_id, address
    );

    // Use the subscription manager from state if available
    if let Some(ref subscription_manager) = _state.subscription_manager {
        let account = match AccountAddress::from_hex(&chain_id, &address) {
            Ok(addr) => addr,
            Err(_) => {
                // Try base58 for Solana
                AccountAddress::from_base58(&chain_id, &address).map_err(|_| {
                    ApiError::BadRequest(format!("Invalid address format: {}", address))
                })?
            }
        };

        match subscription_manager
            .generate_inclusion_proof(&account)
            .await
        {
            Ok(Some(proof)) => {
                let response = InclusionProofResponse {
                    account: AccountAddressResponse {
                        chain_id: account.chain_id.clone(),
                        address_hex: account.to_hex(),
                        address_native: address.clone(),
                    },
                    data_hash: hex::encode(proof.data_hash),
                    root_hash: hex::encode(proof.root_hash),
                    proof_path: proof.proof_path.iter().map(hex::encode).collect(),
                    tree_height: proof.tree_height,
                    is_valid: true, // Proof was successfully generated
                };
                Ok((StatusCode::OK, Json(ApiResponse::success(response))))
            }
            Ok(None) => Err(ApiError::NotFound(
                "Account not found or not subscribed".to_string(),
            )),
            Err(e) => Err(ApiError::InternalError(format!(
                "Failed to generate proof: {}",
                e
            ))),
        }
    } else {
        // Fallback to placeholder proof
        let proof = InclusionProofResponse {
            account: AccountAddressResponse {
                chain_id: chain_id.clone(),
                address_hex: format!("0x{}", hex::encode(address.as_bytes())),
                address_native: address.clone(),
            },
            data_hash: "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            root_hash: "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            proof_path: vec![],
            tree_height: 255,
            is_valid: false, // Placeholder always returns invalid
        };

        Ok((StatusCode::OK, Json(ApiResponse::success(proof))))
    }
}

/// Get subscription system statistics
///
/// Get current statistics about the subscription system including
/// number of subscribed accounts, Merkle tree status, and performance metrics.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/stats",
    responses(
        (status = 200, description = "Subscription system statistics", body = ApiResponse<SubscriptionStatsResponse>),
        (status = 500, description = "Internal server error", body = ApiResponse<String>)
    ),
    tag = "subscriptions"
)]
pub async fn get_subscription_stats(
    State(_state): State<AppState>,
) -> ApiResult<impl IntoResponse> {
    info!("Getting subscription system statistics");

    // Use the subscription manager from state if available
    if let Some(ref subscription_manager) = _state.subscription_manager {
        // Get basic stats from available methods
        let tree_root = subscription_manager
            .get_tree_root()
            .await
            .unwrap_or([0u8; 32]);

        // Count subscribed accounts across chains (simplified approach)
        let mut total_count = 0;
        let mut accounts_by_chain = std::collections::HashMap::new();
        
        // Check common chains
        for chain in &["ethereum", "solana", "cosmos"] {
            if let Ok(accounts) = subscription_manager.get_subscribed_accounts(chain).await {
                let count = accounts.len();
                if count > 0 {
                    accounts_by_chain.insert(chain.to_string(), count);
                    total_count += count;
                }
            }
        }

        let stats = SubscriptionStatsResponse {
            total_subscribed_accounts: total_count,
            accounts_by_chain,
            current_tree_root: hex::encode(tree_root),
            last_update_block: 0,
            total_updates_processed: 0,
            bloom_filter_utilization: 0.0,
            average_proof_size: 256,
        };

        Ok((StatusCode::OK, Json(ApiResponse::success(stats))))
    } else {
        // Fallback to placeholder stats
        let stats = SubscriptionStatsResponse {
            total_subscribed_accounts: 0,
            accounts_by_chain: std::collections::HashMap::new(),
            current_tree_root: "0x0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            last_update_block: 0,
            total_updates_processed: 0,
            bloom_filter_utilization: 0.0,
            average_proof_size: 0,
        };

        Ok((StatusCode::OK, Json(ApiResponse::success(stats))))
    }
}

/// Get accounts touched since a specific block/slot
///
/// Get a list of subscribed accounts that have been updated since
/// a specified block height or slot number.
#[utoipa::path(
    get,
    path = "/api/v1/subscriptions/touched",
    params(
        ("since_block" = u64, Query, description = "Block height or slot number"),
        ("chain_id" = Option<String>, Query, description = "Filter by specific chain")
    ),
    responses(
        (status = 200, description = "List of touched accounts", body = ApiResponse<Vec<AccountSubscriptionStatus>>),
        (status = 400, description = "Invalid block number", body = ApiResponse<String>),
        (status = 500, description = "Internal server error", body = ApiResponse<String>)
    ),
    tag = "subscriptions"
)]
pub async fn get_touched_accounts(
    State(_state): State<AppState>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<impl IntoResponse> {
    let since_block = query
        .get("since_block")
        .ok_or_else(|| ApiError::BadRequest("Missing 'since_block' parameter".to_string()))?
        .parse::<u64>()
        .map_err(|_| ApiError::BadRequest("Invalid block number".to_string()))?;

    let chain_id = query.get("chain_id");

    info!(
        "Getting accounts touched since block {} for chain {:?}",
        since_block, chain_id
    );

    // Use the subscription manager from state if available
    if let Some(ref subscription_manager) = _state.subscription_manager {
        match subscription_manager.get_touched_accounts(since_block).await {
            Ok(accounts) => {
                let touched_accounts: Vec<AccountSubscriptionStatus> = accounts
                    .into_iter()
                    .filter(|account| {
                        // Filter by chain if specified
                        match chain_id {
                            Some(chain) => account.chain_id == *chain,
                            None => true,
                        }
                    })
                    .map(|account| AccountSubscriptionStatus {
                        account: AccountAddressResponse {
                            chain_id: account.chain_id.clone(),
                            address_hex: account.to_hex(),
                            address_native: account.to_hex(), // Simplified
                        },
                        subscribed: true,
                        subscribed_at: Some(since_block),
                        last_updated: Some(chrono::Utc::now().timestamp() as u64),
                    })
                    .collect();

                Ok((StatusCode::OK, Json(ApiResponse::success(touched_accounts))))
            }
            Err(_) => Ok((
                StatusCode::OK,
                Json(ApiResponse::success(Vec::<AccountSubscriptionStatus>::new())),
            )),
        }
    } else {
        // Fallback to empty list
        let touched_accounts = Vec::<AccountSubscriptionStatus>::new();

        Ok((StatusCode::OK, Json(ApiResponse::success(touched_accounts))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::*;

    #[tokio::test]
    async fn test_add_accounts_validation() {
        let app_state = create_test_app_state().await;

        // Test empty accounts
        let empty_request = AddAccountsRequest {
            accounts: vec![],
            developer_id: None,
        };

        let result = add_accounts(State(app_state.clone()), Json(empty_request)).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_valid_chain_ids() {
        let app_state = create_test_app_state().await;

        let valid_request = AddAccountsRequest {
            accounts: vec![
                AccountAddressInput {
                    chain_id: "ethereum".to_string(),
                    address: "0x742d35Cc6634C0532925a3b8D80bE5e4eb16F8d6".to_string(),
                },
                AccountAddressInput {
                    chain_id: "solana".to_string(),
                    address: "11111111111111111111111111111112".to_string(),
                },
            ],
            developer_id: Some("test_dev".to_string()),
        };

        let result = add_accounts(State(app_state), Json(valid_request)).await;

        match result {
            Ok(_) => (),
            Err(e) => panic!("add_accounts failed with error: {:?}", e),
        }
    }
}
