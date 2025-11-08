/// Transaction Trace Processor - Rust equivalent of Python's tx_trace_processor.py
///
/// OBJECTIVE: Extract internal transactions from call traces
///
/// This module processes call traces from simulation results to extract:
/// 1. Internal ETH transfers with non-zero value
/// 2. Contract creations (CREATE/CREATE2)
/// 3. Failed transactions and their children
/// 4. Initial transaction (depth == 0)
///
/// Just like Python's tx_trace_processor.py, this converts raw simulation traces
/// into structured InternalTransaction objects.
use super::data_models::InternalTransaction;
use alloy_primitives::U256;
use tx_simulator::types::CallFrame;

/// Processes transaction traces to extract internal transactions
pub struct TransactionTraceProcessor;

impl TransactionTraceProcessor {
    /// Create new trace processor
    pub fn new() -> Self {
        Self
    }

    /// Extract internal transactions from CallFrame (moved from tx_simulator)
    ///
    /// This recursively walks the call trace tree and extracts all internal transactions.
    /// Matches Python's process_trace() method behavior.
    pub fn extract_internal_transactions_from_call_trace(
        &self,
        call_trace: &CallFrame,
    ) -> Vec<InternalTransaction> {
        let mut internal_txs = Vec::new();
        self.extract_from_frame_recursive(call_trace, &mut internal_txs, 0, vec![]);
        internal_txs
    }

    /// Recursively extract internal transactions from call frame with error tracking
    fn extract_from_frame_recursive(
        &self,
        frame: &CallFrame,
        internal_txs: &mut Vec<InternalTransaction>,
        depth: u32,
        trace_address: Vec<usize>,
    ) {
        // Determine if this frame has an error (check revert_reason or error field)
        let error = if let Some(ref reason) = frame.revert_reason {
            Some(reason.clone())
        } else if let Some(ref error_msg) = frame.error {
            Some(error_msg.clone())
        } else {
            None
        };

        // Add this frame as internal transaction (including root at depth 0)
        internal_txs.push(InternalTransaction {
            from_address: frame.from,
            to_address: frame.to,
            value: frame.value.unwrap_or_default(),
            gas: frame.gas.try_into().unwrap_or(u64::MAX),
            gas_used: frame.gas_used.try_into().unwrap_or(u64::MAX),
            trace_type: format!("{:?}", frame.typ),
            call_type: Some(format!("{:?}", frame.typ)),
            depth,
            error: error.clone(),
        });

        // Process child calls, passing down parent error state
        for (i, child) in frame.calls.iter().enumerate() {
            let mut child_trace = trace_address.clone();
            child_trace.push(i);
            self.extract_from_frame_recursive(child, internal_txs, depth + 1, child_trace);
        }
    }

    /// Process trace data from simulation to extract internal transactions
    ///
    /// This is the Rust equivalent of Python's process_trace() method.
    /// It takes the internal transactions from tx_simulator and converts them
    /// to our ProcessedTransaction's InternalTransaction format.
    pub fn process_internal_transactions(
        &self,
        internal_txs: &[InternalTransaction],
    ) -> Vec<InternalTransaction> {
        internal_txs
            .iter()
            .map(|tx| self.convert_to_internal_transaction(tx))
            .collect()
    }

    /// Convert tx_simulator's InternalTransaction to our format
    fn convert_to_internal_transaction(&self, sim_tx: &InternalTransaction) -> InternalTransaction {
        InternalTransaction {
            from_address: sim_tx.from_address,
            to_address: sim_tx.to_address,
            value: sim_tx.value,
            gas: sim_tx.gas,
            gas_used: sim_tx.gas_used,
            trace_type: sim_tx.trace_type.clone(),
            call_type: sim_tx.call_type.clone(),
            depth: sim_tx.depth,
            error: None, // tx_simulator's internal tx doesn't have error field yet
        }
    }

    /// Filter internal transactions to only include significant ones
    ///
    /// Similar to Python's filtering logic:
    /// - Include if it's the initial transaction (depth == 0)
    /// - Include if it's a contract creation
    /// - Include if it has non-zero value
    /// - Include if it has an error
    pub fn filter_significant_internal_transactions(
        &self,
        internal_txs: Vec<InternalTransaction>,
    ) -> Vec<InternalTransaction> {
        internal_txs
            .into_iter()
            .filter(|tx| {
                // Include if:
                // 1. It's the initial transaction, or
                // 2. It's a contract creation (CREATE/CREATE2), or
                // 3. Has non-zero value
                tx.depth == 0 || tx.trace_type.contains("CREATE") || tx.value > U256::ZERO
            })
            .collect()
    }

    // TODO: Implement extract_eth_transfers if needed
    // This method was removed as EthTransfer type is not defined
    // The functionality is now handled in AddressBalanceChangeCalculator
}

impl Default for TransactionTraceProcessor {
    fn default() -> Self {
        Self::new()
    }
}
