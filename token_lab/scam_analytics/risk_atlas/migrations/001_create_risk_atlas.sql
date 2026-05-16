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
