use std::time::Instant;

use alloy_primitives::keccak256;
use alloy_primitives::Address;
use reth_chain_query::{provider::RethQueryProvider, to_checksum_address};
use rlp::RlpStream;
use tracing::info;

use crate::{
    token_tracking::token_parameter_extraction::fetch_token_metadata,
    tx_router::TransactionCategory,
};

use super::{
    logging::format_contract_creation_context, pending_sequences::SequenceKey, SimulationManager,
    SimulationResult, TxSimulationJob,
};

impl SimulationManager {
    pub(super) async fn handle_contract_creation(
        &self,
        request: &TxSimulationJob,
        now: Instant,
    ) -> SimulationResult {
        let mut result = SimulationResult {
            request: request.clone(),
            pool_viability_result: None,
            error: None,
            simulation_time_ms: 0.0,
            token_address: None,
            pool_address: None,
            pool_type: None,
            debug_info: None,
            liquidity_removal_result: None,
        };

        let context = format_contract_creation_context(request);

        let processed = match self.build_processed_transaction(request, true).await {
            Ok(tx) => tx,
            Err(err) => {
                result.error = Some(format!(
                    "Failed to process contract creation ({}): {}",
                    context, err
                ));
                return result;
            }
        };

        let contract_address = processed.contract_address.or_else(|| {
            processed
                .contract_creation_events
                .first()
                .map(|evt| evt.contract_address)
                .or_else(|| {
                    // Fallback: derive from sender + nonce when receipt/event is missing
                    derive_contract_address(processed.from_address, processed.nonce)
                })
        });

        let Some(contract_address) = contract_address else {
            result.error = Some(format!(
                "Unable to determine deployed contract address ({})",
                context
            ));
            return result;
        };

        result.token_address = Some(contract_address);

        if let TransactionCategory::ContractCreation { deployer, .. } = &request.category {
            let key = SequenceKey {
                creator: deployer.clone(),
                token: to_checksum_address(&contract_address),
            };
            self.record_pending_transaction(key, processed, now).await;
        }

        match RethQueryProvider::with_simulator(self.mempool_simulator.get_tx_simulator()) {
            Ok(provider) => match fetch_token_metadata(&provider, contract_address, None).await {
                Ok(Some(metadata)) => {
                    result.debug_info = Some(format!(
                        "Tracked new deployment {} (symbol: {}, decimals: {})",
                        to_checksum_address(&contract_address),
                        metadata.symbol,
                        metadata.decimals
                    ));
                }
                Ok(None) => {
                    result.debug_info = Some(format!(
                        "Tracked new deployment {} (non-ERC20 bytecode)",
                        to_checksum_address(&contract_address)
                    ));
                }
                Err(err) => {
                    result.debug_info = Some(format!(
                        "Tracked new deployment but metadata lookup failed: {}",
                        err
                    ));
                }
            },
            Err(err) => {
                result.debug_info = Some(format!(
                    "Tracked new deployment; failed to build query provider: {}",
                    err
                ));
            }
        }

        info!(
            "📦 Recorded contract creation for creator {}",
            match &request.category {
                TransactionCategory::ContractCreation { deployer, .. } => deployer,
                _ => "unknown",
            }
        );

        result
    }
}

/// Derive a contract address from sender + nonce (Create opcode rules).
fn derive_contract_address(sender: Address, nonce: u64) -> Option<Address> {
    let mut stream = RlpStream::new_list(2);
    stream.append(&sender.as_slice());
    stream.append(&nonce);
    let hash = keccak256(stream.out());
    // Last 20 bytes
    Some(Address::from_slice(&hash[12..]))
}
