use crate::mempool_fetcher::MempoolTransaction;
use alloy_primitives::{Address, U256};

pub const ERC721_APPROVE_SELECTOR: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3];
pub const SET_APPROVAL_FOR_ALL_SELECTOR: [u8; 4] = [0xa2, 0x2c, 0xb4, 0x65];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PositionApprovalCall {
    SinglePosition {
        position_manager: Address,
        spender: Address,
        token_id: U256,
    },
    OperatorForAll {
        position_manager: Address,
        operator: Address,
        approved: bool,
    },
}

impl PositionApprovalCall {
    pub fn position_manager(&self) -> Address {
        match self {
            Self::SinglePosition {
                position_manager, ..
            }
            | Self::OperatorForAll {
                position_manager, ..
            } => *position_manager,
        }
    }
}

pub fn decode_position_approval_call(tx: &MempoolTransaction) -> Option<PositionApprovalCall> {
    let to = tx.to.as_ref().map(|to| Address::from_slice(to))?;
    let selector = tx.input.get(0..4)?;

    if selector == ERC721_APPROVE_SELECTOR.as_slice() {
        let spender = calldata_address_param(&tx.input, 0)?;
        let token_id = calldata_u256_param(&tx.input, 1)?;
        return Some(PositionApprovalCall::SinglePosition {
            position_manager: to,
            spender,
            token_id,
        });
    }

    if selector == SET_APPROVAL_FOR_ALL_SELECTOR.as_slice() {
        let operator = calldata_address_param(&tx.input, 0)?;
        let approved = calldata_u256_param(&tx.input, 1)? != U256::ZERO;
        return Some(PositionApprovalCall::OperatorForAll {
            position_manager: to,
            operator,
            approved,
        });
    }

    None
}

pub fn is_position_approval_selector(selector: &[u8]) -> bool {
    selector == ERC721_APPROVE_SELECTOR.as_slice()
        || selector == SET_APPROVAL_FOR_ALL_SELECTOR.as_slice()
}

pub fn position_approval_function_name(selector: &[u8]) -> &'static str {
    if selector == SET_APPROVAL_FOR_ALL_SELECTOR.as_slice() {
        "setApprovalForAll"
    } else {
        "approve"
    }
}

fn calldata_address_param(input: &[u8], param_idx: usize) -> Option<Address> {
    let start = 4 + param_idx * 32 + 12;
    let end = start + 20;
    if input.len() < end {
        return None;
    }
    Some(Address::from_slice(&input[start..end]))
}

fn calldata_u256_param(input: &[u8], param_idx: usize) -> Option<U256> {
    let start = 4 + param_idx * 32;
    let end = start + 32;
    if input.len() < end {
        return None;
    }
    Some(U256::from_be_slice(&input[start..end]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;
    use serde_json::json;
    use std::time::Instant;

    #[test]
    fn decodes_single_position_approval() {
        let manager = address!("1111111111111111111111111111111111111111");
        let spender = address!("2222222222222222222222222222222222222222");
        let tx = tx(
            manager,
            single_approval_calldata(spender, U256::from(42u64)),
        );

        let call = decode_position_approval_call(&tx).expect("position approval");

        assert_eq!(
            call,
            PositionApprovalCall::SinglePosition {
                position_manager: manager,
                spender,
                token_id: U256::from(42u64)
            }
        );
    }

    #[test]
    fn decodes_operator_approval() {
        let manager = address!("1111111111111111111111111111111111111111");
        let operator = address!("2222222222222222222222222222222222222222");
        let tx = tx(manager, operator_approval_calldata(operator, true));

        let call = decode_position_approval_call(&tx).expect("operator approval");

        assert_eq!(
            call,
            PositionApprovalCall::OperatorForAll {
                position_manager: manager,
                operator,
                approved: true
            }
        );
    }

    fn tx(to: Address, input: Vec<u8>) -> MempoolTransaction {
        MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").to_vec(),
            to: Some(to.to_vec()),
            input,
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: Vec::new(),
            function_category: None,
        }
    }

    fn single_approval_calldata(spender: Address, token_id: U256) -> Vec<u8> {
        let mut input = ERC721_APPROVE_SELECTOR.to_vec();
        input.extend_from_slice(&pad_address(spender));
        input.extend_from_slice(&token_id.to_be_bytes::<32>());
        input
    }

    fn operator_approval_calldata(operator: Address, approved: bool) -> Vec<u8> {
        let mut input = SET_APPROVAL_FOR_ALL_SELECTOR.to_vec();
        input.extend_from_slice(&pad_address(operator));
        input.extend_from_slice(&U256::from(approved as u8).to_be_bytes::<32>());
        input
    }

    fn pad_address(address: Address) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[12..].copy_from_slice(address.as_slice());
        bytes
    }
}
