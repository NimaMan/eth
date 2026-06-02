//! Reconcile ledger-expected balances against actual on-chain `balanceOf`.
//!
//! The ledger ([`super::ledger`]) is built from `Transfer` events, so it cannot
//! see an event-less backdoor that moves a holder's balance without emitting a
//! `Transfer` (the Session case). The real `balanceOf` from state *does* reflect
//! it. So a holder whose actual balance is materially below the ledger
//! expectation — with no Transfer to explain the drop — is a realized custody
//! confiscation victim.
//!
//! The balance reader is abstracted via [`HolderBalanceReader`] so this module
//! stays free of any chain-provider dependency; callers (services, examples)
//! implement it over `RethQueryProvider::get_token_balance` or the live
//! simulator.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::ledger::HolderBalanceLedger;

/// Reads the actual on-chain `balanceOf(holder)` at a block, scaled to token
/// decimals (matching the ledger's units).
#[async_trait]
pub trait HolderBalanceReader {
    async fn balance_of(&self, holder: &str, block: u64) -> eyre::Result<f64>;
}

/// Tuning for what counts as a confiscation.
#[derive(Clone, Copy, Debug)]
pub struct ReconcileConfig {
    /// Ignore holders whose ledger-expected balance is below this (dust).
    pub min_expected_balance: f64,
    /// A holder is drained when `actual <= expected * (1 - min_drop_fraction)`.
    /// e.g. 0.9 ⇒ flag when ≥90% of the expected balance is unexplainedly gone.
    pub min_drop_fraction: f64,
}

impl Default for ReconcileConfig {
    fn default() -> Self {
        Self {
            min_expected_balance: 0.0,
            min_drop_fraction: 0.9,
        }
    }
}

/// A holder whose actual balance fell materially below the ledger expectation
/// without a Transfer to explain it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustodyDrainVictim {
    pub holder: String,
    pub block_number: u64,
    pub expected_balance: f64,
    pub actual_balance: f64,
    /// expected - actual (the unexplained confiscated amount).
    pub missing_balance: f64,
    /// fraction of the expected balance that disappeared, in [0.0, 1.0].
    pub drained_fraction: f64,
    /// expected share of circulating supply before the drain, in [0.0, 1.0].
    pub expected_supply_share: f64,
}

/// Compare every material holder's ledger-expected balance to actual on-chain
/// `balanceOf` at `block`, returning the confiscation victims.
pub async fn reconcile_custody_drain<R: HolderBalanceReader>(
    ledger: &HolderBalanceLedger,
    reader: &R,
    block: u64,
    config: &ReconcileConfig,
) -> eyre::Result<Vec<CustodyDrainVictim>> {
    let mut victims = Vec::new();
    for (holder, expected) in ledger.material_holders(config.min_expected_balance) {
        if expected <= 0.0 {
            continue;
        }
        let actual = reader.balance_of(&holder, block).await?;
        let threshold = expected * (1.0 - config.min_drop_fraction);
        if actual <= threshold {
            let missing = (expected - actual).max(0.0);
            victims.push(CustodyDrainVictim {
                expected_supply_share: ledger.share_of_supply(&holder),
                holder,
                block_number: block,
                expected_balance: expected,
                actual_balance: actual,
                missing_balance: missing,
                drained_fraction: (missing / expected).clamp(0.0, 1.0),
            });
        }
    }
    Ok(victims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MapReader(HashMap<String, f64>);

    #[async_trait]
    impl HolderBalanceReader for MapReader {
        async fn balance_of(&self, holder: &str, _block: u64) -> eyre::Result<f64> {
            Ok(self
                .0
                .get(&holder.trim().to_ascii_lowercase())
                .copied()
                .unwrap_or(0.0))
        }
    }

    #[tokio::test]
    async fn flags_event_less_drain_victim() {
        let mut ledger = HolderBalanceLedger::default();
        ledger.apply_transfer(
            "0x0000000000000000000000000000000000000000",
            "0xvault",
            9_871_580.0,
        );
        ledger.apply_transfer(
            "0x0000000000000000000000000000000000000000",
            "0xclean",
            1_000.0,
        );
        // State: vault was drained to 93 (no Transfer event); clean holder intact.
        let reader = MapReader(HashMap::from([
            ("0xvault".to_string(), 93.0),
            ("0xclean".to_string(), 1_000.0),
        ]));
        let victims =
            reconcile_custody_drain(&ledger, &reader, 25_202_411, &ReconcileConfig::default())
                .await
                .unwrap();
        assert_eq!(victims.len(), 1);
        assert_eq!(victims[0].holder, "0xvault");
        assert!(victims[0].drained_fraction > 0.99);
        assert!((victims[0].actual_balance - 93.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn does_not_flag_intact_holders() {
        let mut ledger = HolderBalanceLedger::default();
        ledger.apply_transfer("0x0000000000000000000000000000000000000000", "0xa", 500.0);
        let reader = MapReader(HashMap::from([("0xa".to_string(), 500.0)]));
        let victims = reconcile_custody_drain(&ledger, &reader, 1, &ReconcileConfig::default())
            .await
            .unwrap();
        assert!(victims.is_empty());
    }
}
