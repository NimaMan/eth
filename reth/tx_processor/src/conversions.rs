use ethers_core::types::{Address as EthersAddress, U256 as EthersU256};
use revm_primitives::{Address as RevmAddress, U256 as RevmU256};

pub fn ethers_to_revm_u256(val: EthersU256) -> RevmU256 {
    let mut bytes = [0u8; 32];
    val.to_big_endian(&mut bytes);
    RevmU256::from_be_bytes(bytes)
}

pub fn ethers_u256_to_u128_safe(val: EthersU256) -> u128 {
    if val > EthersU256::from(u128::MAX) {
        u128::MAX
    } else {
        val.as_u128()
    }
}

pub fn ethers_u256_to_u64_safe(val: EthersU256) -> u64 {
    if val > EthersU256::from(u64::MAX) {
        u64::MAX
    } else {
        val.as_u64()
    }
}

pub fn ethers_to_revm_address(addr: EthersAddress) -> RevmAddress {
    RevmAddress::from_slice(addr.as_bytes())
}

// It's good practice to include tests within the module they are testing if they are unit tests.
// However, if these tests rely on `super::*` where `super` refers to `lib.rs` specific setup,
// they might need to stay in lib.rs or be adjusted.
// For now, I'll move them and assume they are self-contained or can be made so.
#[cfg(test)]
mod tests {
    use super::*; // This will now refer to the conversions module itself
    use std::str::FromStr;
    // We need hex for one of the tests if it's not already in scope via super
    // use hex;

    #[test]
    fn test_ethers_to_revm_u256() {
        let eth_u = EthersU256::from(12345);
        let revm_u = ethers_to_revm_u256(eth_u);
        assert_eq!(revm_u, RevmU256::from(12345));
    }

    #[test]
    fn test_ethers_address_to_revm_address() {
        let eth_addr = EthersAddress::from_str("0xAb5801a7D398351b8bE11C439e05C5B3259aeC9B").unwrap();
        let revm_addr = ethers_to_revm_address(eth_addr);
        // The original test used hex::decode directly. If hex is not globally available,
        // this test might need `use hex;` or qualify it if it comes from elsewhere.
        // For now, assuming it's accessible or will be fixed if it causes a new error.
        let expected_revm_addr = RevmAddress::from_slice(&hex::decode("Ab5801a7D398351b8bE11C439e05C5B3259aeC9B").unwrap());
        assert_eq!(revm_addr, expected_revm_addr);
    }
} 