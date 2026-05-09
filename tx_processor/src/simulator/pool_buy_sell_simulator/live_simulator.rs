use std::sync::Arc;

use eyre::Result;
use tx_simulator::{LiveTxSimulator, TxSimulator, UnsignedTxChainSimulation};

use crate::simulator::types::{PoolBuySellParameters, PoolBuySellSimulationResult};
use crate::tx_processor::TxProcessor;

use super::entry::{check_can_buy_sell_pool, check_can_buy_sell_pool_with_chain};

#[derive(Clone)]
pub struct LivePoolBuySellSimulator {
    live_tx_simulator: LiveTxSimulator,
    tx_processor: Arc<TxProcessor>,
}

impl LivePoolBuySellSimulator {
    pub fn new(live_tx_simulator: LiveTxSimulator, tx_processor: Arc<TxProcessor>) -> Self {
        Self {
            live_tx_simulator,
            tx_processor,
        }
    }

    pub fn from_simulator(simulator: Arc<TxSimulator>) -> Self {
        Self::new(
            LiveTxSimulator::from_simulator(simulator),
            Arc::new(TxProcessor::new()),
        )
    }

    pub fn live_tx_simulator(&self) -> &LiveTxSimulator {
        &self.live_tx_simulator
    }

    pub fn simulator(&self) -> Arc<TxSimulator> {
        self.live_tx_simulator.simulator()
    }

    pub fn tx_processor(&self) -> Arc<TxProcessor> {
        Arc::clone(&self.tx_processor)
    }

    pub async fn check_pool(
        &self,
        mut config: PoolBuySellParameters,
    ) -> Result<PoolBuySellSimulationResult> {
        if config.block_number.is_none() {
            let status = self.live_tx_simulator.latest_state_status().await?;
            config.block_number = Some(status.selected_block_number);
        }
        check_can_buy_sell_pool(self.simulator(), self.tx_processor(), config).await
    }

    pub async fn check_pool_with_chain(
        &self,
        config: PoolBuySellParameters,
        chain: UnsignedTxChainSimulation,
    ) -> Result<PoolBuySellSimulationResult> {
        check_can_buy_sell_pool_with_chain(self.simulator(), self.tx_processor(), config, chain)
            .await
    }

    pub async fn check_pool_at_block(
        &self,
        config: PoolBuySellParameters,
        block_number: u64,
    ) -> Result<PoolBuySellSimulationResult> {
        self.check_pool(config.with_block(block_number)).await
    }

    pub async fn check_pool_before_live_block(
        &self,
        config: PoolBuySellParameters,
        block_number: u64,
    ) -> Result<PoolBuySellSimulationResult> {
        self.check_pool_at_block(config, block_number.saturating_sub(1))
            .await
    }
}
