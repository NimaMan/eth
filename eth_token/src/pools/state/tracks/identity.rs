use serde::{Deserialize, Serialize};

use crate::pools::base::BasePool;
use crate::pools::state::evidence::{EvidenceConfidence, EvidenceRef, EvidenceSourceKind};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct IdentityTrack {
    pub pool_address: String,
    pub token_address: String,
    pub denom_address: String,
    pub denom_symbol: Option<String>,
    pub protocol: String,
    pub creator_address: Option<String>,
    pub creation_block: Option<u64>,
    pub creation_tx: Option<String>,
    pub creation_timestamp: Option<u64>,
    pub evidence: Vec<EvidenceRef>,
}

impl IdentityTrack {
    pub fn from_base_pool(base: &BasePool) -> Self {
        let mut evidence = vec![EvidenceRef::base_projection()
            .at_block(base.creation_block)
            .with_tx(base.creation_tx.clone())];
        if base.creation_block.is_some() || base.creation_tx.is_some() {
            evidence.push(
                EvidenceRef::new(EvidenceSourceKind::PoolEvent, EvidenceConfidence::High)
                    .at_block(base.creation_block)
                    .with_tx(base.creation_tx.clone())
                    .with_note("pool creation"),
            );
        }

        Self {
            pool_address: base.identity.pool_address.clone(),
            token_address: base.identity.token_address.clone(),
            denom_address: base.identity.denom_address.clone(),
            denom_symbol: denom_symbol_for_address(&base.identity.denom_address),
            protocol: base.identity.protocol.clone(),
            creator_address: base.creator_address.clone(),
            creation_block: base.creation_block,
            creation_tx: base.creation_tx.clone(),
            creation_timestamp: base.creation_timestamp,
            evidence,
        }
    }
}

fn denom_symbol_for_address(address: &str) -> Option<String> {
    match address.trim().to_ascii_lowercase().as_str() {
        "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => Some("WETH".to_string()),
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" => Some("USDC".to_string()),
        "0xdac17f958d2ee523a2206206994597c13d831ec7" => Some("USDT".to_string()),
        "0x6b175474e89094c44da98b954eedeac495271d0f" => Some("DAI".to_string()),
        _ => None,
    }
}
