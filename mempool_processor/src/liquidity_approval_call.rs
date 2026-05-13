use crate::mempool_fetcher::MempoolTransaction;
use alloy_primitives::{address, Address, U256};
use reth_chain_query::common_addresses::{POOL_FACTORIES, ROUTERS};

pub const ERC20_APPROVE_SELECTOR: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3];
pub const PERMIT2_APPROVE_SELECTOR: [u8; 4] = [0x87, 0x51, 0x7c, 0x45];
pub const PERMIT2_ADDRESS: Address = address!("000000000022D473030F116dDEE9F6B43aC78BA3");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiquidityApprovalProtocol {
    Erc20Approve,
    Permit2Approve,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiquidityApprovalCall {
    pub ownership_token: Address,
    pub spender: Address,
    pub amount: U256,
    pub protocol: LiquidityApprovalProtocol,
}

pub fn decode_liquidity_approval_call(tx: &MempoolTransaction) -> Option<LiquidityApprovalCall> {
    let selector = tx.input.get(0..4)?;

    if selector == ERC20_APPROVE_SELECTOR.as_slice() {
        let ownership_token = tx.to.as_ref().map(|to| Address::from_slice(to))?;
        let spender = calldata_address_param(&tx.input, 0)?;
        let amount = calldata_u256_param(&tx.input, 1)?;
        return Some(LiquidityApprovalCall {
            ownership_token,
            spender,
            amount,
            protocol: LiquidityApprovalProtocol::Erc20Approve,
        });
    }

    if selector == PERMIT2_APPROVE_SELECTOR.as_slice() {
        let to = tx.to.as_ref().map(|to| Address::from_slice(to))?;
        if to != PERMIT2_ADDRESS {
            return None;
        }
        let ownership_token = calldata_address_param(&tx.input, 0)?;
        let spender = calldata_address_param(&tx.input, 1)?;
        let amount = calldata_u256_param(&tx.input, 2)?;
        return Some(LiquidityApprovalCall {
            ownership_token,
            spender,
            amount,
            protocol: LiquidityApprovalProtocol::Permit2Approve,
        });
    }

    None
}

pub fn is_liquidity_approval_selector(selector: &[u8]) -> bool {
    selector == ERC20_APPROVE_SELECTOR.as_slice() || selector == PERMIT2_APPROVE_SELECTOR.as_slice()
}

pub fn liquidity_approval_function_name(selector: &[u8]) -> &'static str {
    if selector == PERMIT2_APPROVE_SELECTOR.as_slice() {
        "permit2_approve"
    } else {
        "approve"
    }
}

pub fn is_known_liquidity_approval_spender(spender: &Address) -> bool {
    *spender == PERMIT2_ADDRESS
        || *spender == POOL_FACTORIES["balancer_vault"]
        || ROUTERS.values().any(|router| router == spender)
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
    use crate::mempool_fetcher::MempoolTransaction;
    use serde_json::json;
    use std::time::Instant;

    #[test]
    fn decodes_permit2_liquidity_approval_call() {
        let token = address!("1111111111111111111111111111111111111111");
        let spender = address!("2222222222222222222222222222222222222222");
        let tx = tx(
            Some(PERMIT2_ADDRESS),
            permit2_approve_calldata(token, spender),
        );

        let call = decode_liquidity_approval_call(&tx).expect("permit2 approval");

        assert_eq!(call.protocol, LiquidityApprovalProtocol::Permit2Approve);
        assert_eq!(call.ownership_token, token);
        assert_eq!(call.spender, spender);
        assert_eq!(call.amount, U256::from(1000u64));
    }

    #[test]
    fn rejects_permit2_selector_sent_to_other_contract() {
        let token = address!("1111111111111111111111111111111111111111");
        let spender = address!("2222222222222222222222222222222222222222");
        let tx = tx(
            Some(address!("3333333333333333333333333333333333333333")),
            permit2_approve_calldata(token, spender),
        );

        assert!(decode_liquidity_approval_call(&tx).is_none());
    }

    fn tx(to: Option<Address>, input: Vec<u8>) -> MempoolTransaction {
        MempoolTransaction {
            hash: "0xtx".to_string(),
            data: json!({}),
            detection_ns: 0,
            detection_time: Instant::now(),
            latency_ns: 0,
            from: address!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").to_vec(),
            to: to.map(|address| address.to_vec()),
            input,
            value: U256::ZERO,
            gas_price: Some(U256::ZERO),
            functions: Vec::new(),
            function_category: None,
        }
    }

    fn permit2_approve_calldata(token: Address, spender: Address) -> Vec<u8> {
        let mut input = PERMIT2_APPROVE_SELECTOR.to_vec();
        input.extend_from_slice(&pad_address(token));
        input.extend_from_slice(&pad_address(spender));
        input.extend_from_slice(&U256::from(1000u64).to_be_bytes::<32>());
        input.extend_from_slice(&U256::from(1234u64).to_be_bytes::<32>());
        input
    }

    fn pad_address(address: Address) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[12..].copy_from_slice(address.as_slice());
        bytes
    }
}
