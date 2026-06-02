//! Custody Risk: does the contract/owner retain privileged power over the
//! tokens a holder already owns?
//!
//! This is a distinct axis from `can_sell`. `can_sell` answers "does the swap
//! path work for a holder?" — a honeypot fails it. Custody Risk answers "can the
//! tokens you hold be frozen, seized, burned, or clawed back after you buy?" The
//! Session case (`holder_balance_backdoor_drain`) is `can_sell = true` while the
//! vault's balance was confiscated to dead — the two axes are orthogonal.
//!
//! Each capability has two states:
//! - `Latent`: the power exists (privileged function / active owner) but has not
//!   been used. Predictive; harder to prove.
//! - `Realized`: the power was observed firing against a holder. Confirmed; this
//!   is what the first version detects.
//!
//! Detection layers:
//! - [`ledger`]: per-holder *expected* balance reconstructed from `Transfer`
//!   events (transfer data). Catches balance moves that *do* emit events.
//! - [`reconciliation`]: compares ledger-expected balances to actual on-chain
//!   `balanceOf` from state. The unexplained negative discrepancy is the
//!   unforgeable signature of an event-less confiscation (the Session backdoor).

pub mod ledger;
pub mod reconciliation;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use ledger::HolderBalanceLedger;
pub use reconciliation::{
    reconcile_custody_drain, CustodyDrainVictim, HolderBalanceReader, ReconcileConfig,
};

/// A specific power the contract retains over holder balances.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustodyCapability {
    /// Move a holder's balance to another address (steal).
    Seize,
    /// Block a holder from transferring (blacklist / pause / freeze).
    Freeze,
    /// Drain/burn a holder's balance to a dead address. The Session case.
    BurnDrain,
    /// Reclaim previously-distributed tokens.
    Clawback,
}

impl CustodyCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Seize => "custody_seize",
            Self::Freeze => "custody_freeze",
            Self::BurnDrain => "custody_burn_drain",
            Self::Clawback => "custody_clawback",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Seize => "Custody Seize",
            Self::Freeze => "Custody Freeze",
            Self::BurnDrain => "Custody Burn / Drain",
            Self::Clawback => "Custody Clawback",
        }
    }
}

/// Whether the capability is only possible (`Latent`) or has been observed
/// firing against a holder (`Realized`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustodyState {
    Latent,
    Realized,
}

impl CustodyState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Latent => "latent",
            Self::Realized => "realized",
        }
    }
}

/// A custody-risk finding for a token/pool: which power, what state, and the
/// supporting evidence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustodyFinding {
    pub capability: CustodyCapability,
    pub state: CustodyState,
    pub block_number: Option<u64>,
    pub evidence: Value,
}
