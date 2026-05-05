use crate::{
    error::{EthTxExecutorError, Result},
    gas::{effective_priority_fee, TxGasProfile},
};
use ethers_core::types::U256;
use ethers_providers::{Http, Provider};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPositionEstimate {
    pub base_fee_per_gas: U256,
    pub effective_priority_fee: U256,
    pub competing_tx_count_before: u64,
    pub gas_before: U256,
    pub block_gas_limit: U256,
    pub likely_fits_in_next_block: bool,
    pub source: String,
}

#[derive(Clone)]
pub struct MempoolPositionEstimator {
    provider: Provider<Http>,
}

impl MempoolPositionEstimator {
    pub fn new(provider: Provider<Http>) -> Self {
        Self { provider }
    }

    pub async fn estimate(&self, tx: TxGasProfile) -> Result<BlockPositionEstimate> {
        let block = self.latest_block().await?;
        let base_fee =
            quantity_from_value(block.get("baseFeePerGas"), "latest block baseFeePerGas")
                .unwrap_or_else(|_| U256::zero());
        let block_gas_limit = quantity_from_value(block.get("gasLimit"), "latest block gasLimit")?;
        let our_tip =
            effective_priority_fee(tx.max_fee_per_gas, tx.max_priority_fee_per_gas, base_fee);

        let txpool = self.txpool_content().await?;
        let mut competing_tx_count_before = 0u64;
        let mut gas_before = U256::zero();

        if let Some(pending) = txpool.get("pending").and_then(Value::as_object) {
            for sender_txs in pending.values().filter_map(Value::as_object) {
                for candidate in sender_txs.values() {
                    let candidate_gas = quantity_from_value(candidate.get("gas"), "txpool tx gas")
                        .unwrap_or_else(|_| U256::zero());
                    let candidate_tip = candidate_effective_tip(candidate, base_fee);
                    if candidate_tip > our_tip {
                        competing_tx_count_before += 1;
                        gas_before += candidate_gas;
                    }
                }
            }
        }

        Ok(BlockPositionEstimate {
            base_fee_per_gas: base_fee,
            effective_priority_fee: our_tip,
            competing_tx_count_before,
            gas_before,
            block_gas_limit,
            likely_fits_in_next_block: gas_before + tx.gas_limit <= block_gas_limit,
            source: "txpool_content".to_string(),
        })
    }

    async fn latest_block(&self) -> Result<Value> {
        self.provider
            .request("eth_getBlockByNumber", serde_json::json!(["latest", false]))
            .await
            .map_err(|err| EthTxExecutorError::Rpc(format!("eth_getBlockByNumber failed: {err}")))
    }

    async fn txpool_content(&self) -> Result<Value> {
        self.provider
            .request("txpool_content", serde_json::json!([]))
            .await
            .map_err(|err| {
                EthTxExecutorError::PositionUnavailable(format!("txpool_content failed: {err}"))
            })
    }
}

fn candidate_effective_tip(candidate: &Value, base_fee: U256) -> U256 {
    let max_fee = quantity_from_value(candidate.get("maxFeePerGas"), "maxFeePerGas")
        .or_else(|_| quantity_from_value(candidate.get("gasPrice"), "gasPrice"))
        .unwrap_or_else(|_| U256::zero());
    let max_priority = quantity_from_value(
        candidate.get("maxPriorityFeePerGas"),
        "maxPriorityFeePerGas",
    )
    .or_else(|_| quantity_from_value(candidate.get("gasPrice"), "gasPrice"))
    .unwrap_or_else(|_| U256::zero());
    effective_priority_fee(max_fee, max_priority, base_fee)
}

fn quantity_from_value(value: Option<&Value>, label: &str) -> Result<U256> {
    let Some(value) = value else {
        return Err(EthTxExecutorError::Rpc(format!("{label} missing")));
    };
    let Some(text) = value.as_str() else {
        return Err(EthTxExecutorError::Rpc(format!("{label} was not a string")));
    };
    let hex = text.strip_prefix("0x").unwrap_or(text);
    U256::from_str_radix(hex, 16)
        .map_err(|err| EthTxExecutorError::Rpc(format!("invalid {label} quantity {text:?}: {err}")))
}
