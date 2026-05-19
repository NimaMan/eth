pub(crate) const MIGRATIONS: &[&str] = &[
    "CREATE SCHEMA IF NOT EXISTS alpha_trading",
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.trader_runs (
        run_id TEXT PRIMARY KEY,
        mode TEXT NOT NULL,
        status TEXT NOT NULL,
        config JSONB NOT NULL DEFAULT '{}'::jsonb,
        metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
        started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        last_heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        stopped_at TIMESTAMPTZ
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.order_intents (
        id BIGSERIAL PRIMARY KEY,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        trade_id TEXT,
        portfolio_id TEXT NOT NULL,
        wallet_id TEXT NOT NULL,
        strategy_name TEXT NOT NULL,
        side TEXT NOT NULL,
        token_address TEXT NOT NULL,
        pool_address TEXT NOT NULL,
        protocol TEXT,
        amount_raw TEXT NOT NULL,
        amount_decimals SMALLINT NOT NULL,
        max_slippage_bps INTEGER NOT NULL,
        deadline_secs BIGINT NOT NULL,
        reason_code TEXT,
        reason_category TEXT,
        reason_label TEXT,
        reason_source TEXT,
        reason_details JSONB,
        payload JSONB NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    "ALTER TABLE alpha_trading.order_intents ADD COLUMN IF NOT EXISTS trade_id TEXT",
    "ALTER TABLE alpha_trading.order_intents ADD COLUMN IF NOT EXISTS protocol TEXT",
    "ALTER TABLE alpha_trading.order_intents ALTER COLUMN protocol SET DEFAULT 'unknown'",
    "ALTER TABLE alpha_trading.order_intents ADD COLUMN IF NOT EXISTS reason_code TEXT",
    "ALTER TABLE alpha_trading.order_intents ADD COLUMN IF NOT EXISTS reason_category TEXT",
    "ALTER TABLE alpha_trading.order_intents ADD COLUMN IF NOT EXISTS reason_label TEXT",
    "ALTER TABLE alpha_trading.order_intents ADD COLUMN IF NOT EXISTS reason_source TEXT",
    "ALTER TABLE alpha_trading.order_intents ADD COLUMN IF NOT EXISTS reason_details JSONB",
    r#"
    CREATE INDEX IF NOT EXISTS order_intents_run_created_idx
    ON alpha_trading.order_intents (run_id, created_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS order_intents_reason_code_idx
    ON alpha_trading.order_intents (run_id, reason_code, created_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS order_intents_token_created_idx
    ON alpha_trading.order_intents (token_address, created_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.execution_reports (
        id BIGSERIAL PRIMARY KEY,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        position_id TEXT,
        trade_id TEXT,
        order_side TEXT,
        order_id TEXT NOT NULL,
        status TEXT NOT NULL,
        tx_hash TEXT,
        block_number BIGINT,
        filled_amount_raw TEXT,
        filled_amount_decimals SMALLINT,
        gas_used BIGINT,
        error TEXT,
        payload JSONB NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    "ALTER TABLE alpha_trading.execution_reports ADD COLUMN IF NOT EXISTS position_id TEXT",
    "ALTER TABLE alpha_trading.execution_reports ADD COLUMN IF NOT EXISTS trade_id TEXT",
    "ALTER TABLE alpha_trading.execution_reports ADD COLUMN IF NOT EXISTS order_side TEXT",
    r#"
    CREATE INDEX IF NOT EXISTS execution_reports_run_created_idx
    ON alpha_trading.execution_reports (run_id, created_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS execution_reports_position_created_idx
    ON alpha_trading.execution_reports (run_id, position_id, created_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS execution_reports_order_created_idx
    ON alpha_trading.execution_reports (order_id, created_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.positions (
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        position_id TEXT NOT NULL,
        trade_id TEXT,
        portfolio_id TEXT NOT NULL,
        wallet_id TEXT NOT NULL,
        strategy_name TEXT NOT NULL,
        token_address TEXT NOT NULL,
        pool_address TEXT NOT NULL,
        protocol TEXT,
        state TEXT NOT NULL,
        entry_order_id TEXT,
        exit_order_id TEXT,
        entry_block BIGINT,
        exit_block BIGINT,
        payload JSONB NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        PRIMARY KEY (run_id, position_id)
    )
    "#,
    "ALTER TABLE alpha_trading.positions ADD COLUMN IF NOT EXISTS trade_id TEXT",
    "ALTER TABLE alpha_trading.positions ADD COLUMN IF NOT EXISTS protocol TEXT",
    "ALTER TABLE alpha_trading.positions ALTER COLUMN protocol SET DEFAULT 'unknown'",
    "ALTER TABLE alpha_trading.positions ADD COLUMN IF NOT EXISTS entry_block BIGINT",
    "ALTER TABLE alpha_trading.positions ADD COLUMN IF NOT EXISTS exit_block BIGINT",
    "UPDATE alpha_trading.positions SET trade_id = position_id WHERE trade_id IS NULL",
    r#"
    CREATE INDEX IF NOT EXISTS positions_run_state_idx
    ON alpha_trading.positions (run_id, state, updated_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS positions_token_idx
    ON alpha_trading.positions (token_address, updated_at DESC)
    "#,
    r#"
    CREATE UNIQUE INDEX IF NOT EXISTS positions_run_trade_idx
    ON alpha_trading.positions (run_id, trade_id)
    WHERE trade_id IS NOT NULL
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.position_snapshots (
        id BIGSERIAL PRIMARY KEY,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        position_id TEXT NOT NULL,
        trade_id TEXT,
        state TEXT NOT NULL,
        block_number BIGINT NOT NULL,
        observed_block_number BIGINT,
        valuation_block_number BIGINT,
        current_value_eth TEXT NOT NULL,
        realized_profit_eth TEXT NOT NULL,
        unrealized_profit_eth TEXT NOT NULL,
        roi TEXT NOT NULL,
        payload JSONB NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    "ALTER TABLE alpha_trading.position_snapshots ADD COLUMN IF NOT EXISTS trade_id TEXT",
    "ALTER TABLE alpha_trading.position_snapshots ADD COLUMN IF NOT EXISTS observed_block_number BIGINT",
    "ALTER TABLE alpha_trading.position_snapshots ADD COLUMN IF NOT EXISTS valuation_block_number BIGINT",
    "UPDATE alpha_trading.position_snapshots SET trade_id = position_id WHERE trade_id IS NULL",
    r#"
    CREATE INDEX IF NOT EXISTS position_snapshots_position_block_idx
    ON alpha_trading.position_snapshots (run_id, position_id, block_number DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.backtest_result_sets (
        result_set_id TEXT PRIMARY KEY,
        mode TEXT NOT NULL,
        status TEXT NOT NULL,
        strategy_suite TEXT,
        start_block BIGINT,
        end_block BIGINT,
        config JSONB NOT NULL DEFAULT '{}'::jsonb,
        metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        stopped_at TIMESTAMPTZ
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.backtest_result_set_runs (
        result_set_id TEXT NOT NULL REFERENCES alpha_trading.backtest_result_sets(result_set_id) ON DELETE CASCADE,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        PRIMARY KEY (result_set_id, run_id)
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.trades (
        trade_id TEXT PRIMARY KEY,
        result_set_id TEXT NOT NULL REFERENCES alpha_trading.backtest_result_sets(result_set_id) ON DELETE CASCADE,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        position_id TEXT,
        strategy_name TEXT NOT NULL,
        token_address TEXT NOT NULL,
        pool_address TEXT NOT NULL,
        protocol TEXT,
        state TEXT NOT NULL,
        entry_order_id TEXT,
        exit_order_id TEXT,
        entry_block BIGINT,
        exit_block BIGINT,
        latest_snapshot_block BIGINT,
        latest_observed_block BIGINT,
        latest_valuation_block BIGINT,
        entry_cost_eth TEXT,
        exit_value_eth TEXT,
        current_value_eth TEXT,
        realized_pnl_eth TEXT,
        unrealized_pnl_eth TEXT,
        total_pnl_eth TEXT,
        gas_cost_eth TEXT,
        roi TEXT,
        payload JSONB NOT NULL DEFAULT '{}'::jsonb,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    "ALTER TABLE alpha_trading.trades ADD COLUMN IF NOT EXISTS protocol TEXT",
    "ALTER TABLE alpha_trading.trades ALTER COLUMN protocol SET DEFAULT 'unknown'",
    r#"
    CREATE INDEX IF NOT EXISTS trades_result_strategy_idx
    ON alpha_trading.trades (result_set_id, strategy_name, updated_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS trades_result_strategy_trade_idx
    ON alpha_trading.trades (result_set_id, strategy_name, trade_id)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS trades_token_pool_idx
    ON alpha_trading.trades (token_address, pool_address, updated_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS trades_result_protocol_idx
    ON alpha_trading.trades (result_set_id, protocol, updated_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.trade_events (
        id BIGSERIAL PRIMARY KEY,
        trade_id TEXT NOT NULL REFERENCES alpha_trading.trades(trade_id) ON DELETE CASCADE,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        event_type TEXT NOT NULL,
        order_side TEXT NOT NULL,
        status TEXT NOT NULL,
        order_id TEXT NOT NULL,
        tx_hash TEXT,
        block_number BIGINT,
        filled_amount_raw TEXT,
        filled_amount_decimals SMALLINT,
        gas_used BIGINT,
        gas_cost_eth TEXT,
        error TEXT,
        payload JSONB NOT NULL DEFAULT '{}'::jsonb,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS trade_events_trade_created_idx
    ON alpha_trading.trade_events (trade_id, created_at ASC, id ASC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.trade_snapshots (
        id BIGSERIAL PRIMARY KEY,
        trade_id TEXT NOT NULL REFERENCES alpha_trading.trades(trade_id) ON DELETE CASCADE,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        position_id TEXT,
        state TEXT NOT NULL,
        block_number BIGINT NOT NULL,
        observed_block_number BIGINT,
        valuation_block_number BIGINT,
        current_value_eth TEXT NOT NULL,
        realized_pnl_eth TEXT NOT NULL,
        unrealized_pnl_eth TEXT NOT NULL,
        total_pnl_eth TEXT NOT NULL,
        roi TEXT NOT NULL,
        pool_price_to_initial_price_ratio TEXT,
        pool_initial_price_denom_per_token TEXT,
        pool_price_denom_per_token TEXT,
        pool_liquidity_denom TEXT,
        pool_token_reserve TEXT,
        pool_denom_symbol TEXT,
        payload JSONB NOT NULL DEFAULT '{}'::jsonb,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS trade_snapshots_trade_block_idx
    ON alpha_trading.trade_snapshots (trade_id, block_number DESC, id DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS trade_snapshots_trade_effective_block_idx
    ON alpha_trading.trade_snapshots (
        trade_id,
        (COALESCE(valuation_block_number, observed_block_number, block_number)) DESC,
        id DESC
    )
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.risk_events (
        id BIGSERIAL PRIMARY KEY,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        kind TEXT NOT NULL,
        severity TEXT NOT NULL,
        token_address TEXT NOT NULL,
        pool_address TEXT,
        pending_tx_hash TEXT,
        observed_block BIGINT,
        message TEXT NOT NULL,
        payload JSONB NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS risk_events_run_created_idx
    ON alpha_trading.risk_events (run_id, created_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS risk_events_token_created_idx
    ON alpha_trading.risk_events (token_address, created_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.strategy_decisions (
        id BIGSERIAL PRIMARY KEY,
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        strategy_name TEXT NOT NULL,
        event_source TEXT NOT NULL,
        event_key TEXT NOT NULL,
        block_number BIGINT,
        token_address TEXT,
        pool_address TEXT,
        action TEXT NOT NULL,
        reason TEXT,
        reason_code TEXT,
        reason_category TEXT,
        reason_label TEXT,
        reason_source TEXT,
        reason_details JSONB,
        order_side TEXT,
        payload JSONB NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    )
    "#,
    "ALTER TABLE alpha_trading.strategy_decisions ADD COLUMN IF NOT EXISTS reason_code TEXT",
    "ALTER TABLE alpha_trading.strategy_decisions ADD COLUMN IF NOT EXISTS reason_category TEXT",
    "ALTER TABLE alpha_trading.strategy_decisions ADD COLUMN IF NOT EXISTS reason_label TEXT",
    "ALTER TABLE alpha_trading.strategy_decisions ADD COLUMN IF NOT EXISTS reason_source TEXT",
    "ALTER TABLE alpha_trading.strategy_decisions ADD COLUMN IF NOT EXISTS reason_details JSONB",
    r#"
    CREATE INDEX IF NOT EXISTS strategy_decisions_run_event_idx
    ON alpha_trading.strategy_decisions (run_id, event_source, block_number, id)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS strategy_decisions_reason_code_idx
    ON alpha_trading.strategy_decisions (run_id, reason_code, block_number DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS strategy_decisions_token_idx
    ON alpha_trading.strategy_decisions (token_address, created_at DESC)
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS alpha_trading.strategy_observations (
        run_id TEXT NOT NULL REFERENCES alpha_trading.trader_runs(run_id) ON DELETE CASCADE,
        strategy_name TEXT NOT NULL,
        event_source TEXT NOT NULL,
        event_key TEXT NOT NULL,
        token_address TEXT,
        pool_address TEXT,
        block_number BIGINT,
        event_timestamp TEXT,
        decision TEXT NOT NULL,
        report_count INTEGER NOT NULL DEFAULT 0,
        payload JSONB NOT NULL,
        first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        PRIMARY KEY (run_id, strategy_name, event_source, event_key)
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS strategy_observations_run_seen_idx
    ON alpha_trading.strategy_observations (run_id, strategy_name, last_seen_at DESC)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS strategy_observations_token_idx
    ON alpha_trading.strategy_observations (token_address, last_seen_at DESC)
    "#,
];
