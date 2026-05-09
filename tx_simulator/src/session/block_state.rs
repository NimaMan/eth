use std::sync::Arc;

use eyre::Result;
use reth_primitives_traits::SealedHeader;

use crate::{
    simulator::TxSimulator,
    tx_chain::{sequential::ForkedState, unsigned::UnsignedTxChainSimulation},
};

/// A block-scoped state session that keeps one forked state warm and creates
/// isolated simulation branches from it.
pub struct BlockStateSession {
    simulator: Arc<TxSimulator>,
    block_number: u64,
    forked_state: ForkedState,
}

impl BlockStateSession {
    pub(crate) async fn new(simulator: Arc<TxSimulator>, block_number: u64) -> Result<Self> {
        let forked_state = simulator.create_forked_state(block_number)?;
        Ok(Self {
            simulator,
            block_number,
            forked_state,
        })
    }

    pub(crate) async fn new_with_header(
        simulator: Arc<TxSimulator>,
        block_number: u64,
        block_header: SealedHeader,
    ) -> Result<Self> {
        let forked_state = simulator.create_forked_state_with_header(block_number, block_header)?;
        Ok(Self {
            simulator,
            block_number,
            forked_state,
        })
    }

    pub const fn block_number(&self) -> u64 {
        self.block_number
    }

    pub fn simulation_chain(&self) -> UnsignedTxChainSimulation {
        UnsignedTxChainSimulation::new(Arc::clone(&self.simulator), self.forked_state.clone())
    }
}

impl TxSimulator {
    /// Open a block-scoped state session for repeated branch simulations at a
    /// single historical or live-cache block state.
    pub async fn block_state_session(&self, block_number: u64) -> Result<BlockStateSession> {
        BlockStateSession::new(Arc::new(self.clone()), block_number).await
    }

    /// Open a block-scoped state session using a caller-supplied header.
    pub async fn block_state_session_with_header(
        &self,
        block_number: u64,
        block_header: SealedHeader,
    ) -> Result<BlockStateSession> {
        BlockStateSession::new_with_header(Arc::new(self.clone()), block_number, block_header).await
    }
}
