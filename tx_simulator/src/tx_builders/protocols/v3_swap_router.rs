use crate::UnsignedTransaction;
use alloy_primitives::{address, Address, Bytes, U256};

pub const WETH_ADDRESS: Address = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactInputSingleAbi {
    WithDeadline,
    WithoutDeadline,
}

pub struct ExactInputSingleRequest {
    pub router: Address,
    pub caller: Address,
    pub token_in: Address,
    pub token_out: Address,
    pub fee: u32,
    pub recipient: Address,
    pub deadline: U256,
    pub amount_in: U256,
    pub amount_out_minimum: U256,
    pub sqrt_price_limit_x96: U256,
    pub value: U256,
    pub abi: ExactInputSingleAbi,
}

pub fn build_exact_input_single_tx(request: ExactInputSingleRequest) -> UnsignedTransaction {
    let calldata = encode_exact_input_single(
        request.abi,
        request.token_in,
        request.token_out,
        request.fee,
        request.recipient,
        request.deadline,
        request.amount_in,
        request.amount_out_minimum,
        request.sqrt_price_limit_x96,
    );

    UnsignedTransaction {
        from: Some(request.caller),
        to: Some(request.router),
        gas: Some(350_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(request.value),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

pub fn build_approve_for_router(
    router: Address,
    owner: Address,
    token: Address,
    amount: U256,
) -> UnsignedTransaction {
    let calldata = encode_approve(router, amount);
    UnsignedTransaction {
        from: Some(owner),
        to: Some(token),
        gas: Some(120_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    }
}

fn encode_exact_input_single(
    abi: ExactInputSingleAbi,
    token_in: Address,
    token_out: Address,
    fee: u32,
    recipient: Address,
    deadline: U256,
    amount_in: U256,
    amount_out_minimum: U256,
    sqrt_price_limit_x96: U256,
) -> Bytes {
    match abi {
        ExactInputSingleAbi::WithDeadline => encode_exact_input_single_with_deadline(
            token_in,
            token_out,
            fee,
            recipient,
            deadline,
            amount_in,
            amount_out_minimum,
            sqrt_price_limit_x96,
        ),
        ExactInputSingleAbi::WithoutDeadline => encode_exact_input_single_without_deadline(
            token_in,
            token_out,
            fee,
            recipient,
            amount_in,
            amount_out_minimum,
            sqrt_price_limit_x96,
        ),
    }
}

fn encode_exact_input_single_with_deadline(
    token_in: Address,
    token_out: Address,
    fee: u32,
    recipient: Address,
    deadline: U256,
    amount_in: U256,
    amount_out_minimum: U256,
    sqrt_price_limit_x96: U256,
) -> Bytes {
    let mut data = vec![0x41, 0x4b, 0xf3, 0x89];
    push_address(&mut data, token_in);
    push_address(&mut data, token_out);
    push_uint24(&mut data, fee);
    push_address(&mut data, recipient);
    data.extend_from_slice(&deadline.to_be_bytes::<32>());
    data.extend_from_slice(&amount_in.to_be_bytes::<32>());
    data.extend_from_slice(&amount_out_minimum.to_be_bytes::<32>());
    data.extend_from_slice(&sqrt_price_limit_x96.to_be_bytes::<32>());
    Bytes::from(data)
}

fn encode_exact_input_single_without_deadline(
    token_in: Address,
    token_out: Address,
    fee: u32,
    recipient: Address,
    amount_in: U256,
    amount_out_minimum: U256,
    sqrt_price_limit_x96: U256,
) -> Bytes {
    let mut data = vec![0x04, 0xe4, 0x5a, 0xaf];
    push_address(&mut data, token_in);
    push_address(&mut data, token_out);
    push_uint24(&mut data, fee);
    push_address(&mut data, recipient);
    data.extend_from_slice(&amount_in.to_be_bytes::<32>());
    data.extend_from_slice(&amount_out_minimum.to_be_bytes::<32>());
    data.extend_from_slice(&sqrt_price_limit_x96.to_be_bytes::<32>());
    Bytes::from(data)
}

fn encode_approve(spender: Address, amount: U256) -> Bytes {
    let mut data = vec![0x09, 0x5e, 0xa7, 0xb3];
    push_address(&mut data, spender);
    data.extend_from_slice(&amount.to_be_bytes::<32>());
    Bytes::from(data)
}

fn push_address(data: &mut Vec<u8>, address: Address) {
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(address.as_slice());
}

fn push_uint24(data: &mut Vec<u8>, value: u32) {
    data.extend_from_slice(&[0u8; 29]);
    let bytes = value.to_be_bytes();
    data.extend_from_slice(&bytes[1..]);
}
