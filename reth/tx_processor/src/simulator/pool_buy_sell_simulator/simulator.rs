use std::sync::Arc;

use eyre::Result;
use tx_simulator::TxSimulator;

use crate::simulator::types::{PoolBuySellParameters, PoolBuySellSimulationResult};
use crate::tx_processor::TxProcessor;

use super::entry::check_can_buy_sell_pool;

#[derive(Clone)]
pub struct PoolBuySellSimulator {
    simulator: Arc<TxSimulator>,
    tx_processor: Arc<TxProcessor>,
}

impl PoolBuySellSimulator {
    pub fn new(simulator: Arc<TxSimulator>, tx_processor: Arc<TxProcessor>) -> Self {
        Self {
            simulator,
            tx_processor,
        }
    }

    pub fn from_simulator(simulator: Arc<TxSimulator>) -> Self {
        Self::new(simulator, Arc::new(TxProcessor::new()))
    }

    pub fn simulator(&self) -> Arc<TxSimulator> {
        Arc::clone(&self.simulator)
    }

    pub fn tx_processor(&self) -> Arc<TxProcessor> {
        Arc::clone(&self.tx_processor)
    }

    pub async fn check_pool(
        &self,
        config: PoolBuySellParameters,
    ) -> Result<PoolBuySellSimulationResult> {
        check_can_buy_sell_pool(self.simulator(), self.tx_processor(), config).await
    }

    pub async fn check_pool_at_block(
        &self,
        config: PoolBuySellParameters,
        block_number: u64,
    ) -> Result<PoolBuySellSimulationResult> {
        self.check_pool(config.with_block(block_number)).await
    }

    pub async fn check_pool_before_block(
        &self,
        config: PoolBuySellParameters,
        block_number: u64,
    ) -> Result<PoolBuySellSimulationResult> {
        self.check_pool_at_block(config, block_number.saturating_sub(1))
            .await
    }
}
