use alloy_primitives::Address;
use eyre::Result;

/// Block number type stored by the address participation index.
pub type ParticipationBlockNumber = u64;

/// Address participation index: dup-sorted table with `address` as the key and
/// every block where that address appeared in processed transaction data stored
/// as a duplicate value.
pub struct AddressBlockParticipationIndex;

impl AddressBlockParticipationIndex {
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
    pub fn encode_value(block_number: ParticipationBlockNumber) -> [u8; 8] {
        block_number.to_be_bytes()
    }

    /// Decode a block number.
    pub fn decode_value(bytes: &[u8]) -> Result<ParticipationBlockNumber> {
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
        let key = AddressBlockParticipationIndex::encode_key(address);
        assert_eq!(Address::from_slice(&key), address);

        let value = AddressBlockParticipationIndex::encode_value(42);
        assert_eq!(
            AddressBlockParticipationIndex::decode_value(&value).unwrap(),
            42
        );
    }

    #[test]
    fn encoded_block_numbers_sort_numerically() {
        let mut encoded = [10, 2, 300, 1]
            .into_iter()
            .map(AddressBlockParticipationIndex::encode_value)
            .collect::<Vec<_>>();
        encoded.sort();
        let decoded = encoded
            .iter()
            .map(|value| AddressBlockParticipationIndex::decode_value(value).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(decoded, vec![1, 2, 10, 300]);
    }
}
