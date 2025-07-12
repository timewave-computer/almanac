-- Migration: Create Valence Authorization related tables

CREATE TABLE valence_authorization_contracts (
    id VARCHAR PRIMARY KEY,                         -- Unique ID (e.g., chain_id:contract_address)
    chain_id VARCHAR NOT NULL,
    contract_address VARCHAR NOT NULL,
    created_at_block BIGINT NOT NULL,
    created_at_tx VARCHAR NOT NULL,
    current_owner VARCHAR,                          -- Nullable if renounced
    active_policy_id VARCHAR,                       -- ID of the current active policy
    last_updated_block BIGINT NOT NULL,
    last_updated_tx VARCHAR NOT NULL,

    CONSTRAINT uq_valence_auth_contracts_chain_address UNIQUE (chain_id, contract_address)
);

CREATE INDEX idx_valence_auth_contracts_owner ON valence_authorization_contracts (current_owner);
CREATE INDEX idx_valence_auth_contracts_chain ON valence_authorization_contracts (chain_id);

COMMENT ON TABLE valence_authorization_contracts IS 'Valence authorization contracts for managing access rights';

-- Policy storage
CREATE TABLE valence_authorization_policies (
    id VARCHAR PRIMARY KEY,                         -- Unique policy ID (UUID or hash)
    auth_id VARCHAR NOT NULL REFERENCES valence_authorization_contracts(id) ON DELETE CASCADE,
    version INT NOT NULL,                           -- Policy version number
    content_hash VARCHAR NOT NULL,                  -- Hash of policy content for verification
    created_at_block BIGINT NOT NULL,
    created_at_tx VARCHAR NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT false,       -- Whether this policy is currently active
    metadata JSONB,                                 -- Additional metadata about the policy
    
    CONSTRAINT uq_valence_auth_policies_version UNIQUE (auth_id, version)
);

CREATE INDEX idx_valence_auth_policies_contract ON valence_authorization_policies (auth_id);
CREATE INDEX idx_valence_auth_policies_active ON valence_authorization_policies (auth_id, is_active);

COMMENT ON TABLE valence_authorization_policies IS 'Policy definitions for Valence authorization contracts';

-- Individual grants
CREATE TABLE valence_authorization_grants (
    id VARCHAR PRIMARY KEY,                         -- Unique grant ID
    auth_id VARCHAR NOT NULL REFERENCES valence_authorization_contracts(id) ON DELETE CASCADE,
    grantee VARCHAR NOT NULL,                       -- Address given authorization
    permissions TEXT[] NOT NULL,                    -- Array of permission strings
    resources TEXT[] NOT NULL,                      -- Resources the permissions apply to
    granted_at_block BIGINT NOT NULL,
    granted_at_tx VARCHAR NOT NULL,
    expiry BIGINT,                                  -- Optional expiration (block number or timestamp)
    is_active BOOLEAN NOT NULL DEFAULT true,        -- Whether this grant is still active
    revoked_at_block BIGINT,                        -- When the grant was revoked (if applicable)
    revoked_at_tx VARCHAR,                          -- Transaction that revoked the grant
    
    CONSTRAINT uq_valence_auth_grants UNIQUE (auth_id, grantee, resources)
);

CREATE INDEX idx_valence_auth_grants_contract ON valence_authorization_grants (auth_id);
CREATE INDEX idx_valence_auth_grants_grantee ON valence_authorization_grants (grantee);
CREATE INDEX idx_valence_auth_grants_active ON valence_authorization_grants (is_active);

COMMENT ON TABLE valence_authorization_grants IS 'Authorization grants to address for specific resources';
COMMENT ON COLUMN valence_authorization_grants.permissions IS 'Array of permission strings granted';
COMMENT ON COLUMN valence_authorization_grants.resources IS 'Resources the permissions apply to';

-- Authorization requests and decisions
CREATE TYPE valence_auth_decision AS ENUM ('pending', 'approved', 'denied', 'error');

CREATE TABLE valence_authorization_requests (
    id VARCHAR PRIMARY KEY,                         -- Unique request ID
    auth_id VARCHAR NOT NULL REFERENCES valence_authorization_contracts(id) ON DELETE CASCADE,
    requester VARCHAR NOT NULL,                     -- Address requesting authorization
    action VARCHAR NOT NULL,                        -- Requested action
    resource VARCHAR NOT NULL,                      -- Resource to act upon
    request_data TEXT,                              -- Additional data related to the request
    decision valence_auth_decision NOT NULL DEFAULT 'pending',
    requested_at_block BIGINT NOT NULL,
    requested_at_tx VARCHAR NOT NULL,
    processed_at_block BIGINT,                      -- When the request was processed
    processed_at_tx VARCHAR,                        -- Transaction that processed the request
    reason TEXT,                                    -- Reason for the decision
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_valence_auth_requests_contract ON valence_authorization_requests (auth_id);
CREATE INDEX idx_valence_auth_requests_requester ON valence_authorization_requests (requester);
CREATE INDEX idx_valence_auth_requests_resource ON valence_authorization_requests (resource);
CREATE INDEX idx_valence_auth_requests_decision ON valence_authorization_requests (decision);
CREATE INDEX idx_valence_auth_requests_block ON valence_authorization_requests (requested_at_block);

COMMENT ON TABLE valence_authorization_requests IS 'Record of authorization requests and decisions';
COMMENT ON COLUMN valence_authorization_requests.action IS 'Action being requested (e.g., read, write, execute)';
COMMENT ON COLUMN valence_authorization_requests.resource IS 'Resource identifier the action applies to'; 