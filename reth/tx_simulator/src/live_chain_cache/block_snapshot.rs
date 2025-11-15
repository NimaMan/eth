use serde::Deserialize;
use serde_json::Value;

/// Snapshot of a processed block stored in Redis.
///
/// This mirrors the payload emitted by `LiveBlockProcessor`, which contains the
/// block number, the serialized header JSON, and optionally the processed
/// transaction list.
#[derive(Debug, Clone, Deserialize)]
pub struct ProcessedBlockSnapshot {
    pub block_number: Option<u64>,
    pub header: Option<String>,
    #[serde(default)]
    pub tx_count: Option<usize>,
    #[serde(default)]
    pub transactions: Vec<Value>,
}

impl ProcessedBlockSnapshot {
    /// Returns the resolved block number if present.
    pub fn block_number(&self) -> Option<u64> {
        self.block_number
    }

    pub fn header_json(&self) -> Option<&str> {
        self.header.as_deref()
    }

    pub fn transactions(&self) -> &[Value] {
        &self.transactions
    }
}
