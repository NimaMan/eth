use alloy_primitives::{Address, Bytes, I256, U256};
use ethers::types::Address as EthersAddress;
use ethers::utils::get_contract_address;
use eyre::{eyre, Result};
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::UnsignedTransaction;

/// Bytecode for the MinimalV4Router contract stored at
/// `sol/baygus-router/contracts/uniswap_v4/MinimalV4Router.bin`.
const ROUTER_BYTECODE_HEX: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../sol/baygus-router/contracts/uniswap_v4/MinimalV4Router.bin"
));

static ROUTER_BYTECODE: Lazy<Vec<u8>> = Lazy::new(|| {
    hex::decode(ROUTER_BYTECODE_HEX.trim()).expect("Invalid MinimalV4Router bytecode hex")
});

const BAYGUS_ROUTER_ARTIFACT_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../sol/baygus-router/out/BaygusRouter.sol/BaygusRouter.json"
));

#[derive(Debug, Deserialize)]
struct FoundryBytecode {
    object: String,
}

#[derive(Debug, Deserialize)]
struct FoundryArtifact {
    bytecode: FoundryBytecode,
}

impl FoundryArtifact {
    fn object_hex(&self) -> &str {
        self.bytecode.object.trim_start_matches("0x")
    }
}

static BAYGUS_ROUTER_BYTECODE: Lazy<Vec<u8>> = Lazy::new(|| {
    let artifact: FoundryArtifact = serde_json::from_str(BAYGUS_ROUTER_ARTIFACT_JSON)
        .expect("failed to parse BaygusRouter artifact JSON");
    let object = artifact.object_hex();
    hex::decode(object).expect("invalid BaygusRouter bytecode hex")
});

/// Minimum sqrt price ratio supported by Uniswap v4 pools (in Q64.96 format).
static MIN_SQRT_RATIO_X96: Lazy<U256> = Lazy::new(|| U256::from(4_295_128_739u64));
/// Maximum sqrt price ratio supported by Uniswap v4 pools (in Q64.96 format).
static MAX_SQRT_RATIO_X96: Lazy<U256> = Lazy::new(|| {
    U256::from_str_radix("1461446703485210103287273052203988822378723970342", 10)
        .expect("invalid MAX_SQRT_RATIO constant")
});

/// Canonical WETH deposit selector.
const WETH_DEPOSIT_SELECTOR: [u8; 4] = [0xd0, 0xe3, 0x0d, 0xb0];
/// Canonical WETH withdraw selector.
const WETH_WITHDRAW_SELECTOR: [u8; 4] = [0x2e, 0x1a, 0x7d, 0x4d];
/// ERC20 approve selector.
const ERC20_APPROVE_SELECTOR: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3];
/// Minimal router swap selector keccak256("swapExactInputSingle((address,address,uint24,int24,address,bool,uint128,uint128,address,bool,bytes))")
const SWAP_EXACT_INPUT_SINGLE_SELECTOR: [u8; 4] = [0x2c, 0xc3, 0x0a, 0x09];
/// Baygus router multihop selector keccak256("swapExactInputPath((tuple(tuple(address,address,uint24,int24,address),tuple(bool,int256,uint160),bytes,address,int128,int128)[],address,int128,int128))")
const SWAP_EXACT_INPUT_PATH_SELECTOR: [u8; 4] = [0x57, 0xdb, 0x97, 0x6f];

/// Uniswap V4 PoolKey parameters required to perform a swap.
#[derive(Debug, Clone)]
pub struct UniswapV4PoolKey {
    pub currency0: Address,
    pub currency1: Address,
    pub fee: u32,
    pub tick_spacing: i32,
    pub hooks: Address,
}

#[derive(Debug, Clone)]
pub struct UniswapV4SwapParams {
    pub zero_for_one: bool,
    pub amount_specified: I256,
    pub sqrt_price_limit_x96: U256,
}

#[derive(Debug, Clone)]
pub struct UniswapV4BaygusHop {
    pub key: UniswapV4PoolKey,
    pub params: UniswapV4SwapParams,
    pub hook_data: Vec<u8>,
    pub hook_adapter: Address,
    pub min_amount0: i128,
    pub min_amount1: i128,
}

#[derive(Debug, Clone)]
pub struct UniswapV4BaygusMultiHopParams {
    pub hops: Vec<UniswapV4BaygusHop>,
    pub recipient: Address,
    pub final_min_amount0: i128,
    pub final_min_amount1: i128,
}

#[derive(Debug, Clone)]
pub struct UniswapV4SwapOrientation {
    pub zero_for_one: bool,
    pub input_currency: Address,
    pub output_currency: Address,
}

#[derive(Debug, Clone)]
pub struct UniswapV4BaygusSingleHopRequest {
    pub pool_key: UniswapV4PoolKey,
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: U256,
    pub recipient: Address,
    pub min_output: Option<U256>,
    pub hook_adapter: Address,
    pub hook_data: Vec<u8>,
    pub sqrt_price_limit_x96: Option<U256>,
}

#[derive(Debug, Clone)]
pub struct UniswapV4BaygusSingleHopCall {
    pub params: UniswapV4BaygusMultiHopParams,
    pub eth_value: U256,
    pub orientation: UniswapV4SwapOrientation,
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
        ..Default::default()
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
        ..Default::default()
    }
}

/// Build a WETH withdraw transaction (`withdraw(uint256)`).
pub fn build_weth_withdraw_tx(owner: Address, weth: Address, amount: U256) -> UnsignedTransaction {
    let mut data = Vec::with_capacity(4 + 32);
    data.extend_from_slice(&WETH_WITHDRAW_SELECTOR);
    data.extend_from_slice(&amount.to_be_bytes::<32>());

    UnsignedTransaction {
        from: Some(owner),
        to: Some(weth),
        gas: Some(150_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        nonce: None,
        ..Default::default()
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
        ..Default::default()
    }
}

/// Return a fresh copy of the Baygus router bytecode for deployment.
pub fn baygus_router_bytecode() -> Vec<u8> {
    BAYGUS_ROUTER_BYTECODE.clone()
}

/// Build the unsigned transaction that deploys the Baygus multi-hop router.
pub fn build_baygus_router_deploy_tx(
    deployer: Address,
    pool_manager: Address,
) -> UnsignedTransaction {
    let mut data = baygus_router_bytecode();
    data.extend_from_slice(&pad_address(pool_manager));

    UnsignedTransaction {
        from: Some(deployer),
        to: None,
        gas: Some(5_500_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        nonce: None,
        ..Default::default()
    }
}

/// Build a `swapExactInputPath` Baygus router transaction for one or more Uniswap v4 hops.
pub fn build_baygus_router_multihop_tx(
    router: Address,
    caller: Address,
    params: &UniswapV4BaygusMultiHopParams,
    eth_value: U256,
) -> Result<UnsignedTransaction> {
    let calldata = encode_swap_exact_input_path(params)?;
    Ok(UnsignedTransaction {
        from: Some(caller),
        to: Some(router),
        gas: None,
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(eth_value),
        data: Some(calldata),
        nonce: None,
        ..Default::default()
    })
}

/// Build a Baygus router single-hop exact-input call descriptor.
pub fn build_baygus_single_hop_exact_input_call(
    request: &UniswapV4BaygusSingleHopRequest,
) -> Result<UniswapV4BaygusSingleHopCall> {
    let orientation = infer_orientation_from_input(&request.pool_key, request.token_in)?;
    if request.token_out != orientation.output_currency {
        return Err(eyre!(
            "token_out {:x} does not match pool currencies (expected {:x})",
            request.token_out,
            orientation.output_currency
        ));
    }

    let amount_in_i256 =
        I256::try_from(request.amount_in).map_err(|_| eyre!("amount_in exceeds i256 range"))?;
    let amount_specified = -amount_in_i256;

    let mut hop = UniswapV4BaygusHop {
        key: request.pool_key.clone(),
        params: UniswapV4SwapParams {
            zero_for_one: orientation.zero_for_one,
            amount_specified,
            sqrt_price_limit_x96: request
                .sqrt_price_limit_x96
                .clone()
                .unwrap_or_else(|| default_sqrt_price_limit(orientation.zero_for_one)),
        },
        hook_data: request.hook_data.clone(),
        hook_adapter: request.hook_adapter,
        min_amount0: 0,
        min_amount1: 0,
    };

    let mut final_min_amount0 = 0i128;
    let mut final_min_amount1 = 0i128;
    if let Some(min_out) = request.min_output {
        let min_i128 = u256_to_i128(min_out)?;
        if orientation.zero_for_one {
            hop.min_amount1 = min_i128;
            final_min_amount1 = min_i128;
        } else {
            hop.min_amount0 = min_i128;
            final_min_amount0 = min_i128;
        }
    }

    let params = UniswapV4BaygusMultiHopParams {
        hops: vec![hop],
        recipient: request.recipient,
        final_min_amount0,
        final_min_amount1,
    };

    let eth_value = if orientation.input_currency == Address::ZERO {
        request.amount_in
    } else {
        U256::ZERO
    };

    Ok(UniswapV4BaygusSingleHopCall {
        params,
        eth_value,
        orientation,
    })
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
        ..Default::default()
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

fn encode_swap_exact_input_path(params: &UniswapV4BaygusMultiHopParams) -> Result<Bytes> {
    if params.hops.is_empty() {
        return Err(eyre!("Baygus router multi-hop requires at least one hop"));
    }

    const HEAD_WORDS: usize = 4;
    let mut data = Vec::with_capacity(4 + HEAD_WORDS * 32);
    data.extend_from_slice(&SWAP_EXACT_INPUT_PATH_SELECTOR);

    data.extend_from_slice(&pad_u64((HEAD_WORDS * 32) as u64));
    data.extend_from_slice(&pad_address(params.recipient));
    data.extend_from_slice(&pad_i128(params.final_min_amount0));
    data.extend_from_slice(&pad_i128(params.final_min_amount1));

    let mut tail = Vec::with_capacity(32);
    tail.extend_from_slice(&pad_u128(params.hops.len() as u128));
    for hop in &params.hops {
        tail.extend_from_slice(&encode_baygus_hop(hop));
    }

    data.extend_from_slice(&tail);
    Ok(Bytes::from(data))
}

fn encode_baygus_hop(hop: &UniswapV4BaygusHop) -> Vec<u8> {
    const HEAD_WORDS: usize = 12;
    let mut out = Vec::with_capacity(HEAD_WORDS * 32);

    out.extend_from_slice(&pad_address(hop.key.currency0));
    out.extend_from_slice(&pad_address(hop.key.currency1));
    out.extend_from_slice(&pad_u32(hop.key.fee));
    out.extend_from_slice(&pad_i32(hop.key.tick_spacing));
    out.extend_from_slice(&pad_address(hop.key.hooks));
    out.extend_from_slice(&pad_bool(hop.params.zero_for_one));
    out.extend_from_slice(&pad_i256(hop.params.amount_specified));
    out.extend_from_slice(&pad_u256(hop.params.sqrt_price_limit_x96));
    out.extend_from_slice(&pad_u64((HEAD_WORDS * 32) as u64));
    out.extend_from_slice(&pad_address(hop.hook_adapter));
    out.extend_from_slice(&pad_i128(hop.min_amount0));
    out.extend_from_slice(&pad_i128(hop.min_amount1));

    let padded = pad_bytes(&hop.hook_data);
    out.extend_from_slice(&pad_u128(hop.hook_data.len() as u128));
    out.extend_from_slice(&padded);

    out
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

fn default_sqrt_price_limit(zero_for_one: bool) -> U256 {
    if zero_for_one {
        MIN_SQRT_RATIO_X96.clone() + U256::from(1u8)
    } else {
        MAX_SQRT_RATIO_X96.clone() - U256::from(1u8)
    }
}

fn pad_u128(value: u128) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[16..].copy_from_slice(&value.to_be_bytes());
    buf
}

fn pad_i128(value: i128) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let bytes = value.to_be_bytes();
    if value >= 0 {
        buf[16..].copy_from_slice(&bytes);
    } else {
        buf.fill(0xff);
        buf[16..].copy_from_slice(&bytes);
    }
    buf
}

fn pad_i256(value: I256) -> [u8; 32] {
    value.to_be_bytes()
}

fn pad_u256(value: U256) -> [u8; 32] {
    value.to_be_bytes::<32>()
}

fn u256_to_i128(value: U256) -> Result<i128> {
    let bytes = value.to_be_bytes::<32>();
    if bytes[..16].iter().any(|&b| b != 0) {
        return Err(eyre!("value {value:#x} exceeds i128 range"));
    }
    let mut lower_bytes = [0u8; 16];
    lower_bytes.copy_from_slice(&bytes[16..]);
    let lower = u128::from_be_bytes(lower_bytes);
    if lower > i128::MAX as u128 {
        return Err(eyre!("value {value:#x} exceeds i128 range"));
    }
    Ok(lower as i128)
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
