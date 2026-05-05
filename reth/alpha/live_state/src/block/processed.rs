use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::BlockNumber;

use super::BlockMeta;

pub const PROCESSED_BLOCK_SNAPSHOT_VERSION: u16 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessedTransactionSnapshot {
    pub tx_hash: B256,
    pub tx_index: u64,
    pub payload: Value,
}

impl ProcessedTransactionSnapshot {
    pub const fn new(tx_hash: B256, tx_index: u64, payload: Value) -> Self {
        Self {
            tx_hash,
            tx_index,
            payload,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProcessedBlockSnapshot {
    pub schema_version: u16,
    pub meta: BlockMeta,
    pub header: Value,
    pub transactions: Vec<ProcessedTransactionSnapshot>,
    pub touched_addresses: Vec<Address>,
}

impl ProcessedBlockSnapshot {
    pub fn new(
        meta: BlockMeta,
        header: Value,
        mut transactions: Vec<ProcessedTransactionSnapshot>,
        touched_addresses: Vec<Address>,
    ) -> Self {
        transactions.sort_by_key(|tx| tx.tx_index);
        Self {
            schema_version: PROCESSED_BLOCK_SNAPSHOT_VERSION,
            meta,
            header,
            transactions,
            touched_addresses,
        }
    }

    pub fn block_number(&self) -> BlockNumber {
        self.meta.block_number
    }

    pub fn block_hash(&self) -> B256 {
        self.meta.block_hash
    }

    pub fn transaction(&self, tx_hash: B256) -> Option<&ProcessedTransactionSnapshot> {
        self.transactions.iter().find(|tx| tx.tx_hash == tx_hash)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockReadyNotification {
    pub block_number: BlockNumber,
    pub block_hash: B256,
    pub processed_transaction_count: usize,
    pub updated_token_count: usize,
}

impl BlockReadyNotification {
    pub fn from_block(block: &ProcessedBlockSnapshot, updated_token_count: usize) -> Self {
        Self {
            block_number: block.block_number(),
            block_hash: block.block_hash(),
            processed_transaction_count: block.transactions.len(),
            updated_token_count,
        }
    }
}
