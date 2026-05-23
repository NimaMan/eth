CREATE TABLE IF NOT EXISTS risk_atlas_runs (
    run_id TEXT PRIMARY KEY,
    chain TEXT NOT NULL DEFAULT 'ethereum',
    source_kind TEXT NOT NULL,
    source_ref TEXT,
    start_block BIGINT,
    end_block BIGINT,
    block_count BIGINT,
    token_count BIGINT,
    pool_count BIGINT,
    scam_label_count BIGINT,
    direct_lp_feature_row_count BIGINT,
    active_observation_row_count BIGINT,
    status TEXT NOT NULL DEFAULT 'draft',
    generated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE TABLE IF NOT EXISTS risk_atlas_distributions (
    run_id TEXT NOT NULL REFERENCES risk_atlas_runs(run_id) ON DELETE CASCADE,
    section TEXT NOT NULL,
    bucket TEXT NOT NULL,
    count BIGINT NOT NULL,
    share DOUBLE PRECISION,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (run_id, section, bucket)
);

CREATE INDEX IF NOT EXISTS idx_risk_atlas_distributions_section
    ON risk_atlas_distributions (run_id, section, sort_order);

CREATE TABLE IF NOT EXISTS risk_atlas_pool_eligibility (
    run_id TEXT NOT NULL REFERENCES risk_atlas_runs(run_id) ON DELETE CASCADE,
    token_address TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    protocol TEXT,
    quote_symbol TEXT,
    eligible BOOLEAN NOT NULL,
    eligibility_block BIGINT,
    eligibility_liquidity DOUBLE PRECISION,
    first_observed_block BIGINT,
    last_observed_block BIGINT,
    PRIMARY KEY (run_id, token_address, pool_address)
);

DROP INDEX IF EXISTS idx_risk_atlas_pool_eligibility_filter;

ALTER TABLE risk_atlas_pool_eligibility
    DROP COLUMN IF EXISTS eligibility_label,
    DROP COLUMN IF EXISTS min_liquidity,
    DROP COLUMN IF EXISTS current_category,
    DROP COLUMN IF EXISTS current_outcome,
    DROP COLUMN IF EXISTS metadata;

CREATE INDEX IF NOT EXISTS idx_risk_atlas_pool_eligibility_filter
    ON risk_atlas_pool_eligibility (run_id, eligible);

CREATE INDEX IF NOT EXISTS idx_risk_atlas_pool_eligibility_block
    ON risk_atlas_pool_eligibility (run_id, eligibility_block);

CREATE TABLE IF NOT EXISTS risk_atlas_event_evidence (
    run_id TEXT NOT NULL REFERENCES risk_atlas_runs(run_id) ON DELETE CASCADE,
    token_address TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    protocol TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    mechanism TEXT,
    block_number BIGINT,
    block_timestamp BIGINT,
    tx_hash TEXT,
    mempool_first_seen_ms BIGINT,
    source TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (run_id, sort_order)
);

CREATE INDEX IF NOT EXISTS idx_risk_atlas_event_evidence_kind
    ON risk_atlas_event_evidence (run_id, protocol, event_kind, block_number);

CREATE INDEX IF NOT EXISTS idx_risk_atlas_event_evidence_pool
    ON risk_atlas_event_evidence (run_id, token_address, pool_address, event_kind);

CREATE INDEX IF NOT EXISTS idx_risk_atlas_event_evidence_tx
    ON risk_atlas_event_evidence (run_id, tx_hash)
    WHERE tx_hash IS NOT NULL;

CREATE TABLE IF NOT EXISTS risk_atlas_observations (
    run_id TEXT NOT NULL REFERENCES risk_atlas_runs(run_id) ON DELETE CASCADE,
    token_address TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    denom_address TEXT NOT NULL,
    protocol TEXT NOT NULL,
    active_observation_index BIGINT NOT NULL,
    block_number BIGINT NOT NULL,
    timestamp BIGINT,
    active_reasons JSONB NOT NULL DEFAULT '[]'::jsonb,
    tx_count INTEGER NOT NULL DEFAULT 0,
    token_transfer_count INTEGER NOT NULL DEFAULT 0,
    denom_transfer_count INTEGER NOT NULL DEFAULT 0,
    buy_volume_denom DOUBLE PRECISION,
    sell_volume_denom DOUBLE PRECISION,
    total_bribe_eth DOUBLE PRECISION,
    can_buy BOOLEAN NOT NULL DEFAULT false,
    can_sell BOOLEAN NOT NULL DEFAULT false,
    effective_can_buy BOOLEAN NOT NULL DEFAULT false,
    effective_can_sell BOOLEAN NOT NULL DEFAULT false,
    buy_tax DOUBLE PRECISION,
    sell_tax DOUBLE PRECISION,
    liquidity_removed_as_of BOOLEAN NOT NULL DEFAULT false,
    liquidity_removal_in_block BOOLEAN NOT NULL DEFAULT false,
    liquidity_removal_block_as_of BIGINT,
    direct_lp_removal_as_of BOOLEAN NOT NULL DEFAULT false,
    direct_lp_removal_in_block BOOLEAN NOT NULL DEFAULT false,
    direct_lp_target_1 BOOLEAN,
    direct_lp_target_2 BOOLEAN,
    direct_lp_target_3 BOOLEAN,
    direct_lp_target_5 BOOLEAN,
    direct_lp_target_10 BOOLEAN,
    denom_reserve DOUBLE PRECISION,
    token_reserve DOUBLE PRECISION,
    total_liquidity_denom DOUBLE PRECISION,
    price_to_initial_ratio DOUBLE PRECISION,
    lp_approved_pct_as_of DOUBLE PRECISION,
    token_decimals INTEGER,
    price_denom_per_token DOUBLE PRECISION,
    initial_price_denom_per_token DOUBLE PRECISION,
    lp_approval_count_in_block INTEGER NOT NULL DEFAULT 0,
    lp_approval_seen_as_of BOOLEAN,
    lp_total_supply DOUBLE PRECISION,
    lp_max_approval_amount_as_of DOUBLE PRECISION,
    lp_max_approval_pct_as_of DOUBLE PRECISION,
    lp_removable_pct_as_of DOUBLE PRECISION,
    lp_router_removable_pct_as_of DOUBLE PRECISION,
    lp_approval_owner_is_creator BOOLEAN,
    creator_lp_balance_pct_as_of DOUBLE PRECISION,
    creator_lp_approved_pct_as_of DOUBLE PRECISION,
    creator_lp_removable_pct_as_of DOUBLE PRECISION,
    creator_lp_router_removable_pct_as_of DOUBLE PRECISION,
    creator_lp_approved_gt_90_pct_as_of BOOLEAN,
    creator_lp_router_removable_gt_90_pct_as_of BOOLEAN,
    last_lp_approval_amount_pct_of_total_supply DOUBLE PRECISION,
    first_lp_approval_to_as_of_chain_block_delta BIGINT,
    last_lp_approval_to_as_of_chain_block_delta BIGINT,
    pool_creation_to_first_lp_approval_chain_block_delta BIGINT,
    pool_creation_to_last_lp_approval_chain_block_delta BIGINT,
    trading_enabled_to_first_lp_approval_chain_block_delta BIGINT,
    trading_enabled_to_last_lp_approval_chain_block_delta BIGINT,
    control_transfer_from_after_renounce_seen_as_of BOOLEAN,
    control_transfer_from_after_renounce_in_block BOOLEAN,
    control_transfer_from_holder_to_burn_seen_as_of BOOLEAN,
    control_transfer_from_holder_to_burn_in_block BOOLEAN,
    control_transfer_from_pair_seen_as_of BOOLEAN,
    control_transfer_from_pair_in_block BOOLEAN,
    control_transfer_from_without_transfer_log_seen_as_of BOOLEAN,
    control_transfer_from_without_transfer_log_in_block BOOLEAN,
    pair_token_to_control_seen_as_of BOOLEAN,
    pair_token_to_control_in_block BOOLEAN,
    pair_token_to_control_to_pool_reserve_ratio DOUBLE PRECISION,
    pair_balance_backdoor_signal_seen_as_of BOOLEAN,
    pair_balance_backdoor_signal_in_block BOOLEAN,
    last_pair_balance_backdoor_signal_to_as_of_chain_block_delta BIGINT,
    token_transfer_to_total_supply_ratio DOUBLE PRECISION,
    token_transfer_to_pool_token_reserve_ratio DOUBLE PRECISION,
    observation JSONB NOT NULL,
    features JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (run_id, token_address, pool_address, active_observation_index)
);

ALTER TABLE risk_atlas_observations
    ADD COLUMN IF NOT EXISTS token_decimals INTEGER,
    ADD COLUMN IF NOT EXISTS price_denom_per_token DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS initial_price_denom_per_token DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS lp_approval_count_in_block INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS lp_approval_seen_as_of BOOLEAN,
    ADD COLUMN IF NOT EXISTS lp_total_supply DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS lp_max_approval_amount_as_of DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS lp_max_approval_pct_as_of DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS lp_removable_pct_as_of DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS lp_router_removable_pct_as_of DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS creator_lp_balance_pct_as_of DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS creator_lp_approved_pct_as_of DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS creator_lp_removable_pct_as_of DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS creator_lp_router_removable_pct_as_of DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS creator_lp_approved_gt_90_pct_as_of BOOLEAN,
    ADD COLUMN IF NOT EXISTS creator_lp_router_removable_gt_90_pct_as_of BOOLEAN,
    ADD COLUMN IF NOT EXISTS last_lp_approval_amount_pct_of_total_supply DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS first_lp_approval_to_as_of_chain_block_delta BIGINT,
    ADD COLUMN IF NOT EXISTS last_lp_approval_to_as_of_chain_block_delta BIGINT,
    ADD COLUMN IF NOT EXISTS pool_creation_to_first_lp_approval_chain_block_delta BIGINT,
    ADD COLUMN IF NOT EXISTS pool_creation_to_last_lp_approval_chain_block_delta BIGINT,
    ADD COLUMN IF NOT EXISTS trading_enabled_to_first_lp_approval_chain_block_delta BIGINT,
    ADD COLUMN IF NOT EXISTS trading_enabled_to_last_lp_approval_chain_block_delta BIGINT,
    ADD COLUMN IF NOT EXISTS lp_approval_owner_is_creator BOOLEAN,
    ADD COLUMN IF NOT EXISTS control_transfer_from_after_renounce_seen_as_of BOOLEAN,
    ADD COLUMN IF NOT EXISTS control_transfer_from_after_renounce_in_block BOOLEAN,
    ADD COLUMN IF NOT EXISTS control_transfer_from_holder_to_burn_seen_as_of BOOLEAN,
    ADD COLUMN IF NOT EXISTS control_transfer_from_holder_to_burn_in_block BOOLEAN,
    ADD COLUMN IF NOT EXISTS control_transfer_from_pair_seen_as_of BOOLEAN,
    ADD COLUMN IF NOT EXISTS control_transfer_from_pair_in_block BOOLEAN,
    ADD COLUMN IF NOT EXISTS control_transfer_from_without_transfer_log_seen_as_of BOOLEAN,
    ADD COLUMN IF NOT EXISTS control_transfer_from_without_transfer_log_in_block BOOLEAN,
    ADD COLUMN IF NOT EXISTS pair_token_to_control_seen_as_of BOOLEAN,
    ADD COLUMN IF NOT EXISTS pair_token_to_control_in_block BOOLEAN,
    ADD COLUMN IF NOT EXISTS pair_token_to_control_to_pool_reserve_ratio DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS pair_balance_backdoor_signal_seen_as_of BOOLEAN,
    ADD COLUMN IF NOT EXISTS pair_balance_backdoor_signal_in_block BOOLEAN,
    ADD COLUMN IF NOT EXISTS last_pair_balance_backdoor_signal_to_as_of_chain_block_delta BIGINT;

CREATE INDEX IF NOT EXISTS idx_risk_atlas_observations_pool_block
    ON risk_atlas_observations (run_id, token_address, pool_address, block_number);

CREATE INDEX IF NOT EXISTS idx_risk_atlas_observations_direct_lp_targets
    ON risk_atlas_observations (
        run_id,
        direct_lp_target_1,
        direct_lp_target_2,
        direct_lp_target_3,
        direct_lp_target_5,
        direct_lp_target_10
    );

CREATE TABLE IF NOT EXISTS risk_atlas_numeric_stats (
    run_id TEXT NOT NULL REFERENCES risk_atlas_runs(run_id) ON DELETE CASCADE,
    section TEXT NOT NULL,
    metric TEXT NOT NULL,
    count BIGINT NOT NULL,
    min DOUBLE PRECISION,
    p25 DOUBLE PRECISION,
    median DOUBLE PRECISION,
    p75 DOUBLE PRECISION,
    p90 DOUBLE PRECISION,
    p95 DOUBLE PRECISION,
    max DOUBLE PRECISION,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (run_id, section, metric)
);

CREATE TABLE IF NOT EXISTS risk_atlas_active_targets (
    run_id TEXT NOT NULL REFERENCES risk_atlas_runs(run_id) ON DELETE CASCADE,
    target_key TEXT NOT NULL,
    row_kind TEXT NOT NULL,
    horizon_active_observations INTEGER,
    rows BIGINT NOT NULL,
    unique_pools BIGINT,
    positives BIGINT,
    negatives BIGINT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (run_id, target_key)
);

CREATE TABLE IF NOT EXISTS risk_atlas_decision_questions (
    run_id TEXT NOT NULL REFERENCES risk_atlas_runs(run_id) ON DELETE CASCADE,
    question_id TEXT NOT NULL,
    category TEXT NOT NULL,
    question TEXT NOT NULL,
    headline TEXT,
    answer TEXT,
    status TEXT NOT NULL DEFAULT 'answered',
    denominator_label TEXT,
    denominator_count BIGINT,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (run_id, question_id)
);

CREATE INDEX IF NOT EXISTS idx_risk_atlas_decision_questions_order
    ON risk_atlas_decision_questions (run_id, sort_order);

CREATE TABLE IF NOT EXISTS risk_atlas_review_examples (
    run_id TEXT NOT NULL REFERENCES risk_atlas_runs(run_id) ON DELETE CASCADE,
    queue TEXT NOT NULL,
    token_address TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    protocol TEXT,
    symbol TEXT,
    mechanism TEXT,
    mechanism_label TEXT,
    label_block BIGINT,
    trading_enabled_block BIGINT,
    age_blocks BIGINT,
    liquidity_eth DOUBLE PRECISION,
    price_ratio_to_initial DOUBLE PRECISION,
    evidence_summary TEXT,
    evidence_ref TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (run_id, queue, token_address, pool_address)
);

CREATE INDEX IF NOT EXISTS idx_risk_atlas_review_examples_queue
    ON risk_atlas_review_examples (run_id, queue, sort_order);

CREATE TABLE IF NOT EXISTS risk_atlas_model_readiness (
    run_id TEXT NOT NULL REFERENCES risk_atlas_runs(run_id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    detail TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (run_id, name)
);

CREATE TABLE IF NOT EXISTS risk_atlas_page_snapshots (
    snapshot_id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES risk_atlas_runs(run_id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    summary JSONB NOT NULL DEFAULT '{}'::jsonb,
    sections JSONB NOT NULL DEFAULT '[]'::jsonb,
    review_queues JSONB NOT NULL DEFAULT '[]'::jsonb,
    model_readiness JSONB NOT NULL DEFAULT '[]'::jsonb
);

CREATE INDEX IF NOT EXISTS idx_risk_atlas_page_snapshots_run
    ON risk_atlas_page_snapshots (run_id, generated_at DESC);
