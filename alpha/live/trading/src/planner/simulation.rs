use alloy_primitives::{hex, keccak256, Address, Bytes, Log as AlloyLog, B256, U256};
use async_trait::async_trait;
use eth_alpha_core::{
    amount::{Amount, DecimalAmount},
    order::OrderSide,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tx_simulator::UnsignedTransaction;

use crate::{PreSubmitSimulation, PreparedSellRoute};

use super::{LivePrioritySellPlannerError, LivePrioritySellPlannerInput};

const BOUGHT_V2_SIGNATURE: &str = "BoughtV2(address,uint256,uint256,uint256)";
const EMERGENCY_SOLD_V2_SIGNATURE: &str = "EmergencySoldV2(address,uint256,uint256,uint256)";

#[async_trait]
pub trait PreSubmitSimulator: Send + Sync {
    async fn simulate(
        &self,
        input: &LivePrioritySellPlannerInput,
        route: &PreparedSellRoute,
    ) -> Result<PreSubmitSimulation, LivePrioritySellPlannerError>;
}

#[derive(Clone, Debug)]
pub struct FixedPreSubmitSimulator {
    simulation: PreSubmitSimulation,
}

impl FixedPreSubmitSimulator {
    pub fn new(simulation: PreSubmitSimulation) -> Self {
        Self { simulation }
    }
}

#[async_trait]
impl PreSubmitSimulator for FixedPreSubmitSimulator {
    async fn simulate(
        &self,
        _input: &LivePrioritySellPlannerInput,
        _route: &PreparedSellRoute,
    ) -> Result<PreSubmitSimulation, LivePrioritySellPlannerError> {
        Ok(self.simulation.clone())
    }
}

#[derive(Clone, Debug)]
pub struct ChainServerLivePreSubmitSimulator {
    http: reqwest::Client,
    base_url: String,
    vault_address: Address,
}

impl ChainServerLivePreSubmitSimulator {
    pub fn new(base_url: impl Into<String>, vault_address: Address) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.into(),
            vault_address,
        }
    }

    fn endpoint(&self) -> String {
        format!(
            "{}/api/v1/eth/live-tx-simulator/simulations/unsigned-transaction",
            self.base_url.trim_end_matches('/')
        )
    }
}

#[derive(Debug, Clone, Serialize)]
struct ChainServerLiveUnsignedTxSimulationRequest {
    block: u64,
    transaction: UnsignedTransaction,
}

#[derive(Debug, Clone, Deserialize)]
struct ChainServerLiveUnsignedTxSimulationResponse {
    schema: String,
    block: u64,
    block_hash: Option<B256>,
    state_source: String,
    success: bool,
    gas_used: u64,
    effective_gas_price_wei: Option<String>,
    tx_type: Option<u8>,
    revert_reason: Option<String>,
    log_count: usize,
    logs: Vec<AlloyLog>,
    base_fee_per_gas_wei: Option<String>,
}

#[async_trait]
impl PreSubmitSimulator for ChainServerLivePreSubmitSimulator {
    async fn simulate(
        &self,
        input: &LivePrioritySellPlannerInput,
        route: &PreparedSellRoute,
    ) -> Result<PreSubmitSimulation, LivePrioritySellPlannerError> {
        if route.protocol != "uniswap_v2_trading_vault" {
            return Err(LivePrioritySellPlannerError::Simulation(format!(
                "chain-server live simulator received unsupported route protocol {:?}",
                route.protocol
            )));
        }
        let from = input.context.tx.from.parse::<Address>().map_err(|error| {
            LivePrioritySellPlannerError::Simulation(format!(
                "invalid pre-submit simulation from address {:?}: {error}",
                input.context.tx.from
            ))
        })?;
        let to = route.router_address.parse::<Address>().map_err(|error| {
            LivePrioritySellPlannerError::Simulation(format!(
                "invalid pre-submit simulation route target {:?}: {error}",
                route.router_address
            ))
        })?;
        if to != self.vault_address {
            return Err(LivePrioritySellPlannerError::Simulation(format!(
                "chain-server live simulator target mismatch: route target {to} != configured vault {}",
                self.vault_address
            )));
        }

        let required_state_block = input.context.required_state_block(&input.pool);
        let request = ChainServerLiveUnsignedTxSimulationRequest {
            block: required_state_block,
            transaction: UnsignedTransaction {
                from: Some(from),
                to: Some(to),
                gas: Some(route.gas_limit),
                gas_price: None,
                max_fee_per_gas: None,
                max_priority_fee_per_gas: None,
                value: Some(parse_u256_quantity(&route.value_wei, "route value_wei")?),
                data: Some(decode_hex_bytes(&route.calldata, "route calldata")?),
                nonce: None,
                ..Default::default()
            },
        };
        let response = self
            .http
            .post(self.endpoint())
            .json(&request)
            .send()
            .await
            .map_err(|error| {
                LivePrioritySellPlannerError::Simulation(format!(
                    "chain-server live simulation request failed: {error}"
                ))
            })?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            LivePrioritySellPlannerError::Simulation(format!(
                "chain-server live simulation response body read failed: {error}"
            ))
        })?;
        if !status.is_success() {
            return Err(LivePrioritySellPlannerError::Simulation(format!(
                "chain-server live simulation returned HTTP {}: {}",
                status.as_u16(),
                body
            )));
        }
        let result = serde_json::from_str::<ChainServerLiveUnsignedTxSimulationResponse>(&body)
            .map_err(|error| {
                LivePrioritySellPlannerError::Simulation(format!(
                    "chain-server live simulation JSON decode failed: {error}; body={body}"
                ))
            })?;
        if result.block != required_state_block {
            return Err(LivePrioritySellPlannerError::Simulation(format!(
                "chain-server live simulation block mismatch: returned {} required {}",
                result.block, required_state_block
            )));
        }

        let base_metadata = json!({
            "provider": "chain_server_live_tx_simulator_uniswap_v2_trading_vault",
            "route_protocol": route.protocol,
            "vault_address": self.vault_address.to_string(),
            "from": from.to_string(),
            "to": to.to_string(),
            "observed_block": input.context.current_block,
            "required_state_block": required_state_block,
            "pool_creation_block": input.pool.creation_block,
            "pool_latest_block": input.pool.latest_block,
            "tx_observed_block": input.context.tx.observed_block,
            "simulation_block": result.block,
            "simulation_schema": result.schema,
            "state_source": result.state_source,
            "gas_limit": route.gas_limit,
            "gas_used": result.gas_used,
            "log_count": result.log_count,
            "effective_gas_price_wei": result.effective_gas_price_wei,
            "base_fee_per_gas_wei": result.base_fee_per_gas_wei,
            "tx_type": result.tx_type,
            "revert_reason": result.revert_reason,
        });

        if !result.success {
            return Ok(PreSubmitSimulation {
                block_number: result.block,
                block_hash: result.block_hash.map(|hash| hash.to_string()),
                state_root: None,
                expected_output_token: Some("ETH".to_string()),
                expected_output_amount: None,
                min_output_amount: input.min_output_amount.clone(),
                expected_recovery_eth: DecimalAmount::ZERO,
                gas_used: Some(result.gas_used),
                would_revert: true,
                metadata: base_metadata,
            });
        }

        match input.intent.side {
            OrderSide::Buy => {
                let fill = extract_bought_v2_fill_from_logs(
                    &result.logs,
                    self.vault_address,
                    input.intent.token_address,
                )?
                .ok_or_else(|| {
                    LivePrioritySellPlannerError::Simulation(format!(
                        "chain-server live simulation succeeded but emitted no matching BoughtV2 event for token {}",
                        input.intent.token_address
                    ))
                })?;
                Ok(PreSubmitSimulation {
                    block_number: result.block,
                    block_hash: result.block_hash.map(|hash| hash.to_string()),
                    state_root: None,
                    expected_output_token: Some(input.intent.token_address.to_string()),
                    expected_output_amount: Some(fill.tokens_received.to_string()),
                    min_output_amount: input.min_output_amount.clone(),
                    expected_recovery_eth: Amount {
                        raw: fill.eth_spent,
                        decimals: 18,
                    }
                    .to_decimal(),
                    gas_used: Some(result.gas_used),
                    would_revert: false,
                    metadata: json!({
                        "provider": "chain_server_live_tx_simulator_uniswap_v2_trading_vault",
                        "event": "BoughtV2",
                        "eth_spent_wei": fill.eth_spent.to_string(),
                        "tokens_received_raw": fill.tokens_received.to_string(),
                        "base": base_metadata,
                    }),
                })
            }
            OrderSide::Sell => {
                let fill = extract_emergency_sold_v2_fill_from_logs(
                    &result.logs,
                    self.vault_address,
                    input.intent.token_address,
                )?
                .ok_or_else(|| {
                    LivePrioritySellPlannerError::Simulation(format!(
                        "chain-server live simulation succeeded but emitted no matching EmergencySoldV2 event for token {}",
                        input.intent.token_address
                    ))
                })?;
                Ok(PreSubmitSimulation {
                    block_number: result.block,
                    block_hash: result.block_hash.map(|hash| hash.to_string()),
                    state_root: None,
                    expected_output_token: Some("ETH".to_string()),
                    expected_output_amount: Some(fill.eth_received.to_string()),
                    min_output_amount: input.min_output_amount.clone(),
                    expected_recovery_eth: Amount {
                        raw: fill.eth_received,
                        decimals: 18,
                    }
                    .to_decimal(),
                    gas_used: Some(result.gas_used),
                    would_revert: false,
                    metadata: json!({
                        "provider": "chain_server_live_tx_simulator_uniswap_v2_trading_vault",
                        "event": "EmergencySoldV2",
                        "amount_in_raw": fill.amount_in.to_string(),
                        "eth_received_wei": fill.eth_received.to_string(),
                        "base": base_metadata,
                    }),
                })
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BoughtV2Fill {
    eth_spent: U256,
    tokens_received: U256,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EmergencySoldV2Fill {
    amount_in: U256,
    eth_received: U256,
}

fn extract_bought_v2_fill_from_logs(
    logs: &[AlloyLog],
    vault_address: Address,
    token_address: Address,
) -> Result<Option<BoughtV2Fill>, LivePrioritySellPlannerError> {
    let event_topic = event_signature_topic(BOUGHT_V2_SIGNATURE);
    let expected_token_topic = indexed_address_topic(token_address);
    for log in logs {
        if log.address != vault_address {
            continue;
        }
        let topics = log.data.topics();
        if topics.first().copied() != Some(event_topic) {
            continue;
        }
        if topics.get(1).copied() != Some(expected_token_topic) {
            continue;
        }
        let words = decode_event_words(log.data.data.as_ref(), 3)?;
        return Ok(Some(BoughtV2Fill {
            eth_spent: words[0],
            tokens_received: words[1],
        }));
    }
    Ok(None)
}

fn extract_emergency_sold_v2_fill_from_logs(
    logs: &[AlloyLog],
    vault_address: Address,
    token_address: Address,
) -> Result<Option<EmergencySoldV2Fill>, LivePrioritySellPlannerError> {
    let event_topic = event_signature_topic(EMERGENCY_SOLD_V2_SIGNATURE);
    let expected_token_topic = indexed_address_topic(token_address);
    for log in logs {
        if log.address != vault_address {
            continue;
        }
        let topics = log.data.topics();
        if topics.first().copied() != Some(event_topic) {
            continue;
        }
        if topics.get(1).copied() != Some(expected_token_topic) {
            continue;
        }
        let words = decode_event_words(log.data.data.as_ref(), 3)?;
        return Ok(Some(EmergencySoldV2Fill {
            amount_in: words[0],
            eth_received: words[1],
        }));
    }
    Ok(None)
}

fn parse_u256_quantity(value: &str, label: &str) -> Result<U256, LivePrioritySellPlannerError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(LivePrioritySellPlannerError::Simulation(format!(
            "{label} is empty"
        )));
    }
    let parsed = if let Some(hex) = trimmed.strip_prefix("0x") {
        U256::from_str_radix(hex, 16)
    } else {
        U256::from_str_radix(trimmed, 10)
    };
    parsed.map_err(|error| {
        LivePrioritySellPlannerError::Simulation(format!(
            "invalid {label} quantity {trimmed:?}: {error}"
        ))
    })
}

fn decode_hex_bytes(value: &str, label: &str) -> Result<Bytes, LivePrioritySellPlannerError> {
    let hex_value = value.trim().strip_prefix("0x").ok_or_else(|| {
        LivePrioritySellPlannerError::Simulation(format!("{label} must be 0x-prefixed"))
    })?;
    let bytes = hex::decode(hex_value).map_err(|error| {
        LivePrioritySellPlannerError::Simulation(format!("invalid {label} hex: {error}"))
    })?;
    if bytes.is_empty() {
        return Err(LivePrioritySellPlannerError::Simulation(format!(
            "{label} is empty"
        )));
    }
    Ok(Bytes::from(bytes))
}

fn decode_event_words(
    data: &[u8],
    expected_words: usize,
) -> Result<Vec<U256>, LivePrioritySellPlannerError> {
    let expected_len = expected_words.checked_mul(32).ok_or_else(|| {
        LivePrioritySellPlannerError::Simulation("event word count overflow".to_string())
    })?;
    if data.len() < expected_len {
        return Err(LivePrioritySellPlannerError::Simulation(format!(
            "event data too short: got {} bytes, need {expected_len}",
            data.len()
        )));
    }
    Ok((0..expected_words)
        .map(|index| U256::from_be_slice(&data[index * 32..(index + 1) * 32]))
        .collect())
}

fn event_signature_topic(signature: &str) -> B256 {
    keccak256(signature.as_bytes())
}

fn indexed_address_topic(address: Address) -> B256 {
    let mut topic = [0u8; 32];
    topic[12..].copy_from_slice(address.as_slice());
    B256::from(topic)
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{Log, LogData};

    use super::*;

    #[test]
    fn extracts_eth_received_from_v2_vault_event() {
        let vault = Address::with_last_byte(0xaa);
        let token = Address::with_last_byte(0xbb);
        let amount_in = U256::from(10u64);
        let eth_received = U256::from(123_456u64);
        let min_out = U256::from(100_000u64);
        let mut data = Vec::new();
        data.extend_from_slice(&amount_in.to_be_bytes::<32>());
        data.extend_from_slice(&eth_received.to_be_bytes::<32>());
        data.extend_from_slice(&min_out.to_be_bytes::<32>());
        let logs = vec![Log {
            address: vault,
            data: LogData::new_unchecked(
                vec![
                    event_signature_topic(EMERGENCY_SOLD_V2_SIGNATURE),
                    indexed_address_topic(token),
                ],
                Bytes::from(data),
            ),
        }];

        let extracted = extract_emergency_sold_v2_fill_from_logs(&logs, vault, token).unwrap();

        assert_eq!(
            extracted,
            Some(EmergencySoldV2Fill {
                amount_in,
                eth_received
            })
        );
    }

    #[test]
    fn extracts_tokens_received_from_v2_vault_buy_event() {
        let vault = Address::with_last_byte(0xaa);
        let token = Address::with_last_byte(0xbb);
        let eth_spent = U256::from(10u64);
        let tokens_received = U256::from(123_456u64);
        let min_out = U256::from(100_000u64);
        let mut data = Vec::new();
        data.extend_from_slice(&eth_spent.to_be_bytes::<32>());
        data.extend_from_slice(&tokens_received.to_be_bytes::<32>());
        data.extend_from_slice(&min_out.to_be_bytes::<32>());
        let logs = vec![Log {
            address: vault,
            data: LogData::new_unchecked(
                vec![
                    event_signature_topic(BOUGHT_V2_SIGNATURE),
                    indexed_address_topic(token),
                ],
                Bytes::from(data),
            ),
        }];

        let extracted = extract_bought_v2_fill_from_logs(&logs, vault, token).unwrap();

        assert_eq!(
            extracted,
            Some(BoughtV2Fill {
                eth_spent,
                tokens_received
            })
        );
    }
}
