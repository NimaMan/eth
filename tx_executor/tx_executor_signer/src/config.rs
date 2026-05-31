use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use config::Config;

#[derive(Debug, Clone)]
pub struct EthSignerConfig {
    pub socket_path: PathBuf,
    pub socket_mode: u32,
    pub chain_id: u64,
    pub expected_signer_address: Option<String>,
    pub key_backend: EthSignerKeyBackend,
    pub policy: EthSignerPolicyConfig,
    pub journal_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum EthSignerKeyBackend {
    Env {
        private_key_env: String,
    },
    Keystore {
        path: PathBuf,
        password_file: Option<PathBuf>,
        password_env: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct EthSignerPolicyConfig {
    pub allowed_targets: Vec<String>,
    pub allowed_selectors: Vec<String>,
    pub max_value_wei: u128,
    pub max_gas_limit: u64,
    pub max_fee_per_gas_wei: u128,
    pub max_priority_fee_per_gas_wei: u128,
    pub max_transaction_cost_wei: u128,
}

pub fn load_eth_signer_config() -> Result<EthSignerConfig> {
    let settings = load_settings()?;

    let socket_path = env_or_setting(
        "ETH_TX_SIGNER_SOCKET_PATH",
        &settings,
        "eth_signer.socket_path",
    )
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("/run/eth-tx-executor/signer.sock"));

    let socket_mode = env_or_setting(
        "ETH_TX_SIGNER_SOCKET_MODE",
        &settings,
        "eth_signer.socket_mode",
    )
    .map(|value| parse_socket_mode(&value))
    .transpose()?
    .unwrap_or(0o600);

    let chain_id = env_or_setting_u64("ETH_TX_SIGNER_CHAIN_ID", &settings, "eth_signer.chain_id")?
        .unwrap_or(1);

    let expected_signer_address = env_or_setting(
        "ETH_TX_SIGNER_ADDRESS",
        &settings,
        "eth_signer.signer_address",
    )
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty());

    let key_backend = load_key_backend(&settings)?;
    let policy = load_policy_config(&settings)?;

    let journal_path = env_or_setting(
        "ETH_TX_SIGNER_JOURNAL_PATH",
        &settings,
        "eth_signer.journal_path",
    )
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
    .map(PathBuf::from);

    Ok(EthSignerConfig {
        socket_path,
        socket_mode,
        chain_id,
        expected_signer_address,
        key_backend,
        policy,
        journal_path,
    })
}

fn load_key_backend(settings: &Config) -> Result<EthSignerKeyBackend> {
    let backend = env_or_setting(
        "ETH_TX_SIGNER_KEY_BACKEND",
        settings,
        "eth_signer.key_backend",
    )
    .unwrap_or_else(|| "keystore".to_string());

    match backend.trim().to_ascii_lowercase().as_str() {
        "env" | "private_key_env" => {
            let private_key_env = env_or_setting(
                "ETH_TX_SIGNER_PRIVATE_KEY_ENV",
                settings,
                "eth_signer.private_key_env",
            )
            .unwrap_or_else(|| "ETH_EXECUTOR_PRIVATE_KEY".to_string());
            Ok(EthSignerKeyBackend::Env { private_key_env })
        }
        "keystore" | "encrypted_keystore" => {
            let path = env_or_setting(
                "ETH_TX_SIGNER_KEYSTORE_PATH",
                settings,
                "eth_signer.keystore_path",
            )
            .map(PathBuf::from)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "ETH_TX_SIGNER_KEYSTORE_PATH / eth_signer.keystore_path is required for keystore backend"
                )
            })?;
            let password_file = env_or_setting(
                "ETH_TX_SIGNER_PASSWORD_FILE",
                settings,
                "eth_signer.password_file",
            )
            .map(PathBuf::from);
            let password_env = env_or_setting(
                "ETH_TX_SIGNER_PASSWORD_ENV",
                settings,
                "eth_signer.password_env",
            )
            .or_else(|| Some("ETH_TX_SIGNER_KEYSTORE_PASSWORD".to_string()));
            if password_file.is_none() && password_env.is_none() {
                bail!("keystore backend requires password_file or password_env");
            }
            Ok(EthSignerKeyBackend::Keystore {
                path,
                password_file,
                password_env,
            })
        }
        other => {
            bail!("invalid ETH_TX_SIGNER_KEY_BACKEND value {other:?}; expected keystore or env")
        }
    }
}

fn load_policy_config(settings: &Config) -> Result<EthSignerPolicyConfig> {
    Ok(EthSignerPolicyConfig {
        allowed_targets: env_or_setting_list(
            "ETH_TX_SIGNER_ALLOWED_TARGETS",
            settings,
            "eth_signer.allowed_targets",
        ),
        allowed_selectors: env_or_setting_list(
            "ETH_TX_SIGNER_ALLOWED_SELECTORS",
            settings,
            "eth_signer.allowed_selectors",
        ),
        max_value_wei: env_or_setting_u128(
            "ETH_TX_SIGNER_MAX_VALUE_WEI",
            settings,
            "eth_signer.max_value_wei",
        )?
        .unwrap_or(0),
        max_gas_limit: env_or_setting_u64(
            "ETH_TX_SIGNER_MAX_GAS_LIMIT",
            settings,
            "eth_signer.max_gas_limit",
        )?
        .unwrap_or(500_000),
        max_fee_per_gas_wei: env_or_setting_u128(
            "ETH_TX_SIGNER_MAX_FEE_WEI",
            settings,
            "eth_signer.max_fee_per_gas_wei",
        )?
        .unwrap_or(1_000_000_000_000),
        max_priority_fee_per_gas_wei: env_or_setting_u128(
            "ETH_TX_SIGNER_MAX_PRIORITY_FEE_WEI",
            settings,
            "eth_signer.max_priority_fee_per_gas_wei",
        )?
        .unwrap_or(500_000_000_000),
        max_transaction_cost_wei: env_or_setting_u128(
            "ETH_TX_SIGNER_MAX_TRANSACTION_COST_WEI",
            settings,
            "eth_signer.max_transaction_cost_wei",
        )?
        .unwrap_or(0),
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

    for key in ["ETH_TX_SIGNER_CONFIG_PATH", "TX_EXECUTOR_CONFIG_PATH"] {
        let Ok(path) = std::env::var(key) else {
            continue;
        };
        builder = builder.add_source(config::File::from(Path::new(&path)).required(false));
    }

    builder
        .build()
        .context("failed to load ETH tx signer config")
}

fn env_or_setting(env_key: &str, settings: &Config, setting_key: &str) -> Option<String> {
    std::env::var(env_key)
        .ok()
        .or_else(|| settings.get_string(setting_key).ok())
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

fn normalize_string_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn parse_csv_list(value: &str) -> Vec<String> {
    normalize_string_list(value.split(',').map(str::to_owned).collect())
}

fn env_or_setting_u64(env_key: &str, settings: &Config, setting_key: &str) -> Result<Option<u64>> {
    if let Ok(value) = std::env::var(env_key) {
        return parse_u64_value(&value, env_key).map(Some);
    }
    if let Ok(value) = settings.get_string(setting_key) {
        return parse_u64_value(&value, setting_key).map(Some);
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
        return parse_u128_value(&value, env_key).map(Some);
    }
    if let Ok(value) = settings.get_string(setting_key) {
        return parse_u128_value(&value, setting_key).map(Some);
    }
    match settings.get_int(setting_key) {
        Ok(value) if value >= 0 => Ok(Some(value as u128)),
        Ok(_) => bail!("{setting_key} must not be negative"),
        Err(_) => Ok(None),
    }
}

fn parse_u64_value(value: &str, label: &str) -> Result<u64> {
    value
        .trim()
        .parse::<u64>()
        .with_context(|| format!("invalid {label} value"))
}

fn parse_u128_value(value: &str, label: &str) -> Result<u128> {
    value
        .trim()
        .parse::<u128>()
        .with_context(|| format!("invalid {label} value"))
}

fn parse_socket_mode(value: &str) -> Result<u32> {
    let trimmed = value.trim();
    let octal = trimmed.strip_prefix("0o").unwrap_or(trimmed);
    u32::from_str_radix(octal, 8).with_context(|| format!("invalid socket mode {value:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_socket_mode_as_octal() {
        assert_eq!(parse_socket_mode("0660").unwrap(), 0o660);
        assert_eq!(parse_socket_mode("0o600").unwrap(), 0o600);
    }

    #[test]
    fn default_key_backend_requires_keystore_path() {
        let settings = Config::builder().build().unwrap();
        assert!(load_key_backend(&settings).is_err());
    }
}
