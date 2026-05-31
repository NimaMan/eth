use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BroadcastMode {
    DryRun,
    Broadcast,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthTxExecutorConfig {
    pub chain_id: u64,
    pub rpc_url: String,
    pub broadcast_mode: BroadcastMode,
    pub max_priority_fee_per_gas_wei: u128,
    pub max_fee_per_gas_wei: u128,
    pub local_journal_path: Option<PathBuf>,
}

impl EthTxExecutorConfig {
    pub fn mainnet_local_reth(rpc_url: impl Into<String>) -> Self {
        Self {
            chain_id: 1,
            rpc_url: rpc_url.into(),
            broadcast_mode: BroadcastMode::DryRun,
            max_priority_fee_per_gas_wei: 500_000_000_000,
            max_fee_per_gas_wei: 1_000_000_000_000,
            local_journal_path: None,
        }
    }
}
