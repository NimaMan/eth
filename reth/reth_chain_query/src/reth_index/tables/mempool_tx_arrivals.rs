/// Mempool Tx Arrival Times Table
///
/// Minimal mapping: TxNumber (u64 big-endian) -> first_seen_ms (u64 big-endian)
/// Implemented in RethIndexDB via reth-libmdbx environment.

use eyre::Result;

pub struct MempoolTxArrivalTable;

impl MempoolTxArrivalTable {
    /// Table name inside MDBX
    pub const TABLE_NAME: &'static str = "mempool_tx_arrival_times";

    #[inline]
    pub fn encode_key(tx_number: u64) -> [u8; 8] { tx_number.to_be_bytes() }

    #[inline]
    pub fn decode_key(key: &[u8]) -> Result<u64> {
        if key.len() != 8 { return Err(eyre::eyre!("invalid key length")); }
        let mut arr = [0u8;8];
        arr.copy_from_slice(key);
        Ok(u64::from_be_bytes(arr))
    }

    #[inline]
    pub fn encode_value(first_seen_ms: u64) -> [u8; 8] { first_seen_ms.to_be_bytes() }

    #[inline]
    pub fn decode_value(val: &[u8]) -> Result<u64> {
        if val.len() != 8 { return Err(eyre::eyre!("invalid value length")); }
        let mut arr = [0u8;8];
        arr.copy_from_slice(val);
        Ok(u64::from_be_bytes(arr))
    }
}
