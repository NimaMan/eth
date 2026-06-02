CREATE SCHEMA IF NOT EXISTS token_state;

CREATE TABLE IF NOT EXISTS token_state.token_latest (
    scope_id TEXT NOT NULL DEFAULT 'live',
    chain_id BIGINT NOT NULL DEFAULT 1,
    token_address TEXT NOT NULL,
    source_run_id TEXT,
    as_of_block BIGINT,
    source_reason TEXT NOT NULL DEFAULT 'latest',
    name TEXT,
    symbol TEXT,
    decimals SMALLINT NOT NULL,
    total_supply_raw TEXT NOT NULL,
    creation_block BIGINT,
    creation_timestamp BIGINT,
    creation_tx TEXT,
    creator_address TEXT,
    latest_activity_block BIGINT,
    latest_activity_timestamp BIGINT,
    latest_pool_activity_block BIGINT,
    lifecycle_status TEXT,
    is_scam BOOLEAN NOT NULL DEFAULT false,
    scam_label TEXT,
    scam_mechanism TEXT,
    scam_mechanism_label TEXT,
    pool_count BIGINT NOT NULL DEFAULT 0,
    active_pool_count BIGINT NOT NULL DEFAULT 0,
    retained_pool_count BIGINT NOT NULL DEFAULT 0,
    liquidity_removal_pool_count BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (scope_id, chain_id, token_address)
);

CREATE INDEX IF NOT EXISTS idx_token_latest_scope_activity
    ON token_state.token_latest (scope_id, latest_activity_block DESC NULLS LAST);

CREATE INDEX IF NOT EXISTS idx_token_latest_scope_pool_activity
    ON token_state.token_latest (scope_id, latest_pool_activity_block DESC NULLS LAST);

CREATE TABLE IF NOT EXISTS token_state.pool_latest (
    scope_id TEXT NOT NULL DEFAULT 'live',
    chain_id BIGINT NOT NULL DEFAULT 1,
    token_address TEXT NOT NULL,
    pool_id TEXT NOT NULL,
    source_run_id TEXT,
    as_of_block BIGINT,
    source_reason TEXT NOT NULL DEFAULT 'latest',
    protocol TEXT,
    denom_address TEXT NOT NULL,
    token_decimals SMALLINT NOT NULL,
    denom_decimals SMALLINT NOT NULL,
    creation_block BIGINT,
    creation_timestamp BIGINT,
    creation_tx TEXT,
    latest_activity_block BIGINT,
    latest_sync_block BIGINT,
    latest_reserve_block BIGINT,
    latest_swap_block BIGINT,
    latest_mint_block BIGINT,
    latest_burn_block BIGINT,
    token_reserve DOUBLE PRECISION,
    denom_reserve DOUBLE PRECISION,
    total_liquidity DOUBLE PRECISION,
    price_denom_per_token DOUBLE PRECISION,
    price_token_per_denom DOUBLE PRECISION,
    valuation_status TEXT NOT NULL,
    lifecycle_status TEXT,
    is_dust_pool BOOLEAN NOT NULL DEFAULT false,
    can_buy BOOLEAN NOT NULL DEFAULT false,
    can_sell BOOLEAN NOT NULL DEFAULT false,
    effective_can_buy BOOLEAN NOT NULL DEFAULT false,
    effective_can_sell BOOLEAN NOT NULL DEFAULT false,
    economic_sellable BOOLEAN,
    buy_tax DOUBLE PRECISION,
    sell_tax DOUBLE PRECISION,
    tax_check_block BIGINT,
    has_liquidity_removal BOOLEAN NOT NULL DEFAULT false,
    liquidity_removal_block BIGINT,
    liquidity_removal_tx TEXT,
    liquidity_removal_label TEXT,
    total_swaps BIGINT NOT NULL DEFAULT 0,
    total_mints BIGINT NOT NULL DEFAULT 0,
    total_burns BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (scope_id, chain_id, pool_id),
    FOREIGN KEY (scope_id, chain_id, token_address)
        REFERENCES token_state.token_latest(scope_id, chain_id, token_address)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pool_latest_scope_token
    ON token_state.pool_latest (scope_id, chain_id, token_address);

CREATE INDEX IF NOT EXISTS idx_pool_latest_scope_activity
    ON token_state.pool_latest (scope_id, latest_activity_block DESC NULLS LAST);

CREATE INDEX IF NOT EXISTS idx_pool_latest_scope_denom
    ON token_state.pool_latest (scope_id, denom_address);

CREATE INDEX IF NOT EXISTS idx_pool_latest_scope_valuation
    ON token_state.pool_latest (scope_id, valuation_status);
