use eth_alpha_core::{amount::DecimalAmount, ids::BlockNumber, market::PoolSnapshot};

#[derive(Clone, Debug, PartialEq)]
pub struct EntryInitEvidence {
    pub entry_block: BlockNumber,
    pub creation_block: Option<BlockNumber>,
    pub entry_age_blocks: Option<u64>,
    pub price_ratio_to_initial: Option<DecimalAmount>,
    pub denom_reserve: DecimalAmount,
    pub can_buy: bool,
    pub can_sell: bool,
}

impl EntryInitEvidence {
    pub fn from_pool_at_block(pool: &PoolSnapshot, entry_block: BlockNumber) -> Self {
        Self {
            entry_block,
            creation_block: pool.creation_block,
            entry_age_blocks: pool
                .creation_block
                .and_then(|creation_block| entry_block.checked_sub(creation_block)),
            price_ratio_to_initial: pool.price_ratio_to_initial,
            denom_reserve: pool.denom_reserve,
            can_buy: pool.can_buy,
            can_sell: pool.can_sell,
        }
    }
}
