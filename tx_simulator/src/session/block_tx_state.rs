use std::sync::Arc;
use std::time::Instant;

use alloy_primitives::B256;
use eyre::{eyre, Result};
use reth_ethereum_primitives::TransactionSigned;
use reth_primitives_traits::{SealedHeader, SignerRecoverable};
use reth_provider::BlockReader;
use reth_revm::Database;

use crate::{
    simulator::TxSimulator,
    tx_chain::{sequential::ForkedState, unsigned::UnsignedTxChainSimulation},
    types::{FullSimulationResult, SimulationResult},
};

/// A block-scoped replay session that advances one canonical signed prefix and
/// branches simulations from that warmed state.
pub struct BlockTxStateSession {
    simulator: Arc<TxSimulator>,
    block_number: u64,
    block_hash: B256,
    block_header: SealedHeader,
    transactions: Vec<TransactionSigned>,
    canonical_state: ForkedState,
    next_prefix_index: usize,
}

#[derive(Debug, Clone)]
pub struct BlockTxAdvanceProfile {
    pub from_tx_index: usize,
    pub to_tx_index: usize,
    pub applied_txs: usize,
    pub elapsed_ms: f64,
}

#[derive(Debug, Clone)]
pub struct BlockTxTraceProfile {
    pub tx_index: usize,
    pub tx_hash: B256,
    pub branch_clone_ms: f64,
    pub trace_ms: f64,
    pub result: FullSimulationResult,
}

#[derive(Debug, Clone)]
pub struct BlockTxExecuteProfile {
    pub tx_index: usize,
    pub tx_hash: B256,
    pub branch_clone_ms: f64,
    pub execute_ms: f64,
    pub result: SimulationResult,
}

impl BlockTxStateSession {
    pub(crate) async fn new(simulator: Arc<TxSimulator>, block_number: u64) -> Result<Self> {
        simulator.refresh_static_file_provider()?;
        let provider = simulator.provider_factory.provider()?;
        let block = provider
            .block_by_number(block_number)?
            .ok_or_else(|| eyre!("Block {} not found", block_number))?;
        let block_hash = block.header.hash_slow();
        let block_header = SealedHeader::new(block.header.clone(), block_hash);
        let parent_number = block_number
            .checked_sub(1)
            .ok_or_else(|| eyre!("cannot create block tx state session for genesis"))?;
        let mut canonical_state = simulator.create_forked_state(parent_number)?;
        canonical_state.block_number = block_number;
        canonical_state.block_header = block_header.clone();
        canonical_state.nonces.clear();
        if let Some(withdrawals) = block.body.withdrawals.as_ref() {
            for withdrawal in withdrawals {
                let mut account = canonical_state
                    .db
                    .basic(withdrawal.address)?
                    .unwrap_or_default();
                account.balance += withdrawal.amount_wei();
                canonical_state
                    .db
                    .insert_account_info(withdrawal.address, account);
            }
        }

        Ok(Self {
            simulator,
            block_number,
            block_hash,
            block_header,
            transactions: block.body.transactions.clone(),
            canonical_state,
            next_prefix_index: 0,
        })
    }

    pub const fn block_number(&self) -> u64 {
        self.block_number
    }

    pub const fn block_hash(&self) -> B256 {
        self.block_hash
    }

    pub const fn block_header(&self) -> &SealedHeader {
        &self.block_header
    }

    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }

    pub fn transaction_hash(&self, tx_index: usize) -> Result<B256> {
        let tx = self
            .transactions
            .get(tx_index)
            .ok_or_else(|| self.tx_index_error(tx_index))?;
        Ok(*tx.tx_hash())
    }

    /// Advance the canonical prefix to the state immediately before `tx_index`.
    pub fn advance_to_before_tx(&mut self, tx_index: usize) -> Result<BlockTxAdvanceProfile> {
        if tx_index > self.transactions.len() {
            return Err(self.tx_index_error(tx_index));
        }
        if tx_index < self.next_prefix_index {
            return Err(eyre!(
                "block tx state session only advances monotonically: current={}, requested={}",
                self.next_prefix_index,
                tx_index
            ));
        }

        let started = Instant::now();
        let from_tx_index = self.next_prefix_index;
        for index in self.next_prefix_index..tx_index {
            let tx = &self.transactions[index];
            self.simulator
                .simulate_signed_on_fork_without_trace(&mut self.canonical_state, tx)
                .map_err(|err| {
                    let tx_hash = *tx.tx_hash();
                    let sender = tx.recover_signer().ok();
                    eyre!(
                        "failed to advance block tx session block_number={} tx_index={} tx_hash={:?} sender={:?}: {}",
                        self.block_number,
                        index,
                        tx_hash,
                        sender,
                        err
                    )
                })?;
        }
        self.next_prefix_index = tx_index;

        Ok(BlockTxAdvanceProfile {
            from_tx_index,
            to_tx_index: tx_index,
            applied_txs: tx_index.saturating_sub(from_tx_index),
            elapsed_ms: elapsed_ms(started),
        })
    }

    /// Return an isolated unsigned simulation chain at the state before `tx_index`.
    pub fn simulation_chain_before_tx(
        &mut self,
        tx_index: usize,
    ) -> Result<UnsignedTxChainSimulation> {
        self.advance_to_before_tx(tx_index)?;
        Ok(self.branch_unsigned_chain())
    }

    /// Return an isolated unsigned simulation chain at the state after `tx_index`.
    pub fn simulation_chain_after_tx(
        &mut self,
        tx_index: usize,
    ) -> Result<UnsignedTxChainSimulation> {
        let after_index = tx_index
            .checked_add(1)
            .ok_or_else(|| eyre!("tx index overflow"))?;
        self.advance_to_before_tx(after_index)?;
        Ok(self.branch_unsigned_chain())
    }

    pub fn trace_mined_tx_at_index(&mut self, tx_index: usize) -> Result<BlockTxTraceProfile> {
        self.advance_to_before_tx(tx_index)?;
        let tx = self
            .transactions
            .get(tx_index)
            .ok_or_else(|| self.tx_index_error(tx_index))?
            .clone();
        let tx_hash = *tx.tx_hash();
        let branch_started = Instant::now();
        let mut branch = self.canonical_state.clone();
        let branch_clone_ms = elapsed_ms(branch_started);

        let trace_started = Instant::now();
        let result = self
            .simulator
            .simulate_signed_on_fork_with_trace(&mut branch, &tx)?;

        Ok(BlockTxTraceProfile {
            tx_index,
            tx_hash,
            branch_clone_ms,
            trace_ms: elapsed_ms(trace_started),
            result,
        })
    }

    pub fn execute_mined_tx_at_index(&mut self, tx_index: usize) -> Result<BlockTxExecuteProfile> {
        self.advance_to_before_tx(tx_index)?;
        let tx = self
            .transactions
            .get(tx_index)
            .ok_or_else(|| self.tx_index_error(tx_index))?
            .clone();
        let tx_hash = *tx.tx_hash();
        let branch_started = Instant::now();
        let mut branch = self.canonical_state.clone();
        let branch_clone_ms = elapsed_ms(branch_started);

        let execute_started = Instant::now();
        let result = self
            .simulator
            .simulate_signed_on_fork_without_trace(&mut branch, &tx)?;

        Ok(BlockTxExecuteProfile {
            tx_index,
            tx_hash,
            branch_clone_ms,
            execute_ms: elapsed_ms(execute_started),
            result,
        })
    }

    fn branch_unsigned_chain(&self) -> UnsignedTxChainSimulation {
        UnsignedTxChainSimulation::new(Arc::clone(&self.simulator), self.canonical_state.clone())
    }

    fn tx_index_error(&self, tx_index: usize) -> eyre::Report {
        eyre!(
            "tx index {} out of range for block {} with {} txs",
            tx_index,
            self.block_number,
            self.transactions.len()
        )
    }
}

impl TxSimulator {
    /// Open a block-scoped state session for fast tx-index and pool simulations.
    pub async fn block_tx_state_session(&self, block_number: u64) -> Result<BlockTxStateSession> {
        BlockTxStateSession::new(Arc::new(self.clone()), block_number).await
    }
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}
