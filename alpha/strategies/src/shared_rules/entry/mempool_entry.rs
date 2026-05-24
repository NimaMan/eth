use eth_alpha_core::{market::PoolSnapshot, mempool_entry::MempoolEntryEvidence, risk::RiskEvent};

use crate::baseline::snipe_all::rule::RuleDecision;

pub const RULE_NAME: &str = "entry.mempool_entry_evidence";

#[derive(Clone, Debug, PartialEq)]
pub enum MempoolEntryDecision {
    Accept { pool: PoolSnapshot },
    Reject { reason: String },
}

pub fn evaluate(event: &RiskEvent) -> MempoolEntryDecision {
    let Some(pool_address) = event.pool_address.clone() else {
        return reject("missing_pool_address");
    };
    let Some(evidence_value) = event.evidence.as_ref() else {
        return reject("missing_evidence");
    };
    let evidence = match MempoolEntryEvidence::from_risk_evidence(evidence_value) {
        Some(Ok(evidence)) => evidence,
        Some(Err(_)) => return reject("invalid_evidence"),
        None => return reject("missing_entry_evidence"),
    };
    if !evidence.has_successful_exact_vault_buy() {
        return reject("missing_successful_exact_vault_buy");
    }

    let pool = evidence.to_projected_pool_snapshot(event.token_address, pool_address);
    MempoolEntryDecision::Accept { pool }
}

impl MempoolEntryDecision {
    pub fn rule_decision(&self) -> RuleDecision {
        match self {
            Self::Accept { .. } => RuleDecision::Enter { rule: RULE_NAME },
            Self::Reject { reason } => RuleDecision::hold(RULE_NAME, reason.clone()),
        }
    }
}

fn reject(reason: impl Into<String>) -> MempoolEntryDecision {
    MempoolEntryDecision::Reject {
        reason: reason.into(),
    }
}
