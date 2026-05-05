use alloy_primitives::{Address, Bytes, I256, U256};
use eyre::{eyre, Result};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::UnsignedTransaction;

const MINIMAL_ROUTER_BYTECODE_RELATIVE_PATH: &str = "contracts/uniswap_v4/MinimalV4Router.bin";
const BAYGUS_ROUTER_ARTIFACT_RELATIVE_PATH: &str = "out/BaygusRouter.sol/BaygusRouter.json";
const MOCK_POOL_MANAGER_ARTIFACT_RELATIVE_PATH: &str =
    "out/MockPoolManager.sol/MockPoolManager.json";
const MOCK_ERC20_ARTIFACT_RELATIVE_PATH: &str = "out/MockERC20.sol/MockERC20.json";

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

/// Repository-local Baygus router artifact root under `soleth`.
pub fn soleth_baygus_router_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("soleth/baygus-router")
}

pub fn minimal_router_bytecode_path() -> PathBuf {
    soleth_baygus_router_dir().join(MINIMAL_ROUTER_BYTECODE_RELATIVE_PATH)
}

pub fn baygus_router_artifact_path() -> PathBuf {
    soleth_baygus_router_dir().join(BAYGUS_ROUTER_ARTIFACT_RELATIVE_PATH)
}

pub fn mock_pool_manager_artifact_path() -> PathBuf {
    soleth_baygus_router_dir().join(MOCK_POOL_MANAGER_ARTIFACT_RELATIVE_PATH)
}

pub fn mock_erc20_artifact_path() -> PathBuf {
    soleth_baygus_router_dir().join(MOCK_ERC20_ARTIFACT_RELATIVE_PATH)
}

fn decode_hex_bytecode(hex_value: &str, label: &str) -> Result<Vec<u8>> {
    let trimmed = hex_value.trim().trim_start_matches("0x");
    if trimmed.is_empty() {
        return Err(eyre!("{label} bytecode is empty"));
    }

    hex::decode(trimmed).map_err(|err| eyre!("invalid {label} bytecode hex: {err}"))
}

fn read_raw_bytecode_file(path: &Path, label: &str) -> Result<Vec<u8>> {
    let contents = fs::read_to_string(path).map_err(|err| {
        eyre!(
            "failed to read {label} bytecode from {}: {err}; build or restore soleth/baygus-router artifacts first",
            path.display()
        )
    })?;
    decode_hex_bytecode(&contents, label)
}

fn read_foundry_artifact_bytecode(path: &Path, label: &str) -> Result<Vec<u8>> {
    let contents = fs::read_to_string(path).map_err(|err| {
        eyre!(
            "failed to read {label} artifact from {}: {err}; build or restore soleth/baygus-router artifacts first",
            path.display()
        )
    })?;
    let artifact: FoundryArtifact = serde_json::from_str(&contents)
        .map_err(|err| eyre!("failed to parse {label} artifact {}: {err}", path.display()))?;
    decode_hex_bytecode(artifact.object_hex(), label)
}

/// Minimum sqrt price ratio supported by Uniswap v4 pools (in Q64.96 format).
const MIN_SQRT_RATIO_X96: U256 = U256::from_limbs([4_295_128_739, 0, 0, 0]);
/// Maximum sqrt price ratio supported by Uniswap v4 pools (in Q64.96 format).
const MAX_SQRT_RATIO_X96: U256 =
    U256::from_limbs([0x5d951d5263988d26, 0xefd1fc6a50648849, 0xfffd8963, 0]);

/// Canonical WETH deposit selector.
const WETH_DEPOSIT_SELECTOR: [u8; 4] = [0xd0, 0xe3, 0x0d, 0xb0];
/// Canonical WETH withdraw selector.
const WETH_WITHDRAW_SELECTOR: [u8; 4] = [0x2e, 0x1a, 0x7d, 0x4d];
/// ERC20 approve selector.
const ERC20_APPROVE_SELECTOR: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3];
/// MockERC20 mint selector keccak256("mint(address,uint256)")
const MOCK_ERC20_MINT_SELECTOR: [u8; 4] = [0x40, 0xc1, 0x0f, 0x19];
/// ERC20 balanceOf selector keccak256("balanceOf(address)")
const ERC20_BALANCE_OF_SELECTOR: [u8; 4] = [0x70, 0xa0, 0x82, 0x31];
/// Minimal router swap selector keccak256("swapExactInputSingle((address,address,uint24,int24,address,bool,uint128,uint128,address,bool,bytes))")
const SWAP_EXACT_INPUT_SINGLE_SELECTOR: [u8; 4] = [0x2c, 0xc3, 0x0a, 0x09];
/// Baygus router multihop selector keccak256("swapExactInputPath((((address,address,uint24,int24,address),(bool,int256,uint160),bytes,address,int128,int128)[],address,int128,int128))")
const SWAP_EXACT_INPUT_PATH_SELECTOR: [u8; 4] = [0xc8, 0x33, 0x24, 0x4d];
/// Baygus router single hop selector keccak256("swapExactInputSingle(((address,address,uint24,int24,address),(bool,int256,uint160),address,bytes,address,int128,int128))")
const BAYGUS_SWAP_EXACT_INPUT_SINGLE_SELECTOR: [u8; 4] = [0xa2, 0xda, 0x1d, 0x92];

/// MockPoolManager setRouter selector keccak256("setRouter(address)")
const MOCK_PM_SET_ROUTER_SELECTOR: [u8; 4] = [0xb8, 0x70, 0x0e, 0x6d];
/// MockPoolManager queueDelta selector keccak256("queueDelta(int128,int128)")
const MOCK_PM_QUEUE_DELTA_SELECTOR: [u8; 4] = [0x3f, 0x52, 0xf2, 0x77];

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

#[derive(Debug, Clone)]
pub struct BaygusRouterAdapterConfig {
    pub uniswap_v2_router: Address,
    pub sushiswap_router: Address,
    pub uniswap_v3_router: Address,
    pub balancer_vault: Address,
    pub permit2: Address,
}

fn parse_mainnet_address(value: &str, label: &str) -> Result<Address> {
    Address::from_str(value).map_err(|err| eyre!("invalid {label} address {value}: {err}"))
}

pub fn default_baygus_adapter_config() -> Result<BaygusRouterAdapterConfig> {
    Ok(BaygusRouterAdapterConfig {
        uniswap_v2_router: parse_mainnet_address(
            "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
            "Uniswap V2 router",
        )?,
        sushiswap_router: parse_mainnet_address(
            "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F",
            "SushiSwap router",
        )?,
        uniswap_v3_router: parse_mainnet_address(
            "0xE592427A0AEce92De3Edee1F18E0157C05861564",
            "Uniswap V3 router",
        )?,
        balancer_vault: parse_mainnet_address(
            "0xBA12222222228d8Ba445958a75a0704d566BF2C8",
            "Balancer vault",
        )?,
        permit2: parse_mainnet_address("0x000000000022D473030F116dDEE9F6B43aC78BA3", "Permit2")?,
    })
}

/// Load the minimal Uniswap v4 router bytecode for deployment.
pub fn router_bytecode() -> Result<Vec<u8>> {
    read_raw_bytecode_file(&minimal_router_bytecode_path(), "MinimalV4Router")
}

/// Load the MockPoolManager bytecode for deployment.
pub fn mock_pool_manager_bytecode() -> Result<Vec<u8>> {
    read_foundry_artifact_bytecode(&mock_pool_manager_artifact_path(), "MockPoolManager")
}

/// Load the MockERC20 bytecode for deployment.
pub fn mock_erc20_bytecode() -> Result<Vec<u8>> {
    read_foundry_artifact_bytecode(&mock_erc20_artifact_path(), "MockERC20")
}

/// Build the unsigned transaction that deploys the MockPoolManager.
pub fn build_mock_pool_manager_deploy_tx(deployer: Address) -> Result<UnsignedTransaction> {
    Ok(UnsignedTransaction {
        from: Some(deployer),
        to: None,
        gas: Some(3_000_000),
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(Bytes::from(mock_pool_manager_bytecode()?)),
        nonce: None,
        ..Default::default()
    })
}

/// Build the unsigned transaction that deploys the MockERC20.
/// Constructor: `constructor(string name, string symbol, uint8 decimals)`
pub fn build_mock_erc20_deploy_tx(
    deployer: Address,
    name: &str,
    symbol: &str,
    decimals: u8,
) -> Result<UnsignedTransaction> {
    // Manual encoding of string/uint8 params
    // HEAD: Offset(name), Offset(symbol), decimals (padded)
    // TAIL: Name len+bytes, Symbol len+bytes
    let mut data = mock_erc20_bytecode()?;

    let mut args = Vec::new();
    let name_bytes = name.as_bytes();
    let symbol_bytes = symbol.as_bytes();

    // Offsets
    let name_offset = 3 * 32; // name_off, symbol_off, decimals
    let symbol_offset = name_offset + ((name_bytes.len() + 31) / 32 * 32) + 32;

    args.extend_from_slice(&pad_u64(name_offset as u64));
    args.extend_from_slice(&pad_u64(symbol_offset as u64));
    args.extend_from_slice(&pad_u64(decimals as u64)); // u8 padded to 32 bytes

    // Name
    args.extend_from_slice(&pad_u64(name_bytes.len() as u64));
    args.extend_from_slice(&pad_bytes(name_bytes));

    // Symbol
    args.extend_from_slice(&pad_u64(symbol_bytes.len() as u64));
    args.extend_from_slice(&pad_bytes(symbol_bytes));

    data.extend_from_slice(&args);

    Ok(UnsignedTransaction {
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
    })
}

/// Build transaction to call `mint(address,uint256)` on MockERC20.
pub fn build_mock_erc20_mint_tx(
    deployer: Address,
    token: Address,
    recipient: Address,
    amount: U256,
) -> UnsignedTransaction {
    let mut data = Vec::with_capacity(4 + 32 + 32);
    data.extend_from_slice(&MOCK_ERC20_MINT_SELECTOR);
    data.extend_from_slice(&pad_address(recipient));
    data.extend_from_slice(&pad_u256(amount));

    UnsignedTransaction {
        from: Some(deployer),
        to: Some(token),
        gas: None,
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        nonce: None,
        ..Default::default()
    }
}

/// Build transaction to call `balanceOf(address)` on ERC20.
pub fn build_erc20_balance_of_tx(
    caller: Address,
    token: Address,
    account: Address,
) -> UnsignedTransaction {
    let mut data = Vec::with_capacity(4 + 32);
    data.extend_from_slice(&ERC20_BALANCE_OF_SELECTOR);
    data.extend_from_slice(&pad_address(account));

    UnsignedTransaction {
        from: Some(caller),
        to: Some(token),
        gas: None,
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        nonce: None,
        ..Default::default()
    }
}

/// Build transaction to call `setRouter(address)` on MockPoolManager.
pub fn build_mock_pm_set_router_tx(
    deployer: Address,
    pool_manager: Address,
    router: Address,
) -> UnsignedTransaction {
    let mut data = Vec::with_capacity(4 + 32);
    data.extend_from_slice(&MOCK_PM_SET_ROUTER_SELECTOR);
    data.extend_from_slice(&pad_address(router));

    UnsignedTransaction {
        from: Some(deployer),
        to: Some(pool_manager),
        gas: None,
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        nonce: None,
        ..Default::default()
    }
}

/// Build transaction to call `queueDelta(int128,int128)` on MockPoolManager.
pub fn build_mock_pm_queue_delta_tx(
    deployer: Address,
    pool_manager: Address,
    amount0: i128,
    amount1: i128,
) -> UnsignedTransaction {
    let mut data = Vec::with_capacity(4 + 32 + 32);
    data.extend_from_slice(&MOCK_PM_QUEUE_DELTA_SELECTOR);
    data.extend_from_slice(&pad_i128(amount0));
    data.extend_from_slice(&pad_i128(amount1));

    UnsignedTransaction {
        from: Some(deployer),
        to: Some(pool_manager),
        gas: None,
        gas_price: None,
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        value: Some(U256::ZERO),
        data: Some(Bytes::from(data)),
        nonce: None,
        ..Default::default()
    }
}

/// Build the unsigned transaction that deploys the minimal Uniswap v4 router.
pub fn build_router_deploy_tx(
    deployer: Address,
    pool_manager: Address,
    weth: Address,
) -> Result<UnsignedTransaction> {
    let mut data = router_bytecode()?;
    data.extend_from_slice(&encode_constructor_args(pool_manager, weth));

    Ok(UnsignedTransaction {
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
    })
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

/// Load the Baygus router bytecode for deployment.
pub fn baygus_router_bytecode() -> Result<Vec<u8>> {
    read_foundry_artifact_bytecode(&baygus_router_artifact_path(), "BaygusRouter")
}

/// Build the unsigned transaction that deploys the Baygus multi-hop router.
pub fn build_baygus_router_deploy_tx(
    deployer: Address,
    pool_manager: Address,
) -> Result<UnsignedTransaction> {
    build_baygus_router_deploy_tx_with_adapters(
        deployer,
        pool_manager,
        &default_baygus_adapter_config()?,
    )
}

pub fn build_baygus_router_deploy_tx_with_adapters(
    deployer: Address,
    pool_manager: Address,
    adapters: &BaygusRouterAdapterConfig,
) -> Result<UnsignedTransaction> {
    let mut data = baygus_router_bytecode()?;
    data.extend_from_slice(&pad_address(pool_manager));
    data.extend_from_slice(&pad_address(adapters.uniswap_v2_router));
    data.extend_from_slice(&pad_address(adapters.sushiswap_router));
    data.extend_from_slice(&pad_address(adapters.uniswap_v3_router));
    data.extend_from_slice(&pad_address(adapters.balancer_vault));
    data.extend_from_slice(&pad_address(adapters.permit2));

    Ok(UnsignedTransaction {
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
    })
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

/// Build a `swapExactInputSingle` Baygus router transaction.
pub fn build_baygus_swap_exact_input_single_tx(
    router: Address,
    caller: Address,
    request: &UniswapV4BaygusSingleHopRequest,
) -> Result<UnsignedTransaction> {
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

    let params = UniswapV4SwapParams {
        zero_for_one: orientation.zero_for_one,
        amount_specified,
        sqrt_price_limit_x96: request
            .sqrt_price_limit_x96
            .clone()
            .unwrap_or_else(|| default_sqrt_price_limit(orientation.zero_for_one)),
    };

    let mut min_amount0 = 0i128;
    let mut min_amount1 = 0i128;
    if let Some(min_out) = request.min_output {
        let min_i128 = u256_to_i128(min_out)?;
        if orientation.zero_for_one {
            min_amount1 = min_i128;
        } else {
            min_amount0 = min_i128;
        }
    }

    let calldata = encode_baygus_swap_exact_input_single(
        &request.pool_key,
        &params,
        request.recipient,
        &request.hook_data,
        request.hook_adapter,
        min_amount0,
        min_amount1,
    )?;

    let eth_value = if orientation.input_currency == Address::ZERO {
        request.amount_in
    } else {
        U256::ZERO
    };

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

fn encode_baygus_swap_exact_input_single(
    key: &UniswapV4PoolKey,
    params: &UniswapV4SwapParams,
    recipient: Address,
    hook_data: &[u8],
    hook_adapter: Address,
    min_amount0: i128,
    min_amount1: i128,
) -> Result<Bytes> {
    // Struct SwapExactInputSingleParams:
    // key: 5 words
    // params: 3 words
    // recipient: 1 word
    // hookData: 1 word (offset)
    // hookAdapter: 1 word
    // minAmount0: 1 word
    // minAmount1: 1 word
    // Total Head: 13 words
    const HEAD_WORDS: usize = 13;

    let mut data = Vec::with_capacity(4 + HEAD_WORDS * 32);
    data.extend_from_slice(&BAYGUS_SWAP_EXACT_INPUT_SINGLE_SELECTOR);

    // Dynamic argument (the struct itself) is at offset 32 (after the offset pointer)
    // Standard Solidity encoding for a dynamic struct as an argument:
    // 1. Offset to the start of the struct data.
    data.extend_from_slice(&pad_u64(32));

    // Struct encoding start
    // key
    data.extend_from_slice(&pad_address(key.currency0));
    data.extend_from_slice(&pad_address(key.currency1));
    data.extend_from_slice(&pad_u32(key.fee));
    data.extend_from_slice(&pad_i32(key.tick_spacing));
    data.extend_from_slice(&pad_address(key.hooks));
    // params
    data.extend_from_slice(&pad_bool(params.zero_for_one));
    data.extend_from_slice(&pad_i256(params.amount_specified));
    data.extend_from_slice(&pad_u256(params.sqrt_price_limit_x96));
    // recipient
    data.extend_from_slice(&pad_address(recipient));

    // offset to hookData (relative to start of struct)
    // The struct head size is 13 words.
    data.extend_from_slice(&pad_u64((HEAD_WORDS * 32) as u64));

    // hookAdapter, minAmount0, minAmount1
    data.extend_from_slice(&pad_address(hook_adapter));
    data.extend_from_slice(&pad_i128(min_amount0));
    data.extend_from_slice(&pad_i128(min_amount1));

    // Tail: hookData length + bytes
    data.extend_from_slice(&pad_u128(hook_data.len() as u128));
    data.extend_from_slice(&pad_bytes(hook_data));

    Ok(Bytes::from(data))
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
    let mut data = Vec::with_capacity(4 + 32 + HEAD_WORDS * 32);
    data.extend_from_slice(&SWAP_EXACT_INPUT_PATH_SELECTOR);

    data.extend_from_slice(&pad_u64(32));
    data.extend_from_slice(&pad_u64((HEAD_WORDS * 32) as u64));
    data.extend_from_slice(&pad_address(params.recipient));
    data.extend_from_slice(&pad_i128(params.final_min_amount0));
    data.extend_from_slice(&pad_i128(params.final_min_amount1));

    let mut offsets = Vec::new();
    let mut bodies = Vec::new();
    let mut current_relative_offset = (params.hops.len() * 32) as u64;

    for hop in &params.hops {
        let encoded_hop = encode_baygus_hop(hop);
        offsets.push(current_relative_offset);
        bodies.extend_from_slice(&encoded_hop);
        current_relative_offset += encoded_hop.len() as u64;
    }

    let mut tail = Vec::new();
    tail.extend_from_slice(&pad_u128(params.hops.len() as u128));
    for offset in offsets {
        tail.extend_from_slice(&pad_u64(offset));
    }
    tail.extend_from_slice(&bodies);

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
        MIN_SQRT_RATIO_X96 + U256::from(1u8)
    } else {
        MAX_SQRT_RATIO_X96 - U256::from(1u8)
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
    deployer.create(nonce)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn decodes_hex_bytecode_with_optional_prefix() {
        assert_eq!(
            decode_hex_bytecode("0x60016002", "test").expect("prefixed hex should decode"),
            vec![0x60, 0x01, 0x60, 0x02]
        );
        assert_eq!(
            decode_hex_bytecode("6003", "test").expect("raw hex should decode"),
            vec![0x60, 0x03]
        );
    }

    #[test]
    fn reads_foundry_artifact_bytecode_from_runtime_path() {
        let dir = tempdir().expect("tempdir");
        let artifact_path = dir.path().join("Artifact.json");
        fs::write(&artifact_path, r#"{"bytecode":{"object":"0x60016002"}}"#)
            .expect("write artifact");

        let bytecode = read_foundry_artifact_bytecode(&artifact_path, "TestArtifact")
            .expect("artifact bytecode");

        assert_eq!(bytecode, vec![0x60, 0x01, 0x60, 0x02]);
    }

    #[test]
    fn default_artifact_paths_point_at_soleth() {
        assert!(minimal_router_bytecode_path()
            .ends_with("soleth/baygus-router/contracts/uniswap_v4/MinimalV4Router.bin"));
        assert!(baygus_router_artifact_path()
            .ends_with("soleth/baygus-router/out/BaygusRouter.sol/BaygusRouter.json"));
        assert!(mock_pool_manager_artifact_path()
            .ends_with("soleth/baygus-router/out/MockPoolManager.sol/MockPoolManager.json"));
        assert!(mock_erc20_artifact_path()
            .ends_with("soleth/baygus-router/out/MockERC20.sol/MockERC20.json"));
    }

    #[test]
    fn infers_v4_swap_orientation_from_input() {
        let token0 = Address::from([0x11; 20]);
        let token1 = Address::from([0x22; 20]);
        let key = UniswapV4PoolKey {
            currency0: token0,
            currency1: token1,
            fee: 3_000,
            tick_spacing: 60,
            hooks: Address::ZERO,
        };

        let zero_for_one = infer_orientation_from_input(&key, token0).expect("token0 input");
        assert!(zero_for_one.zero_for_one);
        assert_eq!(zero_for_one.output_currency, token1);

        let one_for_zero = infer_orientation_from_input(&key, token1).expect("token1 input");
        assert!(!one_for_zero.zero_for_one);
        assert_eq!(one_for_zero.output_currency, token0);
    }

    #[test]
    fn default_sqrt_price_limits_stay_inside_uniswap_v4_bounds() {
        assert_eq!(
            default_sqrt_price_limit(true),
            MIN_SQRT_RATIO_X96 + U256::from(1u8)
        );
        assert_eq!(
            default_sqrt_price_limit(false),
            MAX_SQRT_RATIO_X96 - U256::from(1u8)
        );
    }

    #[test]
    fn encodes_baygus_path_as_dynamic_struct_argument() {
        let token0 = Address::from([0x11; 20]);
        let token1 = Address::from([0x22; 20]);
        let recipient = Address::from([0x33; 20]);
        let params = UniswapV4BaygusMultiHopParams {
            hops: vec![UniswapV4BaygusHop {
                key: UniswapV4PoolKey {
                    currency0: token0,
                    currency1: token1,
                    fee: 500,
                    tick_spacing: 10,
                    hooks: Address::ZERO,
                },
                params: UniswapV4SwapParams {
                    zero_for_one: true,
                    amount_specified: I256::try_from(U256::from(1u8)).unwrap(),
                    sqrt_price_limit_x96: default_sqrt_price_limit(true),
                },
                hook_data: Vec::new(),
                hook_adapter: Address::ZERO,
                min_amount0: 0,
                min_amount1: 0,
            }],
            recipient,
            final_min_amount0: 0,
            final_min_amount1: 0,
        };

        let calldata = encode_swap_exact_input_path(&params).expect("path calldata");
        let data = calldata.as_ref();
        assert_eq!(&data[..4], &SWAP_EXACT_INPUT_PATH_SELECTOR);
        assert_eq!(U256::from_be_slice(&data[4..36]), U256::from(32));
        assert_eq!(U256::from_be_slice(&data[36..68]), U256::from(128));
    }
}
