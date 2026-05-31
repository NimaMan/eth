use crate::{
    config::EthTxExecutorConfig,
    error::{EthTxExecutorError, Result},
    request::DirectRawTransactionRequest,
    types::PreparedDirectRawTransaction,
};
use ethers_core::types::{Address, U256};
use std::str::FromStr;

pub fn prepare_direct_raw_request(
    config: &EthTxExecutorConfig,
    request: DirectRawTransactionRequest,
    signer_address: Address,
) -> Result<PreparedDirectRawTransaction> {
    if request.chain_id != config.chain_id {
        return Err(EthTxExecutorError::Validation(format!(
            "request chain_id {} does not match executor chain_id {}",
            request.chain_id, config.chain_id
        )));
    }

    let from = parse_address(&request.from, "from")?;
    if from != signer_address {
        return Err(EthTxExecutorError::Validation(format!(
            "request from {from:?} does not match signer {signer_address:?}"
        )));
    }

    let to = parse_address(&request.to, "to")?;
    let value = parse_u256(&request.value, "value")?;
    let data = parse_hex_bytes(&request.data, "data")?;
    let gas_limit = parse_u256(&request.gas_limit, "gas_limit")?;
    let mut max_fee_per_gas = parse_u256(&request.max_fee_per_gas, "max_fee_per_gas")?;
    let mut max_priority_fee_per_gas = parse_u256(
        &request.max_priority_fee_per_gas,
        "max_priority_fee_per_gas",
    )?;
    let nonce = request
        .nonce
        .as_deref()
        .map(|value| parse_u256(value, "nonce"))
        .transpose()?;

    if gas_limit.is_zero() {
        return Err(EthTxExecutorError::Validation(
            "gas_limit must be greater than zero".to_string(),
        ));
    }
    if max_fee_per_gas.is_zero() {
        return Err(EthTxExecutorError::Validation(
            "max_fee_per_gas must be greater than zero".to_string(),
        ));
    }
    if let Some(bribe) = request.bribe.as_ref() {
        let requested_priority =
            parse_u256(&bribe.priority_fee_per_gas, "bribe.priority_fee_per_gas")?;
        max_priority_fee_per_gas = max_priority_fee_per_gas.max(requested_priority);
        if let Some(requested_max_fee) = bribe.max_fee_per_gas.as_deref() {
            max_fee_per_gas = parse_u256(requested_max_fee, "bribe.max_fee_per_gas")?;
        }
    }

    let max_priority_cap = U256::from(config.max_priority_fee_per_gas_wei);
    if max_priority_fee_per_gas > max_priority_cap {
        return Err(EthTxExecutorError::Validation(format!(
            "priority fee {} exceeds configured cap {}",
            max_priority_fee_per_gas, max_priority_cap
        )));
    }

    let max_fee_cap = U256::from(config.max_fee_per_gas_wei);
    if max_fee_per_gas > max_fee_cap {
        return Err(EthTxExecutorError::Validation(format!(
            "max fee {} exceeds configured cap {}",
            max_fee_per_gas, max_fee_cap
        )));
    }
    if max_fee_per_gas < max_priority_fee_per_gas {
        return Err(EthTxExecutorError::Validation(format!(
            "max_fee_per_gas {} is below max_priority_fee_per_gas {}",
            max_fee_per_gas, max_priority_fee_per_gas
        )));
    }

    let attempt_id = request
        .attempt_id
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    Ok(PreparedDirectRawTransaction {
        attempt_id,
        chain_id: request.chain_id,
        from,
        to,
        value,
        data,
        gas_limit,
        max_fee_per_gas,
        max_priority_fee_per_gas,
        nonce,
        simulation: request.simulation,
        metadata: request.metadata,
    })
}

pub fn parse_address(value: &str, label: &str) -> Result<Address> {
    Address::from_str(value.trim()).map_err(|err| {
        EthTxExecutorError::Validation(format!("invalid {label} address {value:?}: {err}"))
    })
}

pub fn parse_u256(value: &str, label: &str) -> Result<U256> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed.strip_prefix("0x") {
        U256::from_str_radix(hex, 16).map_err(|err| {
            EthTxExecutorError::Validation(format!("invalid {label} hex quantity {value:?}: {err}"))
        })
    } else {
        U256::from_dec_str(trimmed).map_err(|err| {
            EthTxExecutorError::Validation(format!(
                "invalid {label} decimal quantity {value:?}: {err}"
            ))
        })
    }
}

pub fn parse_hex_bytes(value: &str, label: &str) -> Result<Vec<u8>> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if hex.is_empty() {
        return Ok(Vec::new());
    }
    if hex.len() % 2 != 0 {
        return Err(EthTxExecutorError::Validation(format!(
            "{label} hex string must have even length"
        )));
    }
    hex::decode(hex)
        .map_err(|err| EthTxExecutorError::Validation(format!("invalid {label} hex bytes: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_or_decimal_u256() {
        assert_eq!(parse_u256("10", "value").unwrap(), U256::from(10));
        assert_eq!(parse_u256("0x10", "value").unwrap(), U256::from(16));
    }

    #[test]
    fn accepts_low_priority_fee_below_strategy_floor() {
        let signer_address =
            Address::from_str("0x0000000000000000000000000000000000000001").unwrap();
        let request = DirectRawTransactionRequest {
            attempt_id: Some("attempt-1".to_string()),
            chain_id: 1,
            from: format!("{signer_address:?}"),
            to: "0x0000000000000000000000000000000000000002".to_string(),
            value: "0".to_string(),
            data: "0x".to_string(),
            gas_limit: "21000".to_string(),
            max_fee_per_gas: "1000000000".to_string(),
            max_priority_fee_per_gas: "1".to_string(),
            nonce: None,
            bribe: None,
            simulation: None,
            metadata: serde_json::Value::Null,
        };
        let config = EthTxExecutorConfig::mainnet_local_reth("http://127.0.0.1:8545");

        let prepared = prepare_direct_raw_request(&config, request, signer_address).unwrap();

        assert_eq!(prepared.max_priority_fee_per_gas, U256::from(1));
    }
}
