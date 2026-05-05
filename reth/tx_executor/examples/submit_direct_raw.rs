use serde_json::json;
use tx_executor::{BroadcastMode, DirectRawTransactionRequest, EthTxExecutor, EthTxExecutorConfig};

#[tokio::main]
async fn main() -> tx_executor::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();
    let mut config = EthTxExecutorConfig::mainnet_local_reth(args.rpc_url);
    config.broadcast_mode = if args.broadcast {
        BroadcastMode::PublicMempool
    } else {
        BroadcastMode::DryRun
    };
    config.local_journal_path = args.journal_path;

    let executor = EthTxExecutor::from_env(config.clone(), &args.private_key_env)?;
    let request = DirectRawTransactionRequest {
        attempt_id: args.attempt_id,
        chain_id: config.chain_id,
        from: args.from,
        to: args.to,
        value: args.value,
        data: args.data,
        gas_limit: args.gas_limit,
        max_fee_per_gas: args.max_fee_per_gas,
        max_priority_fee_per_gas: args.max_priority_fee_per_gas,
        nonce: args.nonce,
        bribe: None,
        simulation: None,
        metadata: json!({ "source": "submit_direct_raw_example" }),
    };

    let result = executor.submit_direct_raw(request).await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

struct Args {
    rpc_url: String,
    private_key_env: String,
    from: String,
    to: String,
    value: String,
    data: String,
    gas_limit: String,
    max_fee_per_gas: String,
    max_priority_fee_per_gas: String,
    nonce: Option<String>,
    attempt_id: Option<String>,
    journal_path: Option<std::path::PathBuf>,
    broadcast: bool,
}

impl Args {
    fn parse() -> Self {
        let mut args = std::env::args().skip(1);
        let mut parsed = Self {
            rpc_url: "http://127.0.0.1:8545".to_string(),
            private_key_env: "ETH_EXECUTOR_PRIVATE_KEY".to_string(),
            from: std::env::var("ETH_EXECUTOR_FROM").unwrap_or_default(),
            to: "0x0000000000000000000000000000000000000000".to_string(),
            value: "0".to_string(),
            data: "0x".to_string(),
            gas_limit: "21000".to_string(),
            max_fee_per_gas: "50000000000".to_string(),
            max_priority_fee_per_gas: "2000000000".to_string(),
            nonce: None,
            attempt_id: None,
            journal_path: None,
            broadcast: false,
        };

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--rpc-url" => parsed.rpc_url = require_value(&arg, &mut args),
                "--private-key-env" => parsed.private_key_env = require_value(&arg, &mut args),
                "--from" => parsed.from = require_value(&arg, &mut args),
                "--to" => parsed.to = require_value(&arg, &mut args),
                "--value" => parsed.value = require_value(&arg, &mut args),
                "--data" => parsed.data = require_value(&arg, &mut args),
                "--gas-limit" => parsed.gas_limit = require_value(&arg, &mut args),
                "--max-fee-per-gas" => parsed.max_fee_per_gas = require_value(&arg, &mut args),
                "--max-priority-fee-per-gas" => {
                    parsed.max_priority_fee_per_gas = require_value(&arg, &mut args)
                }
                "--nonce" => parsed.nonce = Some(require_value(&arg, &mut args)),
                "--attempt-id" => parsed.attempt_id = Some(require_value(&arg, &mut args)),
                "--journal" => parsed.journal_path = Some(require_value(&arg, &mut args).into()),
                "--broadcast" => parsed.broadcast = true,
                "--dry-run" => parsed.broadcast = false,
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                other => panic!("unknown argument {other}; use --help"),
            }
        }

        if parsed.from.trim().is_empty() {
            panic!("--from is required unless ETH_EXECUTOR_FROM is set");
        }
        parsed
    }
}

fn require_value(flag: &str, args: &mut impl Iterator<Item = String>) -> String {
    args.next()
        .unwrap_or_else(|| panic!("{flag} requires a value"))
}

fn print_help() {
    println!(
        "submit_direct_raw --from <addr> --to <addr> [--broadcast]\n\
         Defaults: --rpc-url http://127.0.0.1:8545 --gas-limit 21000 \
         --max-fee-per-gas 50000000000 --max-priority-fee-per-gas 2000000000"
    );
}
