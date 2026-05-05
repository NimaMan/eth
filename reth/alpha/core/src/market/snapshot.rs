use crate::{
    ids::{BlockNumber, PoolAddress, TokenAddress},
    market::{PoolSnapshot, TokenSnapshot},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MarketSnapshotRef {
    pub block_number: BlockNumber,
    pub token_address: TokenAddress,
    pub pool_address: Option<PoolAddress>,
    pub token: Option<TokenSnapshot>,
    pub pool: Option<PoolSnapshot>,
}
