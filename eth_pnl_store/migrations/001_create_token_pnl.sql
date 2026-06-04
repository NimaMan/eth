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
    native_priority_fee_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    token_transfer_count BIGINT NOT NULL DEFAULT 0,
    denom_transfer_count BIGINT NOT NULL DEFAULT 0,
    token_creator_address TEXT,
    pool_creator_address TEXT,
    can_buy BOOLEAN NOT NULL DEFAULT false,
    can_sell BOOLEAN NOT NULL DEFAULT false,
    lifecycle TEXT,
    is_scam BOOLEAN NOT NULL DEFAULT false,
    scam_label TEXT,
    scam_mechanism TEXT,
    eligible BOOLEAN NOT NULL DEFAULT false,
    eligible_outcome TEXT,
    pool_labels JSONB NOT NULL DEFAULT '[]'::jsonb,
    pool_state_flags JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (run_id, pool_id)
);

ALTER TABLE token_pnl.pool_pnl_states
    ADD COLUMN IF NOT EXISTS pool_labels JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS pool_state_flags JSONB NOT NULL DEFAULT '{}'::jsonb;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_pnl_states'
          AND column_name = 'native_bribe_raw'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_pnl_states'
          AND column_name = 'native_priority_fee_raw'
    ) THEN
        ALTER TABLE token_pnl.pool_pnl_states
            RENAME COLUMN native_bribe_raw TO native_priority_fee_raw;
    ELSIF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_pnl_states'
          AND column_name = 'native_bribe_raw'
    ) AND EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_pnl_states'
          AND column_name = 'native_priority_fee_raw'
    ) THEN
        UPDATE token_pnl.pool_pnl_states
        SET native_priority_fee_raw = native_bribe_raw
        WHERE native_priority_fee_raw = 0 AND native_bribe_raw <> 0;

        ALTER TABLE token_pnl.pool_pnl_states
            DROP COLUMN native_bribe_raw;
    END IF;
END $$;

ALTER TABLE token_pnl.pool_pnl_states
    ADD COLUMN IF NOT EXISTS native_priority_fee_raw NUMERIC(78,0) NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS token_pnl.pool_address_pnl (
    run_id TEXT NOT NULL,
    pool_id TEXT NOT NULL,
    address TEXT NOT NULL,
    token_in_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    token_out_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    denom_in_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    denom_out_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    native_fee_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    native_priority_fee_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    token_balance_raw TEXT NOT NULL,
    denom_cashflow_raw TEXT NOT NULL,
    token_balance NUMERIC,
    denom_cashflow NUMERIC,
    native_fee NUMERIC,
    native_priority_fee NUMERIC,
    marked_token_value_denom NUMERIC,
    pnl_proxy_denom NUMERIC,
    position_status TEXT NOT NULL DEFAULT 'unknown',
    valuation_status TEXT NOT NULL DEFAULT 'unknown',
    reconciliation_status TEXT NOT NULL DEFAULT 'unknown',
    realized_pnl_denom NUMERIC,
    unrealized_value_denom NUMERIC,
    total_pnl_denom NUMERIC,
    movement_rows_retained BIGINT NOT NULL DEFAULT 0,
    movement_rows_backed BOOLEAN NOT NULL DEFAULT false,
    actor_roles JSONB NOT NULL DEFAULT '[]'::jsonb,
    is_user_candidate BOOLEAN NOT NULL DEFAULT false,
    accounting_context JSONB NOT NULL DEFAULT '{}'::jsonb,
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

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_address_pnl'
          AND column_name = 'native_bribe_raw'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_address_pnl'
          AND column_name = 'native_priority_fee_raw'
    ) THEN
        ALTER TABLE token_pnl.pool_address_pnl
            RENAME COLUMN native_bribe_raw TO native_priority_fee_raw;
    ELSIF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_address_pnl'
          AND column_name = 'native_bribe_raw'
    ) AND EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_address_pnl'
          AND column_name = 'native_priority_fee_raw'
    ) THEN
        UPDATE token_pnl.pool_address_pnl
        SET native_priority_fee_raw = native_bribe_raw
        WHERE native_priority_fee_raw = 0 AND native_bribe_raw <> 0;

        ALTER TABLE token_pnl.pool_address_pnl
            DROP COLUMN native_bribe_raw;
    END IF;

    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_address_pnl'
          AND column_name = 'native_bribe'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_address_pnl'
          AND column_name = 'native_priority_fee'
    ) THEN
        ALTER TABLE token_pnl.pool_address_pnl
            RENAME COLUMN native_bribe TO native_priority_fee;
    ELSIF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_address_pnl'
          AND column_name = 'native_bribe'
    ) AND EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_address_pnl'
          AND column_name = 'native_priority_fee'
    ) THEN
        UPDATE token_pnl.pool_address_pnl
        SET native_priority_fee = native_bribe
        WHERE native_priority_fee IS NULL AND native_bribe IS NOT NULL;

        ALTER TABLE token_pnl.pool_address_pnl
            DROP COLUMN native_bribe;
    END IF;
END $$;

ALTER TABLE token_pnl.pool_address_pnl
    ADD COLUMN IF NOT EXISTS native_priority_fee_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS native_priority_fee NUMERIC;

ALTER TABLE token_pnl.pool_address_pnl
    ADD COLUMN IF NOT EXISTS position_status TEXT NOT NULL DEFAULT 'unknown',
    ADD COLUMN IF NOT EXISTS valuation_status TEXT NOT NULL DEFAULT 'unknown',
    ADD COLUMN IF NOT EXISTS reconciliation_status TEXT NOT NULL DEFAULT 'unknown',
    ADD COLUMN IF NOT EXISTS realized_pnl_denom NUMERIC,
    ADD COLUMN IF NOT EXISTS unrealized_value_denom NUMERIC,
    ADD COLUMN IF NOT EXISTS total_pnl_denom NUMERIC,
    ADD COLUMN IF NOT EXISTS movement_rows_retained BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS movement_rows_backed BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS actor_roles JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS is_user_candidate BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS accounting_context JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE INDEX IF NOT EXISTS idx_pool_address_pnl_position_status
    ON token_pnl.pool_address_pnl (run_id, position_status, valuation_status);

CREATE INDEX IF NOT EXISTS idx_pool_address_pnl_actor_roles
    ON token_pnl.pool_address_pnl USING GIN (actor_roles);

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
    native_priority_fee_raw NUMERIC(78,0) NOT NULL DEFAULT 0,
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

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_pnl_movements'
          AND column_name = 'native_bribe_raw'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_pnl_movements'
          AND column_name = 'native_priority_fee_raw'
    ) THEN
        ALTER TABLE token_pnl.pool_pnl_movements
            RENAME COLUMN native_bribe_raw TO native_priority_fee_raw;
    ELSIF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_pnl_movements'
          AND column_name = 'native_bribe_raw'
    ) AND EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'token_pnl'
          AND table_name = 'pool_pnl_movements'
          AND column_name = 'native_priority_fee_raw'
    ) THEN
        UPDATE token_pnl.pool_pnl_movements
        SET native_priority_fee_raw = native_bribe_raw
        WHERE native_priority_fee_raw = 0 AND native_bribe_raw <> 0;

        ALTER TABLE token_pnl.pool_pnl_movements
            DROP COLUMN native_bribe_raw;
    END IF;
END $$;

ALTER TABLE token_pnl.pool_pnl_movements
    ADD COLUMN IF NOT EXISTS native_priority_fee_raw NUMERIC(78,0) NOT NULL DEFAULT 0;
