// Simple standalone checksum test
use tiny_keccak::{Hasher, Keccak};

/// Convert an address string to EIP-55 checksummed format
fn checksum_address(address_str: &str) -> String {
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
            let hash_byte = hash[i / 2];
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

fn main() {
    // Test with known addresses from EIP-55 examples
    let test1 = checksum_address("0xd8da6bf26964af9d7eed9e03e53415d37aa96045");
    println!("Test 1: {}", test1);
    assert_eq!(test1, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
    
    let test2 = checksum_address("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed");
    println!("Test 2: {}", test2);
    assert_eq!(test2, "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed");
    
    // Test the address from the Python output we saw earlier
    let test3 = checksum_address("0x98c3d3183c4b8a650614ad179a1a98be0a8d6b8e");
    println!("Test 3: {}", test3);
    assert_eq!(test3, "0x98C3d3183C4b8A650614ad179A1a98be0a8d6B8E");
    
    println!("✅ All checksum tests passed!");
}