use crate::{
    SessionTransaction, SignedTransaction, SimulationResult, SimulationSession,
    SimulationSessionOptions, TxSimulator, UnsignedTransaction, UnsignedTxChainSimulation,
};
use eyre::Result;
use std::sync::Arc;

/// Source selected for live-first state.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LiveStateSource {
    /// State comes from the latest block persisted in Reth MDBX.
    PersistedMdbx,
    /// State comes from a Redis chain-state overlay written by the live block processor.
    LiveOverlay,
}

/// Diagnostic state-selection result for live simulation.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct LiveStateStatus {
    pub selected_block_number: u64,
    pub source: LiveStateSource,
    pub latest_persisted_block_number: u64,
    pub latest_live_block_number: Option<u64>,
    pub latest_chain_state_block_number: Option<u64>,
}

impl LiveStateStatus {
    pub const fn mdbx_lags_selected_state(&self) -> bool {
        matches!(self.source, LiveStateSource::LiveOverlay)
    }
}

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
    /// Redis chain-state overlays are used when they are ahead of MDBX because
    /// they represent exact live block state written by the live block processor.
    /// MDBX is used when it is already caught up or when no overlay has been
    /// published yet.
    pub async fn latest_state_block_number(&self) -> Result<u64> {
        Ok(self.latest_state_status().await?.selected_block_number)
    }

    /// Full state-selection diagnostics for live simulation.
    pub async fn latest_state_status(&self) -> Result<LiveStateStatus> {
        let latest_persisted = self.latest_persisted_block_number()?;
        let Some(cache) = self.simulator.live_chain_cache() else {
            return Ok(select_state_status(latest_persisted, None, None));
        };

        let latest_live = cache.latest_block_number().await?;
        let latest_chain_state = cache.latest_chain_state_block_number().await?;
        Ok(select_state_status(
            latest_persisted,
            latest_live,
            latest_chain_state,
        ))
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
        let block_number = self.latest_state_status().await?.selected_block_number;
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

    /// Start a mixed signed/unsigned session at the latest exact live-first state.
    pub async fn start_latest_session(&self) -> Result<SimulationSession> {
        let block_number = self.latest_state_status().await?.selected_block_number;
        self.start_session_at(block_number).await
    }

    /// Start a mixed signed/unsigned session at a specific block.
    pub async fn start_session_at(&self, block_number: u64) -> Result<SimulationSession> {
        self.simulator
            .simulation_session_with_options(SimulationSessionOptions {
                at_block: Some(block_number),
                gas_block_number: None,
            })
            .await
    }

    /// Simulate one unsigned transaction against the latest tracked state.
    pub async fn simulate_transaction(
        &self,
        transaction: UnsignedTransaction,
    ) -> Result<SimulationResult> {
        let mut session = self.start_latest_session().await?;
        session.step_unsigned(transaction)
    }

    /// Simulate one signed transaction against the latest tracked state.
    pub async fn simulate_signed_transaction(
        &self,
        transaction: &SignedTransaction,
    ) -> Result<SimulationResult> {
        let mut session = self.start_latest_session().await?;
        session.step_signed(transaction)
    }

    /// Simulate a sequence of unsigned transactions against the latest tracked state.
    pub async fn simulate_sequence(
        &self,
        transactions: Vec<UnsignedTransaction>,
    ) -> Result<Vec<SimulationResult>> {
        let mut session = self.start_latest_session().await?;
        let mut results = Vec::with_capacity(transactions.len());
        for transaction in transactions {
            results.push(session.step_unsigned(transaction)?);
        }
        Ok(results)
    }

    /// Simulate a mixed signed/unsigned sequence against the latest tracked state.
    pub async fn simulate_mixed_sequence(
        &self,
        transactions: Vec<SessionTransaction>,
    ) -> Result<Vec<SimulationResult>> {
        let mut session = self.start_latest_session().await?;
        let mut results = Vec::with_capacity(transactions.len());
        for transaction in transactions {
            results.push(session.step(transaction)?);
        }
        Ok(results)
    }
}

fn select_state_status(
    latest_persisted: u64,
    latest_live: Option<u64>,
    latest_chain_state: Option<u64>,
) -> LiveStateStatus {
    if let Some(chain_state) = latest_chain_state.filter(|block| *block > latest_persisted) {
        return LiveStateStatus {
            selected_block_number: chain_state,
            source: LiveStateSource::LiveOverlay,
            latest_persisted_block_number: latest_persisted,
            latest_live_block_number: latest_live,
            latest_chain_state_block_number: latest_chain_state,
        };
    }

    LiveStateStatus {
        selected_block_number: latest_persisted,
        source: LiveStateSource::PersistedMdbx,
        latest_persisted_block_number: latest_persisted,
        latest_live_block_number: latest_live,
        latest_chain_state_block_number: latest_chain_state,
    }
}

#[cfg(test)]
mod tests {
    use super::{select_state_status, LiveStateSource};

    #[test]
    fn uses_persisted_when_no_live_state_exists() {
        let status = select_state_status(100, None, None);
        assert_eq!(status.selected_block_number, 100);
        assert_eq!(status.source, LiveStateSource::PersistedMdbx);
        assert!(!status.mdbx_lags_selected_state());
    }

    #[test]
    fn uses_persisted_when_mdbx_is_caught_up() {
        let status = select_state_status(105, Some(105), Some(105));
        assert_eq!(status.selected_block_number, 105);
        assert_eq!(status.source, LiveStateSource::PersistedMdbx);
    }

    #[test]
    fn uses_live_overlay_when_snapshot_is_ahead_of_mdbx() {
        let status = select_state_status(100, Some(102), Some(102));
        assert_eq!(status.selected_block_number, 102);
        assert_eq!(status.source, LiveStateSource::LiveOverlay);
        assert!(status.mdbx_lags_selected_state());
    }

    #[test]
    fn does_not_use_live_head_without_exact_state_snapshot() {
        let status = select_state_status(100, Some(102), None);
        assert_eq!(status.selected_block_number, 100);
        assert_eq!(status.source, LiveStateSource::PersistedMdbx);
    }
}
