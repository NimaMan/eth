use alloy_primitives::Address;
use eyre::Result;

/// Transaction number type (matches reth's `Txumber`).
pub type Txumber = u64;

/// Each shard stores at most this many transaction numbers to keep values small.
pub const SHARD_TX_CAPACITY: usize = 2_000;

/// Address → transactions table helpers.
pub struct AddressIndex;

impl AddressIndex {
    /// MDBX table name.
    pub const TABLE_NAME: &'static str = "address_to_txs";

    /// Encode `(address, shard_id)` into a 28-byte key: `address || shard_id_be`.
    pub fn encode_shard_key(address: Address, shard_id: u64) -> [u8; 28] {
        let mut key = [0u8; 28];
        key[..20].copy_from_slice(address.as_slice());
        key[20..].copy_from_slice(&shard_id.to_be_bytes());
        key
    }

    /// Prefix key used for range scans (`address || 0`).
    pub fn encode_shard_prefix(address: Address) -> [u8; 28] {
        Self::encode_shard_key(address, 0)
    }

    /// Decode a shard key into `(address, shard_id)`.
    pub fn decode_shard_key(bytes: &[u8]) -> Result<(Address, u64)> {
        if bytes.len() != 28 {
            return Err(eyre::eyre!("invalid shard key length: {}", bytes.len()));
        }

        let mut address_bytes = [0u8; 20];
        address_bytes.copy_from_slice(&bytes[..20]);

        let mut shard_bytes = [0u8; 8];
        shard_bytes.copy_from_slice(&bytes[20..]);

        Ok((
            Address::from(address_bytes),
            u64::from_be_bytes(shard_bytes),
        ))
    }

    /// Return the address portion of a shard key (no allocation).
    pub fn key_address(bytes: &[u8]) -> Result<Address> {
        if bytes.len() < 20 {
            return Err(eyre::eyre!("invalid shard key length: {}", bytes.len()));
        }
        let mut address_bytes = [0u8; 20];
        address_bytes.copy_from_slice(&bytes[..20]);
        Ok(Address::from(address_bytes))
    }

    /// Encode a sorted list of transaction numbers. Format: `[len:u32_le][tx0_le][tx1_le]...`.
    pub fn encode_values(txs: &[Txumber]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(4 + txs.len() * 8);
        bytes.extend_from_slice(&(txs.len() as u32).to_le_bytes());
        for tx in txs {
            bytes.extend_from_slice(&tx.to_le_bytes());
        }
        bytes
    }

    /// Decode the list encoded by [`encode_values`].
    pub fn decode_values(bytes: &[u8]) -> Result<Vec<Txumber>> {
        if bytes.len() < 4 {
            return Err(eyre::eyre!("invalid shard payload: too short"));
        }

        let mut len_bytes = [0u8; 4];
        len_bytes.copy_from_slice(&bytes[..4]);
        let len = u32::from_le_bytes(len_bytes) as usize;

        let expected = 4 + len * 8;
        if bytes.len() != expected {
            return Err(eyre::eyre!(
                "invalid shard payload length: expected {}, got {}",
                expected,
                bytes.len()
            ));
        }

        let mut txs = Vec::with_capacity(len);
        for chunk in bytes[4..].chunks_exact(8) {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(chunk);
            txs.push(u64::from_le_bytes(arr));
        }

        Ok(txs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shard_key_round_trip() {
        let addr = Address::repeat_byte(0x11);
        let key = AddressIndex::encode_shard_key(addr, 42);
        let (decoded, shard) = AddressIndex::decode_shard_key(&key).unwrap();
        assert_eq!(decoded, addr);
        assert_eq!(shard, 42);
    }

    #[test]
    fn encode_decode_values() {
        let txs = vec![1u64, 2, 3, 10_000];
        let bytes = AddressIndex::encode_values(&txs);
        let decoded = AddressIndex::decode_values(&bytes).unwrap();
        assert_eq!(decoded, txs);
    }
}
