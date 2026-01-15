use alloy_primitives::Address;
use eyre::Result;

/// Transaction number type (matches reth's `Txumber`).
pub type Txumber = u64;

/// Address participation index: dup-sorted table with `address` as the key and
/// every touched transaction number stored as a duplicate value.
pub struct AddressIndex;

impl AddressIndex {
    /// MDBX table name.
    pub const TABLE_NAME: &'static str = "address_to_txs";

    /// Encode the 20-byte address key.
    pub fn encode_key(address: Address) -> [u8; 20] {
        let mut key = [0u8; 20];
        key.copy_from_slice(address.as_slice());
        key
    }

    /// Encode a transaction number as little-endian bytes.
    pub fn encode_value(tx_number: Txumber) -> [u8; 8] {
        tx_number.to_le_bytes()
    }

    /// Decode a transaction number.
    pub fn decode_value(bytes: &[u8]) -> Result<Txumber> {
        if bytes.len() != 8 {
            return Err(eyre::eyre!(
                "invalid tx_number payload length: expected 8, got {}",
                bytes.len()
            ));
        }
        let mut arr = [0u8; 8];
        arr.copy_from_slice(bytes);
        Ok(u64::from_le_bytes(arr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_round_trip() {
        let address = Address::repeat_byte(0x11);
        let key = AddressIndex::encode_key(address);
        assert_eq!(Address::from_slice(&key), address);

        let value = AddressIndex::encode_value(42);
        assert_eq!(AddressIndex::decode_value(&value).unwrap(), 42);
    }
}
