use anyhow::{Result, anyhow};
use revm_primitives::{
    Bytes as RevmBytes, Log as RevmLog
};
use revm_context::{BlockEnv, CfgEnv, TxEnv, Context as RevmContext, Journal, result::{HaltReason, SuccessReason}};
use revm_context::result::ExecutionResult;
use revm::handler::{ExecuteCommitEvm, MainBuilder};
use revm::database::{CacheDB, WrapDatabaseAsync, AlloyDB};
use alloy_network::Ethereum as AlloyEthereum;
use alloy_provider::DynProvider as AlloyDynProvider;
use std::sync::Arc;
use tracing::error;

// Re-exporting from lib.rs, so these specific imports might not be needed here if already in scope
// use revm::database::{CacheDB, WrapDatabaseAsync}; 
// use revm_primitives::{Address as RevmAddress, U256 as RevmU256, SpecId as RevmSpecId, KECCAK_EMPTY as REVM_KECCAK_EMPTY};

// Assuming SimCacheDB is brought into scope by the crate's lib.rs or a prelude
// pub use crate::SimCacheDB; // Make sure SimCacheDB is correctly referenced if not defined here

// Define SimCacheDB alias directly in this file
pub type SimCacheDB = CacheDB<WrapDatabaseAsync<AlloyDB<AlloyEthereum, Arc<AlloyDynProvider>>>>;

pub struct SimulationOutput {
    pub result_type: ExecutionResultType,
    pub gas_used: u64,
    pub gas_refunded: u64,
    pub logs: Vec<RevmLog>,
    pub output_data: RevmBytes,
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

    let mut evm_context: RevmContext<
        BlockEnv, 
        TxEnv, 
        CfgEnv, 
        SimCacheDB, 
        Journal<SimCacheDB>,
        ()
    > = RevmContext::new(cache_db, spec_id);

    evm_context.cfg = cfg_env;     
    evm_context.block = block_env; 
    evm_context.tx = tx_env.clone(); 

    let mut mainnet_evm = evm_context.build_mainnet();

    match mainnet_evm.transact_commit(tx_env) { 
        Ok(transaction_outcome) => {
            let (result_type, logs, output_data, gas_used, gas_refunded) = match transaction_outcome {
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

            let sim_output = SimulationOutput {
                result_type,
                gas_used,
                gas_refunded,
                logs,
                output_data,
            };
            Ok((sim_output, mainnet_evm.ctx.journaled_state.database))
        }
        Err(e) => {
            error!("REVM critical error during transact_commit: {:?}", e);
            Err(anyhow!("REVM critical error during transact_commit: {}", e))
        }
    }
}

// We might also want a struct to hold the EVM context if we do multiple transactions
// or want to manage the DB state more explicitly across calls.
// For now, a single function taking and returning the DB might be simpler for the example. 