use alloy_primitives::{address, U256};
use tx_simulator::tx_builders::baygus_router::{
    build_baygus_execute_tx, BaygusExecutePlan, CMD_TRANSFER_FROM, CMD_V2_SWAP,
};

fn main() {
    let caller = address!("0C96c602b1b332B8AB2093E5d72D804a24bd5689");
    let router = address!("1111111111111111111111111111111111111111");
    let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
    let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
    let amount_in = U256::from(1_000_000_000_000_000_000u128);

    let mut plan = BaygusExecutePlan::new();
    plan.transfer_from(weth, amount_in)
        .v2_swap(amount_in, U256::ZERO, vec![weth, usdc], caller);

    let tx = build_baygus_execute_tx(router, caller, &plan);
    let calldata_len = tx.data.as_ref().map(|data| data.len()).unwrap_or_default();

    assert_eq!(
        plan.commands_bytes().as_ref(),
        &[CMD_TRANSFER_FROM, CMD_V2_SWAP]
    );
    println!(
        "Baygus command bytes: 0x{}",
        hex::encode(plan.commands_bytes())
    );
    println!("Baygus execute calldata bytes: {calldata_len}");
}
