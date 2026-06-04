use eth_alpha_live_runner::run_live_backtest;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    run_live_backtest().await
}
