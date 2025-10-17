use alloy_primitives::{Address, Bytes, U256};
use ethers::types::Address as EthersAddress;
use ethers::utils::get_contract_address;
use eyre::{eyre, Result};
use once_cell::sync::Lazy;

use tx_simulator::UnsignedTransaction;

/// Bytecode for the MinimalV4Router contract compiled from `contracts/uniswap_v4/MinimalV4Router.sol`.
const ROUTER_BYTECODE_HEX: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/uniswap_v4/MinimalV4Router.bin"
));

static ROUTER_BYTECODE: Lazy<Vec<u8>> = Lazy::new(|| {
    hex::decode(ROUTER_BYTECODE_HEX.trim()).expect("Invalid MinimalV4Router bytecode hex")
});

/// Canonical WETH deposit selector.
const WETH_DEPOSIT_SELECTOR: [u8; 4] = [0xd0, 0xe3, 0x0d, 0xb0];
/// ERC20 approve selector.
const ERC20_APPROVE_SELECTOR: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3];
/// Minimal router swap selector keccak256("swapExactInputSingle((address,address,uint24,int24,address,bool,uint128,uint128,address,bool,bytes))")
const SWAP_EXACT_INPUT_SINGLE_SELECTOR: [u8; 4] = [0x2c, 0xc3, 0x0a, 0x09];

/// Uniswap V4 PoolKey parameters required to perform a swap.
#[derive(Debug, Clone)]
pub struct UniswapV4PoolKey {
    pub currency0: Address,
    pub currency1: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,
}

/// Return a fresh copy of the router bytecode for deployment.
pub fn router_bytecode() -> Vec<u8> {
    ROUTER_BYTECODE.clone()
}

/// Build the unsigned transaction that deploys the minimal Uniswap v4 router.
pub fn build_router_deploy_tx(
    deployer: Address,
    pool_manager: Address,
    weth: Address,
) -> UnsignedTransaction {
    let mut data = router_bytecode();
    data.extend_from_slice(&encode_constructor_args(pool_manager, weth));

    UnsignedTransaction {
        from: Some(deployer),
        to: None,
        gas: Some(3_000_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        nonce: None,
    }
}

/// Build a WETH deposit transaction (`deposit() payable`).
pub fn build_weth_deposit_tx(owner: Address, weth: Address, amount: U256) -> UnsignedTransaction {
    UnsignedTransaction {
        from: Some(owner),
        to: Some(weth),
        gas: Some(120_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(amount),
        data: Some(Bytes::from(WETH_DEPOSIT_SELECTOR.to_vec())),
        nonce: None,
    }
}

/// Build an ERC20 approve transaction with the router as spender.
pub fn build_token_approval_tx(
    owner: Address,
    token: Address,
    router: Address,
    amount: U256,
) -> UnsignedTransaction {
    let mut data = Vec::with_capacity(4 + 32 + 32);
    data.extend_from_slice(&ERC20_APPROVE_SELECTOR);
    data.extend_from_slice(&pad_address(router));
    data.extend_from_slice(&amount.to_be_bytes::<32>());

    UnsignedTransaction {
        from: Some(owner),
        to: Some(token),
        gas: Some(120_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        nonce: None,
    }
}

/// Build a `swapExactInputSingle` call to the minimal router.
#[allow(clippy::too_many_arguments)]
pub fn build_swap_exact_input_single_tx(
    router: Address,
    caller: Address,
    key: &UniswapV4PoolKey,
    zero_for_one: bool,
    amount_in: U256,
    amount_out_min: U256,
    recipient: Address,
    unwrap_native: bool,
    hook_data: &[u8],
) -> Result<UnsignedTransaction> {
    let amount_in_u128: u128 = amount_in
        .try_into()
        .map_err(|_| eyre!("amount_in exceeds uint128"))?;
    let amount_out_min_u128: u128 = amount_out_min
        .try_into()
        .map_err(|_| eyre!("amount_out_min exceeds uint128"))?;

    let calldata = encode_swap_exact_input_single(
        key,
        zero_for_one,
        amount_in_u128,
        amount_out_min_u128,
        recipient,
        unwrap_native,
        hook_data,
    )?;

    let (currency_in, _) = resolve_currencies(key, zero_for_one);
    let value = if currency_in == Address::ZERO {
        U256::from(amount_in_u128)
    } else {
        U256::ZERO
    };

    Ok(UnsignedTransaction {
        from: Some(caller),
        to: Some(router),
        gas: Some(900_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(value),
        data: Some(calldata),
        nonce: None,
    })
}

fn encode_swap_exact_input_single(
    key: &UniswapV4PoolKey,
    zero_for_one: bool,
    amount_in: u128,
    amount_out_min: u128,
    recipient: Address,
    unwrap_native: bool,
    hook_data: &[u8],
) -> Result<Bytes> {
    const STATIC_WORDS: usize = 11;
    let padded_hook_len = ((hook_data.len() + 31) / 32) * 32;
    let mut data = Vec::with_capacity(4 + STATIC_WORDS * 32 + 32 + padded_hook_len);
    data.extend_from_slice(&SWAP_EXACT_INPUT_SINGLE_SELECTOR);

    // PoolKey encoding
    data.extend_from_slice(&pad_address(key.currency0));
    data.extend_from_slice(&pad_address(key.currency1));
    data.extend_from_slice(&pad_u32(key.fee));
    data.extend_from_slice(&pad_i32(key.tick_spacing));
    data.extend_from_slice(&pad_address(key.hooks));

    // Remaining tuple elements
    data.extend_from_slice(&pad_bool(zero_for_one));
    data.extend_from_slice(&pad_u128(amount_in));
    data.extend_from_slice(&pad_u128(amount_out_min));
    data.extend_from_slice(&pad_address(recipient));
    data.extend_from_slice(&pad_bool(unwrap_native));

    let offset = (STATIC_WORDS * 32) as u64;
    data.extend_from_slice(&pad_u64(offset));

    // Dynamic hook data
    data.extend_from_slice(&pad_u128(hook_data.len() as u128));
    data.extend_from_slice(&pad_bytes(hook_data));

    Ok(Bytes::from(data))
}

fn encode_constructor_args(pool_manager: Address, weth: Address) -> [u8; 64] {
    let mut args = [0u8; 64];
    args[12..32].copy_from_slice(pool_manager.as_slice());
    args[44..64].copy_from_slice(weth.as_slice());
    args
}

fn resolve_currencies(key: &UniswapV4PoolKey, zero_for_one: bool) -> (Address, Address) {
    if zero_for_one {
        (key.currency0, key.currency1)
    } else {
        (key.currency1, key.currency0)
    }
}

fn pad_address(address: Address) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[12..].copy_from_slice(address.as_slice());
    buf
}

fn pad_bool(value: bool) -> [u8; 32] {
    let mut buf = [0u8; 32];
    if value {
        buf[31] = 1;
    }
    buf
}

fn pad_u32(value: u32) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[28..].copy_from_slice(&value.to_be_bytes());
    buf
}

fn pad_u64(value: u64) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[24..].copy_from_slice(&value.to_be_bytes());
    buf
}

fn pad_i32(value: i32) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let bytes = value.to_be_bytes();
    if value >= 0 {
        buf[28..].copy_from_slice(&bytes);
    } else {
        buf.fill(0xff);
        buf[28..].copy_from_slice(&bytes);
    }
    buf
}

fn pad_u128(value: u128) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[16..].copy_from_slice(&value.to_be_bytes());
    buf
}

fn pad_bytes(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::from(data);
    while out.len() % 32 != 0 {
        out.push(0);
    }
    out
}

/// Compute the CREATE contract address for the deployer + nonce pair.
pub fn compute_contract_address(deployer: Address, nonce: u64) -> Address {
    let deployer_eth = EthersAddress::from_slice(deployer.as_slice());
    let contract = get_contract_address(deployer_eth, ethers::types::U256::from(nonce));
    Address::from_slice(contract.as_bytes())
}
