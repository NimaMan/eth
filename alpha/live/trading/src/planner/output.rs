use serde::{Deserialize, Serialize};

use crate::{LiveTraderTxSignal, PriorityFeeBudget, TxPrepReject};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum PrioritySellPlannerOutcome {
    Submit {
        signal: LiveTraderTxSignal,
        budget: PriorityFeeBudget,
    },
    Reject(TxPrepReject),
}
