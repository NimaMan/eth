CREATE SCHEMA IF NOT EXISTS token_pnl;

CREATE TABLE IF NOT EXISTS token_pnl.calculation_runs (
    run_id TEXT PRIMARY KEY,
    chain_id BIGINT NOT NULL DEFAULT 1,
    mode TEXT NOT NULL,
    algorithm_version TEXT NOT NULL,
    start_block BIGINT,
    end_block BIGINT,
    include_traces BOOLEAN NOT NULL DEFAULT true,
    status TEXT NOT NULL DEFAULT 'running',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE IF NOT EXISTS token_pnl.pool_pnl_states (
    run_id TEXT NOT NULL REFERENCES token_pnl.calculation_runs(run_id) ON DELETE CASCADE,
    pool_id TEXT NOT NULL,
    token_address TEXT NOT NULL,
    denom_address TEXT NOT NULL,
    protocol TEXT,
    token_decimals SMALLINT NOT NULL,
    denom_decimals SMALLINT NOT NULL,
    tx_count BIGINT NOT NULL DEFAULT 0,
    latest_block BIGINT,
    latest_timestamp BIGINT,
    token_in_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    token_out_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    denom_in_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    denom_out_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    pool_token_in_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    pool_token_out_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    pool_denom_in_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    pool_denom_out_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    native_fee_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    native_bribe_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    token_transfer_count BIGINT NOT NULL DEFAULT 0,
    denom_transfer_count BIGINT NOT NULL DEFAULT 0,
    token_creator_address TEXT,
    pool_creator_address TEXT,
    can_buy BOOLEAN NOT NULL DEFAULT false,
    can_sell BOOLEAN NOT NULL DEFAULT false,
    lifecycle TEXT,
    is_scam BOOLEAN NOT NULL DEFAULT false,
    scam_label TEXT,
    eligible BOOLEAN NOT NULL DEFAULT false,
    eligible_outcome TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (run_id, pool_id)
);

CREATE TABLE IF NOT EXISTS token_pnl.pool_address_pnl (
    run_id TEXT NOT NULL,
    pool_id TEXT NOT NULL,
    address TEXT NOT NULL,
    token_in_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    token_out_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    denom_in_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    denom_out_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    native_fee_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    native_bribe_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    token_balance_raw TEXT NOT NULL,
    denom_cashflow_raw TEXT NOT NULL,
    token_balance NUMERIC,
    denom_cashflow NUMERIC,
    native_fee NUMERIC,
    native_bribe NUMERIC,
    marked_token_value_denom NUMERIC,
    pnl_proxy_denom NUMERIC,
    first_block BIGINT,
    latest_block BIGINT,
    movement_count BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (run_id, pool_id, address),
    FOREIGN KEY (run_id, pool_id)
        REFERENCES token_pnl.pool_pnl_states(run_id, pool_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pool_address_pnl_pool_latest
    ON token_pnl.pool_address_pnl (run_id, pool_id, latest_block);

CREATE INDEX IF NOT EXISTS idx_pool_address_pnl_pool_denom_cashflow
    ON token_pnl.pool_address_pnl (run_id, pool_id, denom_cashflow);

CREATE TABLE IF NOT EXISTS token_pnl.pool_pnl_movements (
    run_id TEXT NOT NULL,
    pool_id TEXT NOT NULL,
    entry_index BIGINT NOT NULL,
    tx_hash TEXT NOT NULL,
    block_number BIGINT NOT NULL,
    block_timestamp BIGINT NOT NULL,
    tx_index BIGINT NOT NULL,
    log_index BIGINT,
    address TEXT NOT NULL,
    kind TEXT NOT NULL,
    token_in_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    token_out_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    denom_in_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    denom_out_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    native_fee_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    native_bribe_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    pool_direct BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (run_id, pool_id, entry_index),
    FOREIGN KEY (run_id, pool_id)
        REFERENCES token_pnl.pool_pnl_states(run_id, pool_id)
        ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_pool_pnl_movements_pool_block
    ON token_pnl.pool_pnl_movements (run_id, pool_id, block_number, tx_index);

CREATE INDEX IF NOT EXISTS idx_pool_pnl_movements_address
    ON token_pnl.pool_pnl_movements (run_id, pool_id, address);

CREATE INDEX IF NOT EXISTS idx_pool_pnl_movements_tx_hash
    ON token_pnl.pool_pnl_movements (tx_hash);
