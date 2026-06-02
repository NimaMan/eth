use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScammerCaseConfig {
    pub case_id: String,
    pub title: String,
    pub status: String,
    pub chain: String,
    pub suspect_address: String,
    pub token_address: Option<String>,
    pub pool_address: Option<String>,
    pub denom_address: Option<String>,
    pub forwarder_address: Option<String>,
    pub key_txs: KeyTransactionConfig,
    #[serde(default)]
    pub staged_tokens: Vec<StagedTokenConfig>,
    #[serde(default)]
    pub forward_txs: Vec<String>,
    #[serde(default)]
    pub links: BTreeMap<String, String>,
}

impl ScammerCaseConfig {
    pub fn ordered_transactions(&self) -> Vec<(String, String)> {
        let mut seen = BTreeSet::new();
        let mut txs = Vec::new();

        for (label, tx_hash) in self.key_txs.ordered() {
            push_unique_tx(&mut seen, &mut txs, label, tx_hash);
        }

        for token in &self.staged_tokens {
            for (label, tx_hash) in token.ordered() {
                push_unique_tx(
                    &mut seen,
                    &mut txs,
                    format!("staged_token:{}:{label}", token.token_address),
                    tx_hash,
                );
            }
        }

        for tx_hash in &self.forward_txs {
            push_unique_tx(
                &mut seen,
                &mut txs,
                "forwarder_distribution".to_string(),
                tx_hash.clone(),
            );
        }

        txs
    }
}

fn push_unique_tx(
    seen: &mut BTreeSet<String>,
    txs: &mut Vec<(String, String)>,
    label: String,
    tx_hash: String,
) {
    let normalized = tx_hash.to_ascii_lowercase();
    if seen.insert(normalized) {
        txs.push((label, tx_hash));
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KeyTransactionConfig {
    pub token_deploy: Option<String>,
    pub token_fund: Option<String>,
    pub open_trading: Option<String>,
    pub lp_approval: Option<String>,
    pub liquidity_removal: Option<String>,
    pub victim_buy: Option<String>,
    pub balance_drain: Option<String>,
    pub manual_exit: Option<String>,
}

impl KeyTransactionConfig {
    pub fn ordered(&self) -> Vec<(String, String)> {
        let mut txs = Vec::new();
        push_optional(&mut txs, "token_deploy", &self.token_deploy);
        push_optional(&mut txs, "token_fund", &self.token_fund);
        push_optional(&mut txs, "open_trading", &self.open_trading);
        push_optional(&mut txs, "lp_approval", &self.lp_approval);
        push_optional(&mut txs, "liquidity_removal", &self.liquidity_removal);
        push_optional(&mut txs, "victim_buy", &self.victim_buy);
        push_optional(&mut txs, "balance_drain", &self.balance_drain);
        push_optional(&mut txs, "manual_exit", &self.manual_exit);
        txs
    }
}

fn push_optional(txs: &mut Vec<(String, String)>, label: &str, tx_hash: &Option<String>) {
    if let Some(tx_hash) = tx_hash {
        txs.push((label.to_string(), tx_hash.clone()));
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StagedTokenConfig {
    pub token_address: String,
    pub deploy_tx: Option<String>,
    pub token_transfer_tx: Option<String>,
    pub eth_fund_tx: Option<String>,
    pub note: Option<String>,
}

impl StagedTokenConfig {
    pub fn ordered(&self) -> Vec<(String, String)> {
        let mut txs = Vec::new();
        push_optional(&mut txs, "deploy", &self.deploy_tx);
        push_optional(&mut txs, "token_transfer", &self.token_transfer_tx);
        push_optional(&mut txs, "eth_fund", &self.eth_fund_tx);
        txs
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ScammerCaseReport {
    pub config: ScammerCaseConfig,
    pub summary: ScammerCaseSummary,
    pub transactions: Vec<TransactionEvidence>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ScammerCaseSummary {
    pub transaction_count: usize,
    pub failed_transaction_count: usize,
    pub total_creator_direct_eth_in: f64,
    pub total_creator_direct_eth_out: f64,
    pub total_forwarded_eth: f64,
    pub forwarder_trace_count: usize,
    pub unique_forwarder_sinks: usize,
    pub unique_token_contracts_touched: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionEvidence {
    pub label: String,
    pub tx_hash: String,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub tx_index: u64,
    pub status: bool,
    pub nonce: u64,
    pub from_address: String,
    pub to_address: Option<String>,
    pub contract_address: Option<String>,
    pub value_wei: String,
    pub value_eth: f64,
    pub gas_cost_eth: f64,
    pub tx_type: String,
    pub actions: Vec<String>,
    pub eth_movements: Vec<EthMovementEvidence>,
    pub token_movements: Vec<TokenMovementEvidence>,
    pub forwarder_trace: Option<ForwarderTrace>,
}

#[derive(Clone, Debug, Serialize)]
pub struct EthMovementEvidence {
    pub from_address: String,
    pub to_address: String,
    pub amount_wei: String,
    pub amount_eth: f64,
    pub movement_type: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TokenMovementEvidence {
    pub token_address: String,
    pub from_address: String,
    pub to_address: String,
    pub amount_raw: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ForwarderTrace {
    pub pattern: String,
    pub forwarder_address: String,
    pub ephemeral_address: Option<String>,
    pub sink_address: String,
    pub amount_wei: String,
    pub amount_eth: f64,
    pub evidence: Vec<String>,
}
