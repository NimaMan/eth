use eth_alpha_core::{amount::DecimalAmount, ids::BlockNumber, market::PoolSnapshot};

#[derive(Clone, Debug, PartialEq)]
pub struct EntryInitEvidence {
    pub entry_block: BlockNumber,
    pub pool_creation_block: Option<BlockNumber>,
    pub entry_age_blocks: Option<u64>,
    pub price_ratio_to_initial: Option<DecimalAmount>,
    pub denom_reserve: DecimalAmount,
    pub can_buy: bool,
    pub can_sell: bool,
}

impl EntryInitEvidence {
    pub fn from_pool_at_block(pool: &PoolSnapshot, entry_block: BlockNumber) -> Self {
        let pool_creation_block = pool.creation_block;
        Self {
            entry_block,
            pool_creation_block,
            entry_age_blocks: pool_creation_block
                .and_then(|pool_creation_block| entry_block.checked_sub(pool_creation_block)),
            price_ratio_to_initial: pool.price_ratio_to_initial,
            denom_reserve: pool.denom_reserve,
            can_buy: pool.can_buy,
            can_sell: pool.can_sell,
        }
    }
}
