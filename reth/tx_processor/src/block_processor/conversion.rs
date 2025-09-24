use alloy_primitives::{Log as AlloyLog, U256};
use reth_chain_query::provider::{
    CallFrame as ProviderCallFrame, Log as ProviderLog, TransactionTrace,
};
use tx_simulator::CallFrame;

/// Convert logs from provider format to alloy primitives log type used by tx_processor
pub fn convert_logs(raw_logs: &[ProviderLog]) -> Vec<AlloyLog> {
    raw_logs
        .iter()
        .map(|log| AlloyLog::new_unchecked(log.address, log.topics.clone(), log.data.clone()))
        .collect()
}

/// Convert provider call frame (simplified trace) into tx_simulator call frame
pub fn convert_call_frame(frame: &ProviderCallFrame) -> CallFrame {
    let mut converted = CallFrame {
        from: frame.from,
        gas: U256::from(frame.gas_limit),
        gas_used: U256::from(frame.gas_used),
        to: frame.to,
        input: frame.input.clone(),
        output: if frame.output.is_empty() {
            None
        } else {
            Some(frame.output.clone())
        },
        error: None,
        revert_reason: None,
        calls: frame.subcalls.iter().map(convert_call_frame).collect(),
        logs: Vec::new(),
        value: Some(frame.value),
        typ: frame.call_type.to_string(),
    };

    // Propagate revert reason if present in any subcall
    if converted
        .calls
        .iter()
        .any(|child| child.error.is_some() || child.revert_reason.is_some())
    {
        converted.error = converted
            .calls
            .iter()
            .find_map(|child| child.error.clone().or_else(|| child.revert_reason.clone()));
    }

    converted
}

/// Convert entire transaction trace into tx_simulator call frame, stitching in metadata
pub fn convert_transaction_trace(trace: &TransactionTrace) -> CallFrame {
    let mut root = convert_call_frame(&trace.call_frame);

    if !trace.output.is_empty() {
        root.output = Some(trace.output.clone());
    }

    let gas_used = U256::from(trace.gas_used);
    root.gas_used = gas_used;

    if let Some(error) = &trace.error {
        root.error = Some(error.clone());
    }

    root
}
