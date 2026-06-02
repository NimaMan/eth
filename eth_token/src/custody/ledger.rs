//! Per-holder balance reconstructed from `Transfer` events ("transfer data").
//!
//! This is the *expected* balance: what each holder would hold if every balance
//! change were a normal, event-emitting transfer. It is the baseline the
//! [`super::reconciliation`] step compares against the real on-chain
//! `balanceOf`. A material gap (expected high, actual ~0, no Transfer to explain
//! it) is the signature of an event-less custody confiscation.
//!
//! Mints (`from` = zero/burn) credit the recipient; burns (`to` = zero/dead)
//! debit the sender. Burn addresses are not treated as holders.

use std::collections::HashMap;

use reth_chain_query::common_addresses::is_burn_address_str;

use crate::erc20::ERC20Token;

/// Expected per-holder balances reconstructed from a token's Transfer events.
#[derive(Clone, Debug, Default)]
pub struct HolderBalanceLedger {
    /// normalized holder address -> expected balance (token-decimal scaled).
    balances: HashMap<String, f64>,
    /// circulating supply implied by mints minus burns (scaled).
    circulating_supply: f64,
}

impl HolderBalanceLedger {
    /// Build the ledger from all of a token's recorded Transfer events.
    pub fn from_token(token: &ERC20Token) -> Self {
        Self::from_token_through_block(token, u64::MAX)
    }

    /// Build the ledger from Transfer events up to and including `block`.
    pub fn from_token_through_block(token: &ERC20Token, block: u64) -> Self {
        let mut ledger = Self::default();
        for records in token.transfer_tracker.erc20_transfers.values() {
            for record in records.iter().filter(|record| record.block_number <= block) {
                ledger.apply_transfer(&record.from_address, &record.to_address, record.amount);
            }
        }
        ledger
    }

    /// Apply a single Transfer: debit `from` (unless mint), credit `to` (unless
    /// burn), and track circulating supply.
    pub fn apply_transfer(&mut self, from: &str, to: &str, amount: f64) {
        if !amount.is_finite() || amount <= 0.0 {
            return;
        }
        let from_burn = is_burn_address_str(from);
        let to_burn = is_burn_address_str(to);

        if from_burn && !to_burn {
            self.circulating_supply += amount;
        } else if to_burn && !from_burn {
            self.circulating_supply -= amount;
        }

        if !from_burn {
            *self.balances.entry(normalize(from)).or_default() -= amount;
        }
        if !to_burn {
            *self.balances.entry(normalize(to)).or_default() += amount;
        }
    }

    /// Expected balance for a holder (0.0 if unknown).
    pub fn expected_balance(&self, holder: &str) -> f64 {
        self.balances
            .get(&normalize(holder))
            .copied()
            .unwrap_or(0.0)
    }

    /// Circulating supply implied by the transfer history (mints minus burns).
    pub fn circulating_supply(&self) -> f64 {
        self.circulating_supply.max(0.0)
    }

    /// Holder's expected share of circulating supply, in [0.0, 1.0].
    pub fn share_of_supply(&self, holder: &str) -> f64 {
        let supply = self.circulating_supply();
        if supply <= 0.0 {
            return 0.0;
        }
        (self.expected_balance(holder) / supply).clamp(0.0, 1.0)
    }

    /// Holders with at least `min_balance` expected tokens, descending by
    /// balance. Used to scope reconciliation to material positions.
    pub fn material_holders(&self, min_balance: f64) -> Vec<(String, f64)> {
        let mut holders = self
            .balances
            .iter()
            .filter(|(_, balance)| **balance >= min_balance)
            .map(|(holder, balance)| (holder.clone(), *balance))
            .collect::<Vec<_>>();
        holders.sort_by(|left, right| {
            right
                .1
                .partial_cmp(&left.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        holders
    }
}

fn normalize(address: &str) -> String {
    address.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mint_credits_recipient_and_supply() {
        let mut ledger = HolderBalanceLedger::default();
        ledger.apply_transfer(
            "0x0000000000000000000000000000000000000000",
            "0xholderA",
            100.0,
        );
        assert_eq!(ledger.expected_balance("0xHolderA"), 100.0);
        assert_eq!(ledger.circulating_supply(), 100.0);
        assert!((ledger.share_of_supply("0xholdera") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn transfer_moves_balance_between_holders() {
        let mut ledger = HolderBalanceLedger::default();
        ledger.apply_transfer("0x0000000000000000000000000000000000000000", "0xa", 100.0);
        ledger.apply_transfer("0xa", "0xb", 40.0);
        assert_eq!(ledger.expected_balance("0xa"), 60.0);
        assert_eq!(ledger.expected_balance("0xb"), 40.0);
        // supply unchanged by a holder-to-holder move
        assert_eq!(ledger.circulating_supply(), 100.0);
    }

    #[test]
    fn burn_debits_supply_and_holder() {
        let mut ledger = HolderBalanceLedger::default();
        ledger.apply_transfer("0x0000000000000000000000000000000000000000", "0xa", 100.0);
        ledger.apply_transfer("0xa", "0x000000000000000000000000000000000000dEaD", 30.0);
        assert_eq!(ledger.expected_balance("0xa"), 70.0);
        assert_eq!(ledger.circulating_supply(), 70.0);
    }

    #[test]
    fn material_holders_filters_and_sorts() {
        let mut ledger = HolderBalanceLedger::default();
        ledger.apply_transfer("0x0000000000000000000000000000000000000000", "0xbig", 100.0);
        ledger.apply_transfer("0x0000000000000000000000000000000000000000", "0xmid", 10.0);
        ledger.apply_transfer("0x0000000000000000000000000000000000000000", "0xdust", 0.5);
        let holders = ledger.material_holders(1.0);
        assert_eq!(holders.len(), 2);
        assert_eq!(holders[0].0, "0xbig");
        assert_eq!(holders[1].0, "0xmid");
    }
}
