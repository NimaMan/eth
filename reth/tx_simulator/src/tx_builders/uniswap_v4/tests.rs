use super::artifacts::decode_hex_bytecode;
use super::*;
use std::fs;
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
    fs::write(&artifact_path, r#"{"bytecode":{"object":"0x60016002"}}"#).expect("write artifact");

    let bytecode =
        read_foundry_artifact_bytecode(&artifact_path, "TestArtifact").expect("artifact bytecode");

    assert_eq!(bytecode, vec![0x60, 0x01, 0x60, 0x02]);
}

#[test]
fn default_artifact_paths_point_at_soleth() {
    assert!(minimal_router_bytecode_path()
        .ends_with("soleth/baygus-executor/contracts/uniswap_v4/MinimalV4Router.bin"));
    assert!(baygus_executor_artifact_path()
        .ends_with("soleth/baygus-executor/out/BaygusExecutor.sol/BaygusExecutor.json"));
    assert!(mock_pool_manager_artifact_path()
        .ends_with("soleth/baygus-executor/out/MockPoolManager.sol/MockPoolManager.json"));
    assert!(mock_erc20_artifact_path()
        .ends_with("soleth/baygus-executor/out/MockERC20.sol/MockERC20.json"));
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
