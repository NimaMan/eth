CREATE SCHEMA IF NOT EXISTS eth_tx_execution;

CREATE TABLE IF NOT EXISTS eth_tx_execution.eth_tx_policy_daily_spend (
    spend_day DATE PRIMARY KEY,
    spent_wei TEXT NOT NULL DEFAULT '0',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS eth_tx_execution.eth_tx_spend_reservations (
    attempt_id TEXT PRIMARY KEY,
    spend_day DATE NOT NULL REFERENCES eth_tx_execution.eth_tx_policy_daily_spend(spend_day) ON DELETE CASCADE,
    cost_wei TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS eth_tx_execution.eth_tx_policy_decisions (
    id BIGSERIAL PRIMARY KEY,
    attempt_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    route TEXT NOT NULL DEFAULT '/eth/tx/direct-raw',
    decision TEXT NOT NULL,
    policy_version TEXT NOT NULL,
    reasons_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    request_json JSONB NOT NULL,
    normalized_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata_json JSONB NOT NULL DEFAULT '{}'::jsonb,
    strategy_name TEXT,
    strategy_run_id TEXT,
    trade_id TEXT,
    token_address TEXT,
    pool_address TEXT,
    from_address TEXT,
    to_address TEXT,
    selector TEXT,
    value_wei TEXT,
    gas_limit TEXT,
    max_fee_per_gas_wei TEXT,
    max_priority_fee_per_gas_wei TEXT,
    estimated_worst_case_cost_wei TEXT,
    simulation_block_number BIGINT,
    latest_block_number BIGINT
);

ALTER TABLE eth_tx_execution.eth_tx_policy_decisions
    ADD COLUMN IF NOT EXISTS route TEXT NOT NULL DEFAULT '/eth/tx/direct-raw';

CREATE INDEX IF NOT EXISTS idx_eth_tx_policy_decisions_attempt
    ON eth_tx_execution.eth_tx_policy_decisions(attempt_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_eth_tx_policy_decisions_created
    ON eth_tx_execution.eth_tx_policy_decisions(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_eth_tx_policy_decisions_decision
    ON eth_tx_execution.eth_tx_policy_decisions(decision, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_eth_tx_policy_decisions_strategy
    ON eth_tx_execution.eth_tx_policy_decisions(strategy_name, strategy_run_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_eth_tx_policy_decisions_token
    ON eth_tx_execution.eth_tx_policy_decisions(token_address, created_at DESC);

