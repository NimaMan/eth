#![recursion_limit = "512"]

use std::path::PathBuf;

use eth_chain_server::{app::logging, config, http, ChainServerConfig};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    if let Some(config_path) = parse_config_path()? {
        config::set_config_path(config_path)?;
    }
    let _log_guards = logging::init_logging()?;

    let config = ChainServerConfig::from_config_file()?;
    http::serve(config).await
}

fn parse_config_path() -> eyre::Result<Option<PathBuf>> {
    let mut config_path = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--config" {
            let Some(path) = args.next() else {
                eyre::bail!("--config requires a path");
            };
            config_path = Some(PathBuf::from(path));
        } else {
            eyre::bail!(
                "unknown argument {}; usage: eth_chain_server [--config <path>]",
                arg.to_string_lossy()
            );
        }
    }
    Ok(config_path)
}
