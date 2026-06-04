use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TokenPnlStoreError {
    #[error("failed to read shared config file {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse shared config file {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        source: toml::de::Error,
    },

    #[error("missing required config value {key} in {path}")]
    MissingConfigValue { key: &'static str, path: PathBuf },

    #[error("integer field {field}={value} does not fit into Postgres BIGINT")]
    IntegerOutOfRange { field: &'static str, value: u64 },

    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

pub type Result<T> = std::result::Result<T, TokenPnlStoreError>;

pub(crate) fn u64_to_i64(value: u64, field: &'static str) -> Result<i64> {
    i64::try_from(value).map_err(|_| TokenPnlStoreError::IntegerOutOfRange { field, value })
}
