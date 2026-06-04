use eyre::Result;

use super::{
    cli::{parse_live_backtest_args, parse_live_real_args},
    run,
    support::TraderExecutionMode,
};

pub async fn run_live_backtest() -> Result<()> {
    run(
        "eth_alpha_live_backtest_trader",
        parse_live_backtest_args(),
        TraderExecutionMode::ChainSim,
        None,
    )
    .await
}

pub async fn run_live_real() -> Result<()> {
    let (args, real_args) = parse_live_real_args();
    run(
        "eth_alpha_live_trader",
        args,
        TraderExecutionMode::EthTxExecutorReal,
        Some(real_args),
    )
    .await
}
