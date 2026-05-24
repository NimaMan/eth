use crate::{
    amount::DecimalAmount,
    ids::{BlockNumber, PoolAddress, TokenAddress},
    market::{PoolProtocol, PoolSnapshot},
    risk::{RiskEvent, RiskKind},
};
use alloy_primitives::U256;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MempoolVaultBuyEvidenceQuality {
    SuccessfulExactVault,
    RevertingExactVault,
    IncompleteExactVault,
    GenericProbe,
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
        self.vault_buy_evidence_quality() == MempoolVaultBuyEvidenceQuality::SuccessfulExactVault
    }

    pub fn vault_buy_evidence_quality(&self) -> MempoolVaultBuyEvidenceQuality {
        if self.vault_buy_simulation.route != "uniswap_v2_trading_vault" {
            return MempoolVaultBuyEvidenceQuality::GenericProbe;
        }
        if self
            .vault_buy_simulation
            .metadata
            .get("exact_vault_calldata")
            != Some(&Value::Bool(true))
        {
            return MempoolVaultBuyEvidenceQuality::IncompleteExactVault;
        }
        if self.vault_buy_simulation.would_revert {
            return MempoolVaultBuyEvidenceQuality::RevertingExactVault;
        }
        if self.vault_buy_simulation.gas_used.unwrap_or_default() == 0
            || !raw_amount_is_nonzero(self.vault_buy_simulation.eth_spent_wei.as_deref())
            || !raw_amount_is_nonzero(self.vault_buy_simulation.tokens_received_raw.as_deref())
        {
            return MempoolVaultBuyEvidenceQuality::IncompleteExactVault;
        }
        MempoolVaultBuyEvidenceQuality::SuccessfulExactVault
    }
}

fn raw_amount_is_nonzero(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return false;
    };
    value
        .trim()
        .parse::<U256>()
        .map(|amount| amount > U256::ZERO)
        .unwrap_or(false)
}

pub fn projected_pool_from_risk_event(event: &RiskEvent) -> Option<PoolSnapshot> {
    if event.kind != RiskKind::TradingEnabled {
        return None;
    }
    let pool_address = event.pool_address.clone()?;
    let evidence_value = event.evidence.as_ref()?;
    let evidence = MempoolEntryEvidence::from_risk_evidence(evidence_value)?.ok()?;
    if evidence.evidence_version != MEMPOOL_ENTRY_EVIDENCE_VERSION {
        return None;
    }
    Some(evidence.to_projected_pool_snapshot(event.token_address, pool_address))
}

fn default_evidence_version() -> String {
    MEMPOOL_ENTRY_EVIDENCE_VERSION.to_string()
}

#[cfg(test)]
mod tests {
    use alloy_primitives::address;
    use serde_json::json;

    use crate::{
        ids::TokenPoolId,
        risk::{RiskEvent, RiskKind, RiskSeverity},
    };

    use super::{
        projected_pool_from_risk_event, MempoolEntryEvidence, MempoolVaultBuyEvidenceQuality,
        MEMPOOL_ENTRY_EVIDENCE_KEY,
    };

    #[test]
    fn projected_pool_from_trading_enabled_risk_event_uses_mempool_evidence() {
        let token_address = address!("1111111111111111111111111111111111111111");
        let pool_address =
            TokenPoolId::new(token_address, "0x2222222222222222222222222222222222222222");
        let event = RiskEvent {
            kind: RiskKind::TradingEnabled,
            severity: RiskSeverity::Info,
            source: None,
            token_address,
            pool_address: Some(pool_address.clone()),
            pending_tx_hash: None,
            observed_block: Some(12),
            message: "trading enabled".to_string(),
            evidence: Some(json!({
                MEMPOOL_ENTRY_EVIDENCE_KEY: {
                    "evidence_version": "mempool_entry_evidence_v1",
                    "base_block": 12,
                    "simulated_block": 12,
                    "projected_pool": {
                        "protocol": "UNISWAP-V2",
                        "denom_reserve": "1",
                        "token_reserve": "100",
                        "creation_block": 11,
                        "latest_block": 12,
                        "can_buy": true,
                        "can_sell": true,
                        "is_scam": false
                    },
                    "viability": {
                        "can_buy": true,
                        "can_approve": true,
                        "can_sell": true
                    },
                    "vault_buy_simulation": {
                        "route": "pool_buy_sell_probe",
                        "would_revert": false,
                        "gas_used": 176000,
                        "eth_spent_wei": "10000000000000000",
                        "tokens_received_raw": "1000000"
                    }
                }
            })),
        };

        let pool = projected_pool_from_risk_event(&event).expect("projected pool");

        assert_eq!(pool.address, pool_address);
        assert_eq!(pool.latest_block, 12);
        assert!(pool.can_buy);
        assert!(pool.can_sell);
    }

    #[test]
    fn exact_vault_buy_quality_requires_explicit_exact_calldata_metadata() {
        let evidence: MempoolEntryEvidence = serde_json::from_value(json!({
            "evidence_version": "mempool_entry_evidence_v1",
            "base_block": 12,
            "projected_pool": {
                "protocol": "UNISWAP-V2",
                "denom_reserve": "1",
                "token_reserve": "100",
                "can_buy": true,
                "can_sell": true
            },
            "viability": {
                "can_buy": true,
                "can_approve": true,
                "can_sell": true
            },
            "vault_buy_simulation": {
                "route": "uniswap_v2_trading_vault",
                "would_revert": false,
                "gas_used": 176000,
                "eth_spent_wei": "10000000000000000",
                "tokens_received_raw": "1000000",
                "metadata": {
                    "exact_vault_calldata": true
                }
            }
        }))
        .unwrap();

        assert_eq!(
            evidence.vault_buy_evidence_quality(),
            MempoolVaultBuyEvidenceQuality::SuccessfulExactVault
        );
        assert!(evidence.has_successful_exact_vault_buy());
    }

    #[test]
    fn route_alone_is_not_exact_vault_buy_quality() {
        let evidence: MempoolEntryEvidence = serde_json::from_value(json!({
            "evidence_version": "mempool_entry_evidence_v1",
            "base_block": 12,
            "projected_pool": {
                "protocol": "UNISWAP-V2",
                "denom_reserve": "1",
                "token_reserve": "100",
                "can_buy": true,
                "can_sell": true
            },
            "viability": {
                "can_buy": true,
                "can_approve": true,
                "can_sell": true
            },
            "vault_buy_simulation": {
                "route": "uniswap_v2_trading_vault",
                "would_revert": false,
                "gas_used": 176000,
                "eth_spent_wei": "10000000000000000",
                "tokens_received_raw": "1000000"
            }
        }))
        .unwrap();

        assert_eq!(
            evidence.vault_buy_evidence_quality(),
            MempoolVaultBuyEvidenceQuality::IncompleteExactVault
        );
        assert!(!evidence.has_successful_exact_vault_buy());
    }
}
