use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};

pub type ChainId = u64;
pub type BlockNumber = u64;
pub type BlockHash = B256;
pub type TxHash = B256;

pub type WalletAddress = Address;
pub type TokenAddress = Address;
pub type PoolAddress = Address;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PortfolioId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct WalletId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct StrategyName(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PositionId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct OrderId(pub String);

impl From<&str> for PortfolioId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<&str> for WalletId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<&str> for StrategyName {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<&str> for PositionId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<&str> for OrderId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}
