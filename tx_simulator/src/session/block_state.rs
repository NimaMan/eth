use std::sync::Arc;

use alloy_primitives::{B256, U256};
use alloy_rpc_types_trace::geth::PreStateFrame;
use eyre::{eyre, Result};
use reth_primitives_traits::SealedHeader;

use crate::{
    block_context::apply_prestate_diff,
    simulator::TxSimulator,
    tx_chain::{sequential::ForkedState, unsigned::UnsignedTxChainSimulation},
};

/// A block-scoped state session that keeps one forked state warm and creates
/// isolated simulation branches from it.
#[derive(Clone)]
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
        if block_header.number != block_number {
            return Err(eyre!(
                "block state session header mismatch: header={}, requested={}",
                block_header.number,
                block_number
            ));
        }
        let forked_state = simulator.create_forked_state_with_header(block_number, block_header)?;
        Ok(Self {
            simulator,
            block_number,
            forked_state,
        })
    }

    pub(crate) fn from_forked_state(
        simulator: Arc<TxSimulator>,
        block_number: u64,
        forked_state: ForkedState,
    ) -> Result<Self> {
        if forked_state.block_number != block_number {
            return Err(eyre!(
                "block state session fork mismatch: fork={}, requested={}",
                forked_state.block_number,
                block_number
            ));
        }
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
    /// single historical block state.
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

    /// Open a block-scoped state session from exact live prestate diffs supplied
    /// by the caller.
    pub async fn block_state_session_from_prestate_diffs(
        &self,
        block_number: u64,
        block_hash: B256,
        parent_hash: B256,
        block_header: SealedHeader,
        state_diffs: &[PreStateFrame],
    ) -> Result<BlockStateSession> {
        let forked_state = self
            .block_context_loader()
            .direct_forked_state_from_prestate_diffs(
                block_number,
                block_hash,
                parent_hash,
                block_header,
                state_diffs,
            )
            .await?;
        BlockStateSession::from_forked_state(Arc::new(self.clone()), block_number, forked_state)
    }

    /// Open a block-scoped state session by advancing a prior direct live
    /// session with exact prestate diffs for the next block.
    pub async fn block_state_session_from_parent_prestate_diffs(
        &self,
        parent_session: &BlockStateSession,
        block_number: u64,
        block_hash: B256,
        parent_hash: B256,
        block_header: SealedHeader,
        state_diffs: &[PreStateFrame],
    ) -> Result<BlockStateSession> {
        let expected_parent = block_number
            .checked_sub(1)
            .ok_or_else(|| eyre!("cannot build direct live state for genesis block"))?;
        if parent_session.block_number != expected_parent {
            return Err(eyre!(
                "direct live state parent session mismatch: parent session={}, expected={}",
                parent_session.block_number,
                expected_parent
            ));
        }
        if parent_session.forked_state.block_header.hash() != parent_hash {
            return Err(eyre!(
                "direct live state parent hash mismatch for block {}: parent session={}, expected={}",
                block_number,
                parent_session.forked_state.block_header.hash(),
                parent_hash
            ));
        }
        if block_header.number != block_number {
            return Err(eyre!(
                "direct live state header block mismatch: header={}, expected={}",
                block_header.number,
                block_number
            ));
        }
        if block_header.hash() != block_hash {
            return Err(eyre!(
                "direct live state header hash mismatch for block {}: header={}, expected={}",
                block_number,
                block_header.hash(),
                block_hash
            ));
        }
        if block_header.parent_hash != parent_hash {
            return Err(eyre!(
                "direct live state parent hash mismatch for block {}: header={}, expected={}",
                block_number,
                block_header.parent_hash,
                parent_hash
            ));
        }

        let mut forked_state = parent_session.forked_state.clone();
        forked_state.block_number = block_header.number;
        forked_state.block_header = block_header;
        forked_state.nonces.clear();
        forked_state
            .db
            .cache
            .block_hashes
            .insert(U256::from(expected_parent), parent_hash);
        forked_state
            .db
            .cache
            .block_hashes
            .insert(U256::from(block_number), block_hash);
        for (tx_index, frame) in state_diffs.iter().enumerate() {
            let diff = frame.as_diff().ok_or_else(|| {
                eyre!(
                    "direct live state diff for block {} tx index {} was not diffMode",
                    block_number,
                    tx_index
                )
            })?;
            apply_prestate_diff(&mut forked_state, diff)?;
        }

        BlockStateSession::from_forked_state(Arc::new(self.clone()), block_number, forked_state)
    }
}
