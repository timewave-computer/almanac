-- Migration: Create Valence Library related tables

CREATE TABLE valence_libraries (
    id VARCHAR PRIMARY KEY,                         -- Unique ID (e.g., chain_id:contract_address)
    chain_id VARCHAR NOT NULL,
    contract_address VARCHAR NOT NULL,
    library_type VARCHAR NOT NULL,                  -- Type of library (e.g., "swap", "bridge", "messaging")
    created_at_block BIGINT NOT NULL,
    created_at_tx VARCHAR NOT NULL,
    current_owner VARCHAR,                          -- Nullable if renounced
    current_version INT,                            -- Current active version (if any)
    last_updated_block BIGINT NOT NULL,
    last_updated_tx VARCHAR NOT NULL,

    CONSTRAINT uq_valence_libraries_chain_address UNIQUE (chain_id, contract_address)
);

CREATE INDEX idx_valence_libraries_owner ON valence_libraries (current_owner);
CREATE INDEX idx_valence_libraries_chain ON valence_libraries (chain_id);
CREATE INDEX idx_valence_libraries_type ON valence_libraries (library_type);

COMMENT ON TABLE valence_libraries IS 'Valence library contracts providing reusable functionality';
COMMENT ON COLUMN valence_libraries.library_type IS 'Type/category of library functionality';
COMMENT ON COLUMN valence_libraries.current_version IS 'Current active version number of the library';

-- Library versions tracking
CREATE TABLE valence_library_versions (
    id VARCHAR PRIMARY KEY,                         -- Unique version ID
    library_id VARCHAR NOT NULL REFERENCES valence_libraries(id) ON DELETE CASCADE,
    version INT NOT NULL,                           -- Version number
    code_hash VARCHAR NOT NULL,                     -- Hash of version's code for verification
    created_at_block BIGINT NOT NULL,
    created_at_tx VARCHAR NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT false,       -- Whether this version is active/current
    features TEXT[],                                -- Array of features in this version
    metadata JSONB,                                 -- Additional version metadata
    
    CONSTRAINT uq_valence_library_versions UNIQUE (library_id, version)
);

CREATE INDEX idx_valence_library_versions_library ON valence_library_versions (library_id);
CREATE INDEX idx_valence_library_versions_active ON valence_library_versions (library_id, is_active);

COMMENT ON TABLE valence_library_versions IS 'Versions of Valence libraries';
COMMENT ON COLUMN valence_library_versions.features IS 'Features supported by this version';

-- Library usage tracking
CREATE TABLE valence_library_usage (
    id VARCHAR PRIMARY KEY,                         -- Unique usage ID
    library_id VARCHAR NOT NULL REFERENCES valence_libraries(id) ON DELETE CASCADE,
    user_address VARCHAR NOT NULL,                  -- Address using the library
    account_id VARCHAR,                             -- If used by a Valence account
    function_name VARCHAR,                          -- Function being used, if known
    usage_at_block BIGINT NOT NULL,
    usage_at_tx VARCHAR NOT NULL,
    gas_used BIGINT,                                -- Gas used by the library call
    success BOOLEAN NOT NULL DEFAULT true,          -- Whether the usage was successful
    error TEXT,                                     -- Error message if failed
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_valence_library_usage_library ON valence_library_usage (library_id);
CREATE INDEX idx_valence_library_usage_user ON valence_library_usage (user_address);
CREATE INDEX idx_valence_library_usage_account ON valence_library_usage (account_id);
CREATE INDEX idx_valence_library_usage_function ON valence_library_usage (function_name);
CREATE INDEX idx_valence_library_usage_block ON valence_library_usage (usage_at_block);

COMMENT ON TABLE valence_library_usage IS 'Records of Valence library usage';
COMMENT ON COLUMN valence_library_usage.account_id IS 'Optional Valence account ID using the library';
COMMENT ON COLUMN valence_library_usage.function_name IS 'Name of the function being used, if available';

-- Library approvals tracking
CREATE TABLE valence_library_approvals (
    id VARCHAR PRIMARY KEY,                         -- Unique approval ID
    library_id VARCHAR NOT NULL REFERENCES valence_libraries(id) ON DELETE CASCADE,
    account_id VARCHAR NOT NULL,                    -- Account approving the library
    approved_at_block BIGINT NOT NULL,
    approved_at_tx VARCHAR NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,        -- Whether approval is still active
    revoked_at_block BIGINT,                        -- When the approval was revoked
    revoked_at_tx VARCHAR,                          -- Transaction that revoked the approval
    
    CONSTRAINT uq_valence_library_approvals UNIQUE (library_id, account_id)
);

CREATE INDEX idx_valence_library_approvals_library ON valence_library_approvals (library_id);
CREATE INDEX idx_valence_library_approvals_account ON valence_library_approvals (account_id);
CREATE INDEX idx_valence_library_approvals_active ON valence_library_approvals (is_active);

COMMENT ON TABLE valence_library_approvals IS 'Records of Valence library approvals by accounts';
COMMENT ON COLUMN valence_library_approvals.account_id IS 'Account approving use of the library'; 