/// EIP-55 checksum address encoding
use alloy_primitives::{Address, hex as alloy_hex, keccak256};
use serde::{Deserialize, Serializer, Deserializer};

/// Convert an alloy Address to EIP-55 checksum format string
pub fn alloy_address_to_checksum(address: Address) -> String {
    to_checksum_address(&address)
}

/// Convert an address to EIP-55 checksum format
pub fn to_checksum_address(address: &Address) -> String {
    let hex = alloy_hex::encode(address.as_slice());
    
    // Compute Keccak256 hash of the lowercase hex string using alloy
    let hash = keccak256(hex.as_bytes());
    
    // Apply checksum based on hash
    let mut checksum = String::with_capacity(42);
    checksum.push_str("0x");
    
    let hash_bytes = hash.as_slice();
    for (i, ch) in hex.chars().enumerate() {
        if ch.is_ascii_alphabetic() {
            // Check if the corresponding nibble in the hash is >= 8
            let hash_byte = hash_bytes[i / 2];
            let nibble = if i % 2 == 0 { hash_byte >> 4 } else { hash_byte & 0x0f };
            
            if nibble >= 8 {
                checksum.push(ch.to_ascii_uppercase());
            } else {
                checksum.push(ch.to_ascii_lowercase());
            }
        } else {
            checksum.push(ch);
        }
    }
    
    checksum
}

/// Serialize an Address as a checksummed string
pub fn serialize_address_checksum<S>(address: &Address, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let checksum = to_checksum_address(address);
    serializer.serialize_str(&checksum)
}

/// Deserialize a checksummed address string to Address
pub fn deserialize_address_checksum<'de, D>(deserializer: D) -> Result<Address, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<Address>()
        .map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_checksum_address() {
        // Test with known checksum addresses
        let test_cases = vec![
            ("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed", "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed"),
            ("0xfb6916095ca1df60bb79ce92ce3ea74c37c5d359", "0xfB6916095ca1df60bB79Ce92cE3Ea74c37c5d359"),
            ("0xdbf03b407c01e7cd3cbea99509d93f8dddc8c6fb", "0xdbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB"),
            ("0xd1220a0cf47c7b9be7a2e6ba89f429762e7b9adb", "0xD1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb"),
        ];
        
        for (input, expected) in test_cases {
            let addr = Address::from_str(input).unwrap();
            let checksum = to_checksum_address(&addr);
            assert_eq!(checksum, expected, "Failed for address: {}", input);
        }
    }
}
