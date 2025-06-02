
use crate::mempool_processor::types::TransactionView;
use revm_tx_simulator_lib::{
    simulation_core::{simulate_transaction, SimCacheDB, ExecutionResultType},
    state_diff_utils::{generate_calculated_account_changes, CalculatedAccountChanges},
};
use crate::tx_simulator::conversions::transaction_view_to_revm_tx_env;

use revm_context::{CfgEnv as RevmCfgEnv_ctx, BlockEnv as RevmBlockEnv_ctx, TransactTo as RevmTransactTo_ctx};
use revm_primitives::{Address as RevmAddress, U256 as RevmU256, hardfork::SpecId as RevmSpecId};
use revm::database::{CacheDB, WrapDatabaseAsync, AlloyDB};
use alloy_provider::{DynProvider as AlloyDynProvider, ProviderBuilder, Provider as AlloyProviderTrait};
use alloy_network::Ethereum as AlloyEthereum;
use alloy_eips::BlockId as AlloyBlockId;

use eyre::{Result, WrapErr};
use tracing::{debug, info, warn};
use std::collections::HashMap;
use std::sync::Arc;

/// Transaction simulator that uses REVM to get detailed state changes.
pub struct TransactionSimulator {
    alloy_provider: Arc<AlloyDynProvider<AlloyEthereum>>,
    cfg_env: RevmCfgEnv_ctx, 
}

impl TransactionSimulator {
    /// Create a new transaction simulator.
    pub async fn new(
        rpc_url: &str,
        chain_id: u64,
        spec_id: RevmSpecId,
    ) -> Result<Self> {
        let provider_instance = ProviderBuilder::new()
            .connect(rpc_url)
            .await
            .wrap_err("Failed to connect to Alloy Provider")?;
        let alloy_provider = Arc::new(provider_instance.erased());

        let mut cfg_env = RevmCfgEnv_ctx::default();
        cfg_env.chain_id = chain_id;
        cfg_env.spec = spec_id;

        Ok(Self {
            alloy_provider,
            cfg_env,
        })
    }
    
    /// Process a mempool transaction and get its detailed state changes.
    /// BlockEnv should represent the context in which the transaction is to be simulated (e.g., pending block).
    pub async fn process_transaction(
        &self, 
        tx_view: &TransactionView,
        block_env: &RevmBlockEnv_ctx,
    ) -> Result<Option<HashMap<RevmAddress, CalculatedAccountChanges>>> {
        
        info!("Preparing to simulate transaction hash: {:?}", hex::encode(&tx_view.hash));

        let revm_tx_env = match transaction_view_to_revm_tx_env(tx_view, self.cfg_env.chain_id) {
            Ok(env) => env,
            Err(e) => {
                warn!("Failed to convert TransactionView to RevmTxEnv for tx {:?}: {}. Skipping simulation.", hex::encode(&tx_view.hash), e);
                return Ok(None);
            }
        };
        
        debug!("Converted TxEnv: {:?}", revm_tx_env);

        let current_block_number_u256 = block_env.number;
        let fork_block_number_u256 = if current_block_number_u256 > RevmU256::ZERO {
            current_block_number_u256 - RevmU256::from(1)
        } else {
            RevmU256::ZERO
        };

        let fork_block_id = if fork_block_number_u256 == RevmU256::ZERO {
            AlloyBlockId::latest()
        } else {
            AlloyBlockId::from(fork_block_number_u256.into_limbs()[0])
        };

        let alloy_db = AlloyDB::new(self.alloy_provider.clone(), fork_block_id);
        let wrapped_db = WrapDatabaseAsync::new(alloy_db);
        let cache_db: SimCacheDB = CacheDB::new(wrapped_db.expect("Database wrapping failed unexpectedly"));

        let mut initial_eth_balances = HashMap::new();
        let caller_address = revm_tx_env.caller;
        match self.alloy_provider.get_balance(caller_address.into()).block_id(fork_block_id).await {
            Ok(bal) => {
                initial_eth_balances.insert(caller_address, bal);
            }
            Err(e) => warn!("Failed to get initial balance for caller {}: {}", caller_address, e),
        }
        if let RevmTransactTo_ctx::Call(to_address) = revm_tx_env.kind {
            match self.alloy_provider.get_balance(to_address.into()).block_id(fork_block_id).await {
                Ok(bal) => {
                    initial_eth_balances.insert(to_address, bal);
                }
                Err(e) => warn!("Failed to get initial balance for receiver {}: {}", to_address, e),
            }
        }
        
        match simulate_transaction(
            revm_tx_env.clone(),
            block_env.clone(),
            self.cfg_env.clone(),
            cache_db,
        ) {
            Ok((sim_output, final_db_state)) => {
                info!("REVM simulation successful for tx {:?}. Result: {:?}, Gas: {}", hex::encode(&tx_view.hash), sim_output.result_type, sim_output.gas_used);

                if matches!(sim_output.result_type, ExecutionResultType::Success(_)) {
                    match generate_calculated_account_changes(
                        &final_db_state,
                        &initial_eth_balances,
                        &sim_output.logs,
                        &revm_tx_env,
                        &block_env,
                        sim_output.gas_used,
                        self.alloy_provider.clone(),
                        fork_block_id,
                    ).await {
                        Ok(changes) => {
                            if changes.is_empty() {
                                info!("Simulation for tx {:?} resulted in no calculated state changes.", hex::encode(&tx_view.hash));
                                Ok(None)
                            } else {
                                info!("Successfully generated state changes for tx {:?}, {} accounts affected.", hex::encode(&tx_view.hash), changes.len());
                                Ok(Some(changes))
                            }
                        },
                        Err(e) => {
                            warn!("Failed to generate calculated state changes for tx {:?}: {}", hex::encode(&tx_view.hash), e);
                            Err(eyre::eyre!(e).wrap_err("Failed to generate calculated state changes"))
                        }
                    }
                } else {
                    info!("Transaction {:?} did not succeed (Reverted or Halted). No state changes to calculate beyond gas.", hex::encode(&tx_view.hash));
                    Ok(None)
                }
            },
            Err(e) => {
                warn!("REVM simulation library failed for tx {:?}: {}", hex::encode(&tx_view.hash), e);
                Err(eyre::eyre!(e).wrap_err(format!(
                    "Core REVM simulation failed for transaction hash: {:?}",
                    hex::encode(&tx_view.hash)
                )))
            }
        }
    }
} 