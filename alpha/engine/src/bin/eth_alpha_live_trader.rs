use eth_alpha_engine::live_trader::run_live_real;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    run_live_real().await
}
