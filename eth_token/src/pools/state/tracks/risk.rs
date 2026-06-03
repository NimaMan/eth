use serde::{Deserialize, Serialize};

use crate::pools::base::BasePool;
use crate::pools::flags::PoolStateFlags;
use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RiskTrack {
    pub terminal_position_risk: bool,
    pub scam_mechanism: Option<String>,
    pub scam_label: Option<String>,
    pub direct_lp_liquidity_removal: bool,
    pub pair_balance_backdoor_drain: bool,
    pub holder_balance_backdoor_drain: bool,
    pub custody_buyer_token_confiscation: bool,
    pub privileged_seller_reserve_drain: bool,
    pub reserve_dump_drain: bool,
    pub unknown_reserve_drain: bool,
    pub block_number: Option<u64>,
    pub tx_hash: Option<String>,
    pub evidence: Vec<EvidenceRef>,
}

impl RiskTrack {
    pub fn from_base_and_flags(base: &BasePool, flags: &PoolStateFlags) -> Self {
        Self {
            terminal_position_risk: flags.risk.terminal_position_risk,
            scam_mechanism: flags.risk.scam_mechanism.clone(),
            scam_label: flags.risk.scam_label.clone(),
            direct_lp_liquidity_removal: flags.risk.direct_lp_liquidity_removal,
            pair_balance_backdoor_drain: flags.risk.pair_balance_backdoor_drain,
            holder_balance_backdoor_drain: flags.risk.holder_balance_backdoor_drain,
            custody_buyer_token_confiscation: flags.risk.custody_buyer_token_confiscation,
            privileged_seller_reserve_drain: flags.risk.privileged_seller_reserve_drain,
            reserve_dump_drain: flags.risk.reserve_dump_drain,
            unknown_reserve_drain: flags.risk.unknown_reserve_drain,
            block_number: base.scam_block,
            tx_hash: base.scam_tx_hash.clone(),
            evidence: vec![EvidenceRef::new(
                EvidenceSourceKind::PoolStateFlags,
                if flags.risk.scam_mechanism.is_some() {
                    EvidenceConfidence::High
                } else {
                    EvidenceConfidence::Inferred
                },
            )
            .at_block(base.scam_block)
            .with_tx(base.scam_tx_hash.clone())],
        }
    }
}
