use crate::ids::{BlockHash, BlockNumber};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockTime {
    pub block_number: BlockNumber,
    pub block_hash: Option<BlockHash>,
    pub timestamp_unix_secs: u64,
}
