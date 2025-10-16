use alloy_primitives::{Address, Bytes, U256};
use eyre::Result;
use std::str::FromStr;
use tx_simulator::{CallFrame, TxSimulator, UnsignedTransaction};

const ROUTER_V2: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const TREVT_TOKEN: &str = "0xbAB0e2F25336C88f455a6E36c3973F3858daD4F6";
const TREVT_POOL: &str = "0xb83B72d6bDC895A6eA409537e21B5A3EffB5e6c0";
const TEST_BUYER: &str = "0x0C96c602b1b332B8AB2093E5d72D804a24bd5689";
const BLOCK_BEFORE_LIQUIDITY: u64 = 23_581_483;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 Uniswap V2 swap before pool deployment");
    println!("=========================================\n");

    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());

    let simulator = TxSimulator::new(&reth_datadir)?;
    println!("Using block: {}", BLOCK_BEFORE_LIQUIDITY);

    let router = Address::from_str(ROUTER_V2)?;
    let weth = Address::from_str(WETH)?;
    let token = Address::from_str(TREVT_TOKEN)?;
    let pool = Address::from_str(TREVT_POOL)?;
    let buyer = Address::from_str(TEST_BUYER)?;

    let buy_amount = U256::from(10u128.pow(16)); // 0.01 ETH
    let calldata =
        encode_swap_exact_eth_for_tokens(U256::ZERO, &[weth, token], buyer, U256::from(u64::MAX));

    let unsigned = UnsignedTransaction {
        from: Some(buyer),
        to: Some(router),
        gas: Some(500_000),
        value: Some(buy_amount),
        data: Some(calldata),
        ..Default::default()
    };

    let result = simulator
        .simulate_unsigned_transaction_with_trace(
            unsigned.clone(),
            Some(BLOCK_BEFORE_LIQUIDITY),
            None,
        )
        .await?;

    let state = simulator.get_chain_state_at_block(BLOCK_BEFORE_LIQUIDITY)?;
    let pool_code = state.account_code(&pool)?;

    println!("\nSimulation summary:");
    println!("  success       : {}", result.success);
    println!("  gas used      : {}", result.gas_used);
    println!("  revert reason : {:?}", result.revert_reason);

    println!("\nCall trace (failures only):");
    print_failure_frames(&result.call_trace, 0);

    match pool_code {
        Some(code) => println!(
            "\nPool code length at block {}: {} bytes",
            BLOCK_BEFORE_LIQUIDITY,
            code.len()
        ),
        None => println!(
            "\nPool contract has no bytecode at block {} (pair not deployed yet)",
            BLOCK_BEFORE_LIQUIDITY
        ),
    }

    println!("\n✅ Example complete.");
    Ok(())
}

fn encode_swap_exact_eth_for_tokens(
    amount_out_min: U256,
    path: &[Address; 2],
    to: Address,
    deadline: U256,
) -> Bytes {
    let mut data = Vec::with_capacity(4 + 32 * 6);
    data.extend_from_slice(&[0x7f, 0xf3, 0x6a, 0xb5]);
    data.extend_from_slice(&amount_out_min.to_be_bytes::<32>());
    data.extend_from_slice(&[0u8; 28]);
    data.extend_from_slice(&[0, 0, 0, 0x80]);
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(to.as_slice());
    data.extend_from_slice(&deadline.to_be_bytes::<32>());
    data.extend_from_slice(&[0u8; 28]);
    data.extend_from_slice(&[0, 0, 0, 0x02]);
    for address in path {
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(address.as_slice());
    }
    Bytes::from(data)
}

fn print_failure_frames(frame: &CallFrame, depth: usize) {
    for call in &frame.calls {
        print_failure_frames(call, depth + 1);
    }
    if let Some(err) = &frame.error {
        let indent = "  ".repeat(depth);
        println!(
            "{indent}- type: {}, from: {:?}, to: {:?}, error: {err}",
            frame.typ, frame.from, frame.to
        );
    }
}
