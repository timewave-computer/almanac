#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, SubMsg, CosmosMsg, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// State
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: String,
    pub authorized_users: Vec<String>,
}

// Messages
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddAuthorizedUser {
        user: String,
    },
    RemoveAuthorizedUser {
        user: String,
    },
    Execute {
        contract_addr: String,
        msg: Binary,
        funds: Vec<cosmwasm_std::Coin>,
    },
    TransferOwnership {
        new_owner: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetOwner {},
    IsAuthorized {
        user: String,
    },
    ListAuthorizedUsers {},
}

// Responses
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OwnerResponse {
    pub owner: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuthorizedResponse {
    pub is_authorized: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuthorizedUsersResponse {
    pub users: Vec<String>,
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
        authorized_users: vec![],
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
        ExecuteMsg::AddAuthorizedUser { user } => {
            execute_add_authorized_user(deps, info, user)
        },
        ExecuteMsg::RemoveAuthorizedUser { user } => {
            execute_remove_authorized_user(deps, info, user)
        },
        ExecuteMsg::Execute { contract_addr, msg, funds } => {
            execute_contract(deps, info, contract_addr, msg, funds)
        },
        ExecuteMsg::TransferOwnership { new_owner } => {
            execute_transfer_ownership(deps, info, new_owner)
        },
    }
}

pub fn execute_add_authorized_user(
    deps: DepsMut,
    info: MessageInfo,
    user: String,
) -> StdResult<Response> {
    let state_data = deps.storage.get(b"state").unwrap();
    let mut state: State = serde_json::from_slice(&state_data).unwrap();
    
    // Check if sender is owner
    if info.sender.to_string() != state.owner {
        return Err(cosmwasm_std::StdError::generic_err("Unauthorized"));
    }
    
    // Check if user is already authorized
    if state.authorized_users.contains(&user) {
        return Err(cosmwasm_std::StdError::generic_err("User already authorized"));
    }
    
    // Add user to authorized list
    state.authorized_users.push(user.clone());
    deps.storage.set(b"state", &serde_json::to_vec(&state).unwrap());
    
    Ok(Response::new()
        .add_attribute("method", "add_authorized_user")
        .add_attribute("user", user))
}

pub fn execute_remove_authorized_user(
    deps: DepsMut,
    info: MessageInfo,
    user: String,
) -> StdResult<Response> {
    let state_data = deps.storage.get(b"state").unwrap();
    let mut state: State = serde_json::from_slice(&state_data).unwrap();
    
    // Check if sender is owner
    if info.sender.to_string() != state.owner {
        return Err(cosmwasm_std::StdError::generic_err("Unauthorized"));
    }
    
    // Find and remove user from authorized list
    let index = state.authorized_users.iter().position(|u| u == &user);
    match index {
        Some(idx) => {
            state.authorized_users.remove(idx);
            deps.storage.set(b"state", &serde_json::to_vec(&state).unwrap());
            
            Ok(Response::new()
                .add_attribute("method", "remove_authorized_user")
                .add_attribute("user", user))
        },
        None => Err(cosmwasm_std::StdError::generic_err("User not found")),
    }
}

pub fn execute_contract(
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: String,
    msg: Binary,
    funds: Vec<cosmwasm_std::Coin>,
) -> StdResult<Response> {
    let state_data = deps.storage.get(b"state").unwrap();
    let state: State = serde_json::from_slice(&state_data).unwrap();
    
    // Check if sender is owner or authorized
    if info.sender.to_string() != state.owner && !state.authorized_users.contains(&info.sender.to_string()) {
        return Err(cosmwasm_std::StdError::generic_err("Unauthorized"));
    }
    
    // Create message to execute the contract
    let execute_msg = WasmMsg::Execute {
        contract_addr,
        msg,
        funds,
    };
    
    Ok(Response::new()
        .add_attribute("method", "execute_contract")
        .add_attribute("sender", info.sender)
        .add_submessage(SubMsg::new(CosmosMsg::Wasm(execute_msg))))
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
        QueryMsg::GetOwner {} => to_binary(&query_owner(deps)?),
        QueryMsg::IsAuthorized { user } => to_binary(&query_is_authorized(deps, user)?),
        QueryMsg::ListAuthorizedUsers {} => to_binary(&query_list_authorized_users(deps)?),
    }
}

fn query_owner(deps: Deps) -> StdResult<OwnerResponse> {
    let state_data = deps.storage.get(b"state").unwrap();
    let state: State = serde_json::from_slice(&state_data).unwrap();
    
    Ok(OwnerResponse {
        owner: state.owner,
    })
}

fn query_is_authorized(deps: Deps, user: String) -> StdResult<AuthorizedResponse> {
    let state_data = deps.storage.get(b"state").unwrap();
    let state: State = serde_json::from_slice(&state_data).unwrap();
    
    let is_authorized = state.authorized_users.contains(&user);
    
    Ok(AuthorizedResponse {
        is_authorized,
    })
}

fn query_list_authorized_users(deps: Deps) -> StdResult<AuthorizedUsersResponse> {
    let state_data = deps.storage.get(b"state").unwrap();
    let state: State = serde_json::from_slice(&state_data).unwrap();
    
    Ok(AuthorizedUsersResponse {
        users: state.authorized_users,
    })
} 