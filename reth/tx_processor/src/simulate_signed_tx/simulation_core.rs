use anyhow::{Result, anyhow};
use revm_primitives::{
    Bytes as RevmBytes, Log as RevmLog
};
use revm_context::{BlockEnv, CfgEnv, TxEnv, Context as RevmContext, Journal, result::{HaltReason, SuccessReason}};
use revm_context::result::ExecutionResult;
use revm::database::{CacheDB, WrapDatabaseAsync, AlloyDB};
use revm_context::Evm;
use revm_inspector::InspectEvm;
use revm_handler::{EthPrecompiles, instructions::EthInstructions};
use revm_interpreter::interpreter::EthInterpreter;
use alloy_network::Ethereum as AlloyEthereum;
use alloy_provider::DynProvider as AlloyDynProvider;
use std::sync::Arc;
use tracing::error;

// Import CallTracer and InternalTransfer types
use crate::simulate_signed_tx::call_tracer::{CallTracer, InternalTransfer};

// Re-exporting from lib.rs, so these specific imports might not be needed here if already in scope
// use revm::database::{CacheDB, WrapDatabaseAsync}; 
// use revm_primitives::{Address as RevmAddress, U256 as RevmU256, SpecId as RevmSpecId, KECCAK_EMPTY as REVM_KECCAK_EMPTY};

// Assuming SimCacheDB is brought into scope by the crate's lib.rs or a prelude
// pub use crate::SimCacheDB; // Make sure SimCacheDB is correctly referenced if not defined here

// Define SimCacheDB alias directly in this file
pub type SimCacheDB = CacheDB<WrapDatabaseAsync<AlloyDB<AlloyEthereum, Arc<AlloyDynProvider>>>>;

#[derive(Debug)]
pub struct SimulationOutput {
    pub result_type: ExecutionResultType,
    pub gas_used: u64,
    pub gas_refunded: u64,
    pub logs: Vec<RevmLog>,
    pub output_data: RevmBytes,
    pub internal_transfers: Vec<InternalTransfer>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionResultType {
    Success(SuccessReason),
    Revert,
    Halt(HaltReason),
}

pub fn simulate_transaction(
    tx_env: TxEnv,       
    block_env: BlockEnv, 
    cfg_env: CfgEnv,     
    cache_db: SimCacheDB,
) -> Result<(SimulationOutput, SimCacheDB)> {
    let spec_id = cfg_env.spec.clone();

    // Create CallTracer to capture internal transfers
    let call_tracer = CallTracer::new();
    let inspector_for_inspect = call_tracer.clone();

    // Build EVM context with the specified hardfork specification
    let mut ctx: RevmContext<BlockEnv, TxEnv, CfgEnv, _, Journal<_>, ()> = 
        RevmContext::new(cache_db, spec_id);
    ctx.cfg = cfg_env.clone();
    ctx.block = block_env.clone();

    // Create EVM with the inspector using the correct REVM v25 pattern
    let mut evm: Evm<_, CallTracer, EthInstructions<EthInterpreter, _>, EthPrecompiles> = 
        Evm::new_with_inspector(ctx, call_tracer, Default::default(), Default::default());

    // Execute transaction with inspector
    match InspectEvm::inspect(&mut evm, tx_env.clone(), inspector_for_inspect) {
        Ok(execution_result) => {
            let (result_type, logs, output_data, gas_used, gas_refunded) = match execution_result {
                ExecutionResult::Success { reason, gas_used, gas_refunded, logs, output } => {
                    (ExecutionResultType::Success(reason), logs, output.into_data(), gas_used, gas_refunded)
                }
                ExecutionResult::Revert { gas_used, output } => {
                    (ExecutionResultType::Revert, Vec::new(), output, gas_used, 0)
                }
                ExecutionResult::Halt { reason, gas_used } => {
                    (ExecutionResultType::Halt(reason), Vec::new(), RevmBytes::new(), gas_used, 0)
                }
            };

            // Extract internal transfers from the CallTracer
            let internal_transfers = evm.inspector.get_internal_transfers();

            let sim_output = SimulationOutput {
                result_type,
                gas_used,
                gas_refunded,
                logs,
                output_data,
                internal_transfers,
            };
            
            Ok((sim_output, evm.ctx.journaled_state.database))
        }
        Err(e) => {
            // Only log truly unexpected errors, not normal mempool behavior
            let error_msg = format!("{:?}", e);
            if error_msg.contains("LackOfFundForMaxFee") || 
               error_msg.contains("InsufficientFunds") || 
               error_msg.contains("lack of funds") ||
               error_msg.contains("lack of fund") ||
               error_msg.contains("transaction validation error") {
                // These are normal mempool behavior - don't log them at all
                tracing::debug!("REVM simulation failed due to insufficient funds (normal mempool behavior)");
            } else if error_msg.contains("NonceTooHigh") || 
                      error_msg.contains("NonceTooLow") ||
                      error_msg.contains("nonce") {
                // Nonce issues are also normal mempool behavior
                tracing::debug!("REVM simulation failed due to nonce issue (normal mempool behavior)");
            } else {
                // Only log truly unexpected REVM errors
                error!("Unexpected REVM error during inspect: {:?}", e);
            }
            Err(anyhow!("REVM error during transaction inspection: {}", e))
        }
    }
} 