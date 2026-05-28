use alloy_primitives::B256;
use eyre::{eyre, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tx_processor::{PoolBuySellParameters, PoolBuySellSimulationResult, ProcessedTransaction};
use tx_simulator::{
    LiveStateSource, LiveStateStatus, SimulationResult as TxSimulationResult, UnsignedTransaction,
};

#[derive(Clone, Debug)]
pub struct ChainServerLiveTxSimulatorClient {
    base_url: String,
    client: Client,
}

impl ChainServerLiveTxSimulatorClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: Client::new(),
        }
    }

    pub async fn latest_state_status(&self) -> Result<LiveStateStatus> {
        let url = self.endpoint("api/v1/eth/live-tx-simulator/status");
        let response: LiveTxSimulatorStatusResponse = self.fetch_json(&url).await?;
        if !response.available {
            return Err(eyre!(
                "chain-server live tx simulator unavailable: {}",
                response
                    .unavailable_reason
                    .unwrap_or_else(|| "no reason returned".to_string())
            ));
        }
        Ok(LiveStateStatus {
            selected_block_number: response
                .selected_block_number
                .ok_or_else(|| eyre!("chain-server status missing selected_block_number"))?,
            selected_block_hash: response.selected_block_hash,
            source: LiveStateSource::InMemoryLiveBlockSession,
            latest_reth_finished_block_number: response
                .latest_reth_finished_block_number
                .unwrap_or_default(),
            latest_historical_context_block_number: response
                .latest_historical_context_block_number
                .unwrap_or_default(),
            latest_live_block_number: response.latest_live_block_number,
            latest_tracked_state_block_number: response.latest_tracked_state_block_number,
        })
    }

    pub async fn simulate_unsigned_transaction(
        &self,
        block: u64,
        transaction: UnsignedTransaction,
    ) -> Result<TxSimulationResult> {
        let url = self.endpoint("api/v1/eth/live-tx-simulator/simulations/unsigned-transaction");
        let request = LiveUnsignedTxSimulationRequest { block, transaction };
        let response: LiveUnsignedTxSimulationResponse = self.post_json(&url, &request).await?;
        Ok(TxSimulationResult {
            success: response.success,
            gas_used: response.gas_used,
            effective_gas_price: parse_optional_u128(
                response.effective_gas_price_wei.as_deref(),
                "effective_gas_price_wei",
            )?,
            tx_type: response.tx_type,
            revert_reason: response.revert_reason,
            revert_context: None,
        })
    }

    pub async fn process_unsigned_transaction(
        &self,
        block: u64,
        transaction: UnsignedTransaction,
    ) -> Result<ProcessedTransaction> {
        let mut processed = self
            .process_unsigned_transaction_sequence(block, vec![transaction], false)
            .await?;
        processed
            .pop()
            .ok_or_else(|| eyre!("chain-server returned empty processed transaction sequence"))
    }

    pub async fn process_unsigned_transaction_sequence(
        &self,
        block: u64,
        transactions: Vec<UnsignedTransaction>,
        stop_on_revert: bool,
    ) -> Result<Vec<ProcessedTransaction>> {
        let url =
            self.endpoint("api/v1/eth/live-tx-simulator/simulations/unsigned-transaction-sequence");
        let request = LiveUnsignedTxSequenceSimulationRequest {
            block,
            transactions,
            stop_on_revert,
        };
        let response: LiveUnsignedTxSequenceSimulationResponse =
            self.post_json(&url, &request).await?;
        if !response.state_available {
            return Err(eyre!(
                "chain-server live tx sequence simulation state unavailable at block {}: {}",
                response.block,
                response
                    .unavailable_reason
                    .unwrap_or_else(|| "no reason returned".to_string())
            ));
        }
        Ok(response.processed_transactions)
    }

    pub async fn simulate_pool_buy_sell(
        &self,
        block: u64,
        config: PoolBuySellParameters,
    ) -> Result<PoolBuySellSimulationResult> {
        let url = self.endpoint("api/v1/eth/live-tx-simulator/simulations/pool-buy-sell");
        let request = LivePoolBuySellSimulationRequest {
            block,
            config: LivePoolBuySellSimulationConfig::from(config),
        };
        let response: LivePoolBuySellSimulationResponse = self.post_json(&url, &request).await?;
        if !response.state_available {
            return Err(eyre!(
                "chain-server live pool buy/sell state unavailable at block {}: {}",
                response.block,
                response
                    .unavailable_reason
                    .unwrap_or_else(|| "no reason returned".to_string())
            ));
        }
        response
            .result
            .ok_or_else(|| eyre!("chain-server pool buy/sell response missing result"))
    }

    async fn fetch_json<T>(&self, url: &str) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let response = self.client.get(url).send().await?;
        decode_response(url, response).await
    }

    async fn post_json<T, R>(&self, url: &str, request: &R) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
        R: Serialize + ?Sized,
    {
        let response = self.client.post(url).json(request).send().await?;
        decode_response(url, response).await
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }
}

async fn decode_response<T>(url: &str, response: reqwest::Response) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(eyre!("{url} returned HTTP {status}: {body}"));
    }
    response
        .json::<T>()
        .await
        .map_err(|err| eyre!("failed to decode {url}: {err}"))
}

fn parse_optional_u128(value: Option<&str>, field: &str) -> Result<Option<u128>> {
    value
        .map(|value| {
            value
                .parse::<u128>()
                .map_err(|err| eyre!("invalid {field} value {value:?}: {err}"))
        })
        .transpose()
}

#[derive(Debug, Deserialize)]
struct LiveTxSimulatorStatusResponse {
    #[serde(default)]
    available: bool,
    #[serde(default)]
    unavailable_reason: Option<String>,
    #[serde(default)]
    selected_block_number: Option<u64>,
    #[serde(default)]
    selected_block_hash: Option<B256>,
    #[serde(default)]
    latest_reth_finished_block_number: Option<u64>,
    #[serde(default)]
    latest_historical_context_block_number: Option<u64>,
    #[serde(default)]
    latest_live_block_number: Option<u64>,
    #[serde(default)]
    latest_tracked_state_block_number: Option<u64>,
}

#[derive(Debug, Serialize)]
struct LiveUnsignedTxSimulationRequest {
    block: u64,
    transaction: UnsignedTransaction,
}

#[derive(Debug, Deserialize)]
struct LiveUnsignedTxSimulationResponse {
    success: bool,
    gas_used: u64,
    #[serde(default)]
    effective_gas_price_wei: Option<String>,
    #[serde(default)]
    tx_type: Option<u8>,
    #[serde(default)]
    revert_reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct LiveUnsignedTxSequenceSimulationRequest {
    block: u64,
    transactions: Vec<UnsignedTransaction>,
    stop_on_revert: bool,
}

#[derive(Debug, Deserialize)]
struct LiveUnsignedTxSequenceSimulationResponse {
    #[serde(default)]
    state_available: bool,
    #[serde(default)]
    unavailable_reason: Option<String>,
    block: u64,
    #[serde(default)]
    processed_transactions: Vec<ProcessedTransaction>,
}

#[derive(Debug, Serialize)]
struct LivePoolBuySellSimulationRequest {
    block: u64,
    config: LivePoolBuySellSimulationConfig,
}

#[derive(Debug, Serialize)]
struct LivePoolBuySellSimulationConfig {
    token_address: alloy_primitives::Address,
    pool_address: alloy_primitives::Address,
    pool_type: tx_processor::PoolType,
    test_amount: alloy_primitives::U256,
    buyer_address: alloy_primitives::Address,
    prior_txs: Vec<ProcessedTransaction>,
    slippage_tolerance: f64,
    gas_price: Option<u128>,
    max_fee_per_gas: Option<u128>,
    max_priority_fee_per_gas: Option<u128>,
    buy_gas_limit: u64,
    approve_gas_limit: u64,
    sell_gas_limit: u64,
    weth_address: alloy_primitives::Address,
    denom_address: alloy_primitives::Address,
    denom_decimals: u8,
    block_delay: u64,
    token_decimals: u8,
    uniswap_v4_config: Option<tx_processor::UniswapV4PoolConfig>,
}

impl From<PoolBuySellParameters> for LivePoolBuySellSimulationConfig {
    fn from(config: PoolBuySellParameters) -> Self {
        Self {
            token_address: config.token_address,
            pool_address: config.pool_address,
            pool_type: config.pool_type,
            test_amount: config.test_amount,
            buyer_address: config.buyer_address,
            prior_txs: config.prior_txs,
            slippage_tolerance: config.slippage_tolerance,
            gas_price: config.gas_price,
            max_fee_per_gas: config.max_fee_per_gas,
            max_priority_fee_per_gas: config.max_priority_fee_per_gas,
            buy_gas_limit: config.buy_gas_limit,
            approve_gas_limit: config.approve_gas_limit,
            sell_gas_limit: config.sell_gas_limit,
            weth_address: config.weth_address,
            denom_address: config.denom_address,
            denom_decimals: config.denom_decimals,
            block_delay: config.block_delay,
            token_decimals: config.token_decimals,
            uniswap_v4_config: config.uniswap_v4_config,
        }
    }
}

#[derive(Debug, Deserialize)]
struct LivePoolBuySellSimulationResponse {
    #[serde(default)]
    state_available: bool,
    #[serde(default)]
    unavailable_reason: Option<String>,
    block: u64,
    #[serde(default)]
    result: Option<PoolBuySellSimulationResult>,
}
