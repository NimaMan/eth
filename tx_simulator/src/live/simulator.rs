use crate::{
    BlockStateSession, SessionTransaction, SignedTransaction, SimulationResult, SimulationSession,
    SimulationSessionOptions, TxSimulator, UnsignedTransaction, UnsignedTxChainSimulation,
};
use alloy_primitives::B256;
use eyre::{eyre, Result};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::watch;
use tokio::time;

const DEFAULT_LIVE_STATE_WINDOW_CAPACITY: usize = 16;

/// Source selected for a stateful simulation.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LiveStateSource {
    /// State comes from an in-memory mined block session published live.
    InMemoryLiveBlockSession,
    /// State comes from local Reth historical context.
    ///
    /// This source is intentionally not produced by `LiveTxSimulator`. It is
    /// kept for explicit historical/latest-Reth adapters and diagnostics.
    LocalHistoricalContext,
}

/// Diagnostic state-selection result for simulation.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct LiveStateStatus {
    pub selected_block_number: u64,
    pub selected_block_hash: Option<B256>,
    pub source: LiveStateSource,
    pub latest_reth_finished_block_number: u64,
    pub latest_historical_context_block_number: u64,
    pub latest_live_block_number: Option<u64>,
    pub latest_tracked_state_block_number: Option<u64>,
}

impl LiveStateStatus {
    pub const fn uses_tracked_live_state(&self) -> bool {
        matches!(self.source, LiveStateSource::InMemoryLiveBlockSession)
    }

    pub const fn local_context_lags_selected_state(&self) -> bool {
        false
    }

    /// Real live pre-submit simulation is only ready when the selected live
    /// state exactly matches the decision block.
    pub const fn is_ready_for_block(&self, required_block_number: u64) -> bool {
        self.selected_block_number == required_block_number
    }
}

/// Latest mined live block state held in memory by the live block processor.
#[derive(Clone)]
pub struct LiveBlockState {
    pub block_number: u64,
    pub block_hash: Option<B256>,
    pub state_root: Option<B256>,
    session: BlockStateSession,
}

impl LiveBlockState {
    pub fn new(session: BlockStateSession) -> Self {
        Self {
            block_number: session.block_number(),
            block_hash: None,
            state_root: None,
            session,
        }
    }

    pub fn with_block_hash(mut self, block_hash: B256) -> Self {
        self.block_hash = Some(block_hash);
        self
    }

    pub fn with_state_root(mut self, state_root: B256) -> Self {
        self.state_root = Some(state_root);
        self
    }

    pub fn session(&self) -> &BlockStateSession {
        &self.session
    }
}

/// Shared in-memory provider for a small exact live block-state window.
#[derive(Clone)]
pub struct InMemoryLiveBlockStateProvider {
    inner: Arc<RwLock<LiveBlockStateWindow>>,
    latest_block_tx: watch::Sender<Option<u64>>,
}

struct LiveBlockStateWindow {
    states: VecDeque<LiveBlockState>,
    capacity: usize,
}

impl Default for InMemoryLiveBlockStateProvider {
    fn default() -> Self {
        let (latest_block_tx, _) = watch::channel(None);
        Self {
            inner: Arc::new(RwLock::new(LiveBlockStateWindow {
                states: VecDeque::with_capacity(DEFAULT_LIVE_STATE_WINDOW_CAPACITY),
                capacity: DEFAULT_LIVE_STATE_WINDOW_CAPACITY,
            })),
            latest_block_tx,
        }
    }
}

impl InMemoryLiveBlockStateProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn publish_latest(&self, state: LiveBlockState) -> Result<()> {
        let block_number = state.block_number;
        let mut window = self
            .inner
            .write()
            .map_err(|_| eyre!("in-memory live block state lock poisoned"))?;
        if let Some(index) = window
            .states
            .iter()
            .position(|existing| existing.block_number == state.block_number)
        {
            window.states.remove(index);
        }
        window.states.push_front(state);
        while window.states.len() > window.capacity {
            window.states.pop_back();
        }
        drop(window);
        self.latest_block_tx.send_replace(Some(block_number));
        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        let mut window = self
            .inner
            .write()
            .map_err(|_| eyre!("in-memory live block state lock poisoned"))?;
        window.states.clear();
        drop(window);
        self.latest_block_tx.send_replace(None);
        Ok(())
    }

    pub fn latest_state(&self) -> Result<LiveBlockState> {
        self.inner
            .read()
            .map_err(|_| eyre!("in-memory live block state lock poisoned"))?
            .states
            .front()
            .cloned()
            .ok_or_else(|| eyre!("latest in-memory live block state is unavailable"))
    }

    pub fn state_at(&self, block_number: u64) -> Result<LiveBlockState> {
        self.inner
            .read()
            .map_err(|_| eyre!("in-memory live block state lock poisoned"))?
            .states
            .iter()
            .find(|state| state.block_number == block_number)
            .cloned()
            .ok_or_else(|| eyre!("in-memory live block state {block_number} is unavailable"))
    }

    pub fn has_state_at(&self, block_number: u64) -> Result<bool> {
        Ok(self
            .inner
            .read()
            .map_err(|_| eyre!("in-memory live block state lock poisoned"))?
            .states
            .iter()
            .any(|state| state.block_number == block_number))
    }

    /// Wait until the exact live block state is available in the in-memory
    /// window. This is a synchronization primitive for block-coupled live
    /// trading, not a historical lookup.
    pub async fn wait_for_state_at(
        &self,
        block_number: u64,
        timeout: Duration,
    ) -> Result<LiveBlockState> {
        if let Ok(state) = self.state_at(block_number) {
            return Ok(state);
        }

        let deadline = time::Instant::now() + timeout;
        let mut latest_block_rx = self.latest_block_tx.subscribe();
        loop {
            let now = time::Instant::now();
            if now >= deadline {
                return Err(eyre!(
                    "timed out waiting for in-memory live block state {block_number}"
                ));
            }
            match time::timeout(
                deadline.saturating_duration_since(now),
                latest_block_rx.changed(),
            )
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(_)) => {
                    return Err(eyre!(
                        "in-memory live block state notifier closed while waiting for {block_number}"
                    ));
                }
                Err(_) => {
                    return Err(eyre!(
                        "timed out waiting for in-memory live block state {block_number}"
                    ));
                }
            }

            if let Ok(state) = self.state_at(block_number) {
                return Ok(state);
            }
            if let Ok(latest) = self.latest_state() {
                if latest.block_number > block_number {
                    return Err(eyre!(
                        "missed in-memory live block state {block_number}; latest live state is {}",
                        latest.block_number
                    ));
                }
            }
        }
    }
}

/// Live transaction simulator backed only by in-memory mined block sessions.
///
/// `TxSimulator` remains the general historical/Reth DB simulator. This type
/// may use shared `TxSimulator` execution machinery, but it never selects
/// headers or state from Reth DB. If the requested in-memory live block session
/// is unavailable, simulation fails.
#[derive(Clone)]
pub struct LiveTxSimulator {
    simulator: Arc<TxSimulator>,
    provider: InMemoryLiveBlockStateProvider,
}

impl LiveTxSimulator {
    pub fn new(simulator: Arc<TxSimulator>, provider: InMemoryLiveBlockStateProvider) -> Self {
        Self {
            simulator,
            provider,
        }
    }

    pub fn from_latest_state(state: LiveBlockState) -> Self {
        let simulator = state.session().simulator();
        let provider = InMemoryLiveBlockStateProvider::new();
        provider
            .publish_latest(state)
            .expect("fresh in-memory live block provider must be writable");
        Self {
            simulator,
            provider,
        }
    }

    pub fn simulator(&self) -> Arc<TxSimulator> {
        Arc::clone(&self.simulator)
    }

    pub fn provider(&self) -> InMemoryLiveBlockStateProvider {
        self.provider.clone()
    }

    /// Latest block for which this simulator has an exact in-memory session.
    pub async fn latest_state_block_number(&self) -> Result<u64> {
        Ok(self.latest_state_status().await?.selected_block_number)
    }

    /// Full diagnostics for the latest in-memory live simulation state.
    pub async fn latest_state_status(&self) -> Result<LiveStateStatus> {
        self.latest_state_status_blocking()
    }

    /// Full diagnostics for an exact in-memory live block session.
    pub async fn state_status_at(&self, block_number: u64) -> Result<LiveStateStatus> {
        let state = self.provider.state_at(block_number)?;
        Ok(live_state_status_from_state(&state))
    }

    /// Full diagnostics, requiring the exact signal dependency block.
    pub async fn latest_state_status_at_or_after(
        &self,
        required_block_number: u64,
    ) -> Result<LiveStateStatus> {
        let status = self.latest_state_status().await?;
        if !status.is_ready_for_block(required_block_number) {
            return Err(eyre!(
                "live simulation state block mismatch: selected_block={} required_block={} source={:?}",
                status.selected_block_number,
                required_block_number,
                status.source
            ));
        }
        Ok(status)
    }

    /// True when the in-memory live-state window still has an exact block.
    pub async fn has_state_at(&self, block_number: u64) -> Result<bool> {
        self.provider.has_state_at(block_number)
    }

    /// Blocking variant for callers that already run this work outside an async
    /// hot path.
    pub fn latest_state_status_blocking(&self) -> Result<LiveStateStatus> {
        let latest = self.provider.latest_state()?;
        Ok(live_state_status_from_state(&latest))
    }

    /// Wait until an exact in-memory live block session exists and return its
    /// diagnostics.
    pub async fn wait_for_state_at(
        &self,
        block_number: u64,
        timeout: Duration,
    ) -> Result<LiveStateStatus> {
        let state = self
            .provider
            .wait_for_state_at(block_number, timeout)
            .await?;
        Ok(live_state_status_from_state(&state))
    }

    /// Latest block announced by the in-memory live-state source.
    pub async fn latest_live_block_number(&self) -> Result<Option<u64>> {
        Ok(Some(self.provider.latest_state()?.block_number))
    }

    /// Start a stateful simulation chain at the latest in-memory state block.
    pub async fn start_latest_chain(&self) -> Result<UnsignedTxChainSimulation> {
        Ok(self.provider.latest_state()?.session().simulation_chain())
    }

    /// Start a stateful simulation chain at a specific live block.
    pub async fn start_chain_at(&self, block_number: u64) -> Result<UnsignedTxChainSimulation> {
        Ok(self
            .provider
            .state_at(block_number)?
            .session()
            .simulation_chain())
    }

    /// Start a mixed signed/unsigned session at the latest in-memory state.
    pub async fn start_latest_session(&self) -> Result<SimulationSession> {
        Ok(self.provider.latest_state()?.session().simulation_session())
    }

    /// Start a mixed signed/unsigned session at a specific live block.
    pub async fn start_session_at(&self, block_number: u64) -> Result<SimulationSession> {
        Ok(self
            .provider
            .state_at(block_number)?
            .session()
            .simulation_session())
    }

    /// Simulate one unsigned transaction against the latest in-memory state.
    pub async fn simulate_transaction(
        &self,
        transaction: UnsignedTransaction,
    ) -> Result<SimulationResult> {
        let mut session = self.start_latest_session().await?;
        session.step_unsigned(transaction)
    }

    /// Simulate one signed transaction against the latest in-memory state.
    pub async fn simulate_signed_transaction(
        &self,
        transaction: &SignedTransaction,
    ) -> Result<SimulationResult> {
        let mut session = self.start_latest_session().await?;
        session.step_signed(transaction)
    }

    /// Simulate a sequence of unsigned transactions against the latest in-memory state.
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

    /// Simulate a mixed signed/unsigned sequence against the latest in-memory state.
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

fn live_state_status_from_state(state: &LiveBlockState) -> LiveStateStatus {
    LiveStateStatus {
        selected_block_number: state.block_number,
        selected_block_hash: state.block_hash,
        source: LiveStateSource::InMemoryLiveBlockSession,
        latest_reth_finished_block_number: state.block_number,
        latest_historical_context_block_number: state.block_number,
        latest_live_block_number: Some(state.block_number),
        latest_tracked_state_block_number: Some(state.block_number),
    }
}

/// Explicit adapter for callers that want the latest locally-readable Reth
/// historical context. This is not a live pre-submit simulator.
#[derive(Clone)]
pub struct LatestHistoricalTxSimulator {
    simulator: Arc<TxSimulator>,
}

impl LatestHistoricalTxSimulator {
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

    pub async fn latest_state_block_number(&self) -> Result<u64> {
        Ok(self.latest_state_status().await?.selected_block_number)
    }

    pub async fn latest_state_status(&self) -> Result<LiveStateStatus> {
        self.latest_state_status_blocking()
    }

    pub fn latest_state_status_blocking(&self) -> Result<LiveStateStatus> {
        let latest_reth_finished = self.latest_reth_finished_block_number()?;
        let latest_historical_context = self.latest_historical_context_block_number()?;
        Ok(local_historical_context_state_status(
            latest_reth_finished,
            latest_historical_context,
        ))
    }

    pub async fn latest_live_block_number(&self) -> Result<Option<u64>> {
        Ok(None)
    }

    /// Latest block reported by Reth's Finish stage.
    pub fn latest_reth_finished_block_number(&self) -> Result<u64> {
        self.simulator.get_latest_block()
    }

    /// Compatibility alias for callers that still use the old name.
    pub fn latest_persisted_block_number(&self) -> Result<u64> {
        self.latest_reth_finished_block_number()
    }

    /// Latest block that can be simulated entirely from local historical Reth context.
    pub fn latest_historical_context_block_number(&self) -> Result<u64> {
        self.simulator.latest_historical_context_block_number()
    }

    pub async fn start_latest_chain(&self) -> Result<UnsignedTxChainSimulation> {
        let block_number = self.latest_state_status().await?.selected_block_number;
        self.simulator
            .start_simulation_chain(Some(block_number))
            .await
    }

    pub async fn start_chain_at(&self, block_number: u64) -> Result<UnsignedTxChainSimulation> {
        self.simulator
            .start_simulation_chain(Some(block_number))
            .await
    }

    pub async fn start_latest_session(&self) -> Result<SimulationSession> {
        let block_number = self.latest_state_status().await?.selected_block_number;
        self.start_session_at(block_number).await
    }

    pub async fn start_session_at(&self, block_number: u64) -> Result<SimulationSession> {
        self.simulator
            .simulation_session_with_options(SimulationSessionOptions {
                at_block: Some(block_number),
                gas_block_number: None,
            })
            .await
    }

    pub async fn simulate_transaction(
        &self,
        transaction: UnsignedTransaction,
    ) -> Result<SimulationResult> {
        let mut session = self.start_latest_session().await?;
        session.step_unsigned(transaction)
    }
}

fn local_historical_context_state_status(
    latest_reth_finished: u64,
    latest_historical_context: u64,
) -> LiveStateStatus {
    LiveStateStatus {
        selected_block_number: latest_historical_context,
        selected_block_hash: None,
        source: LiveStateSource::LocalHistoricalContext,
        latest_reth_finished_block_number: latest_reth_finished,
        latest_historical_context_block_number: latest_historical_context,
        latest_live_block_number: None,
        latest_tracked_state_block_number: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{local_historical_context_state_status, LiveStateSource};

    #[test]
    fn historical_adapter_status_is_explicitly_local_context() {
        let status = local_historical_context_state_status(106, 105);
        assert_eq!(status.selected_block_number, 105);
        assert_eq!(status.source, LiveStateSource::LocalHistoricalContext);
        assert_eq!(status.latest_reth_finished_block_number, 106);
        assert_eq!(status.latest_historical_context_block_number, 105);
    }

    #[test]
    fn live_readiness_requires_exact_decision_block() {
        let status = super::LiveStateStatus {
            selected_block_number: 102,
            selected_block_hash: None,
            source: LiveStateSource::InMemoryLiveBlockSession,
            latest_reth_finished_block_number: 102,
            latest_historical_context_block_number: 102,
            latest_live_block_number: Some(102),
            latest_tracked_state_block_number: Some(102),
        };
        assert!(status.is_ready_for_block(102));
        assert!(!status.is_ready_for_block(101));
        assert!(!status.is_ready_for_block(103));
    }
}
