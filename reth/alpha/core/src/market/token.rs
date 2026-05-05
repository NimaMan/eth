use crate::ids::{BlockNumber, TokenAddress};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenSnapshot {
    pub address: TokenAddress,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub creation_block: Option<BlockNumber>,
    pub latest_block: BlockNumber,
    pub is_scam: bool,
}
