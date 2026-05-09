use alloy_primitives::{Address, Bytes, U256};
use eyre::{eyre, Result};

mod universal_router;

pub use self::universal_router::{
    build_universal_router_v4_exact_input_single_tx, UniversalRouterV4ExactInputSingleRequest,
    UniversalRouterV4InputPayment,
};

use crate::UnsignedTransaction;

const WETH_DEPOSIT_SELECTOR: [u8; 4] = [0xd0, 0xe3, 0x0d, 0xb0];
const WETH_WITHDRAW_SELECTOR: [u8; 4] = [0x2e, 0x1a, 0x7d, 0x4d];
const ERC20_APPROVE_SELECTOR: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniswapV4PoolKey {
    pub currency0: Address,
    pub currency1: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UniswapV4SwapOrientation {
    pub zero_for_one: bool,
    pub input_currency: Address,
    pub output_currency: Address,
}

pub fn build_weth_deposit_tx(owner: Address, weth: Address, amount: U256) -> UnsignedTransaction {
    UnsignedTransaction {
        from: Some(owner),
        to: Some(weth),
        gas: Some(120_000),
        value: Some(amount),
        data: Some(Bytes::from(WETH_DEPOSIT_SELECTOR.to_vec())),
        ..Default::default()
    }
}

pub fn build_weth_withdraw_tx(owner: Address, weth: Address, amount: U256) -> UnsignedTransaction {
    let mut data = Vec::with_capacity(4 + 32);
    data.extend_from_slice(&WETH_WITHDRAW_SELECTOR);
    data.extend_from_slice(&amount.to_be_bytes::<32>());

    UnsignedTransaction {
        from: Some(owner),
        to: Some(weth),
        gas: Some(150_000),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        ..Default::default()
    }
}

pub fn build_token_approval_tx(
    owner: Address,
    token: Address,
    spender: Address,
    amount: U256,
) -> UnsignedTransaction {
    let mut data = Vec::with_capacity(4 + 32 + 32);
    data.extend_from_slice(&ERC20_APPROVE_SELECTOR);
    data.extend_from_slice(&pad_address(spender));
    data.extend_from_slice(&amount.to_be_bytes::<32>());

    UnsignedTransaction {
        from: Some(owner),
        to: Some(token),
        gas: Some(120_000),
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        ..Default::default()
    }
}

pub fn infer_orientation_from_input(
    key: &UniswapV4PoolKey,
    token_in: Address,
) -> Result<UniswapV4SwapOrientation> {
    if token_in == key.currency0 {
        Ok(UniswapV4SwapOrientation {
            zero_for_one: true,
            input_currency: key.currency0,
            output_currency: key.currency1,
        })
    } else if token_in == key.currency1 {
        Ok(UniswapV4SwapOrientation {
            zero_for_one: false,
            input_currency: key.currency1,
            output_currency: key.currency0,
        })
    } else {
        Err(eyre!(
            "token_in {:x} is not part of the supplied Uniswap v4 pool key",
            token_in
        ))
    }
}

pub fn infer_orientation_from_output(
    key: &UniswapV4PoolKey,
    token_out: Address,
) -> Result<UniswapV4SwapOrientation> {
    if token_out == key.currency1 {
        Ok(UniswapV4SwapOrientation {
            zero_for_one: true,
            input_currency: key.currency0,
            output_currency: key.currency1,
        })
    } else if token_out == key.currency0 {
        Ok(UniswapV4SwapOrientation {
            zero_for_one: false,
            input_currency: key.currency1,
            output_currency: key.currency0,
        })
    } else {
        Err(eyre!(
            "token_out {:x} is not part of the supplied Uniswap v4 pool key",
            token_out
        ))
    }
}

fn pad_address(address: Address) -> [u8; 32] {
    let mut out = [0_u8; 32];
    out[12..].copy_from_slice(address.as_slice());
    out
}
