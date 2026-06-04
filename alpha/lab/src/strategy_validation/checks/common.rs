use eyre::Result;
use serde_json::json;
use sqlx::PgPool;

use super::super::db::scalar_i64;
use super::super::report::{CheckResult, Verdict};

const TOLERANCE_ETH: &str = "0.000001";

pub(super) async fn count_check(
    pool: &PgPool,
    category: impl Into<String>,
    code: impl Into<String>,
    nonzero_verdict: Verdict,
    pass_message: impl Into<String>,
    nonzero_message: impl Into<String>,
    sql: &str,
    result_set_id: &str,
    strategy: Option<&str>,
) -> Result<CheckResult> {
    let count = scalar_i64(pool, sql, result_set_id, strategy).await?;
    let verdict = if count == 0 {
        Verdict::Pass
    } else {
        nonzero_verdict
    };
    let message = if count == 0 {
        pass_message.into()
    } else {
        format!("{}: {count}", nonzero_message.into())
    };
    Ok(check(
        category,
        code,
        verdict,
        message,
        json!({
            "violations": count,
            "tolerance_eth": TOLERANCE_ETH,
        }),
    ))
}

pub(super) fn check(
    category: impl Into<String>,
    code: impl Into<String>,
    verdict: Verdict,
    message: impl Into<String>,
    evidence: serde_json::Value,
) -> CheckResult {
    let code = code.into();
    let (question, description) = check_copy(&code);
    let blocking = is_blocking_code(&code);
    CheckResult {
        category: category.into(),
        code,
        question: question.to_string(),
        description: description.to_string(),
        verdict,
        message: message.into(),
        evidence,
        blocking,
    }
}

/// Promotion-blocking classification for a validation check, keyed on `code`.
///
/// A `Verdict::Fail` on a blocking check is a HARD promotion/broadcast blocker
/// (it increments `ValidationSummary.blocking_failures`, which drives the CLI
/// exit code and the live-real preflight gate). Advisory checks never gate.
///
/// Policy: CORRECTNESS-FIRST. Every check is blocking by DEFAULT; only an
/// explicit allowlist of coverage/informational checks is advisory. This keeps
/// newly added correctness invariants gating automatically unless deliberately
/// downgraded here. Note: this is independent of the verdict — a `Blocked`
/// ("could not evaluate") verdict never counts as a blocking failure regardless
/// of this classification (see `ValidationSummary::from_checks`), so a vacuous
/// historical `tail_entry_coverage=Blocked` does not gate.
pub(super) fn is_blocking_code(code: &str) -> bool {
    !matches!(
        code,
        // Coverage probe: returns Blocked (vacuous) on pure historical runs and
        // is a "did we exercise this path" meter, not a correctness invariant.
        "tail_entry_coverage"
        // Informational: counts whether the strategy produced any trades in
        // scope. Empty scope is a "nothing to validate" signal, not a failed
        // invariant, so it must not block promotion on its own.
        | "strategy_rows"
    )
}

fn check_copy(code: &str) -> (&'static str, &'static str) {
    match code {
        "result_set_status" => (
            "Is this result set in the expected state?",
            "Compares result-set mode with status so completed historical results and running live results are not mixed with partial runs.",
        ),
        "live_runtime_status_matches_result_set" => (
            "Does the live runtime health match the result-set state?",
            "Fails when a live result set is marked running while live runtime metadata reports a failed chain/trader status.",
        ),
        "live_chain_sim_uses_chain_server_simulator" => (
            "Does live chain-sim use the chain-server LiveTxSimulator?",
            "Requires live chain-sim result-set and trader-run metadata to prove simulations came from the chain-server-owned LiveTxSimulator, not an Alpha-local simulator.",
        ),
        "live_block_frame_runtime_metadata" => (
            "Does live chain-sim use block-frame inputs?",
            "Requires live chain-sim runs to record block-frame runtime metadata so decisions are coupled to a specific pushed block frame.",
        ),
        "running_result_set_has_no_stop_marker" => (
            "Is a running live result free of stopped-run markers?",
            "Rejects running result sets or trader runs that still carry stopped_at or shutdown/stale metadata from a prior process lifetime.",
        ),
        "strategy_rows" => (
            "Did this strategy produce trades in this result set?",
            "Counts scoped trade rows before any PnL or validation verdict is treated as meaningful.",
        ),
        "public_trade_id_format" => (
            "Are public trade identifiers opaque and stable?",
            "Requires every scoped trade_id to use the public `trd_` prefix instead of leaking legacy position IDs.",
        ),
        "historical_mempool_rows" => (
            "Did a historical backtest avoid pending mempool evidence?",
            "Checks joined risk events for pending transaction hashes; pure historical results must use mined local evidence only.",
        ),
        "historical_mempool_observations" => (
            "Did a historical backtest avoid mempool strategy observations?",
            "Rejects strategy observations sourced from live mempool signals in historical result sets.",
        ),
        "buy_submitted_has_decision" => (
            "Can every buy submission be explained by a strategy decision?",
            "Joins buy_submitted trade events to same-block submit_buy strategy decisions for the same run, strategy, token, and pool.",
        ),
        "sell_submitted_has_decision" => (
            "Can every sell submission be explained by a strategy decision?",
            "Joins sell_submitted trade events to same-block submit_sell strategy decisions for the same run, strategy, token, and pool.",
        ),
        "risk_sell_has_available_signal" => (
            "Was each risk-triggered sell based on already-available evidence?",
            "Requires risk sell decisions to have a local risk event for the token/pool at or before the decision block.",
        ),
        "risk_sell_signal_block_immediate" => (
            "Did risk-triggered exits submit immediately on the signal block?",
            "Requires risk-sourced submit_sell decisions to match a same-block local risk event for the token/pool.",
        ),
        "configured_critical_risk_has_strategy_response" => (
            "Did configured critical risks receive the strategy response they require?",
            "Compares in-position critical risk events against the strategy config and fails if an enabled exit risk has no sell submission, explicit same-block deferral, or live-backtest mempool-removal gas/value cancellation.",
        ),
        "lp_approval_deferral_has_age_evidence" => (
            "Do LP-approval deferrals carry explicit active-block evidence?",
            "Requires deferred LP-approval decisions to store signal id, age basis, and the active-block age used to defer instead of sell.",
        ),
        "liquidity_removal_risk_kind_matches_source" => (
            "Does liquidity-removal risk kind match its evidence source?",
            "Requires pending mempool liquidity-removal rows to use `mempool_liquidity_removal` and mined-chain rows to use `liquidity_removal`.",
        ),
        "mempool_liquidity_removal_does_not_zero_exposure_snapshot" => (
            "Do mempool liquidity-removal signals avoid marking exposure as drained?",
            "Fails if a pending mempool liquidity-removal signal creates a same-block zero-value exposure snapshot before mined evidence exists.",
        ),
        "mempool_risk_uses_detector_head_block" => (
            "Do mempool risk events use detector-time block evidence?",
            "Requires live mempool risk rows to copy `detected_at_head_block_number` into `observed_block` instead of using Alpha's current block-frame as a fallback.",
        ),
        "pool_update_buy_has_historical_observation" => (
            "Can every historical pool-update buy be traced to an input observation?",
            "Joins pool_update-sourced submit_buy decisions to the replay pool observation at the same token, pool, and block.",
        ),
        "deferred_mempool_signal_has_reason" => (
            "Do deferred mempool observations explain why they were deferred?",
            "Requires live-backtest mempool signals buffered behind chain-sim settlement readiness to persist the explicit settlement-wait reason code.",
        ),
        "chain_sim_trading_enabled_mempool_skip" => (
            "Does chain-sim explicitly skip trading-enabled mempool entries?",
            "Allows first-poll priming but requires processed trading_enabled mempool signals in chain-sim to be ignored with the documented pool-update entry-path reason.",
        ),
        "no_settlement_wait_mempool_deferrals" => (
            "Has the old local settlement-state deferral path been removed?",
            "Fails if mempool observations are still deferred because Alpha is waiting on local chain-sim settlement state.",
        ),
        "submit_decisions_within_result_range" => (
            "Were submitted decisions made inside the replayed input range?",
            "Rejects submitted buy/sell decisions whose decision block is outside the result set start/end blocks.",
        ),
        "trades_match_allowed_protocols" => (
            "Did trades respect configured protocol filters?",
            "Checks every persisted trade protocol against the strategy's allowed_protocols config when that allowlist is non-empty.",
        ),
        "terminal_report_matches_execution_delay" => (
            "Do fills land at the configured execution delay?",
            "Checks each order's terminal confirmed/failed/cancelled report block equals submitted_block + execution_delay_blocks.",
        ),
        "submitted_orders_have_terminal_report_after_delay" => (
            "Does every elapsed submitted order have a terminal execution report?",
            "Fails if a submitted buy or sell order has no confirmed, failed, or cancelled report after the configured execution delay has elapsed.",
        ),
        "live_chain_sim_execution_blocks_align" => (
            "Did live chain-sim use the exact expected simulation block?",
            "For live backtests, requires submitted block N, expected/simulation/receipt/event block N + execution_delay_blocks, and rejects stale simulator state.",
        ),
        "live_chain_sim_block_hash_evidence" => (
            "Did live chain-sim persist exact block hashes?",
            "Requires submitted and terminal live chain-sim events to carry a valid mined-evidence block hash so reorg/same-height ambiguity is auditable.",
        ),
        "chain_sim_has_no_real_execution_artifacts" => (
            "Did chain-sim avoid real execution artifacts?",
            "Rejects tx hashes and ETH tx executor submission errors in chain-sim backtests, which should only contain simulated execution reports.",
        ),
        "pre_submit_simulation_state_ready" => (
            "Was pre-submit simulation state ready for every attempted order?",
            "Fails any run with deferred execution reports caused by the simulator lagging behind the decision block.",
        ),
        "terminal_reports_have_gas_policy_fee_evidence" => (
            "Do terminal reports retain gas-policy fee evidence?",
            "Requires selected gas-policy terminal events to store selected fee/profile/source evidence, and rejected events to store rejection guard evidence.",
        ),
        "confirmed_reports_have_simulation_outputs" => (
            "Are confirmed fills backed by persisted EVM simulation output?",
            "Requires filled amount, gas, gas cost, and buy token output on confirmed trade events.",
        ),
        "chain_sim_has_no_mempool_tail_entry_orders" => (
            "Did chain-sim avoid mempool tail-entry orders?",
            "Live chain-sim validates mined pool-update entries and must not submit tail-after-enabling mempool orders.",
        ),
        "tail_entry_intent_has_exact_vault_buy_evidence" => (
            "Are tail-entry buys backed by exact deployed-vault evidence?",
            "Requires tail-after-enabling buy intents to carry successful `uniswap_v2_trading_vault` calldata evidence, not the generic pool probe.",
        ),
        "tail_entry_coverage" => (
            "Did this validation run actually exercise the tail-entry path?",
            "Counts trading_enabled signals, mempool entry evidence, exact-vault eligible signals, tail-entry intents, and tail-entry execution outcomes so tail-entry checks cannot pass vacuously.",
        ),
        "tail_entry_buy_has_ordering_evidence" => (
            "Do tail-entry buys retain dependency ordering evidence?",
            "Requires tail-entry gas shadow events to include the dependency tx hash plus dependency fee evidence used to place behind the enabling tx.",
        ),
        "tail_entry_buy_priority_undercuts_dependency" => (
            "Do tail-entry buys undercut the dependency priority fee?",
            "Checks that persisted tail-entry gas evidence selected a priority fee below the enabling transaction by the configured undercut.",
        ),
        "tail_entry_buy_uses_live_backtest_n_plus_1_validation" => (
            "Do tail-entry buys use the live-backtest N+1 execution model?",
            "Requires tail-entry fills to be explicitly marked as post-mine N+1 chain-sim validation, not same-block overlay proof.",
        ),
        "entry_block_matches_buy_confirmed" => (
            "Does entry_block mean the buy-confirmed block?",
            "Compares each trade entry_block with its buy_confirmed lifecycle event block.",
        ),
        "exit_block_matches_sell_confirmed" => (
            "Does exit_block mean the sell-confirmed block?",
            "Compares each closed trade exit_block with its sell_confirmed lifecycle event block.",
        ),
        "no_exit_block_before_sell_confirmed" => (
            "Do open or failed trades avoid fake exit blocks?",
            "Fails if any non-sell_confirmed trade has exit_block populated from a snapshot or pending sell.",
        ),
        "trade_rollup_matches_position" => (
            "Does each trade row still match its source position?",
            "Compares trade rollups against position state, order identifiers, entry/exit blocks, and protocol so UI read models do not drift from the execution state.",
        ),
        "execution_reports_mirror_trade_events" => (
            "Do execution reports mirror trade event rows?",
            "Every scoped buy/sell execution report should have a matching trade event row with the same trade, order, side, status, and block so the DB event timeline stays auditable.",
        ),
        "single_submitted_event_per_order" => (
            "Does each order have one submitted event before terminal state?",
            "Groups buy/sell trade events by order and rejects missing or duplicate submitted events, plus duplicate terminal confirmed/failed/cancelled events.",
        ),
        "single_terminal_event_per_trade" => (
            "Does each trade have only one terminal buy and sell confirmation?",
            "Counts buy_confirmed and sell_confirmed events per trade and rejects duplicated terminal lifecycle events.",
        ),
        "event_block_order" => (
            "Are lifecycle event blocks ordered correctly?",
            "Validates buy submit <= buy terminal outcome, and for confirmed buys validates buy confirmation <= sell submit <= sell confirmation where those events exist.",
        ),
        "active_hold_limit_submits_exit" => (
            "Did max-hold positions actually submit exits?",
            "Uses persisted position_open_no_exit decisions to catch buy-confirmed positions whose active pool-update count reached max_hold_blocks without any sell submission.",
        ),
        "entry_cost_matches_buy_fill" => (
            "Does entry cost come from the buy simulation fill?",
            "Compares trade entry_cost_eth with the buy_confirmed report's filled ETH amount.",
        ),
        "exit_value_matches_sell_fill" => (
            "Does exit value come from the sell simulation fill?",
            "Compares trade exit_value_eth with the sell_confirmed report's filled ETH amount.",
        ),
        "gas_cost_matches_trade_events" => (
            "Does gas cost come from the execution event stream?",
            "Sums persisted trade event gas_cost_eth values and compares them with the trade gas_cost_eth rollup used in realized PnL.",
        ),
        "failed_sell_gas_has_same_block_snapshot" => (
            "Does failed sell gas have a matching PnL snapshot?",
            "Requires a sell_failed execution with gas cost to persist a same-block sell_failed snapshot so latest PnL can reflect failed-exit gas.",
        ),
        "total_pnl_equals_realized_plus_unrealized" => (
            "Does total PnL reconcile with realized and unrealized PnL?",
            "Checks total_pnl_eth equals realized_pnl_eth plus unrealized_pnl_eth within a small ETH tolerance.",
        ),
        "open_trade_pnl_formula" => (
            "Does open-trade PnL reconcile with entry, current value, and gas?",
            "For open trades, checks realized_pnl_eth equals negative accumulated gas and unrealized_pnl_eth equals current_value_eth minus entry_cost_eth.",
        ),
        "open_snapshot_pnl_formula" => (
            "Do open-state snapshots reconcile with entry, current value, and gas?",
            "For every open-state snapshot, checks realized PnL equals gas accumulated through the valuation block and unrealized PnL equals snapshot current value minus entry cost.",
        ),
        "realized_sell_pnl_formula" => (
            "Does realized PnL reconcile with entry, exit, and gas?",
            "For closed trades, checks realized_pnl_eth equals exit_value_eth minus entry_cost_eth minus gas_cost_eth.",
        ),
        "closed_trade_has_no_unrealized_value" => (
            "Are closed trades fully realized?",
            "Fails if a sell-confirmed trade still carries current_value_eth or unrealized_pnl_eth.",
        ),
        "closed_trade_snapshots_have_no_unrealized_value" => (
            "Do closed-trade snapshots stay fully realized?",
            "Checks every sell_confirmed snapshot, not only the final trade row, so stale post-exit snapshots cannot corrupt aggregate PnL later.",
        ),
        "latest_snapshot_block_matches_snapshots" => (
            "Does the trade latest snapshot pointer match persisted snapshots?",
            "Compares trades.latest_snapshot_block with the max block_number in trade_snapshots for each trade.",
        ),
        "latest_snapshot_values_match_trade" => (
            "Do trade latest fields match the latest snapshot?",
            "Compares latest snapshot block coordinates, current value, realized/unrealized/total PnL, and ROI against the latest trade_snapshot row.",
        ),
        "snapshot_observed_not_after_valuation" => (
            "Are snapshot pool observations available at valuation time?",
            "Rejects snapshots that attach pool observations from a block later than the valuation block.",
        ),
        "no_duplicate_snapshot_coordinates" => (
            "Does each position have one valuation row per trade, block, state, and valuation block?",
            "Rejects duplicate snapshots that make position evolution ambiguous in APIs and frontend timelines.",
        ),
        "no_duplicate_position_snapshot_coordinates" => (
            "Does each raw position snapshot coordinate have only one row?",
            "Applies the duplicate-coordinate invariant to position_snapshots, not only the derived trade_snapshots read model.",
        ),
        "trade_position_snapshots_match" => (
            "Do trade snapshots mirror their source position snapshots?",
            "Checks every trade_snapshot has a matching position_snapshot, and every scoped position_snapshot has a matching trade_snapshot with the same block coordinates and PnL fields.",
        ),
        "zero_value_snapshots_do_not_reuse_stale_pool_metrics" => (
            "Do zero-value exposure snapshots avoid stale pool metrics?",
            "Fails if a display-near-zero open or failed-exit snapshot reuses positive pool liquidity or price metrics from an older/pre-drain pool state.",
        ),
        "terminal_snapshots_have_no_pool_metrics" => (
            "Do terminal closed snapshots omit pool metrics?",
            "Rejects sell-confirmed terminal snapshots that still carry pool price, reserve, liquidity, or denom symbol metadata.",
        ),
        "no_snapshots_after_sell_confirmed" => (
            "Are closed trades no longer receiving snapshots?",
            "Fails if any trade snapshot row is appended after the first sell_confirmed snapshot for the same trade.",
        ),
        "no_open_snapshot_valued_after_sell_confirmed" => (
            "Were open-state snapshots valued only before the sell block?",
            "Fails if a closed trade has any non-sell_confirmed snapshot whose valuation block is later than the sell_confirmed block.",
        ),
        "no_snapshots_before_buy_confirmed" => (
            "Are valuation snapshots only recorded after buy confirmation?",
            "Rejects trade snapshots whose valuation block is before the trade entry block, so active valuation rows cannot precede the buy_confirmed event.",
        ),
        "closed_trade_final_snapshot" => (
            "Does each closed trade have a final closed snapshot?",
            "Requires a sell_confirmed trade snapshot at the exit_block so the UI can show terminal valuation cleanly.",
        ),
        "closed_trade_latest_snapshot_is_terminal" => (
            "Is the latest closed-trade snapshot terminal?",
            "Requires the latest snapshot by block/id for a sell-confirmed trade to be the sell_confirmed snapshot at the exit block.",
        ),
        "closed_trade_replay_inputs_present" => (
            "Can this closed trade be independently replayed?",
            "Requires buy token amount, sell order amount, and sell-confirmed filled amount to be persisted.",
        ),
        "open_position_balance_drain_has_zero_snapshot" => (
            "Does an open position with a mined drain have a zero-value snapshot?",
            "When a mined liquidity-removal or holder-balance backdoor drain hits a position's pool, a zero/near-zero valuation snapshot must exist at or after the drain block; otherwise the position keeps a stale positive mark after its inventory is gone.",
        ),
        "no_positive_open_snapshot_after_drain" => (
            "Are open positions still valued positive after a mined drain?",
            "Fails if an open-state snapshot is valued above zero at or after a mined drain for its pool, which means the valuation used synthetic/entry inventory instead of the current held balance.",
        ),
        "no_positive_value_after_zero_balance" => (
            "Does a position's value flip back to positive after reaching zero?",
            "Once an open position records a zero/near-zero value snapshot after a mined drain, no later non-terminal snapshot may report a positive value; a flip back to positive resurrects a drained position.",
        ),
        "drained_position_reaches_closed_zero_valuation" => (
            "Are drained positions closed at zero value?",
            "When a mined value-destroying risk event (liquidity_removal/scam_confirmed, mined evidence only) hits a position's pool at or before its latest observed block, the position must reach the closed_zero_valuation state (legacy terminal_zero/scammed or a clean sell_confirmed exit are also accepted); a position left non-terminal/open means the drain-close was missed.",
        ),
        _ => (
            "What invariant is this check validating?",
            "Runs a scoped validation query against alpha_trading and reports any violating rows.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::is_blocking_code;

    #[test]
    fn mandated_blocking_checks_are_blocking() {
        for code in [
            // 3 snapshot drain checks
            "open_position_balance_drain_has_zero_snapshot",
            "no_positive_open_snapshot_after_drain",
            "no_positive_value_after_zero_balance",
            // lifecycle drain terminalization
            "drained_position_reaches_closed_zero_valuation",
            // accounting / PnL-identity checks
            "total_pnl_equals_realized_plus_unrealized",
            "open_trade_pnl_formula",
            "open_snapshot_pnl_formula",
            "realized_sell_pnl_formula",
            "closed_trade_has_no_unrealized_value",
            "closed_trade_snapshots_have_no_unrealized_value",
            "entry_cost_matches_buy_fill",
            "exit_value_matches_sell_fill",
            "gas_cost_matches_trade_events",
            // lifecycle ordering / no-duplicate-terminal / active-hold-exit
            "event_block_order",
            "single_terminal_event_per_trade",
            "single_submitted_event_per_order",
            "active_hold_limit_submits_exit",
            // execution-delay & terminal-report presence
            "terminal_report_matches_execution_delay",
            "submitted_orders_have_terminal_report_after_delay",
            // metadata result-set / runtime-status consistency
            "result_set_status",
            "live_runtime_status_matches_result_set",
            "running_result_set_has_no_stop_marker",
        ] {
            assert!(is_blocking_code(code), "{code} must be promotion-blocking");
        }
    }

    #[test]
    fn coverage_and_informational_checks_are_advisory() {
        assert!(!is_blocking_code("tail_entry_coverage"));
        assert!(!is_blocking_code("strategy_rows"));
    }
}
