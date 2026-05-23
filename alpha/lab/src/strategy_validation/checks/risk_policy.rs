use eyre::Result;
use sqlx::PgPool;

use super::super::report::{CheckResult, Verdict};
use super::common::count_check;

pub(super) async fn liquidity_removal_risk_kind_source_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "risk_policy",
        "liquidity_removal_risk_kind_matches_source",
        Verdict::Fail,
        "liquidity-removal risk kind matches pending or mined source",
        "liquidity-removal risk rows with inconsistent kind/source",
        r#"
        WITH scoped_runs AS (
            SELECT DISTINCT run_id
            FROM alpha_trading.trades
            WHERE result_set_id = $1
              AND ($2::text IS NULL OR strategy_name = $2)
        )
        SELECT count(*)
        FROM alpha_trading.risk_events re
        JOIN scoped_runs sr ON sr.run_id = re.run_id
        WHERE (
              re.kind = 'liquidity_removal'
              AND (
                  re.pending_tx_hash IS NOT NULL
                  OR COALESCE(re.payload->>'source', '') IN (
                      'mempool_signal',
                      'historical_mempool_signal'
                  )
              )
          )
          OR (
              re.kind = 'mempool_liquidity_removal'
              AND re.pending_tx_hash IS NULL
              AND COALESCE(re.payload->>'source', '') NOT IN (
                  'mempool_signal',
                  'historical_mempool_signal'
              )
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn mempool_liquidity_removal_does_not_zero_snapshot_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "risk_policy",
        "mempool_liquidity_removal_does_not_zero_exposure_snapshot",
        Verdict::Fail,
        "mempool liquidity-removal signals do not mark confirmed exposure as zero value",
        "mempool liquidity-removal signals with same-block zero-value exposure snapshots",
        r#"
        WITH mempool_risks AS (
            SELECT re.run_id,
                   re.token_address,
                   re.pool_address,
                   re.observed_block
            FROM alpha_trading.risk_events re
            WHERE re.observed_block IS NOT NULL
              AND re.pool_address IS NOT NULL
              AND (
                  re.kind = 'mempool_liquidity_removal'
                  OR (
                      re.kind = 'liquidity_removal'
                      AND (
                          re.pending_tx_hash IS NOT NULL
                          OR COALESCE(re.payload->>'source', '') IN (
                              'mempool_signal',
                              'historical_mempool_signal'
                          )
                      )
                  )
              )
        )
        SELECT count(DISTINCT ts.id)
        FROM alpha_trading.trades t
        JOIN mempool_risks re
          ON re.run_id = t.run_id
         AND lower(re.token_address) = lower(t.token_address)
         AND lower(re.pool_address) = lower(t.pool_address)
        JOIN alpha_trading.trade_snapshots ts ON ts.trade_id = t.trade_id
        WHERE t.result_set_id = $1
          AND ($2::text IS NULL OR t.strategy_name = $2)
          AND t.entry_block IS NOT NULL
          AND re.observed_block >= t.entry_block
          AND (t.exit_block IS NULL OR re.observed_block <= t.exit_block)
          AND ts.state IN (
              'buy_confirmed',
              'sell_intent_created',
              'sell_submitted',
              'sell_failed',
              'sell_cancelled'
          )
          AND COALESCE(ts.valuation_block_number, ts.block_number) = re.observed_block
          AND abs(coalesce(nullif(ts.current_value_eth, '')::numeric, 0)) <= 0.000001
          AND abs(
              coalesce(nullif(ts.unrealized_pnl_eth, '')::numeric, 0)
              + coalesce(nullif(t.entry_cost_eth, '')::numeric, 0)
          ) <= 0.000001
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn configured_critical_risks_have_strategy_response_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "risk_policy",
        "configured_critical_risk_has_strategy_response",
        Verdict::Fail,
        "configured critical in-position risk events have an explicit strategy response",
        "configured critical in-position risk events without sell or explicit deferral",
        r#"
        WITH strategy_cfg AS (
            SELECT spec->>'strategy_name' AS strategy_name,
                   COALESCE((spec->>'exit_liquidity_removal')::boolean, false) AS exit_liquidity_removal,
                   COALESCE((spec->>'exit_lp_approval')::boolean, false) AS exit_lp_approval,
                   COALESCE((spec->>'exit_tax')::boolean, false) AS exit_tax,
                   COALESCE((spec->>'exit_scam')::boolean, false) AS exit_scam,
                   COALESCE((spec->>'defer_buy_confirm_block_lp_approval_to_max_hold')::boolean, false) AS defer_buy_confirm_block_lp_approval_to_max_hold,
                   NULLIF(spec->>'lp_approval_exit_defer_max_trading_enabled_age_blocks', '')::bigint AS lp_approval_exit_defer_max_trading_enabled_age_blocks
            FROM alpha_trading.backtest_result_sets rs
            CROSS JOIN LATERAL jsonb_array_elements(
                CASE
                    WHEN jsonb_typeof(rs.config->'strategies') = 'array'
                    THEN rs.config->'strategies'
                    ELSE '[]'::jsonb
                END
            ) AS spec
            WHERE rs.result_set_id = $1
            UNION ALL
            SELECT rs.config->>'strategy_name' AS strategy_name,
                   COALESCE((rs.config->>'exit_liquidity_removal')::boolean, false) AS exit_liquidity_removal,
                   COALESCE((rs.config->>'exit_lp_approval')::boolean, false) AS exit_lp_approval,
                   COALESCE((rs.config->>'exit_tax')::boolean, false) AS exit_tax,
                   COALESCE((rs.config->>'exit_scam')::boolean, false) AS exit_scam,
                   COALESCE((rs.config->>'defer_buy_confirm_block_lp_approval_to_max_hold')::boolean, false) AS defer_buy_confirm_block_lp_approval_to_max_hold,
                   NULLIF(rs.config->>'lp_approval_exit_defer_max_trading_enabled_age_blocks', '')::bigint AS lp_approval_exit_defer_max_trading_enabled_age_blocks
            FROM alpha_trading.backtest_result_sets rs
            WHERE rs.result_set_id = $1
              AND rs.config ? 'strategy_name'
        ),
        scoped AS (
            SELECT t.trade_id,
                   t.run_id,
                   t.strategy_name,
                   t.token_address,
                   t.pool_address,
                   t.entry_block,
                   t.exit_block,
                   cfg.exit_liquidity_removal,
                   cfg.exit_lp_approval,
                   cfg.exit_tax,
                   cfg.exit_scam,
                   cfg.defer_buy_confirm_block_lp_approval_to_max_hold,
                   cfg.lp_approval_exit_defer_max_trading_enabled_age_blocks
            FROM alpha_trading.trades t
            JOIN strategy_cfg cfg ON cfg.strategy_name = t.strategy_name
            WHERE t.result_set_id = $1
              AND ($2::text IS NULL OR t.strategy_name = $2)
              AND t.entry_block IS NOT NULL
        ),
        configured_risks AS (
            SELECT s.*,
                   re.kind,
                   re.observed_block,
                   re.severity
            FROM scoped s
            JOIN alpha_trading.risk_events re
              ON re.run_id = s.run_id
             AND lower(re.token_address) = lower(s.token_address)
             AND (re.pool_address IS NULL OR lower(re.pool_address) = lower(s.pool_address))
             AND re.observed_block IS NOT NULL
             AND re.observed_block >= s.entry_block
             AND (s.exit_block IS NULL OR re.observed_block <= s.exit_block)
            WHERE re.severity IN ('critical', 'high')
              AND (
                  (re.kind IN ('liquidity_removal', 'mempool_liquidity_removal') AND s.exit_liquidity_removal)
                  OR (re.kind = 'lp_approval' AND s.exit_lp_approval)
                  OR (re.kind IN ('tax_change', 'honeypot') AND s.exit_tax)
                  OR (re.kind IN ('scam_confirmed', 'honeypot') AND s.exit_scam)
              )
        )
        SELECT count(*)
        FROM configured_risks risk
        WHERE NOT EXISTS (
            SELECT 1
            FROM alpha_trading.trade_events te
            WHERE te.trade_id = risk.trade_id
              AND te.event_type = 'sell_submitted'
              AND te.block_number >= risk.observed_block
        )
          AND NOT (
              risk.kind = 'lp_approval'
              AND risk.defer_buy_confirm_block_lp_approval_to_max_hold
              AND risk.observed_block = risk.entry_block
              AND EXISTS (
                  SELECT 1
                  FROM alpha_trading.strategy_decisions sd
                  WHERE sd.run_id = risk.run_id
                    AND sd.strategy_name = risk.strategy_name
                    AND lower(sd.token_address) = lower(risk.token_address)
                    AND lower(sd.pool_address) = lower(risk.pool_address)
                    AND sd.block_number = risk.observed_block
                    AND sd.reason = 'exit.lp_approval_buy_confirm_block_deferred_to_max_hold'
              )
          )
          AND NOT (
              risk.kind = 'lp_approval'
              AND risk.lp_approval_exit_defer_max_trading_enabled_age_blocks IS NOT NULL
              AND EXISTS (
                  SELECT 1
                  FROM alpha_trading.strategy_decisions sd
                  WHERE sd.run_id = risk.run_id
                    AND sd.strategy_name = risk.strategy_name
                    AND lower(sd.token_address) = lower(risk.token_address)
                    AND lower(sd.pool_address) = lower(risk.pool_address)
                    AND sd.block_number = risk.observed_block
                    AND (
                        sd.reason_code = 'exit.lp_approval.early_approval_deferred_to_max_hold'
                        OR sd.reason = 'exit.lp_approval:early_approval_deferred_to_max_hold'
                        OR sd.reason LIKE 'exit.lp_approval:early_approval_deferred_to_max_hold:%'
                    )
                    AND COALESCE(
                        NULLIF(sd.reason_details->>'trading_enabled_age_blocks', '')::numeric,
                        NULLIF(sd.reason_details->>'pool_age_blocks', '')::numeric,
                        NULLIF(sd.reason_details->>'age_blocks', '')::numeric,
                        NULLIF(sd.reason_details#>>'{risk_event_evidence,trading_enabled_age_blocks_at_signal}', '')::numeric,
                        NULLIF(sd.reason_details#>>'{risk_event_evidence,pool_age_blocks_at_signal}', '')::numeric
                    ) <= risk.lp_approval_exit_defer_max_trading_enabled_age_blocks
              )
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}

pub(super) async fn lp_approval_deferrals_have_age_evidence_check(
    pool: &PgPool,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    count_check(
        pool,
        "risk_policy",
        "lp_approval_deferral_has_age_evidence",
        Verdict::Fail,
        "LP-approval deferrals persist their age basis and active-block age",
        "LP-approval deferral decisions missing structured age evidence",
        r#"
        SELECT count(*)
        FROM alpha_trading.strategy_decisions sd
        JOIN alpha_trading.backtest_result_set_runs rsr ON rsr.run_id = sd.run_id
        WHERE rsr.result_set_id = $1
          AND ($2::text IS NULL OR sd.strategy_name = $2)
          AND (
              sd.reason_code = 'exit.lp_approval.early_approval_deferred_to_max_hold'
              OR sd.reason = 'exit.lp_approval:early_approval_deferred_to_max_hold'
              OR sd.reason LIKE 'exit.lp_approval:early_approval_deferred_to_max_hold:%'
          )
          AND (
              COALESCE(
                  sd.reason_details->>'age_basis',
                  sd.reason_details#>>'{risk_event_evidence,lp_approval_age_basis}'
              ) IS NULL
              OR COALESCE(
                  sd.reason_details->>'trading_enabled_age_blocks',
                  sd.reason_details->>'pool_age_blocks',
                  sd.reason_details->>'age_blocks',
                  sd.reason_details#>>'{risk_event_evidence,trading_enabled_age_blocks_at_signal}',
                  sd.reason_details#>>'{risk_event_evidence,pool_age_blocks_at_signal}'
              ) IS NULL
              OR COALESCE(
                  sd.reason_details->>'signal_id',
                  sd.reason_details#>>'{risk_event_evidence,signal_id}'
              ) IS NULL
          )
        "#,
        result_set_id,
        strategy,
    )
    .await
}
