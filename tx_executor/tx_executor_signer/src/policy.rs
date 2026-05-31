use ethers_core::types::{Address, U256};
use tx_executor::types::PreparedDirectRawTransaction;

use super::config::EthSignerPolicyConfig;

pub fn evaluate_signer_policy(
    config: &EthSignerPolicyConfig,
    signer: Address,
    transaction: &PreparedDirectRawTransaction,
) -> Result<(), Vec<String>> {
    let mut reasons = Vec::new();

    if transaction.from != signer {
        reasons.push(format!(
            "transaction from {:?} does not match signer {:?}",
            transaction.from, signer
        ));
    }

    if config.allowed_targets.is_empty() {
        reasons.push("signer target allowlist is empty".to_string());
    } else if !config.allowed_targets.iter().any(|target| {
        normalize_address(target) == normalize_address(&format!("{:?}", transaction.to))
    }) {
        reasons.push(format!(
            "target {:?} is not signer-allowlisted",
            transaction.to
        ));
    }

    let selector = calldata_selector(&transaction.data);
    if config.allowed_selectors.is_empty() {
        reasons.push("signer selector allowlist is empty".to_string());
    } else if let Some(selector) = selector.as_ref() {
        if !config
            .allowed_selectors
            .iter()
            .any(|allowed| normalize_selector(allowed) == *selector)
        {
            reasons.push(format!("selector {selector} is not signer-allowlisted"));
        }
    } else {
        reasons.push("calldata is shorter than 4-byte selector".to_string());
    }

    if transaction.value > U256::from(config.max_value_wei) {
        reasons.push(format!(
            "value {} exceeds signer cap {}",
            transaction.value, config.max_value_wei
        ));
    }

    if transaction.gas_limit > U256::from(config.max_gas_limit) {
        reasons.push(format!(
            "gas_limit {} exceeds signer cap {}",
            transaction.gas_limit, config.max_gas_limit
        ));
    }

    if transaction.max_fee_per_gas > U256::from(config.max_fee_per_gas_wei) {
        reasons.push(format!(
            "max_fee_per_gas {} exceeds signer cap {}",
            transaction.max_fee_per_gas, config.max_fee_per_gas_wei
        ));
    }

    if transaction.max_priority_fee_per_gas > U256::from(config.max_priority_fee_per_gas_wei) {
        reasons.push(format!(
            "max_priority_fee_per_gas {} exceeds signer cap {}",
            transaction.max_priority_fee_per_gas, config.max_priority_fee_per_gas_wei
        ));
    }

    if config.max_transaction_cost_wei == 0 {
        reasons.push("signer max transaction cost cap is zero".to_string());
    } else {
        let worst_case_cost =
            transaction.value + transaction.gas_limit * transaction.max_fee_per_gas;
        if worst_case_cost > U256::from(config.max_transaction_cost_wei) {
            reasons.push(format!(
                "worst-case cost {} exceeds signer cap {}",
                worst_case_cost, config.max_transaction_cost_wei
            ));
        }
    }

    if reasons.is_empty() {
        Ok(())
    } else {
        Err(reasons)
    }
}

pub fn calldata_selector(data: &[u8]) -> Option<String> {
    if data.len() < 4 {
        return None;
    }
    Some(format!("0x{}", hex::encode(&data[..4])).to_ascii_lowercase())
}

fn normalize_address(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_selector(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tx() -> PreparedDirectRawTransaction {
        PreparedDirectRawTransaction {
            attempt_id: "attempt-1".to_string(),
            chain_id: 1,
            from: "0x0000000000000000000000000000000000000001"
                .parse()
                .unwrap(),
            to: "0x0000000000000000000000000000000000000002"
                .parse()
                .unwrap(),
            value: U256::zero(),
            data: hex::decode("5f413d1000000000000000000000000000000000000000000000000000000000")
                .unwrap(),
            gas_limit: U256::from(100_000),
            max_fee_per_gas: U256::from(100),
            max_priority_fee_per_gas: U256::from(10),
            nonce: Some(U256::from(1)),
            simulation: None,
            metadata: json!({}),
        }
    }

    #[test]
    fn signer_policy_accepts_narrow_allowlist() {
        let config = EthSignerPolicyConfig {
            allowed_targets: vec!["0x0000000000000000000000000000000000000002".to_string()],
            allowed_selectors: vec!["0x5f413d10".to_string()],
            max_value_wei: 0,
            max_gas_limit: 200_000,
            max_fee_per_gas_wei: 200,
            max_priority_fee_per_gas_wei: 20,
            max_transaction_cost_wei: 20_000_000,
        };
        let signer = "0x0000000000000000000000000000000000000001"
            .parse()
            .unwrap();
        assert!(evaluate_signer_policy(&config, signer, &tx()).is_ok());
    }

    #[test]
    fn signer_policy_rejects_empty_allowlists() {
        let config = EthSignerPolicyConfig {
            allowed_targets: vec![],
            allowed_selectors: vec![],
            max_value_wei: 0,
            max_gas_limit: 200_000,
            max_fee_per_gas_wei: 200,
            max_priority_fee_per_gas_wei: 20,
            max_transaction_cost_wei: 20_000_000,
        };
        let signer = "0x0000000000000000000000000000000000000001"
            .parse()
            .unwrap();
        let reasons = evaluate_signer_policy(&config, signer, &tx()).unwrap_err();
        assert!(reasons
            .iter()
            .any(|reason| reason.contains("target allowlist")));
        assert!(reasons
            .iter()
            .any(|reason| reason.contains("selector allowlist")));
    }
}
