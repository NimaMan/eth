use std::{collections::HashMap, fs, path::PathBuf};

use crate::{Result, TokenPnlStoreError};

pub const TOKEN_PNL_DATABASE_CONFIG_KEY: &str = "databases.token_pnl.url";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenPnlStoreConfig {
    pub database_url: String,
}

impl TokenPnlStoreConfig {
    pub fn from_shared_config() -> Result<Self> {
        Self::from_eth_config(&EthConfigFile::load_default()?)
    }

    pub fn from_shared_config_path(path: impl Into<PathBuf>) -> Result<Self> {
        Self::from_eth_config(&EthConfigFile::load(path.into())?)
    }

    pub fn from_eth_config(config: &EthConfigFile) -> Result<Self> {
        let database_url = config.required(TOKEN_PNL_DATABASE_CONFIG_KEY)?;
        Ok(Self { database_url })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EthConfigFile {
    path: PathBuf,
    values: HashMap<String, String>,
}

impl EthConfigFile {
    pub fn load_default() -> Result<Self> {
        Self::load(default_eth_config_path())
    }

    pub fn load(path: PathBuf) -> Result<Self> {
        let contents =
            fs::read_to_string(&path).map_err(|source| TokenPnlStoreError::ConfigRead {
                path: path.clone(),
                source,
            })?;
        Ok(Self {
            values: parse_toml_config(&path, &contents)?,
            path,
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn optional(&self, key: &str) -> Option<String> {
        self.values
            .get(key)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    }

    pub fn required(&self, key: &'static str) -> Result<String> {
        self.optional(key)
            .ok_or_else(|| TokenPnlStoreError::MissingConfigValue {
                key,
                path: self.path.clone(),
            })
    }
}

pub fn default_eth_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../config.toml")
}

fn parse_toml_config(path: &PathBuf, contents: &str) -> Result<HashMap<String, String>> {
    let mut values = HashMap::new();
    let root =
        contents
            .parse::<toml::Value>()
            .map_err(|source| TokenPnlStoreError::ConfigParse {
                path: path.clone(),
                source,
            })?;
    if let Some(url) = root
        .get("databases")
        .and_then(|value| value.get("token_pnl"))
        .and_then(|value| value.get("url"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        values.insert(TOKEN_PNL_DATABASE_CONFIG_KEY.to_string(), url.to_string());
    }
    Ok(values)
}
