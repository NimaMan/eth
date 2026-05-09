use std::fmt;

use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};

pub type ChainId = u64;
pub type BlockNumber = u64;
pub type BlockHash = B256;
pub type TxHash = B256;

pub type WalletAddress = Address;
pub type TokenAddress = Address;

/// Token-scoped pool identity used by alpha decisions and position tracking.
///
/// V2/V3 pools are represented as `token_address:pool_contract_address`.
/// V4 pools are represented as `token_address:pool_manager#pool_id`.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TokenPoolId(pub String);

pub type PoolAddress = TokenPoolId;

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

impl TokenPoolId {
    pub fn new(token_address: TokenAddress, pool_identity: impl AsRef<str>) -> Self {
        let pool_identity = normalize_pool_identity(pool_identity.as_ref());
        Self(format!(
            "{}:{pool_identity}",
            normalize_address(token_address)
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TokenPoolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

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

impl From<&str> for TokenPoolId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for TokenPoolId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

fn normalize_address(address: Address) -> String {
    address.to_string().to_ascii_lowercase()
}

fn normalize_pool_identity(pool_identity: &str) -> String {
    pool_identity.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_pool_id_is_token_scoped_and_normalized() {
        let token = Address::with_last_byte(0x11);
        let id = TokenPoolId::new(token, "  0xABCDEF  ");

        assert_eq!(
            id.as_str(),
            "0x0000000000000000000000000000000000000011:0xabcdef"
        );
    }

    #[test]
    fn token_pool_id_preserves_v4_manager_pool_identity() {
        let token = Address::with_last_byte(0x22);
        let id = TokenPoolId::new(token, "0xMANAGER#0xPOOLID");

        assert_eq!(
            id.as_str(),
            "0x0000000000000000000000000000000000000022:0xmanager#0xpoolid"
        );
    }
}
