use eth_alpha_engine::live_trader::{run, AlphaTraderEntrypoint};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    run(AlphaTraderEntrypoint::LiveBacktest).await
}
