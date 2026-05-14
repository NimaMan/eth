use crate::mempool_fetcher::MempoolTransaction;
use alloy_primitives::{address, Address as AlloyAddress};

pub(crate) fn liquidity_removal_token_candidates(input: &[u8]) -> Vec<String> {
    (0..2)
        .filter_map(|param_idx| calldata_address_param(input, param_idx))
        .collect()
}

fn calldata_address_param(input: &[u8], param_idx: usize) -> Option<String> {
    let start = 4 + param_idx * 32 + 12;
    let end = start + 20;
    if input.len() < end {
        return None;
    }
    Some(format!("0x{}", hex::encode(&input[start..end])))
}

pub(crate) fn is_protocol_liquidity_removal_candidate(tx: &MempoolTransaction) -> bool {
    let Some(selector) = tx.input.get(0..4) else {
        return false;
    };

    matches!(
        selector,
        [0x0c, 0x49, 0xcc, 0xbe]
            | [0xdd, 0x46, 0x50, 0x8f]
            | [0xa3, 0x55, 0xde, 0x88]
            | [0x0d, 0x4f, 0x31, 0x9d]
    ) || (selector == [0xac, 0x96, 0x50, 0xd8].as_slice()
        && tx
            .to
            .as_ref()
            .map(|to| {
                AlloyAddress::from_slice(to) == address!("C36442b4a4522E871399CD717aBDD847Ab11FE88")
            })
            .unwrap_or(false))
}

pub(crate) fn is_v4_modify_liquidity_candidate(tx: &MempoolTransaction) -> bool {
    let Some(selector) = tx.input.get(0..4) else {
        return false;
    };
    matches!(
        selector,
        [0xdd, 0x46, 0x50, 0x8f] | [0xa3, 0x55, 0xde, 0x88] | [0x0d, 0x4f, 0x31, 0x9d]
    )
}

pub(crate) fn is_known_position_manager_candidate(address: &AlloyAddress) -> bool {
    *address == address!("C36442b4a4522E871399CD717aBDD847Ab11FE88")
        || *address == address!("bd216513d74c8cf14cf4747e6aaa6420ff64ee9e")
}
