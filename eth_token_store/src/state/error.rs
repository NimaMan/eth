use std::{num::TryFromIntError, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum TokenStateStoreError {
    #[error("failed to read config {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse config {path}: {source}")]
    ConfigParse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("missing config value {key} in {path}")]
    MissingConfigValue { key: &'static str, path: PathBuf },
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("integer conversion failed for {field}: {source}")]
    IntegerConversion {
        field: &'static str,
        source: TryFromIntError,
    },
}

pub type Result<T> = std::result::Result<T, TokenStateStoreError>;

pub(crate) fn u64_to_i64(value: u64, field: &'static str) -> Result<i64> {
    i64::try_from(value).map_err(|source| TokenStateStoreError::IntegerConversion { field, source })
}

pub(crate) fn optional_u64_to_i64(value: Option<u64>, field: &'static str) -> Result<Option<i64>> {
    value.map(|value| u64_to_i64(value, field)).transpose()
}
