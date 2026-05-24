use crate::{
    SessionTransaction, SignedTransaction, SimulationResult, SimulationSession,
    SimulationSessionOptions, TxSimulator, UnsignedTransaction, UnsignedTxChainSimulation,
};
use eyre::{eyre, Result};
use std::sync::Arc;

/// Source selected for live-first state.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LiveStateSource {
    /// State comes from local Reth providers.
    LocalHistoricalContext,
    /// State comes from a tracked live-state provider.
    TrackedLiveState,
}

/// Diagnostic state-selection result for live simulation.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct LiveStateStatus {
    pub selected_block_number: u64,
    pub source: LiveStateSource,
    pub latest_reth_finished_block_number: u64,
    pub latest_historical_context_block_number: u64,
    pub latest_live_block_number: Option<u64>,
    pub latest_tracked_state_block_number: Option<u64>,
}

impl LiveStateStatus {
    pub const fn uses_tracked_live_state(&self) -> bool {
        matches!(self.source, LiveStateSource::TrackedLiveState)
    }

    pub const fn local_context_lags_selected_state(&self) -> bool {
        self.uses_tracked_live_state()
    }

    pub const fn is_ready_for_block(&self, required_block_number: u64) -> bool {
        self.selected_block_number >= required_block_number
    }
}

/// Live-first transaction simulator for latency-sensitive trading paths.
///
/// `TxSimulator` remains the general-purpose historical/direct-DB engine. This
/// wrapper selects the latest locally readable historical context. In-process
/// live block runtimes that already have exact prestate diffs should use
/// `TxSimulator::block_state_session_from_prestate_diffs` directly.
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

    /// Latest block for which this wrapper can open a simulation session.
    pub async fn latest_state_block_number(&self) -> Result<u64> {
        Ok(self.latest_state_status().await?.selected_block_number)
    }

    /// Full state-selection diagnostics for live simulation.
    pub async fn latest_state_status(&self) -> Result<LiveStateStatus> {
        self.latest_state_status_blocking()
    }

    /// Full state-selection diagnostics, requiring state at or after the
    /// caller's exact signal dependency block.
    pub async fn latest_state_status_at_or_after(
        &self,
        required_block_number: u64,
    ) -> Result<LiveStateStatus> {
        let status = self.latest_state_status().await?;
        if !status.is_ready_for_block(required_block_number) {
            return Err(eyre!(
                "live simulation state is not ready: selected_block={} required_block={} latest_reth_finished_block={} latest_historical_context_block={} source={:?}",
                status.selected_block_number,
                required_block_number,
                status.latest_reth_finished_block_number,
                status.latest_historical_context_block_number,
                status.source
            ));
        }
        Ok(status)
    }

    /// Blocking variant for callers that already run this work outside an async
    /// hot path.
    pub fn latest_state_status_blocking(&self) -> Result<LiveStateStatus> {
        let latest_reth_finished = self.latest_reth_finished_block_number()?;
        let latest_historical_context = self.latest_historical_context_block_number()?;
        select_state_status(latest_reth_finished, latest_historical_context, None, None)
    }

    /// Latest block announced by an external live-state source.
    pub async fn latest_live_block_number(&self) -> Result<Option<u64>> {
        Ok(None)
    }

    /// Latest block reported by Reth's Finish stage.
    pub fn latest_reth_finished_block_number(&self) -> Result<u64> {
        self.simulator.get_latest_block()
    }

    /// Compatibility alias for callers that still use the old name.
    ///
    /// This is raw Reth Finish-stage progress, not necessarily the latest block
    /// whose header/state context is readable from local Reth.
    pub fn latest_persisted_block_number(&self) -> Result<u64> {
        self.latest_reth_finished_block_number()
    }

    /// Latest block that can be simulated entirely from local historical Reth context.
    pub fn latest_historical_context_block_number(&self) -> Result<u64> {
        self.simulator.latest_historical_context_block_number()
    }

    /// Start a stateful simulation chain at the latest selected state block.
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

    /// Start a mixed signed/unsigned session at the latest live-first state.
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

    /// Simulate one unsigned transaction against the latest selected state.
    pub async fn simulate_transaction(
        &self,
        transaction: UnsignedTransaction,
    ) -> Result<SimulationResult> {
        let mut session = self.start_latest_session().await?;
        session.step_unsigned(transaction)
    }

    /// Simulate one signed transaction against the latest selected state.
    pub async fn simulate_signed_transaction(
        &self,
        transaction: &SignedTransaction,
    ) -> Result<SimulationResult> {
        let mut session = self.start_latest_session().await?;
        session.step_signed(transaction)
    }

    /// Simulate a sequence of unsigned transactions against the latest selected state.
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

    /// Simulate a mixed signed/unsigned sequence against the latest selected state.
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
    latest_reth_finished: u64,
    latest_historical_context: u64,
    latest_live: Option<u64>,
    latest_tracked_state: Option<u64>,
) -> Result<LiveStateStatus> {
    let Some(live_head) = latest_live else {
        return Ok(local_historical_context_state_status(
            latest_reth_finished,
            latest_historical_context,
            latest_live,
            latest_tracked_state,
        ));
    };

    if latest_historical_context >= live_head {
        return Ok(local_historical_context_state_status(
            latest_reth_finished,
            latest_historical_context,
            latest_live,
            latest_tracked_state,
        ));
    }

    if let Some(tracked_state) = latest_tracked_state.filter(|block| *block >= live_head) {
        return Ok(LiveStateStatus {
            selected_block_number: tracked_state,
            source: LiveStateSource::TrackedLiveState,
            latest_reth_finished_block_number: latest_reth_finished,
            latest_historical_context_block_number: latest_historical_context,
            latest_live_block_number: latest_live,
            latest_tracked_state_block_number: latest_tracked_state,
        });
    }

    Err(eyre!(
        "live tracked state is behind live head: latest_live={live_head}, latest_historical_context={latest_historical_context}, latest_reth_finished={latest_reth_finished}, latest_tracked_state={latest_tracked_state:?}"
    ))
}

fn local_historical_context_state_status(
    latest_reth_finished: u64,
    latest_historical_context: u64,
    latest_live: Option<u64>,
    latest_tracked_state: Option<u64>,
) -> LiveStateStatus {
    LiveStateStatus {
        selected_block_number: latest_historical_context,
        source: LiveStateSource::LocalHistoricalContext,
        latest_reth_finished_block_number: latest_reth_finished,
        latest_historical_context_block_number: latest_historical_context,
        latest_live_block_number: latest_live,
        latest_tracked_state_block_number: latest_tracked_state,
    }
}

#[cfg(test)]
mod tests {
    use super::{select_state_status, LiveStateSource};

    #[test]
    fn uses_local_historical_context_when_no_live_state_exists() {
        let status = select_state_status(100, 100, None, None).unwrap();
        assert_eq!(status.selected_block_number, 100);
        assert_eq!(status.source, LiveStateSource::LocalHistoricalContext);
        assert!(!status.local_context_lags_selected_state());
    }

    #[test]
    fn uses_local_historical_context_when_it_is_caught_up() {
        let status = select_state_status(105, 105, Some(105), Some(105)).unwrap();
        assert_eq!(status.selected_block_number, 105);
        assert_eq!(status.source, LiveStateSource::LocalHistoricalContext);
    }

    #[test]
    fn uses_historical_context_when_reth_finished_is_ahead_of_static_headers() {
        let status = select_state_status(106, 105, Some(105), Some(105)).unwrap();
        assert_eq!(status.selected_block_number, 105);
        assert_eq!(status.source, LiveStateSource::LocalHistoricalContext);
        assert_eq!(status.latest_reth_finished_block_number, 106);
        assert_eq!(status.latest_historical_context_block_number, 105);
    }

    #[test]
    fn uses_tracked_live_state_when_it_is_ahead_of_local_context() {
        let status = select_state_status(100, 100, Some(102), Some(102)).unwrap();
        assert_eq!(status.selected_block_number, 102);
        assert_eq!(status.source, LiveStateSource::TrackedLiveState);
        assert!(status.uses_tracked_live_state());
        assert!(status.local_context_lags_selected_state());
    }

    #[test]
    fn fails_when_live_head_is_ahead_but_tracked_state_is_missing() {
        assert!(select_state_status(100, 100, Some(102), None).is_err());
    }

    #[test]
    fn fails_when_live_head_is_ahead_but_tracked_state_is_stale() {
        assert!(select_state_status(100, 100, Some(102), Some(101)).is_err());
    }
}
