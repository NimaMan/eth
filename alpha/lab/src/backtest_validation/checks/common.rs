use eyre::Result;
use serde_json::json;
use sqlx::PgPool;

use super::super::db::scalar_i64;
use super::super::report::{CheckResult, Verdict};

const TOLERANCE_ETH: &str = "0.000000000000001";

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
    CheckResult {
        category: category.into(),
        code,
        question: question.to_string(),
        description: description.to_string(),
        verdict,
        message: message.into(),
        evidence,
    }
}

fn check_copy(code: &str) -> (&'static str, &'static str) {
    match code {
        "result_set_status" => (
            "Is this result set in the expected state?",
            "Compares result-set mode with status so completed historical results and running live results are not mixed with partial runs.",
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
        "market_buy_has_historical_observation" => (
            "Can every historical market buy be traced to an input observation?",
            "Joins market-sourced submit_buy decisions to the replay Risk Atlas observation at the same token, pool, and block.",
        ),
        "submit_decisions_within_result_range" => (
            "Were submitted decisions made inside the replayed input range?",
            "Rejects submitted buy/sell decisions whose decision block is outside the result set start/end blocks.",
        ),
        "terminal_report_matches_execution_delay" => (
            "Do fills land at the configured execution delay?",
            "Checks each order's terminal confirmed/failed/cancelled report block equals submitted_block + execution_delay_blocks.",
        ),
        "confirmed_reports_have_simulation_outputs" => (
            "Are confirmed fills backed by persisted EVM simulation output?",
            "Requires filled amount, gas, gas cost, and buy token output on confirmed trade events.",
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
        "single_terminal_event_per_trade" => (
            "Does each trade have only one terminal buy and sell confirmation?",
            "Counts buy_confirmed and sell_confirmed events per trade and rejects duplicated terminal lifecycle events.",
        ),
        "event_block_order" => (
            "Are lifecycle event blocks ordered correctly?",
            "Validates buy submit <= buy terminal outcome, and for confirmed buys validates buy confirmation <= sell submit <= sell confirmation where those events exist.",
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
        "total_pnl_equals_realized_plus_unrealized" => (
            "Does total PnL reconcile with realized and unrealized PnL?",
            "Checks total_pnl_eth equals realized_pnl_eth plus unrealized_pnl_eth within a small ETH tolerance.",
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
        "zero_value_snapshots_do_not_reuse_stale_pool_metrics" => (
            "Do zero-value exposure snapshots avoid stale pool metrics?",
            "Fails if a zero-value open or failed-exit snapshot still displays positive pool liquidity or price metrics from an older/pre-drain pool state.",
        ),
        "no_snapshots_after_sell_confirmed" => (
            "Are closed trades no longer receiving snapshots?",
            "Fails if any trade snapshot row is appended after the first sell_confirmed snapshot for the same trade.",
        ),
        "no_open_snapshot_valued_after_sell_confirmed" => (
            "Were open-state snapshots valued only before the sell block?",
            "Fails if a closed trade has any non-sell_confirmed snapshot whose valuation block is later than the sell_confirmed block.",
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
        "top5_pnl_concentration" => (
            "Is strategy PnL too concentrated in the top winners?",
            "Warns when the top five winners exceed 100% of aggregate PnL, making strategy conclusions fragile.",
        ),
        _ => (
            "What invariant is this check validating?",
            "Runs a scoped validation query against alpha_trading and reports any violating rows.",
        ),
    }
}
