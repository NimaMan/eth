// Address normalization utilities
use revm_primitives::Address;
use tiny_keccak::{Hasher, Keccak};

/// Convert an Address to EIP-55 checksummed format (mixed case)
/// This matches the format used by Python's web3.to_checksum_address()
pub fn to_checksum_address(address: Address) -> String {
    let address_hex = format!("{:040x}", address);
    checksum_address(&address_hex)
}

/// Convert an address string to EIP-55 checksummed format
/// This function implements the EIP-55 checksumming algorithm
pub fn checksum_address(address_str: &str) -> String {
    // Remove 0x prefix if present and convert to lowercase
    let clean_address = address_str.trim().trim_start_matches("0x").to_lowercase();
    
    // Pad to 40 characters if necessary
    let padded_address = if clean_address.len() < 40 {
        format!("{:0>40}", clean_address)
    } else {
        clean_address
    };
    
    // Hash the lowercase address
    let mut hasher = Keccak::v256();
    hasher.update(padded_address.as_bytes());
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);
    
    // Apply EIP-55 checksumming rules
    let mut checksummed = String::with_capacity(42); // "0x" + 40 chars
    checksummed.push_str("0x");
    
    for (i, char) in padded_address.chars().enumerate() {
        if char.is_ascii_hexdigit() && char.is_ascii_alphabetic() {
            // For letters a-f, check if corresponding hash bit is set
            let hash_index = i / 2;
            if hash_index >= hash.len() {
                // Safety check - shouldn't happen with valid addresses
                checksummed.push(char);
                continue;
            }
            let hash_byte = hash[hash_index];
            let hash_nibble = if i % 2 == 0 { hash_byte >> 4 } else { hash_byte & 0x0f };
            
            if hash_nibble >= 8 {
                checksummed.push(char.to_ascii_uppercase());
            } else {
                checksummed.push(char);
            }
        } else {
            // Numbers remain unchanged
            checksummed.push(char);
        }
    }
    
    checksummed
}

/// Normalize an Ethereum address to a consistent lowercase hex format with 0x prefix
/// DEPRECATED: Use to_checksum_address() instead for compatibility with Python
pub fn normalize_address(address: Address) -> String {
    format!("0x{:040x}", address).to_lowercase()
}

/// Normalize an address string to a consistent lowercase hex format with 0x prefix
/// Handles addresses with or without 0x prefix and ensures proper padding
pub fn normalize_address_str(address_str: &str) -> String {
    let cleaned = address_str.trim().trim_start_matches("0x").to_lowercase();
    
    // Pad with zeros if necessary to ensure 40 characters (20 bytes)
    let padded = if cleaned.len() < 40 {
        format!("{:0>40}", cleaned)
    } else {
        cleaned
    };
    
    format!("0x{}", padded)
}

/// Convert a string address to a revm Address type
pub fn str_to_address(address_str: &str) -> Result<Address, String> {
    let cleaned = address_str.trim().trim_start_matches("0x");
    
    if cleaned.len() != 40 {
        return Err(format!("Invalid address length: {} (expected 40 hex chars)", cleaned.len()));
    }
    
    let bytes = hex::decode(cleaned)
        .map_err(|e| format!("Invalid hex in address: {}", e))?;
    
    if bytes.len() != 20 {
        return Err(format!("Invalid address byte length: {} (expected 20 bytes)", bytes.len()));
    }
    
    let mut addr_bytes = [0u8; 20];
    addr_bytes.copy_from_slice(&bytes);
    Ok(Address::from(addr_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_checksum_address() {
        // Test with a known address
        let test_address = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
        let checksummed = checksum_address("0xd8da6bf26964af9d7eed9e03e53415d37aa96045");
        assert_eq!(checksummed, test_address);
        
        // Test another known address
        let test_address2 = "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed";
        let checksummed2 = checksum_address("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed");
        assert_eq!(checksummed2, test_address2);
    }
    
    #[test]
    fn test_to_checksum_address() {
        let addr = Address::from([0xd8, 0xda, 0x6b, 0xf2, 0x69, 0x64, 0xaf, 0x9d,
                                 0x7e, 0xed, 0x9e, 0x03, 0xe5, 0x34, 0x15, 0xd3,
                                 0x7a, 0xa9, 0x60, 0x45]);
        let checksummed = to_checksum_address(addr);
        assert_eq!(checksummed, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
    }

    #[test]
    fn test_normalize_address() {
        let addr = Address::from([0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
                                 0x12, 0x34, 0x56, 0x78, 0x90, 0xab, 0xcd, 0xef,
                                 0x12, 0x34, 0x56, 0x78]);
        let normalized = normalize_address(addr);
        assert_eq!(normalized, "0x1234567890abcdef1234567890abcdef12345678");
    }
    
    #[test]
    fn test_normalize_address_str() {
        // Test with 0x prefix
        assert_eq!(
            normalize_address_str("0x1234567890AbCdEf1234567890aBcDeF12345678"),
            "0x1234567890abcdef1234567890abcdef12345678"
        );
        
        // Test without 0x prefix
        assert_eq!(
            normalize_address_str("1234567890AbCdEf1234567890aBcDeF12345678"),
            "0x1234567890abcdef1234567890abcdef12345678"
        );
        
        // Test with short address (should pad)
        assert_eq!(
            normalize_address_str("0x123"),
            "0x0000000000000000000000000000000000000123"
        );
    }
}