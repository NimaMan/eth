use std::collections::HashMap;

use eyre::{Result, WrapErr};

use super::support::required_shared_config_value;

pub(super) fn resolve_cli_or_config_i64(
    override_value: Option<i64>,
    config: &HashMap<String, String>,
    key: &str,
) -> Result<i64> {
    if let Some(value) = override_value {
        return Ok(value);
    }
    let value = required_shared_config_value(config, key)?;
    value
        .parse::<i64>()
        .wrap_err_with(|| format!("invalid {key} value {value:?}"))
}
