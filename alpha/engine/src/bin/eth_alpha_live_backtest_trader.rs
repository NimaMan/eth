use eth_alpha_engine::live_trader::run_live_backtest;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    run_live_backtest().await
}
