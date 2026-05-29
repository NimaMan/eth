use eyre::Result;
use serde_json::json;
use sqlx::{PgPool, Row};

use super::super::db::ResultSetRecord;
use super::super::report::{CheckResult, Verdict};
use super::common::{check, count_check};

pub(super) async fn execution_delay_check(
    pool: &PgPool,
    result_set: &ResultSetRecord,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "terminal_report_matches_execution_delay",
        Verdict::Fail,
        "terminal execution reports land exactly submitted_block + configured execution_delay_blocks",
        "orders whose terminal report block does not match configured execution delay",
        r#"
        WITH cfg AS (
            SELECT COALESCE(NULLIF(config->>'execution_delay_blocks', '')::bigint, 1) AS delay_blocks
            FROM alpha_trading.backtest_result_sets
            WHERE result_set_id = $1
        ),
        order_events AS (
            SELECT t.trade_id,
                   te.order_id,
                   te.order_side,
                   MIN(te.block_number) FILTER (WHERE te.status = 'submitted') AS submitted_block,
                   MIN(te.block_number) FILTER (WHERE te.status IN ('confirmed', 'failed', 'cancelled')) AS terminal_block
            FROM alpha_trading.trades t
            JOIN alpha_trading.trade_events te ON te.trade_id = t.trade_id
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
            GROUP BY t.trade_id, te.order_id, te.order_side
        )
        SELECT count(*)
        FROM order_events, cfg
        WHERE submitted_block IS NOT NULL
          AND terminal_block IS NOT NULL
          AND terminal_block <> submitted_block + cfg.delay_blocks
        "#,
        &result_set.result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn live_chain_sim_block_alignment_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "live_chain_sim_execution_blocks_align",
        Verdict::Fail,
        "live chain-sim terminal reports use the expected simulation block with no stale state",
        "live chain-sim terminal reports whose decision, expected, simulation, receipt, or event blocks do not align",
        r#"
        WITH cfg AS (
            SELECT COALESCE(NULLIF(config->>'execution_delay_blocks', '')::bigint, 1) AS delay_blocks
            FROM alpha_trading.backtest_result_sets
            WHERE result_set_id = $1
        ),
        terminal_events AS (
            SELECT te.id,
                   te.trade_id,
                   te.order_id,
                   te.order_side,
                   te.block_number AS event_block,
                   NULLIF(te.payload#>>'{mined_evidence,submitted_block_number}', '')::bigint AS decision_block,
                   NULLIF(te.payload#>>'{mined_evidence,expected_confirmation_block}', '')::bigint AS expected_block,
                   NULLIF(te.payload#>>'{mined_evidence,simulation_block_number}', '')::bigint AS simulation_block,
                   NULLIF(te.payload#>>'{mined_evidence,receipt_block_number}', '')::bigint AS receipt_block,
                   NULLIF(te.payload#>>'{mined_evidence,receipt_status}', '') AS receipt_status,
                   (
                       SELECT MIN(sub.block_number)
                       FROM alpha_trading.trade_events sub
                       WHERE sub.trade_id = te.trade_id
                         AND sub.order_id = te.order_id
                         AND sub.order_side = te.order_side
                         AND sub.status = 'submitted'
                   ) AS submitted_event_block
            FROM alpha_trading.trade_events te
            JOIN alpha_trading.trades t ON t.trade_id = te.trade_id
            JOIN alpha_trading.trader_runs tr ON tr.run_id = te.run_id
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
              AND tr.mode = 'chain-sim'
              AND te.status IN ('confirmed', 'failed', 'cancelled')
        )
        SELECT count(*)
        FROM terminal_events, cfg
        WHERE decision_block IS NULL
           OR expected_block IS NULL
           OR simulation_block IS NULL
           OR receipt_block IS NULL
           OR receipt_status IS NULL
           OR event_block IS NULL
           OR submitted_event_block IS NULL
           OR receipt_status <> 'live_backtest_chain_sim'
           OR submitted_event_block <> decision_block
           OR expected_block <> decision_block + cfg.delay_blocks
           OR simulation_block <> expected_block
           OR receipt_block <> simulation_block
           OR event_block <> simulation_block
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn live_chain_sim_block_hash_evidence_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "live_chain_sim_block_hash_evidence",
        Verdict::Fail,
        "live chain-sim execution events retain exact block hashes",
        "live chain-sim execution events missing valid mined-evidence block hash",
        r#"
        SELECT count(*)
        FROM alpha_trading.trade_events te
        JOIN alpha_trading.trades t ON t.trade_id = te.trade_id
        JOIN alpha_trading.trader_runs tr ON tr.run_id = te.run_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND tr.mode = 'chain-sim'
          AND te.status IN ('submitted', 'confirmed', 'failed', 'cancelled')
          AND COALESCE(te.payload#>>'{mined_evidence,receipt_status}', '') IN (
              'live_backtest_chain_sim_submitted',
              'live_backtest_chain_sim'
          )
          AND COALESCE(te.payload#>>'{mined_evidence,block_hash}', '') !~ '^0x[0-9a-fA-F]{64}$'
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn chain_sim_real_execution_artifacts_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "chain_sim_has_no_real_execution_artifacts",
        Verdict::Fail,
        "chain-sim backtests contain no real tx hashes or Kartal submission artifacts",
        "chain-sim backtest execution rows with real tx/Kartal artifacts",
        r#"
        WITH chain_sim_trades AS (
            SELECT t.trade_id, t.run_id
            FROM alpha_trading.trades t
            JOIN alpha_trading.trader_runs tr ON tr.run_id = t.run_id
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
              AND tr.mode = 'chain-sim'
        ),
        violations AS (
            SELECT 1
            FROM alpha_trading.trade_events te
            JOIN chain_sim_trades t
              ON t.trade_id = te.trade_id
             AND t.run_id = te.run_id
            WHERE NULLIF(te.tx_hash, '') IS NOT NULL
               OR NULLIF(te.payload->>'tx_hash', '') IS NOT NULL
               OR lower(COALESCE(te.error, '')) LIKE '%kartal%'
            UNION ALL
            SELECT 1
            FROM alpha_trading.execution_reports er
            JOIN chain_sim_trades t
              ON t.trade_id = er.trade_id
             AND t.run_id = er.run_id
            WHERE NULLIF(er.tx_hash, '') IS NOT NULL
               OR NULLIF(er.payload->>'tx_hash', '') IS NOT NULL
               OR lower(COALESCE(er.error, '')) LIKE '%kartal%'
        )
        SELECT count(*)
        FROM violations
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn pre_submit_simulation_state_ready_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "pre_submit_simulation_state_ready",
        Verdict::Fail,
        "pre-submit simulation had the exact required state for every attempted order",
        "execution reports deferred because pre-submit simulation state lagged behind the decision block",
        r#"
        SELECT count(*)
        FROM alpha_trading.execution_reports er
        JOIN alpha_trading.trades t
          ON t.run_id = er.run_id
         AND t.trade_id = er.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND er.status = 'deferred'
          AND er.error LIKE 'pre-submit simulation state not ready:%'
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn terminal_report_presence_check(
    pool: &PgPool,
    result_set: &ResultSetRecord,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "submitted_orders_have_terminal_report_after_delay",
        Verdict::Fail,
        "submitted orders have a terminal report once the execution delay has elapsed",
        "submitted orders past execution delay without terminal report",
        r#"
        WITH cfg AS (
            SELECT COALESCE(NULLIF(metadata->>'live_current_block', '')::bigint, end_block) AS current_block,
                   COALESCE(NULLIF(config->>'execution_delay_blocks', '')::bigint, 1) AS delay_blocks
            FROM alpha_trading.backtest_result_sets
            WHERE result_set_id = $1
        ),
        order_events AS (
            SELECT t.trade_id,
                   te.order_id,
                   te.order_side,
                   MIN(te.block_number) FILTER (WHERE te.status = 'submitted') AS submitted_block,
                   MIN(te.block_number) FILTER (WHERE te.status IN ('confirmed', 'failed', 'cancelled')) AS terminal_block
            FROM alpha_trading.trades t
            JOIN alpha_trading.trade_events te ON te.trade_id = t.trade_id
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
            GROUP BY t.trade_id, te.order_id, te.order_side
        )
        SELECT count(*)
        FROM order_events, cfg
        WHERE submitted_block IS NOT NULL
          AND terminal_block IS NULL
          AND cfg.current_block IS NOT NULL
          AND cfg.current_block >= submitted_block + cfg.delay_blocks
        "#,
        &result_set.result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn terminal_gas_policy_fee_evidence_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "terminal_reports_have_gas_policy_fee_evidence",
        Verdict::Fail,
        "terminal execution reports retain gas-policy fee or rejection evidence",
        "terminal execution reports missing selected fee evidence or rejection guard evidence",
        r#"
        SELECT count(*)
        FROM alpha_trading.trade_events te
        JOIN alpha_trading.trades t ON t.trade_id = te.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND te.status IN ('confirmed', 'failed', 'cancelled')
          AND te.gas_policy_action IS NOT NULL
          AND (
              NULLIF(te.gas_policy_status, '') IS NULL
              OR (
                  te.gas_policy_status = 'selected'
                  AND (
                      NULLIF(te.payload#>>'{mined_evidence,selected_max_fee_per_gas_wei}', '') IS NULL
                      OR NULLIF(te.payload#>>'{mined_evidence,selected_max_priority_fee_per_gas_wei}', '') IS NULL
                      OR NULLIF(te.gas_policy_profile, '') IS NULL
                      OR NULLIF(te.gas_rank_source, '') IS NULL
                  )
              )
              OR (
                  te.gas_policy_status LIKE 'rejected:%'
                  AND (
                      NULLIF(te.gas_policy_guard, '') IS NULL
                      OR COALESCE(jsonb_array_length(te.gas_policy_profiles), 0) = 0
                  )
              )
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn confirmed_reports_have_simulated_outputs_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "confirmed_reports_have_simulation_outputs",
        Verdict::Fail,
        "confirmed execution reports retain EVM simulation outputs, gas, and buy token amount",
        "confirmed reports missing filled amount, gas, or buy token amount",
        r#"
        SELECT count(*)
        FROM alpha_trading.trade_events te
        JOIN alpha_trading.trades t ON t.trade_id = te.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND te.status = 'confirmed'
          AND (
              nullif(te.filled_amount_raw, '') IS NULL
              OR te.filled_amount_decimals IS NULL
              OR te.gas_used IS NULL
              OR nullif(te.gas_cost_eth, '') IS NULL
              OR (
                  te.order_side = 'buy'
                  AND nullif(te.payload->'token_amount'->>'raw', '') IS NULL
              )
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn chain_sim_tail_entry_absence_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "chain_sim_has_no_mempool_tail_entry_orders",
        Verdict::Fail,
        "chain-sim live backtests do not submit mempool tail-entry orders",
        "chain-sim backtest rows still using the mempool tail-entry order path",
        r#"
        WITH chain_sim_runs AS (
            SELECT rsr.run_id
            FROM alpha_trading.backtest_result_set_runs rsr
            JOIN alpha_trading.trader_runs tr ON tr.run_id = rsr.run_id
            WHERE rsr.result_set_id = $1
              AND tr.mode = 'chain-sim'
        ),
        violations AS (
            SELECT 1
            FROM alpha_trading.order_intents oi
            JOIN chain_sim_runs csr ON csr.run_id = oi.run_id
            WHERE ($2::text IS NULL OR oi.strategy_name = $2)
              AND oi.side = 'buy'
              AND oi.reason_code LIKE 'entry.tail_after_enabling_tx%'
            UNION ALL
            SELECT 1
            FROM alpha_trading.trade_events te
            JOIN chain_sim_runs csr ON csr.run_id = te.run_id
            JOIN alpha_trading.trades t ON t.trade_id = te.trade_id
            WHERE ($2::text IS NULL OR t.strategy_name = $2)
              AND te.gas_policy_action = 'tail_entry_buy'
        )
        SELECT count(*)
        FROM violations
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn tail_entry_ordering_evidence_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "tail_entry_buy_has_ordering_evidence",
        Verdict::Fail,
        "no tail-entry buy events are missing dependency transaction or fee evidence",
        "tail-entry buy events missing dependency tx or fee evidence",
        r#"
        SELECT count(*)
        FROM alpha_trading.trade_events te
        JOIN alpha_trading.backtest_result_set_runs rsr
          ON rsr.run_id = te.run_id
         AND rsr.result_set_id = $1
        JOIN alpha_trading.trades t
          ON t.trade_id = te.trade_id
        WHERE ($2::text IS NULL OR t.strategy_name = $2)
          AND te.gas_policy_action = 'tail_entry_buy'
          AND (
              NULLIF(te.payload#>>'{mined_evidence,gas_policy_tail_after_tx_hash}', '') IS NULL
              OR (
                  NULLIF(te.payload#>>'{mined_evidence,gas_policy_dependency_priority_fee_wei}', '') IS NULL
                  AND NULLIF(te.payload#>>'{mined_evidence,gas_policy_dependency_gas_price_wei}', '') IS NULL
              )
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn tail_entry_priority_undercut_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "tail_entry_buy_priority_undercuts_dependency",
        Verdict::Fail,
        "tail-entry buy priority fee is below the dependency priority fee by policy",
        "tail-entry buy events whose selected priority fee does not undercut the dependency",
        r#"
        WITH cfg AS (
            SELECT COALESCE(
                       NULLIF(config#>>'{gas_policy,tail_entry_priority_undercut_wei}', '')::numeric,
                       0
                   ) AS undercut_wei
            FROM alpha_trading.backtest_result_sets
            WHERE result_set_id = $1
        ),
        tail_events AS (
            SELECT NULLIF(te.payload#>>'{mined_evidence,selected_max_priority_fee_per_gas_wei}', '')::numeric AS selected_priority_wei,
                   NULLIF(te.payload#>>'{mined_evidence,gas_policy_dependency_priority_fee_wei}', '')::numeric AS dependency_priority_wei
            FROM alpha_trading.trade_events te
            JOIN alpha_trading.backtest_result_set_runs rsr
              ON rsr.run_id = te.run_id
             AND rsr.result_set_id = $1
            JOIN alpha_trading.trades t
              ON t.trade_id = te.trade_id
            WHERE ($2::text IS NULL OR t.strategy_name = $2)
              AND te.gas_policy_action = 'tail_entry_buy'
        )
        SELECT count(*)
        FROM tail_events, cfg
        WHERE dependency_priority_wei IS NOT NULL
          AND (
              selected_priority_wei IS NULL
              OR (
                  cfg.undercut_wei > 0
                  AND dependency_priority_wei >= cfg.undercut_wei
                  AND selected_priority_wei > dependency_priority_wei - cfg.undercut_wei
              )
              OR (
                  cfg.undercut_wei <= 0
                  AND selected_priority_wei >= dependency_priority_wei
              )
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn tail_entry_live_backtest_n_plus_1_validation_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "tail_entry_buy_uses_live_backtest_n_plus_1_validation",
        Verdict::Fail,
        "tail-entry buy events are marked as live-backtest N+1 chain-sim validation",
        "tail-entry buy events missing live-backtest N+1 validation marker",
        r#"
        SELECT count(*)
        FROM alpha_trading.trade_events te
        JOIN alpha_trading.backtest_result_set_runs rsr
          ON rsr.run_id = te.run_id
         AND rsr.result_set_id = $1
        JOIN alpha_trading.trades t
          ON t.trade_id = te.trade_id
        WHERE ($2::text IS NULL OR t.strategy_name = $2)
          AND te.gas_policy_action = 'tail_entry_buy'
          AND (
              COALESCE(te.gas_policy_guard, '') NOT LIKE '%tail_entry_validation_mode=post_mine_n_plus_1%'
              OR COALESCE(te.gas_policy_guard, '') NOT LIKE '%exact_overlay_simulation=false%'
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn tail_entry_intents_have_exact_vault_evidence_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "tail_entry_intent_has_exact_vault_buy_evidence",
        Verdict::Fail,
        "no tail-entry buy intents are missing successful deployed-vault calldata evidence",
        "tail-entry buy intents missing successful exact-vault evidence",
        r#"
        SELECT count(*)
        FROM alpha_trading.order_intents oi
        JOIN alpha_trading.backtest_result_set_runs rsr
          ON rsr.run_id = oi.run_id
         AND rsr.result_set_id = $1
        WHERE ($2::text IS NULL OR oi.strategy_name = $2)
          AND oi.side = 'buy'
          AND oi.reason_code LIKE 'entry.tail_after_enabling_tx%'
          AND (
              COALESCE(oi.reason_details#>>'{risk_event_evidence,mempool_entry_evidence,vault_buy_simulation,route}', '') <> 'uniswap_v2_trading_vault'
              OR COALESCE(oi.reason_details#>>'{risk_event_evidence,mempool_entry_evidence,vault_buy_simulation,metadata,exact_vault_calldata}', 'false') <> 'true'
              OR COALESCE(oi.reason_details#>>'{risk_event_evidence,mempool_entry_evidence,vault_buy_simulation,would_revert}', 'true') <> 'false'
              OR NULLIF(oi.reason_details#>>'{risk_event_evidence,mempool_entry_evidence,vault_buy_simulation,gas_used}', '') IS NULL
              OR COALESCE(NULLIF(oi.reason_details#>>'{risk_event_evidence,mempool_entry_evidence,vault_buy_simulation,eth_spent_wei}', ''), '0') = '0'
              OR COALESCE(NULLIF(oi.reason_details#>>'{risk_event_evidence,mempool_entry_evidence,vault_buy_simulation,tokens_received_raw}', ''), '0') = '0'
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

#[derive(Clone, Copy, Debug, Default)]
struct TailEntryCoverage {
    chain_sim_runs: i64,
    trading_enabled_signals: i64,
    ignored_trading_enabled_observations: i64,
    ignored_trading_enabled_with_skip_reason: i64,
    with_mempool_entry_evidence: i64,
    exact_vault_eligible_signals: i64,
    tail_entry_intents: i64,
    submitted: i64,
    confirmed: i64,
    deferred: i64,
    failed: i64,
    cancelled: i64,
}

pub(super) async fn tail_entry_coverage_check(
    pool: &PgPool,
    result_set: &ResultSetRecord,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    let row = sqlx::query(
        r#"
        WITH scoped_runs AS (
            SELECT run_id
            FROM alpha_trading.backtest_result_set_runs
            WHERE result_set_id = $1
        ),
        chain_sim_runs AS (
            SELECT sr.run_id
            FROM scoped_runs sr
            JOIN alpha_trading.trader_runs tr ON tr.run_id = sr.run_id
            WHERE tr.mode = 'chain-sim'
        ),
        trading_signals AS (
            SELECT re.*
            FROM alpha_trading.risk_events re
            JOIN scoped_runs sr ON sr.run_id = re.run_id
            WHERE re.kind = 'trading_enabled'
        ),
        ignored_trading_observations AS (
            SELECT so.*
            FROM alpha_trading.strategy_observations so
            JOIN chain_sim_runs csr ON csr.run_id = so.run_id
            WHERE so.event_source = 'mempool_signal'
              AND so.decision = 'ignored'
              AND COALESCE(so.payload#>>'{extra,signal_type}', so.payload#>>'{signal,signal_type}', '') = 'trading_enabled'
              AND (
                  $2::text IS NULL
                  OR EXISTS (
                      SELECT 1
                      FROM alpha_trading.trades t
                      WHERE t.result_set_id = $1
                        AND t.run_id = so.run_id
                        AND t.strategy_name = $2
                  )
              )
        ),
        tail_intents AS (
            SELECT oi.*
            FROM alpha_trading.order_intents oi
            JOIN scoped_runs sr ON sr.run_id = oi.run_id
            WHERE ($2::text IS NULL OR oi.strategy_name = $2)
              AND oi.side = 'buy'
              AND oi.reason_code LIKE 'entry.tail_after_enabling_tx%'
        ),
        tail_events AS (
            SELECT te.*
            FROM alpha_trading.trade_events te
            JOIN scoped_runs sr ON sr.run_id = te.run_id
            JOIN alpha_trading.trades t ON t.trade_id = te.trade_id
            WHERE ($2::text IS NULL OR t.strategy_name = $2)
              AND te.order_side = 'buy'
              AND te.gas_policy_action = 'tail_entry_buy'
        )
        SELECT
            (SELECT count(*) FROM chain_sim_runs) AS chain_sim_runs,
            (SELECT count(*) FROM trading_signals) AS trading_enabled_signals,
            (SELECT count(*) FROM ignored_trading_observations) AS ignored_trading_enabled_observations,
            (
                SELECT count(*)
                FROM ignored_trading_observations
                WHERE COALESCE(payload#>>'{extra,reason_code}', '') = 'chain_sim.live_backtest.skip_mempool_trading_enabled'
                  AND COALESCE(payload#>>'{extra,execution_mode}', '') = 'chain-sim'
                  AND COALESCE(payload#>>'{extra,entry_path}', '') = 'pool_update'
            ) AS ignored_trading_enabled_with_skip_reason,
            (
                SELECT count(*)
                FROM trading_signals
                WHERE payload#>'{evidence,mempool_entry_evidence}' IS NOT NULL
            ) AS with_mempool_entry_evidence,
            (
                SELECT count(*)
                FROM trading_signals
                WHERE payload#>>'{evidence,mempool_entry_evidence,vault_buy_simulation,route}' = 'uniswap_v2_trading_vault'
                  AND COALESCE(payload#>>'{evidence,mempool_entry_evidence,vault_buy_simulation,metadata,exact_vault_calldata}', 'false') = 'true'
                  AND COALESCE(payload#>>'{evidence,mempool_entry_evidence,vault_buy_simulation,would_revert}', 'true') = 'false'
                  AND NULLIF(payload#>>'{evidence,mempool_entry_evidence,vault_buy_simulation,gas_used}', '') IS NOT NULL
                  AND COALESCE(NULLIF(payload#>>'{evidence,mempool_entry_evidence,vault_buy_simulation,eth_spent_wei}', ''), '0') <> '0'
                  AND COALESCE(NULLIF(payload#>>'{evidence,mempool_entry_evidence,vault_buy_simulation,tokens_received_raw}', ''), '0') <> '0'
            ) AS exact_vault_eligible_signals,
            (SELECT count(*) FROM tail_intents) AS tail_entry_intents,
            (SELECT count(*) FROM tail_events WHERE status = 'submitted') AS submitted,
            (SELECT count(*) FROM tail_events WHERE status = 'confirmed') AS confirmed,
            (SELECT count(*) FROM tail_events WHERE status = 'deferred') AS deferred,
            (SELECT count(*) FROM tail_events WHERE status = 'failed') AS failed,
            (SELECT count(*) FROM tail_events WHERE status = 'cancelled') AS cancelled
        "#,
    )
    .bind(&result_set.result_set_id)
    .bind(strategy)
    .fetch_one(pool)
    .await?;

    let coverage = TailEntryCoverage {
        chain_sim_runs: row.try_get("chain_sim_runs")?,
        trading_enabled_signals: row.try_get("trading_enabled_signals")?,
        ignored_trading_enabled_observations: row
            .try_get("ignored_trading_enabled_observations")?,
        ignored_trading_enabled_with_skip_reason: row
            .try_get("ignored_trading_enabled_with_skip_reason")?,
        with_mempool_entry_evidence: row.try_get("with_mempool_entry_evidence")?,
        exact_vault_eligible_signals: row.try_get("exact_vault_eligible_signals")?,
        tail_entry_intents: row.try_get("tail_entry_intents")?,
        submitted: row.try_get("submitted")?,
        confirmed: row.try_get("confirmed")?,
        deferred: row.try_get("deferred")?,
        failed: row.try_get("failed")?,
        cancelled: row.try_get("cancelled")?,
    };
    let enforced = tail_entry_coverage_enforced(result_set, strategy);
    let (verdict, message) = tail_entry_coverage_verdict(coverage, enforced);

    Ok(check(
        "execution_replay",
        "tail_entry_coverage",
        verdict,
        message,
        json!({
            "scope": {
                "enforced": enforced,
                "result_set_id": result_set.result_set_id.as_str(),
                "strategy_name": strategy,
                "strategy_suite": result_set.strategy_suite.as_deref(),
            },
            "counts": {
                "chain_sim_runs": coverage.chain_sim_runs,
                "trading_enabled_signals": coverage.trading_enabled_signals,
                "ignored_trading_enabled_observations": coverage.ignored_trading_enabled_observations,
                "ignored_trading_enabled_with_skip_reason": coverage.ignored_trading_enabled_with_skip_reason,
                "with_mempool_entry_evidence": coverage.with_mempool_entry_evidence,
                "exact_vault_eligible_signals": coverage.exact_vault_eligible_signals,
                "tail_entry_intents": coverage.tail_entry_intents,
                "submitted": coverage.submitted,
                "confirmed": coverage.confirmed,
                "deferred": coverage.deferred,
                "failed": coverage.failed,
                "cancelled": coverage.cancelled,
            },
        }),
    ))
}

fn tail_entry_coverage_enforced(result_set: &ResultSetRecord, strategy: Option<&str>) -> bool {
    let mut scope = String::new();
    scope.push_str(&result_set.result_set_id);
    if let Some(strategy_suite) = result_set.strategy_suite.as_deref() {
        scope.push(' ');
        scope.push_str(strategy_suite);
    }
    if let Some(strategy) = strategy {
        scope.push(' ');
        scope.push_str(strategy);
    }
    scope.contains("alpha11-univ2-lp30-pool-update-block")
}

fn tail_entry_coverage_verdict(coverage: TailEntryCoverage, enforced: bool) -> (Verdict, String) {
    if !enforced {
        return (
            Verdict::Pass,
            "tail-entry coverage recorded but not enforced for this strategy scope".to_string(),
        );
    }
    if coverage.chain_sim_runs > 0
        && coverage.tail_entry_intents == 0
        && coverage.submitted
            + coverage.confirmed
            + coverage.deferred
            + coverage.failed
            + coverage.cancelled
            == 0
    {
        return (
            Verdict::Pass,
            "tail-entry coverage is not enforced for chain-sim live backtests; trading_enabled mempool signals use the mined pool-update entry path".to_string(),
        );
    }
    if coverage.trading_enabled_signals == 0 {
        return (
            Verdict::Blocked,
            "tail-entry coverage missing: no trading_enabled signals observed".to_string(),
        );
    }
    if coverage.with_mempool_entry_evidence == 0 {
        return (
            Verdict::Blocked,
            "tail-entry coverage missing: trading_enabled signals have no mempool entry evidence"
                .to_string(),
        );
    }
    if coverage.exact_vault_eligible_signals == 0 {
        return (
            Verdict::Blocked,
            "tail-entry coverage missing: no trading_enabled signal had successful exact-vault evidence"
                .to_string(),
        );
    }
    if coverage.tail_entry_intents == 0 {
        return (
            Verdict::Blocked,
            "tail-entry coverage missing: exact-vault evidence did not produce a tail-entry intent"
                .to_string(),
        );
    }
    if coverage.submitted
        + coverage.confirmed
        + coverage.deferred
        + coverage.failed
        + coverage.cancelled
        == 0
    {
        return (
            Verdict::Blocked,
            "tail-entry coverage missing: tail-entry intents did not reach execution events"
                .to_string(),
        );
    }
    if coverage.confirmed + coverage.deferred + coverage.failed + coverage.cancelled == 0 {
        return (
            Verdict::Blocked,
            "tail-entry coverage incomplete: tail-entry execution has no terminal outcome yet"
                .to_string(),
        );
    }
    (
        Verdict::Pass,
        "tail-entry coverage exercised from trading_enabled signal through execution outcome"
            .to_string(),
    )
}

pub(super) async fn execution_replay_inputs_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "execution_replay",
        "closed_trade_replay_inputs_present",
        Verdict::Fail,
        "closed trades retain enough order/report data for independent execution replay",
        "closed trades missing buy token amount, sell amount, or sell fill data",
        r#"
        SELECT count(*)
        FROM alpha_trading.trades t
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND t.state = 'sell_confirmed'
          AND (
              t.entry_order_id IS NULL
              OR t.exit_order_id IS NULL
              OR NOT EXISTS (
                  SELECT 1
                  FROM alpha_trading.trade_events te
                  WHERE te.trade_id = t.trade_id
                    AND te.event_type = 'buy_confirmed'
                    AND te.payload->'token_amount'->>'raw' IS NOT NULL
              )
              OR NOT EXISTS (
                  SELECT 1
                  FROM alpha_trading.order_intents oi
                  WHERE oi.trade_id = t.trade_id
                    AND oi.side = 'sell'
                    AND nullif(oi.amount_raw, '') IS NOT NULL
              )
              OR NOT EXISTS (
                  SELECT 1
                  FROM alpha_trading.trade_events te
                  WHERE te.trade_id = t.trade_id
                    AND te.event_type = 'sell_confirmed'
                    AND nullif(te.filled_amount_raw, '') IS NOT NULL
              )
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::{tail_entry_coverage_verdict, TailEntryCoverage};
    use crate::strategy_validation::report::Verdict;

    #[test]
    fn tail_entry_coverage_blocks_when_enforced_scope_has_no_signal() {
        let (verdict, message) = tail_entry_coverage_verdict(TailEntryCoverage::default(), true);
        assert_eq!(verdict, Verdict::Blocked);
        assert!(message.contains("no trading_enabled signals"));
    }

    #[test]
    fn tail_entry_coverage_blocks_when_exact_vault_evidence_is_absent() {
        let coverage = TailEntryCoverage {
            trading_enabled_signals: 3,
            with_mempool_entry_evidence: 3,
            ..TailEntryCoverage::default()
        };
        let (verdict, message) = tail_entry_coverage_verdict(coverage, true);
        assert_eq!(verdict, Verdict::Blocked);
        assert!(message.contains("successful exact-vault evidence"));
    }

    #[test]
    fn tail_entry_coverage_passes_when_terminal_outcome_exists() {
        let coverage = TailEntryCoverage {
            trading_enabled_signals: 3,
            with_mempool_entry_evidence: 2,
            exact_vault_eligible_signals: 1,
            tail_entry_intents: 1,
            submitted: 1,
            confirmed: 1,
            ..TailEntryCoverage::default()
        };
        let (verdict, _) = tail_entry_coverage_verdict(coverage, true);
        assert_eq!(verdict, Verdict::Pass);
    }

    #[test]
    fn tail_entry_coverage_passes_for_chain_sim_pool_update_contract() {
        let coverage = TailEntryCoverage {
            chain_sim_runs: 1,
            ignored_trading_enabled_observations: 3,
            ignored_trading_enabled_with_skip_reason: 3,
            ..TailEntryCoverage::default()
        };
        let (verdict, message) = tail_entry_coverage_verdict(coverage, true);
        assert_eq!(verdict, Verdict::Pass);
        assert!(message.contains("chain-sim live backtests"));
    }

    #[test]
    fn tail_entry_coverage_is_not_enforced_for_other_scopes() {
        let (verdict, message) = tail_entry_coverage_verdict(TailEntryCoverage::default(), false);
        assert_eq!(verdict, Verdict::Pass);
        assert!(message.contains("not enforced"));
    }
}
