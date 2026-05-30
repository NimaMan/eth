/// Transaction trace processor.
///
/// OBJECTIVE: Extract internal transactions from call traces
///
/// This module processes call traces from simulation results to extract:
/// 1. Internal ETH transfers with non-zero value
/// 2. Contract creations (CREATE/CREATE2)
/// 3. Failed transactions and their children
/// 4. Initial transaction (depth == 0)
///
/// Converts raw simulation traces into structured `InternalTransaction` objects.
use super::data_models::{Erc20CallKind, InternalErc20Call, InternalTransaction};
use alloy_primitives::{Address, U256};
use tx_simulator::types::CallFrame;

const ERC20_TRANSFER_SELECTOR: [u8; 4] = [0xa9, 0x05, 0x9c, 0xbb];
const ERC20_TRANSFER_FROM_SELECTOR: [u8; 4] = [0x23, 0xb8, 0x72, 0xdd];
const ERC20_APPROVE_SELECTOR: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3];

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
    /// Takes internal transactions from tx_simulator and converts them to this
    /// crate's `InternalTransaction` format.
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

    /// Extract standard ERC-20 mutating calls (`transfer` / `transferFrom` /
    /// `approve`) from the call trace, at any depth.
    ///
    /// Unlike the top-level `transfer_from_calls` path, this captures internal
    /// calls (where the top-level `tx.to` is not the token), which is how
    /// custody backdoors move balances without emitting a `Transfer` event.
    pub fn extract_erc20_calls_from_call_trace(
        &self,
        call_trace: &CallFrame,
    ) -> Vec<InternalErc20Call> {
        let mut calls = Vec::new();
        self.extract_erc20_calls_recursive(call_trace, &mut calls, 0);
        calls
    }

    fn extract_erc20_calls_recursive(
        &self,
        frame: &CallFrame,
        out: &mut Vec<InternalErc20Call>,
        depth: u32,
    ) {
        if let (Some(token), Some((kind, decoded_from, to_address, amount))) =
            (frame.to, decode_erc20_call(frame.input.as_ref()))
        {
            // For transferFrom the holder is arg0; for transfer/approve the
            // acting holder/owner is the caller (msg.sender of the call).
            let from_address = match kind {
                Erc20CallKind::TransferFrom => decoded_from,
                Erc20CallKind::Transfer | Erc20CallKind::Approve => frame.from,
            };
            out.push(InternalErc20Call {
                token_address: token,
                caller: frame.from,
                kind,
                from_address,
                to_address,
                amount,
                depth,
                call_type: Some(format!("{:?}", frame.typ)),
                succeeded: frame.error.is_none() && frame.revert_reason.is_none(),
            });
        }
        for child in frame.calls.iter() {
            self.extract_erc20_calls_recursive(child, out, depth + 1);
        }
    }
}

/// Decode a standard ERC-20 mutating call from calldata. Returns
/// `(kind, decoded_from, to_or_spender, amount)`; `decoded_from` is the real
/// arg0 only for `transferFrom` (zero otherwise — the caller fills it in).
fn decode_erc20_call(input: &[u8]) -> Option<(Erc20CallKind, Address, Address, U256)> {
    let selector: [u8; 4] = input.get(0..4)?.try_into().ok()?;
    match selector {
        ERC20_TRANSFER_FROM_SELECTOR => Some((
            Erc20CallKind::TransferFrom,
            read_address(input, 0)?,
            read_address(input, 1)?,
            read_u256(input, 2)?,
        )),
        ERC20_TRANSFER_SELECTOR => Some((
            Erc20CallKind::Transfer,
            Address::ZERO,
            read_address(input, 0)?,
            read_u256(input, 1)?,
        )),
        ERC20_APPROVE_SELECTOR => Some((
            Erc20CallKind::Approve,
            Address::ZERO,
            read_address(input, 0)?,
            read_u256(input, 1)?,
        )),
        _ => None,
    }
}

fn read_word(input: &[u8], index: usize) -> Option<[u8; 32]> {
    let start = 4 + index * 32;
    input.get(start..start + 32)?.try_into().ok()
}

fn read_address(input: &[u8], index: usize) -> Option<Address> {
    let word = read_word(input, index)?;
    Some(Address::from_slice(&word[12..32]))
}

fn read_u256(input: &[u8], index: usize) -> Option<U256> {
    let word = read_word(input, index)?;
    Some(U256::from_be_slice(&word))
}

impl Default for TransactionTraceProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod erc20_call_tests {
    use super::*;
    use alloy_primitives::Bytes;

    fn transfer_from_calldata(from: Address, to: Address, amount: U256) -> Bytes {
        let mut data = ERC20_TRANSFER_FROM_SELECTOR.to_vec();
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(from.as_slice());
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(to.as_slice());
        data.extend_from_slice(&amount.to_be_bytes::<32>());
        Bytes::from(data)
    }

    #[test]
    fn decodes_internal_transfer_from_to_dead() {
        let token = Address::from_slice(&[0x11u8; 20]);
        let helper = Address::from_slice(&[0x22u8; 20]);
        let vault = Address::from_slice(&[0x33u8; 20]);
        let dead = Address::from_slice(&hex_dead());
        let amount = U256::from(9_871_487u64);

        // Top-level frame (helper.multicall) with an internal token.transferFrom.
        let inner = CallFrame {
            from: helper,
            to: Some(token),
            input: transfer_from_calldata(vault, dead, amount),
            ..Default::default()
        };
        let root = CallFrame {
            from: vault,
            to: Some(helper),
            calls: vec![inner],
            ..Default::default()
        };

        let processor = TransactionTraceProcessor::new();
        let calls = processor.extract_erc20_calls_from_call_trace(&root);
        assert_eq!(calls.len(), 1);
        let call = &calls[0];
        assert_eq!(call.kind, Erc20CallKind::TransferFrom);
        assert_eq!(call.token_address, token);
        assert_eq!(call.caller, helper);
        assert_eq!(call.from_address, vault);
        assert_eq!(call.to_address, dead);
        assert_eq!(call.amount, amount);
        assert_eq!(call.depth, 1);
        assert!(call.succeeded);
    }

    fn hex_dead() -> [u8; 20] {
        let mut a = [0u8; 20];
        a[18] = 0xde;
        a[19] = 0xad;
        a
    }
}
