use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenStaticFeatures {
    pub token_creation_block: Option<u64>,
    pub token_age_blocks: Option<u64>,
    pub pool_creation_after_token_blocks: Option<u64>,
    pub decimals: Option<u8>,
    pub total_supply_scaled: Option<f64>,
    pub total_supply_from_transfers: Option<f64>,
}

impl TokenStaticFeatures {
    pub fn from_blocks(
        token_creation_block: Option<u64>,
        pool_creation_block: Option<u64>,
        as_of_block: u64,
    ) -> Self {
        Self {
            token_creation_block,
            token_age_blocks: token_creation_block.map(|block| as_of_block.saturating_sub(block)),
            pool_creation_after_token_blocks: token_creation_block
                .zip(pool_creation_block)
                .map(|(token_block, pool_block)| pool_block.saturating_sub(token_block)),
            ..Default::default()
        }
    }
}
