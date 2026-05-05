/// Global configuration constants for the TxSimulator crate.
///
/// Centralizing these values keeps behaviour consistent across modules and avoids
/// scattering magic numbers throughout the codebase.
pub mod repo {
    use eyre::{eyre, Result};
    use std::{collections::HashMap, env, fs, path::PathBuf};

    pub const ETH_CONFIG_PATH_ENV: &str = "ETH_CONFIG_PATH";
    pub const ETH_NODE_ROOT_ENV: &str = "ETH_NODE_ROOT";
    pub const RETH_DATADIR_ENV: &str = "RETH_DATADIR";
    pub const RETH_DB_PATH_ENV: &str = "RETH_DB_PATH";
    pub const RETH_HTTP_RPC_ENV: &str = "RETH_HTTP_RPC";
    pub const RETH_WS_RPC_ENV: &str = "RETH_WS_RPC";
    pub const LIGHTHOUSE_DATADIR_ENV: &str = "LIGHTHOUSE_DATADIR";
    pub const JWT_PATH_ENV: &str = "JWT_PATH";
    pub const LIVE_BLOCKCHAIN_DATA_REDIS_URL_ENV: &str = "LIVE_BLOCKCHAIN_DATA_REDIS_URL";

    pub const DEFAULT_ETH_NODE_ROOT: &str = "/home/nima/storage/samsung8tb/ethereum";
    pub const DEFAULT_RETH_DATADIR: &str = "/home/nima/storage/samsung8tb/ethereum/reth";
    pub const DEFAULT_RETH_HTTP_RPC: &str = "http://127.0.0.1:8545";
    pub const DEFAULT_RETH_WS_RPC: &str = "ws://127.0.0.1:8546";
    pub const DEFAULT_LIGHTHOUSE_DATADIR: &str = "/home/nima/.lighthouse";
    pub const DEFAULT_JWT_PATH: &str = "/home/nima/storage/samsung8tb/ethereum/jwt/jwt.hex";
    pub const DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL: &str = "redis://localhost:6379/0";

    /// Path to the shared Ethereum workspace config.
    ///
    /// `ETH_CONFIG_PATH` can point to another file. Otherwise this resolves to
    /// `/home/nima/code/crypto/blockchains/eth/config.env` from this crate.
    pub fn config_path() -> PathBuf {
        env::var_os(ETH_CONFIG_PATH_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join("config.env")
            })
    }

    pub fn eth_node_root() -> Result<String> {
        resolve_value(&[ETH_NODE_ROOT_ENV], DEFAULT_ETH_NODE_ROOT)
    }

    pub fn reth_datadir() -> Result<String> {
        if let Ok(value) = env::var(RETH_DATADIR_ENV) {
            if !value.trim().is_empty() {
                return Ok(value);
            }
        }

        let config = load_config_env()?;
        if let Some(value) = config.get(RETH_DATADIR_ENV) {
            if !value.trim().is_empty() {
                return Ok(value.clone());
            }
        }

        if let Ok(value) = env::var(RETH_DB_PATH_ENV) {
            if !value.trim().is_empty() {
                return Ok(value);
            }
        }
        if let Some(value) = config.get(RETH_DB_PATH_ENV) {
            if !value.trim().is_empty() {
                return Ok(value.clone());
            }
        }

        Ok(DEFAULT_RETH_DATADIR.to_string())
    }

    pub fn reth_http_rpc() -> Result<String> {
        resolve_value(&[RETH_HTTP_RPC_ENV], DEFAULT_RETH_HTTP_RPC)
    }

    pub fn reth_ws_rpc() -> Result<String> {
        resolve_value(&[RETH_WS_RPC_ENV], DEFAULT_RETH_WS_RPC)
    }

    pub fn lighthouse_datadir() -> Result<String> {
        resolve_value(&[LIGHTHOUSE_DATADIR_ENV], DEFAULT_LIGHTHOUSE_DATADIR)
    }

    pub fn jwt_path() -> Result<String> {
        resolve_value(&[JWT_PATH_ENV], DEFAULT_JWT_PATH)
    }

    pub fn live_data_redis_url() -> Result<String> {
        resolve_value(
            &[LIVE_BLOCKCHAIN_DATA_REDIS_URL_ENV],
            DEFAULT_LIVE_BLOCKCHAIN_DATA_REDIS_URL,
        )
    }

    fn resolve_value(keys: &[&str], default: &str) -> Result<String> {
        for key in keys {
            if let Ok(value) = env::var(key) {
                if !value.trim().is_empty() {
                    return Ok(value);
                }
            }
        }

        let config = load_config_env()?;
        for key in keys {
            if let Some(value) = config.get(*key) {
                if !value.trim().is_empty() {
                    return Ok(value.clone());
                }
            }
        }

        Ok(default.to_string())
    }

    fn load_config_env() -> Result<HashMap<String, String>> {
        let path = config_path();
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(HashMap::new());
            }
            Err(err) => {
                return Err(eyre!("failed to read {}: {}", path.display(), err));
            }
        };

        parse_env_config(&contents)
    }

    fn parse_env_config(contents: &str) -> Result<HashMap<String, String>> {
        let mut values = HashMap::new();
        for (index, raw_line) in contents.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| eyre!("invalid config line {}: missing '='", index + 1))?;
            let key = key.trim();
            if key.is_empty() {
                return Err(eyre!("invalid config line {}: empty key", index + 1));
            }

            values.insert(key.to_string(), unquote(value.trim()).to_string());
        }

        Ok(values)
    }

    fn unquote(value: &str) -> &str {
        value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_config_env_values() {
            let values = parse_env_config(
                r#"
                # shared config
                RETH_DATADIR=/mnt/eth/reth
                RETH_HTTP_RPC=http://127.0.0.1:8545
                RETH_WS_RPC=ws://127.0.0.1:8546
                JWT_PATH="/mnt/eth/jwt.hex"
                LIVE_BLOCKCHAIN_DATA_REDIS_URL='redis://127.0.0.1:6379/0'
                "#,
            )
            .expect("config should parse");

            assert_eq!(values["RETH_DATADIR"], "/mnt/eth/reth");
            assert_eq!(values["RETH_HTTP_RPC"], "http://127.0.0.1:8545");
            assert_eq!(values["RETH_WS_RPC"], "ws://127.0.0.1:8546");
            assert_eq!(values["JWT_PATH"], "/mnt/eth/jwt.hex");
            assert_eq!(
                values["LIVE_BLOCKCHAIN_DATA_REDIS_URL"],
                "redis://127.0.0.1:6379/0"
            );
        }

        #[test]
        fn rejects_invalid_config_lines() {
            let err = parse_env_config("RETH_DATADIR").expect_err("line without '=' is invalid");
            assert!(err.to_string().contains("missing '='"));
        }
    }
}

pub mod view_call {
    /// Maximum number of attempts to fetch historical state when serving a live view call.
    pub const STATE_RETRY_MAX_ATTEMPTS: usize = 9;

    /// Delay between retry attempts in milliseconds while waiting for the state provider.
    pub const STATE_RETRY_DELAY_MS: u64 = 25;
}
