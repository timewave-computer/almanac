#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// State
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: String,
    pub permissions: Vec<Permission>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Permission {
    pub grant_id: String,
    pub grantee: String,
    pub permissions: Vec<String>,
    pub resources: Vec<String>,
}

// Messages
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    GrantPermission {
        grant_id: String,
        grantee: String,
        permissions: Vec<String>,
        resources: Vec<String>,
    },
    RevokePermission {
        grant_id: String,
    },
    TransferOwnership {
        new_owner: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetPermission {
        grant_id: String,
    },
    CheckPermission {
        grantee: String,
        permission: String,
        resource: String,
    },
    ListPermissions {},
    GetOwner {},
}

// Responses
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PermissionResponse {
    pub grant_id: String,
    pub grantee: String,
    pub permissions: Vec<String>,
    pub resources: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PermissionCheckResponse {
    pub allowed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PermissionsListResponse {
    pub permissions: Vec<Permission>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OwnerResponse {
    pub owner: String,
}

// Contract implementation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let state = State {
        owner: msg.owner,
        permissions: vec![],
    };
    deps.storage.set(b"state", &serde_json::to_vec(&state).unwrap());
    
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::GrantPermission { grant_id, grantee, permissions, resources } => {
            execute_grant_permission(deps, info, grant_id, grantee, permissions, resources)
        },
        ExecuteMsg::RevokePermission { grant_id } => {
            execute_revoke_permission(deps, info, grant_id)
        },
        ExecuteMsg::TransferOwnership { new_owner } => {
            execute_transfer_ownership(deps, info, new_owner)
        },
    }
}

pub fn execute_grant_permission(
    deps: DepsMut,
    info: MessageInfo,
    grant_id: String,
    grantee: String,
    permissions: Vec<String>,
    resources: Vec<String>,
) -> StdResult<Response> {
    let state_data = deps.storage.get(b"state").unwrap();
    let mut state: State = serde_json::from_slice(&state_data).unwrap();
    
    // Check if sender is owner
    if info.sender.to_string() != state.owner {
        return Err(cosmwasm_std::StdError::generic_err("Unauthorized"));
    }
    
    // Check if grant_id exists
    if state.permissions.iter().any(|p| p.grant_id == grant_id) {
        return Err(cosmwasm_std::StdError::generic_err("Grant ID already exists"));
    }
    
    // Add new permission
    let permission = Permission {
        grant_id,
        grantee,
        permissions,
        resources,
    };
    
    state.permissions.push(permission.clone());
    deps.storage.set(b"state", &serde_json::to_vec(&state).unwrap());
    
    Ok(Response::new()
        .add_attribute("method", "grant_permission")
        .add_attribute("grant_id", permission.grant_id)
        .add_attribute("grantee", permission.grantee))
}

pub fn execute_revoke_permission(
    deps: DepsMut,
    info: MessageInfo,
    grant_id: String,
) -> StdResult<Response> {
    let state_data = deps.storage.get(b"state").unwrap();
    let mut state: State = serde_json::from_slice(&state_data).unwrap();
    
    // Check if sender is owner
    if info.sender.to_string() != state.owner {
        return Err(cosmwasm_std::StdError::generic_err("Unauthorized"));
    }
    
    // Find and remove permission with matching grant_id
    let index = state.permissions.iter().position(|p| p.grant_id == grant_id);
    match index {
        Some(idx) => {
            state.permissions.remove(idx);
            deps.storage.set(b"state", &serde_json::to_vec(&state).unwrap());
            
            Ok(Response::new()
                .add_attribute("method", "revoke_permission")
                .add_attribute("grant_id", grant_id))
        },
        None => Err(cosmwasm_std::StdError::generic_err("Grant ID not found")),
    }
}

pub fn execute_transfer_ownership(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: String,
) -> StdResult<Response> {
    let state_data = deps.storage.get(b"state").unwrap();
    let mut state: State = serde_json::from_slice(&state_data).unwrap();
    
    // Check if sender is owner
    if info.sender.to_string() != state.owner {
        return Err(cosmwasm_std::StdError::generic_err("Unauthorized"));
    }
    
    // Update owner
    state.owner = new_owner.clone();
    deps.storage.set(b"state", &serde_json::to_vec(&state).unwrap());
    
    Ok(Response::new()
        .add_attribute("method", "transfer_ownership")
        .add_attribute("new_owner", new_owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPermission { grant_id } => to_binary(&query_permission(deps, grant_id)?),
        QueryMsg::CheckPermission { grantee, permission, resource } => {
            to_binary(&query_check_permission(deps, grantee, permission, resource)?)
        },
        QueryMsg::ListPermissions {} => to_binary(&query_list_permissions(deps)?),
        QueryMsg::GetOwner {} => to_binary(&query_owner(deps)?),
    }
}

fn query_permission(deps: Deps, grant_id: String) -> StdResult<PermissionResponse> {
    let state_data = deps.storage.get(b"state").unwrap();
    let state: State = serde_json::from_slice(&state_data).unwrap();
    
    match state.permissions.iter().find(|p| p.grant_id == grant_id) {
        Some(permission) => Ok(PermissionResponse {
            grant_id: permission.grant_id.clone(),
            grantee: permission.grantee.clone(),
            permissions: permission.permissions.clone(),
            resources: permission.resources.clone(),
        }),
        None => Err(cosmwasm_std::StdError::generic_err("Grant ID not found")),
    }
}

fn query_check_permission(
    deps: Deps,
    grantee: String,
    permission: String,
    resource: String,
) -> StdResult<PermissionCheckResponse> {
    let state_data = deps.storage.get(b"state").unwrap();
    let state: State = serde_json::from_slice(&state_data).unwrap();
    
    let allowed = state.permissions.iter().any(|p| {
        p.grantee == grantee &&
        p.permissions.contains(&permission) &&
        p.resources.contains(&resource)
    });
    
    Ok(PermissionCheckResponse { allowed })
}

fn query_list_permissions(deps: Deps) -> StdResult<PermissionsListResponse> {
    let state_data = deps.storage.get(b"state").unwrap();
    let state: State = serde_json::from_slice(&state_data).unwrap();
    
    Ok(PermissionsListResponse {
        permissions: state.permissions,
    })
}

fn query_owner(deps: Deps) -> StdResult<OwnerResponse> {
    let state_data = deps.storage.get(b"state").unwrap();
    let state: State = serde_json::from_slice(&state_data).unwrap();
    
    Ok(OwnerResponse {
        owner: state.owner,
    })
} 