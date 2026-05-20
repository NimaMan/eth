#[path = "eth_alpha_backtest_trader/common.rs"]
mod common;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    common::run().await
}
