use crate::{
    ids::BlockNumber,
    market::{PoolSnapshot, TokenSnapshot},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MarketEvent {
    TokenUpdated {
        block_number: BlockNumber,
        token: TokenSnapshot,
    },
    PoolUpdated {
        block_number: BlockNumber,
        pool: PoolSnapshot,
    },
    BlockCompleted {
        block_number: BlockNumber,
        updated_tokens: usize,
        updated_pools: usize,
    },
}
