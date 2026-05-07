use crate::{SimulationResult, TxSimulator, UnsignedTransaction, UnsignedTxChainSimulation};
use eyre::Result;
use std::sync::Arc;

/// Live-first transaction simulator for latency-sensitive trading paths.
///
/// `TxSimulator` remains the general-purpose historical/direct-DB engine. This
/// wrapper makes live block-selection explicit: use the latest Redis
/// chain-state overlay when available, and fall back to the latest persisted
/// MDBX block otherwise.
#[derive(Clone)]
pub struct LiveTxSimulator {
    simulator: Arc<TxSimulator>,
}

impl LiveTxSimulator {
    pub fn new(reth_datadir: &str) -> Result<Self> {
        Ok(Self::from_simulator(Arc::new(TxSimulator::new(
            reth_datadir,
        )?)))
    }

    pub fn from_simulator(simulator: Arc<TxSimulator>) -> Self {
        Self { simulator }
    }

    pub fn simulator(&self) -> Arc<TxSimulator> {
        Arc::clone(&self.simulator)
    }

    /// Latest block for which we have an exact state source.
    ///
    /// Redis chain-state overlays are preferred because they represent the live
    /// blocks written by the live block processor. MDBX is used as the fallback
    /// when no overlay has been published yet.
    pub async fn latest_state_block_number(&self) -> Result<u64> {
        if let Some(cache) = self.simulator.live_chain_cache() {
            if let Some(block_number) = cache.latest_chain_state_block_number().await? {
                return Ok(block_number);
            }
        }

        self.simulator.get_latest_block()
    }

    /// Latest block announced in the live Redis cache, if any.
    pub async fn latest_live_block_number(&self) -> Result<Option<u64>> {
        self.simulator.live_latest_block_number().await
    }

    /// Latest block persisted in local MDBX.
    pub fn latest_persisted_block_number(&self) -> Result<u64> {
        self.simulator.get_latest_block()
    }

    /// Start a stateful simulation chain at the latest tracked state block.
    pub async fn start_latest_chain(&self) -> Result<UnsignedTxChainSimulation> {
        let block_number = self.latest_state_block_number().await?;
        self.simulator
            .start_simulation_chain(Some(block_number))
            .await
    }

    /// Start a stateful simulation chain at a specific block.
    pub async fn start_chain_at(&self, block_number: u64) -> Result<UnsignedTxChainSimulation> {
        self.simulator
            .start_simulation_chain(Some(block_number))
            .await
    }

    /// Simulate one unsigned transaction against the latest tracked state.
    pub async fn simulate_transaction(
        &self,
        transaction: UnsignedTransaction,
    ) -> Result<SimulationResult> {
        let mut chain = self.start_latest_chain().await?;
        chain.step(transaction).await
    }

    /// Simulate a sequence of unsigned transactions against the latest tracked state.
    pub async fn simulate_sequence(
        &self,
        transactions: Vec<UnsignedTransaction>,
    ) -> Result<Vec<SimulationResult>> {
        let mut chain = self.start_latest_chain().await?;
        let mut results = Vec::with_capacity(transactions.len());
        for transaction in transactions {
            results.push(chain.step(transaction).await?);
        }
        Ok(results)
    }
}
