use serde::{Deserialize, Serialize};

use crate::RunMetadata;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyReport {
    pub run: RunMetadata,
    pub summary: RunSummary,
    pub concentration: PnlConcentration,
    pub issue_flags: Vec<IssueFlag>,
    pub failures: Vec<FailureBucket>,
    pub buy_failed_entries: Vec<BuyFailedEntry>,
    pub open_failed_exits: Vec<OpenFailedExit>,
    pub protocols: Vec<ProtocolBucket>,
    pub top_winners: Vec<PositionRank>,
    pub worst_losers: Vec<PositionRank>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RunSummary {
    pub positions: i64,
    pub open_positions: i64,
    pub failed_positions: i64,
    pub buy_failed_positions: i64,
    pub sell_failed_positions: i64,
    pub entry_cost_eth: String,
    pub gas_cost_eth: String,
    pub execution_reports: i64,
    pub confirmed_reports: i64,
    pub failed_reports: i64,
    pub snapshots: i64,
    pub snapshot_positions: i64,
    pub open_without_snapshot: i64,
    pub zero_decimal_nonzero_raw: i64,
    pub latest_current_value_eth: String,
    pub realized_pnl_eth: String,
    pub unrealized_pnl_eth: String,
    pub total_pnl_eth: String,
    pub total_roi_on_open_cost: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PnlConcentration {
    pub snapshot_positions: i64,
    pub total_pnl_eth: String,
    pub pnl_ex_top1_eth: String,
    pub pnl_ex_top2_eth: String,
    pub pnl_ex_top5_eth: String,
    pub pnl_ex_top10_eth: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IssueFlag {
    pub severity: String,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FailureBucket {
    pub error_class: String,
    pub reports: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuyFailedEntry {
    pub token_address: String,
    pub pool_address: String,
    pub failed_block: Option<i64>,
    pub protocol: String,
    pub denom_symbol: String,
    pub observed_can_buy: Option<bool>,
    pub observed_can_sell: Option<bool>,
    pub error_class: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OpenFailedExit {
    pub token_address: String,
    pub pool_address: String,
    pub entry_block: Option<i64>,
    pub failed_reports: i64,
    pub first_failed_block: Option<i64>,
    pub last_failed_block: Option<i64>,
    pub latest_snapshot_block: Option<i64>,
    pub current_value_eth: Option<String>,
    pub pnl_eth: Option<String>,
    pub error_class: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolBucket {
    pub protocol: String,
    pub denom_symbol: String,
    pub confirmed_buys: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PositionRank {
    pub token_address: String,
    pub pool_address: String,
    pub entry_block: Option<i64>,
    pub entry_cost_eth: Option<String>,
    pub entry_token_amount: Option<String>,
    pub latest_snapshot_block: Option<i64>,
    pub current_value_eth: Option<String>,
    pub pnl_eth: Option<String>,
    pub roi: Option<String>,
}
