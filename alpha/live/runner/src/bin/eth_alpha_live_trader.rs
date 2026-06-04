use eth_alpha_live_runner::run_live_real;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    run_live_real().await
}
