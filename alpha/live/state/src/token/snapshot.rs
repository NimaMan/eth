use std::collections::BTreeMap;

use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{BlockNumber, TimestampUnixSecs};

use super::PoolSnapshot;

pub const TOKEN_SNAPSHOT_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub total_supply: Option<String>,
}

impl TokenMetadata {
    pub fn empty() -> Self {
        Self {
            name: None,
            symbol: None,
            decimals: None,
            total_supply: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenCreation {
    pub block_number: Option<BlockNumber>,
    pub timestamp_unix_secs: Option<TimestampUnixSecs>,
    pub tx_hash: Option<B256>,
    pub creator: Option<Address>,
    pub creator_nonce: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenLatestBlock {
    pub number: BlockNumber,
    pub timestamp_unix_secs: Option<TimestampUnixSecs>,
    pub hash: Option<B256>,
}

impl TokenLatestBlock {
    pub const fn new(
        number: BlockNumber,
        timestamp_unix_secs: Option<TimestampUnixSecs>,
        hash: Option<B256>,
    ) -> Self {
        Self {
            number,
            timestamp_unix_secs,
            hash,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TokenLifecycle {
    #[serde(rename = "CONTRACT_CREATION")]
    ContractCreation,
    PairCreation,
    TradingEnabled,
    InactiveScam,
    InactiveOther,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenStatus {
    pub lifecycle: Option<TokenLifecycle>,
    pub is_scam: bool,
    pub scam_label: Option<String>,
    pub has_pool: bool,
    pub trading_enabled: bool,
    pub trading_enabled_block: Option<BlockNumber>,
    pub trading_enabled_tx: Option<B256>,
    pub ownership_renounced: bool,
    pub latest_activity_block: Option<BlockNumber>,
}

impl Default for TokenStatus {
    fn default() -> Self {
        Self {
            lifecycle: None,
            is_scam: false,
            scam_label: None,
            has_pool: false,
            trading_enabled: false,
            trading_enabled_block: None,
            trading_enabled_tx: None,
            ownership_renounced: false,
            latest_activity_block: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenControl {
    pub current_owner: Option<Address>,
    pub ownership_renounced_block: Option<BlockNumber>,
    pub control_addresses: Vec<Address>,
    pub tax_setter_addresses: Vec<Address>,
}

impl TokenControl {
    pub fn empty() -> Self {
        Self {
            current_owner: None,
            ownership_renounced_block: None,
            control_addresses: Vec::new(),
            tax_setter_addresses: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenTransferSummary {
    pub total_bribe_amount: Option<String>,
    pub bribes_by_tx: BTreeMap<String, String>,
    pub approved_addresses: Vec<Address>,
    pub address_tx_counter: BTreeMap<String, u64>,
}

impl TokenTransferSummary {
    pub fn empty() -> Self {
        Self {
            total_bribe_amount: None,
            bribes_by_tx: BTreeMap::new(),
            approved_addresses: Vec::new(),
            address_tx_counter: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenSnapshot {
    pub version: u16,
    pub contract_address: Address,
    pub metadata: TokenMetadata,
    pub creation: TokenCreation,
    pub latest_block: TokenLatestBlock,
    pub status: TokenStatus,
    pub control: TokenControl,
    pub pools: Vec<PoolSnapshot>,
    pub transfers: TokenTransferSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transfer_history: Option<Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, Value>,
}

impl TokenSnapshot {
    pub fn new(contract_address: Address, latest_block: TokenLatestBlock) -> Self {
        Self {
            version: TOKEN_SNAPSHOT_VERSION,
            contract_address,
            metadata: TokenMetadata::empty(),
            creation: TokenCreation {
                block_number: None,
                timestamp_unix_secs: None,
                tx_hash: None,
                creator: None,
                creator_nonce: None,
            },
            latest_block,
            status: TokenStatus::default(),
            control: TokenControl::empty(),
            pools: Vec::new(),
            transfers: TokenTransferSummary::empty(),
            transfer_history: None,
            extra: BTreeMap::new(),
        }
    }

    pub fn latest_block_number(&self) -> BlockNumber {
        self.latest_block.number
    }
}
