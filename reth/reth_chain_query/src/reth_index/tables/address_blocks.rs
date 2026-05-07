use alloy_primitives::Address;
use eyre::Result;

/// Block number type stored by the address participation index.
pub type IndexedBlockNumber = u64;

/// Address participation index: dup-sorted table with `address` as the key and
/// every block where that address appeared in processed transaction data stored
/// as a duplicate value.
pub struct AddressBlockIndex;

impl AddressBlockIndex {
    /// MDBX table name.
    pub const TABLE_NAME: &'static str = "address_to_blocks";

    /// Encode the 20-byte address key.
    pub fn encode_key(address: Address) -> [u8; 20] {
        let mut key = [0u8; 20];
        key.copy_from_slice(address.as_slice());
        key
    }

    /// Encode a block number as big-endian bytes so MDBX dupsort byte ordering
    /// matches numeric block ordering.
    pub fn encode_value(block_number: IndexedBlockNumber) -> [u8; 8] {
        block_number.to_be_bytes()
    }

    /// Decode a block number.
    pub fn decode_value(bytes: &[u8]) -> Result<IndexedBlockNumber> {
        if bytes.len() != 8 {
            return Err(eyre::eyre!(
                "invalid block_number payload length: expected 8, got {}",
                bytes.len()
            ));
        }
        let mut arr = [0u8; 8];
        arr.copy_from_slice(bytes);
        Ok(u64::from_be_bytes(arr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_round_trip() {
        let address = Address::repeat_byte(0x11);
        let key = AddressBlockIndex::encode_key(address);
        assert_eq!(Address::from_slice(&key), address);

        let value = AddressBlockIndex::encode_value(42);
        assert_eq!(AddressBlockIndex::decode_value(&value).unwrap(), 42);
    }

    #[test]
    fn encoded_block_numbers_sort_numerically() {
        let mut encoded = [10, 2, 300, 1]
            .into_iter()
            .map(AddressBlockIndex::encode_value)
            .collect::<Vec<_>>();
        encoded.sort();
        let decoded = encoded
            .iter()
            .map(|value| AddressBlockIndex::decode_value(value).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(decoded, vec![1, 2, 10, 300]);
    }
}
