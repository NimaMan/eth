use std::fs;
use std::path::{Path, PathBuf};

use eyre::{eyre, Result};

pub const RISK_ATLAS_DATABASE_CONFIG_KEY: &str = "databases.risk_atlas.url";
pub const DEFAULT_RISK_ATLAS_DATABASE_URL: &str =
    "postgresql://postgres:postgres@localhost:5432/eth_db";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskAtlasConfig {
    pub database_url: String,
}

impl RiskAtlasConfig {
    pub fn from_shared_config() -> Result<Self> {
        Self::from_config_path(default_config_path())
    }

    pub fn from_config_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path).map_err(|error| {
            eyre!(
                "failed to read shared TOML config file {}: {error}",
                path.display()
            )
        })?;
        Self::from_toml_str(&contents, path)
    }

    fn from_toml_str(contents: &str, path: &Path) -> Result<Self> {
        let root = contents.parse::<toml::Value>().map_err(|error| {
            eyre!(
                "failed to parse shared TOML config file {}: {error}",
                path.display()
            )
        })?;
        let database_url = root
            .get("databases")
            .and_then(|value| value.get("risk_atlas"))
            .and_then(|value| value.get("url"))
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| eyre!("{RISK_ATLAS_DATABASE_CONFIG_KEY} must be set"))?;
        Ok(Self { database_url })
    }
}

impl Default for RiskAtlasConfig {
    fn default() -> Self {
        Self {
            database_url: DEFAULT_RISK_ATLAS_DATABASE_URL.to_string(),
        }
    }
}

fn default_config_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_risk_atlas_database_url() {
        let config = RiskAtlasConfig::from_toml_str(
            r#"
            [databases.risk_atlas]
            url = "postgresql://postgres:postgres@localhost:5432/eth_risk_atlas"
            "#,
            Path::new("test-config.toml"),
        )
        .unwrap();

        assert_eq!(
            config.database_url,
            "postgresql://postgres:postgres@localhost:5432/eth_risk_atlas"
        );
    }

    #[test]
    fn requires_risk_atlas_database_url() {
        let error = RiskAtlasConfig::from_toml_str(
            r#"
            [databases.alpha]
            url = "postgresql://postgres:postgres@localhost:5432/eth_db"
            "#,
            Path::new("test-config.toml"),
        )
        .unwrap_err();

        assert!(error.to_string().contains(RISK_ATLAS_DATABASE_CONFIG_KEY));
    }
}
