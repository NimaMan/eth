// Quick test for checksum functionality
use mempool_processor::common::address::{checksum_address, to_checksum_address};
use revm_primitives::Address;

fn main() {
    // Test with known addresses
    let test1 = checksum_address("0xd8da6bf26964af9d7eed9e03e53415d37aa96045");
    println!("Test 1: {}", test1);
    assert_eq!(test1, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
    
    let test2 = checksum_address("0x5aaeb6053f3e94c9b9a09f33669435e7ef1beaed");
    println!("Test 2: {}", test2);
    assert_eq!(test2, "0x5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed");
    
    // Test with Address type
    let addr = Address::from([0xd8, 0xda, 0x6b, 0xf2, 0x69, 0x64, 0xaf, 0x9d,
                             0x7e, 0xed, 0x9e, 0x03, 0xe5, 0x34, 0x15, 0xd3,
                             0x7a, 0xa9, 0x60, 0x45]);
    let test3 = to_checksum_address(addr);
    println!("Test 3: {}", test3);
    assert_eq!(test3, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
    
    println!("✅ All checksum tests passed!");
}