use std::collections::BTreeMap;

use super::progress::LiveTokenError;

pub(super) fn live_error_from_report(
    block_number: Option<u64>,
    tx_index: Option<u64>,
    tx_hash: Option<String>,
    error: &eyre::Report,
    mut context: BTreeMap<String, String>,
) -> LiveTokenError {
    if let Some(root_error) = error.chain().last() {
        context.insert("root_error".to_string(), root_error.to_string());
    }

    LiveTokenError::new(block_number, tx_index, tx_hash, error.to_string())
        .with_detail(error_chain_detail(error))
        .with_context_map(context)
}

pub(super) fn phase_context(phase: &str) -> BTreeMap<String, String> {
    let mut context = BTreeMap::new();
    context.insert("phase".to_string(), phase.to_string());
    context
}

pub(super) fn error_chain_detail(error: &eyre::Report) -> String {
    error
        .chain()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(": ")
}
