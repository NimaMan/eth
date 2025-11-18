use crate::RethQueryProvider;
use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
use tx_simulator::{
    contract_method_simulator::decode_uint256_from_contract_output, UnsignedTxChainSimulation,
};

/// Execute a view function expected to return a 32-byte word. Returns `Ok(None)`
/// if the call fails or produces an unexpected payload length.
pub async fn call_uint256_view(
    provider: &RethQueryProvider,
    contract: Address,
    data: Bytes,
    block_number: u64,
    chain: Option<&mut UnsignedTxChainSimulation>,
) -> Result<Option<U256>> {
    let result = if let Some(chain) = chain {
        chain.simulate_view_call(contract, data)
    } else {
        provider
            .simulate_contract_view_call(contract, data, Some(block_number))
            .await
    };

    match result {
        Ok(res) if res.success && res.output.len() >= 32 => {
            Ok(Some(decode_uint256_from_contract_output(&res.output)))
        }
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Check whether the bytecode blob contains the specified signature/topic.
pub fn contains_signature(bytecode: &[u8], signature: &[u8]) -> bool {
    if bytecode.len() < signature.len() {
        return false;
    }
    bytecode
        .windows(signature.len())
        .any(|window| window == signature)
}

/// Build a selector + two-address payload (e.g., allowance(owner, spender)).
pub fn build_two_address_payload(selector: &[u8; 4], first: Address, second: Address) -> Bytes {
    let mut payload = Vec::with_capacity(4 + 32 + 32);
    payload.extend_from_slice(selector);
    payload.extend_from_slice(&[0u8; 12]);
    payload.extend_from_slice(first.as_slice());
    payload.extend_from_slice(&[0u8; 12]);
    payload.extend_from_slice(second.as_slice());
    Bytes::from(payload)
}
