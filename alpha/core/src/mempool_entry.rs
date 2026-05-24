use crate::{
    amount::DecimalAmount,
    ids::{BlockNumber, PoolAddress, TokenAddress},
    market::{PoolProtocol, PoolSnapshot},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MEMPOOL_ENTRY_EVIDENCE_KEY: &str = "mempool_entry_evidence";
pub const MEMPOOL_ENTRY_EVIDENCE_VERSION: &str = "mempool_entry_evidence_v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MempoolEntryEvidence {
    #[serde(default = "default_evidence_version")]
    pub evidence_version: String,
    pub base_block: BlockNumber,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simulated_block: Option<BlockNumber>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simulated_at: Option<String>,
    #[serde(default)]
    pub dependency_tx_hashes: Vec<String>,
    #[serde(default)]
    pub dependency_fee_metadata: Value,
    pub projected_pool: MempoolProjectedPool,
    pub viability: MempoolEntryViability,
    pub vault_buy_simulation: MempoolVaultBuySimulation,
    #[serde(default)]
    pub strategy_neutral_flags: Value,
    #[serde(default)]
    pub audit: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MempoolProjectedPool {
    pub protocol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denom_address: Option<TokenAddress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denom_symbol: Option<String>,
    pub denom_reserve: DecimalAmount,
    pub token_reserve: DecimalAmount,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_denom_per_token: Option<DecimalAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_price_denom_per_token: Option<DecimalAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_ratio_to_initial: Option<DecimalAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation_block: Option<BlockNumber>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_decimals: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_block: Option<BlockNumber>,
    #[serde(default)]
    pub can_buy: bool,
    #[serde(default)]
    pub can_sell: bool,
    #[serde(default)]
    pub is_scam: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MempoolEntryViability {
    pub can_buy: bool,
    pub can_approve: bool,
    pub can_sell: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buy_tax_percent: Option<DecimalAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sell_tax_percent: Option<DecimalAmount>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MempoolVaultBuySimulation {
    pub route: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_address: Option<TokenAddress>,
    #[serde(default)]
    pub would_revert: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eth_spent_wei: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_received_raw: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

impl MempoolEntryEvidence {
    pub fn from_risk_evidence(value: &Value) -> Option<serde_json::Result<Self>> {
        value
            .get(MEMPOOL_ENTRY_EVIDENCE_KEY)
            .cloned()
            .map(serde_json::from_value)
    }

    pub fn entry_block(&self) -> BlockNumber {
        self.projected_pool
            .latest_block
            .or(self.simulated_block)
            .unwrap_or(self.base_block)
    }

    pub fn to_projected_pool_snapshot(
        &self,
        token_address: TokenAddress,
        pool_address: PoolAddress,
    ) -> PoolSnapshot {
        PoolSnapshot {
            address: pool_address,
            token_address,
            protocol: PoolProtocol::from_label(&self.projected_pool.protocol),
            denom_address: self.projected_pool.denom_address,
            denom_symbol: self.projected_pool.denom_symbol.clone(),
            denom_reserve: self.projected_pool.denom_reserve,
            token_reserve: self.projected_pool.token_reserve,
            price_denom_per_token: self.projected_pool.price_denom_per_token,
            initial_price_denom_per_token: self.projected_pool.initial_price_denom_per_token,
            price_ratio_to_initial: self.projected_pool.price_ratio_to_initial,
            creation_block: self.projected_pool.creation_block,
            token_decimals: self.projected_pool.token_decimals,
            fee_tier: None,
            uniswap_v4: None,
            latest_block: self.entry_block(),
            can_buy: self.viability.can_buy && self.projected_pool.can_buy,
            can_sell: self.viability.can_sell && self.projected_pool.can_sell,
            is_scam: self.projected_pool.is_scam,
        }
    }

    pub fn has_successful_exact_vault_buy(&self) -> bool {
        self.vault_buy_simulation.route == "uniswap_v2_trading_vault"
            && !self.vault_buy_simulation.would_revert
            && self.vault_buy_simulation.gas_used.unwrap_or_default() > 0
            && self
                .vault_buy_simulation
                .eth_spent_wei
                .as_deref()
                .map(|value| value != "0")
                .unwrap_or(false)
            && self
                .vault_buy_simulation
                .tokens_received_raw
                .as_deref()
                .map(|value| value != "0")
                .unwrap_or(false)
    }
}

fn default_evidence_version() -> String {
    MEMPOOL_ENTRY_EVIDENCE_VERSION.to_string()
}
