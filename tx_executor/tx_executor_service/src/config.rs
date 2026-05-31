use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use config::Config;
use tx_executor::{BroadcastMode, EthTxExecutorConfig};

use crate::ExecutionConfig;

#[derive(Debug, Clone)]
pub struct EthTxExecutorServiceConfig {
    pub api_token: Option<String>,
    pub http: EthTxHttpConfig,
    pub private_key_env: String,
    pub signer_backend: EthTxSignerBackendConfig,
    pub execution_disabled: bool,
    pub policy: EthTxPolicyConfig,
    pub executor: EthTxExecutorConfig,
}

#[derive(Debug, Clone)]
pub struct EthTxHttpConfig {
    pub bind_addr: SocketAddr,
    pub max_request_body_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EthTxSignerBackendConfig {
    Env,
    UnixSocket {
        socket_path: PathBuf,
        signer_address: String,
    },
}

#[derive(Debug, Clone)]
pub struct EthTxPolicyConfig {
    pub version: String,
    pub allowed_from_addresses: Vec<String>,
    pub allowed_targets: Vec<String>,
    pub allowed_selectors: Vec<String>,
    pub max_value_wei: u128,
    pub max_gas_limit: u64,
    pub max_fee_per_gas_wei: u128,
    pub max_priority_fee_per_gas_wei: u128,
    pub max_transaction_cost_wei: u128,
    pub max_daily_cost_wei: u128,
    pub require_simulation: bool,
    pub max_simulation_age_blocks: u64,
    pub required_metadata_fields: Vec<String>,
}

pub fn load_eth_tx_executor_service_config() -> Result<EthTxExecutorServiceConfig> {
    let settings = load_settings()?;
    let bind_addr = env_or_setting("ETH_TX_EXECUTOR_BIND", &settings, "eth_tx_executor.bind")
        .unwrap_or_else(|| "127.0.0.1:5006".to_string())
        .parse::<SocketAddr>()
        .context("invalid ETH_TX_EXECUTOR_BIND / eth_tx_executor.bind")?;
    let max_request_body_bytes = env_or_setting_u64(
        "ETH_TX_EXECUTOR_MAX_REQUEST_BODY_BYTES",
        &settings,
        "eth_tx_executor.max_request_body_bytes",
    )?
    .unwrap_or(1_048_576) as usize;

    let rpc_url = env_or_setting(
        "ETH_TX_EXECUTOR_RPC_URL",
        &settings,
        "eth_tx_executor.rpc_url",
    )
    .unwrap_or_else(|| "http://127.0.0.1:8545".to_string());

    let chain_id = env_or_setting_u64(
        "ETH_TX_EXECUTOR_CHAIN_ID",
        &settings,
        "eth_tx_executor.chain_id",
    )?
    .unwrap_or(1);

    let private_key_env = env_or_setting(
        "ETH_TX_EXECUTOR_PRIVATE_KEY_ENV",
        &settings,
        "eth_tx_executor.private_key_env",
    )
    .unwrap_or_else(|| "ETH_EXECUTOR_PRIVATE_KEY".to_string());

    let signer_backend = load_signer_backend_config(&settings)?;

    let api_token = eth_api_token(&settings);
    let execution_disabled = env_or_setting_bool(
        "ETH_TX_EXECUTOR_DISABLED",
        &settings,
        "eth_tx_executor.execution_disabled",
    )?
    .unwrap_or(false);

    let broadcast_mode = env_or_setting(
        "ETH_TX_EXECUTOR_BROADCAST_MODE",
        &settings,
        "eth_tx_executor.broadcast_mode",
    )
    .as_deref()
    .map(parse_broadcast_mode)
    .transpose()?
    .unwrap_or(BroadcastMode::DryRun);

    let max_priority_fee_per_gas_wei = env_or_setting_u128(
        "ETH_TX_EXECUTOR_MAX_PRIORITY_FEE_WEI",
        &settings,
        "eth_tx_executor.max_priority_fee_per_gas_wei",
    )?
    .unwrap_or(500_000_000_000);

    let max_fee_per_gas_wei = env_or_setting_u128(
        "ETH_TX_EXECUTOR_MAX_FEE_WEI",
        &settings,
        "eth_tx_executor.max_fee_per_gas_wei",
    )?
    .unwrap_or(1_000_000_000_000);

    let local_journal_path = env_or_setting(
        "ETH_TX_EXECUTOR_JOURNAL_PATH",
        &settings,
        "eth_tx_executor.local_journal_path",
    )
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
    .map(PathBuf::from);

    let policy = load_policy_config(&settings, max_fee_per_gas_wei, max_priority_fee_per_gas_wei)?;

    Ok(EthTxExecutorServiceConfig {
        api_token,
        http: EthTxHttpConfig {
            bind_addr,
            max_request_body_bytes,
        },
        private_key_env,
        signer_backend,
        execution_disabled,
        policy,
        executor: EthTxExecutorConfig {
            chain_id,
            rpc_url,
            broadcast_mode,
            max_priority_fee_per_gas_wei,
            max_fee_per_gas_wei,
            local_journal_path,
        },
    })
}

pub fn load_eth_tx_execution_config() -> Result<ExecutionConfig> {
    let settings = load_settings()?;
    let database_url = env_or_setting_any(
        &[
            (
                "ETH_TX_EXECUTOR_DATABASE_URL",
                "eth_tx_executor.database_url",
            ),
            ("TX_EXECUTOR_DATABASE_URL", "tx_executor.database_url"),
        ],
        &settings,
    )
    .unwrap_or_else(|| "postgres://postgres:postgres@127.0.0.1:5432/eth_db".to_string());
    Ok(ExecutionConfig::new(database_url))
}

fn load_signer_backend_config(settings: &Config) -> Result<EthTxSignerBackendConfig> {
    let backend = env_or_setting(
        "ETH_TX_EXECUTOR_SIGNER_BACKEND",
        settings,
        "eth_tx_executor.signer_backend",
    )
    .unwrap_or_else(|| "env".to_string());

    match backend.trim().to_ascii_lowercase().as_str() {
        "env" | "private_key_env" | "local" => Ok(EthTxSignerBackendConfig::Env),
        "unix_socket" | "unix-socket" | "socket" | "remote" => {
            let socket_path = env_or_setting(
                "ETH_TX_EXECUTOR_SIGNER_SOCKET_PATH",
                settings,
                "eth_tx_executor.signer_socket_path",
            )
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/run/eth-tx-executor/signer.sock"));
            let signer_address = env_or_setting(
                "ETH_TX_EXECUTOR_SIGNER_ADDRESS",
                settings,
                "eth_tx_executor.signer_address",
            )
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "ETH_TX_EXECUTOR_SIGNER_ADDRESS / eth_tx_executor.signer_address is required for unix_socket signer backend"
                )
            })?;
            Ok(EthTxSignerBackendConfig::UnixSocket {
                socket_path,
                signer_address,
            })
        }
        other => bail!(
            "invalid ETH_TX_EXECUTOR_SIGNER_BACKEND value {other:?}; expected env or unix_socket"
        ),
    }
}

fn load_policy_config(
    settings: &Config,
    default_max_fee_per_gas_wei: u128,
    default_max_priority_fee_per_gas_wei: u128,
) -> Result<EthTxPolicyConfig> {
    let version = env_or_setting("ETH_TX_POLICY_VERSION", settings, "eth_tx_policy.version")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "eth_tx_policy_v1".to_string());
    let allowed_from_addresses = env_or_setting_list(
        "ETH_TX_POLICY_ALLOWED_FROM_ADDRESSES",
        settings,
        "eth_tx_policy.allowed_from_addresses",
    );
    let allowed_targets = env_or_setting_list(
        "ETH_TX_POLICY_ALLOWED_TARGETS",
        settings,
        "eth_tx_policy.allowed_targets",
    );
    let allowed_selectors = env_or_setting_list(
        "ETH_TX_POLICY_ALLOWED_SELECTORS",
        settings,
        "eth_tx_policy.allowed_selectors",
    );
    let max_value_wei = env_or_setting_u128(
        "ETH_TX_POLICY_MAX_VALUE_WEI",
        settings,
        "eth_tx_policy.max_value_wei",
    )?
    .unwrap_or(0);
    let max_gas_limit = env_or_setting_u64(
        "ETH_TX_POLICY_MAX_GAS_LIMIT",
        settings,
        "eth_tx_policy.max_gas_limit",
    )?
    .unwrap_or(500_000);
    let max_fee_per_gas_wei = env_or_setting_u128(
        "ETH_TX_POLICY_MAX_FEE_WEI",
        settings,
        "eth_tx_policy.max_fee_per_gas_wei",
    )?
    .unwrap_or(default_max_fee_per_gas_wei);
    let max_priority_fee_per_gas_wei = env_or_setting_u128(
        "ETH_TX_POLICY_MAX_PRIORITY_FEE_WEI",
        settings,
        "eth_tx_policy.max_priority_fee_per_gas_wei",
    )?
    .unwrap_or(default_max_priority_fee_per_gas_wei);
    let max_transaction_cost_wei = env_or_setting_u128(
        "ETH_TX_POLICY_MAX_TRANSACTION_COST_WEI",
        settings,
        "eth_tx_policy.max_transaction_cost_wei",
    )?
    .unwrap_or(0);
    let max_daily_cost_wei = env_or_setting_u128(
        "ETH_TX_POLICY_MAX_DAILY_COST_WEI",
        settings,
        "eth_tx_policy.max_daily_cost_wei",
    )?
    .unwrap_or(0);
    let require_simulation = env_or_setting_bool(
        "ETH_TX_POLICY_REQUIRE_SIMULATION",
        settings,
        "eth_tx_policy.require_simulation",
    )?
    .unwrap_or(true);
    let max_simulation_age_blocks = env_or_setting_u64(
        "ETH_TX_POLICY_MAX_SIMULATION_AGE_BLOCKS",
        settings,
        "eth_tx_policy.max_simulation_age_blocks",
    )?
    .unwrap_or(2);
    let required_metadata_fields = env_or_setting_list(
        "ETH_TX_POLICY_REQUIRED_METADATA_FIELDS",
        settings,
        "eth_tx_policy.required_metadata_fields",
    );
    let required_metadata_fields = if required_metadata_fields.is_empty() {
        vec![
            "wire_protocol".to_string(),
            "intent_kind".to_string(),
            "strategy_name".to_string(),
            "strategy_run_id".to_string(),
            "trade_id".to_string(),
            "token_address".to_string(),
            "pool_address".to_string(),
        ]
    } else {
        required_metadata_fields
    };

    Ok(EthTxPolicyConfig {
        version,
        allowed_from_addresses,
        allowed_targets,
        allowed_selectors,
        max_value_wei,
        max_gas_limit,
        max_fee_per_gas_wei,
        max_priority_fee_per_gas_wei,
        max_transaction_cost_wei,
        max_daily_cost_wei,
        require_simulation,
        max_simulation_age_blocks,
        required_metadata_fields,
    })
}

fn load_settings() -> Result<Config> {
    let mut builder = Config::builder()
        .add_source(config::File::with_name("config").required(false))
        .add_source(config::File::with_name("../config").required(false))
        .add_source(config::File::with_name("../../config").required(false))
        .add_source(
            config::File::with_name("/home/nima/code/crypto/blockchains/eth/config")
                .required(false),
        );

    for key in ["ETH_TX_EXECUTOR_CONFIG_PATH", "TX_EXECUTOR_CONFIG_PATH"] {
        let Ok(path) = std::env::var(key) else {
            continue;
        };
        builder = builder.add_source(config::File::from(Path::new(&path)).required(false));
    }

    builder
        .build()
        .context("failed to load ETH tx executor config")
}

fn env_or_setting(env_key: &str, settings: &Config, setting_key: &str) -> Option<String> {
    std::env::var(env_key)
        .ok()
        .or_else(|| settings.get_string(setting_key).ok())
}

fn env_or_setting_any(keys: &[(&str, &str)], settings: &Config) -> Option<String> {
    keys.iter()
        .find_map(|(env_key, setting_key)| env_or_setting(env_key, settings, setting_key))
}

fn env_or_setting_list(env_key: &str, settings: &Config, setting_key: &str) -> Vec<String> {
    if let Ok(value) = std::env::var(env_key) {
        return parse_csv_list(&value);
    }
    if let Ok(values) = settings.get::<Vec<String>>(setting_key) {
        return normalize_string_list(values);
    }
    if let Ok(value) = settings.get_string(setting_key) {
        return parse_csv_list(&value);
    }

    Vec::new()
}

fn parse_csv_list(value: &str) -> Vec<String> {
    normalize_string_list(value.split(',').map(str::to_owned).collect())
}

fn normalize_string_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn env_or_setting_bool(
    env_key: &str,
    settings: &Config,
    setting_key: &str,
) -> Result<Option<bool>> {
    if let Some(value) = env_or_setting(env_key, settings, setting_key) {
        return parse_bool(&value)
            .map(Some)
            .with_context(|| format!("invalid boolean value for {env_key} / {setting_key}"));
    }

    Ok(None)
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => bail!("expected boolean, got {other:?}"),
    }
}

fn eth_api_token(settings: &Config) -> Option<String> {
    [
        std::env::var("ETH_TX_EXECUTOR_API_TOKEN").ok(),
        settings.get_string("eth_tx_executor.api_token").ok(),
        std::env::var("TX_EXECUTOR_API_TOKEN").ok(),
        settings.get_string("tx_executor.api_token").ok(),
    ]
    .into_iter()
    .flatten()
    .map(|value| value.trim().to_string())
    .find(|value| !value.is_empty())
}

fn env_or_setting_u64(env_key: &str, settings: &Config, setting_key: &str) -> Result<Option<u64>> {
    if let Ok(value) = std::env::var(env_key) {
        return value
            .trim()
            .parse::<u64>()
            .map(Some)
            .with_context(|| format!("invalid {env_key} value"));
    }

    if let Ok(value) = settings.get_string(setting_key) {
        return value
            .trim()
            .parse::<u64>()
            .map(Some)
            .with_context(|| format!("invalid {setting_key} value"));
    }

    match settings.get_int(setting_key) {
        Ok(value) if value >= 0 => Ok(Some(value as u64)),
        Ok(_) => bail!("{setting_key} must not be negative"),
        Err(_) => Ok(None),
    }
}

fn env_or_setting_u128(
    env_key: &str,
    settings: &Config,
    setting_key: &str,
) -> Result<Option<u128>> {
    if let Ok(value) = std::env::var(env_key) {
        return value
            .trim()
            .parse::<u128>()
            .map(Some)
            .with_context(|| format!("invalid {env_key} value"));
    }

    if let Ok(value) = settings.get_string(setting_key) {
        return value
            .trim()
            .parse::<u128>()
            .map(Some)
            .with_context(|| format!("invalid {setting_key} value"));
    }

    match settings.get_int(setting_key) {
        Ok(value) if value >= 0 => Ok(Some(value as u128)),
        Ok(_) => bail!("{setting_key} must not be negative"),
        Err(_) => Ok(None),
    }
}

pub fn parse_broadcast_mode(value: &str) -> Result<BroadcastMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "dry_run" | "dry-run" | "dryrun" => Ok(BroadcastMode::DryRun),
        "broadcast" => Ok(BroadcastMode::Broadcast),
        other => bail!(
            "invalid ETH_TX_EXECUTOR_BROADCAST_MODE value {other:?}; expected dry_run or broadcast"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        load_policy_config, load_signer_backend_config, parse_bool, parse_broadcast_mode,
        parse_csv_list, EthTxSignerBackendConfig,
    };
    use config::Config;
    use tx_executor::BroadcastMode;

    #[test]
    fn broadcast_mode_parser_accepts_safe_default_name() {
        assert_eq!(
            parse_broadcast_mode("dry_run").unwrap(),
            BroadcastMode::DryRun
        );
        assert_eq!(
            parse_broadcast_mode("dry-run").unwrap(),
            BroadcastMode::DryRun
        );
    }

    #[test]
    fn broadcast_mode_parser_accepts_broadcast_name() {
        assert_eq!(
            parse_broadcast_mode("broadcast").unwrap(),
            BroadcastMode::Broadcast
        );
    }

    #[test]
    fn broadcast_mode_parser_rejects_unknown_values() {
        assert!(parse_broadcast_mode("private_relay").is_err());
        assert!(parse_broadcast_mode("public_mempool").is_err());
    }

    #[test]
    fn bool_parser_accepts_explicit_values() {
        assert!(parse_bool("true").unwrap());
        assert!(!parse_bool("false").unwrap());
        assert!(parse_bool("maybe").is_err());
    }

    #[test]
    fn csv_parser_discards_empty_items() {
        assert_eq!(
            parse_csv_list(" 0xabc, ,0xdef "),
            vec!["0xabc".to_string(), "0xdef".to_string()]
        );
    }

    #[test]
    fn policy_config_has_secure_defaults() {
        let settings = Config::builder().build().unwrap();
        let policy = load_policy_config(&settings, 100, 10).unwrap();

        assert!(policy.allowed_from_addresses.is_empty());
        assert!(policy.allowed_targets.is_empty());
        assert!(policy.allowed_selectors.is_empty());
        assert_eq!(policy.max_value_wei, 0);
        assert_eq!(policy.max_transaction_cost_wei, 0);
        assert_eq!(policy.max_daily_cost_wei, 0);
        assert!(policy.require_simulation);
        assert!(policy
            .required_metadata_fields
            .contains(&"strategy_name".to_string()));
    }

    #[test]
    fn signer_backend_defaults_to_env() {
        let settings = Config::builder().build().unwrap();
        assert_eq!(
            load_signer_backend_config(&settings).unwrap(),
            EthTxSignerBackendConfig::Env
        );
    }

    #[test]
    fn unix_socket_signer_backend_requires_address() {
        let settings = Config::builder()
            .set_override("eth_tx_executor.signer_backend", "unix_socket")
            .unwrap()
            .build()
            .unwrap();
        assert!(load_signer_backend_config(&settings).is_err());
    }
}
