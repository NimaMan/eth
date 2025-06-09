use std::time::Instant;
use std::sync::Arc;
use eyre::Result;
use std::collections::HashMap;

use ethers_providers::{Provider, Http, Middleware};
use ethers_core::types::{H256, U256 as EthersU256, Transaction, TransactionReceipt, Block, TxHash};

use revm_primitives::{
    B256, Address, U256, Bytes, hardfork::SpecId,
};
use revm_context::{
    TxEnv, BlockEnv, CfgEnv, TransactTo,
    result::{ExecutionResult, HaltReason, SuccessReason},
};

use crate::{
    simulate_transaction, SimCacheDB, ExecutionResultType,
    conversions::{ethers_to_revm_address, ethers_to_revm_u256, h256_to_b256},
    call_tracer::{CallTracer, InternalTransfer as TracerTransfer},
};
use alloy_eips;
use super::types::{ProcessedTransaction, InternalTransfer, ProcessingMetrics, StateChange};

pub struct ProcessorConfig {
    pub rpc_url: String,
    pub enable_call_tracing: bool,
    pub enable_state_diff: bool,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            rpc_url: "http://127.0.0.1:8545".to_string(),
            enable_call_tracing: true,
            enable_state_diff: true,
        }
    }
}

pub struct RevmTxProcessor {
    provider: Arc<Provider<Http>>,
    config: ProcessorConfig,
}

impl RevmTxProcessor {
    pub async fn new(config: ProcessorConfig) -> Result<Self> {
        let provider = Provider::<Http>::try_from(&config.rpc_url)?;
        Ok(Self {
            provider: Arc::new(provider),
            config,
        })
    }

    /// Process a transaction by hash with full REVM simulation
    pub async fn process_transaction(&self, tx_hash: H256) -> Result<ProcessedTransaction> {
        let total_start = Instant::now();
        
        // Fetch transaction data
        let fetch_start = Instant::now();
        let tx = self.provider.get_transaction(tx_hash).await?
            .ok_or_else(|| eyre::eyre!("Transaction not found"))?;
        let receipt = self.provider.get_transaction_receipt(tx_hash).await?
            .ok_or_else(|| eyre::eyre!("Receipt not found"))?;
        
        // Get the block with full transactions
        let block_hash = receipt.block_hash.unwrap();
        let block = self.provider.get_block_with_txs(block_hash).await?
            .ok_or_else(|| eyre::eyre!("Block not found"))?;
        
        let fetch_time = fetch_start.elapsed().as_secs_f64() * 1000.0;
        
        // Simulate transaction
        let sim_start = Instant::now();
        let (sim_result, internal_transfers) = self.simulate_tx(&tx, &receipt, &block).await?;
        let sim_time = sim_start.elapsed().as_secs_f64() * 1000.0;
        
        let total_time = total_start.elapsed().as_secs_f64() * 1000.0;
        
        // Convert internal transfers
        let internal_transfers = internal_transfers.into_iter().map(|t| InternalTransfer {
            from: t.from,
            to: t.to,
            value: t.value,
            call_type: format!("{:?}", t.call_type),
        }).collect();
        
        Ok(ProcessedTransaction {
            hash: h256_to_b256(tx_hash),
            block_number: block.number.unwrap().as_u64(),
            from: ethers_to_revm_address(tx.from),
            to: tx.to.map(ethers_to_revm_address),
            value: ethers_to_revm_u256(tx.value),
            gas_used: sim_result.gas_used,
            success: matches!(sim_result.result_type, ExecutionResultType::Success(_)),
            internal_transfers,
            state_changes: Default::default(), // TODO: Extract from simulation
            logs: vec![], // TODO: Convert sim_result.logs
            metrics: ProcessingMetrics {
                fetch_time_ms: fetch_time,
                simulation_time_ms: sim_time,
                total_time_ms: total_time,
            },
        })
    }

    /// Simulate transaction using REVM directly
    async fn simulate_tx(
        &self,
        tx: &Transaction,
        _receipt: &TransactionReceipt,
        block: &Block<Transaction>,
    ) -> Result<(crate::SimulationOutput, Vec<TracerTransfer>)> {
        // Prepare transaction environment
        let mut tx_env = TxEnv::default();
        tx_env.caller = ethers_to_revm_address(tx.from);
        tx_env.gas_limit = tx.gas.as_u64();
        tx_env.gas_price = ethers_to_revm_u256(tx.gas_price.unwrap_or_default()).to::<u128>();
        tx_env.kind = match tx.to {
            Some(to) => TransactTo::Call(ethers_to_revm_address(to)),
            None => TransactTo::Create,
        };
        tx_env.value = ethers_to_revm_u256(tx.value);
        tx_env.data = Bytes(tx.input.0.clone());
        tx_env.nonce = tx.nonce.as_u64();
        tx_env.chain_id = Some(1);

        // Block environment
        let mut block_env = BlockEnv::default();
        block_env.number = U256::from(block.number.unwrap().as_u64());
        block_env.timestamp = ethers_to_revm_u256(block.timestamp);
        block_env.basefee = ethers_to_revm_u256(block.base_fee_per_gas.unwrap_or_default()).to::<u64>();

        // Config
        let mut cfg_env = CfgEnv::default();
        cfg_env.chain_id = 1;
        cfg_env.spec = SpecId::SHANGHAI;

        // Setup database
        use alloy_provider::ProviderBuilder;
        use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync};
        
        let alloy_provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(&self.config.rpc_url.parse()?);

        let block_id = alloy_eips::BlockId::from(block.number.unwrap().as_u64());
        let alloy_db = AlloyDB::new(Arc::new(alloy_provider), block_id.into())
            .map_err(|e| eyre::eyre!("Failed to create AlloyDB: {}", e))?;
        
        let db_ref = WrapDatabaseAsync::new(alloy_db);
        let cache_db = CacheDB::new(db_ref);

        // Simulate the transaction
        let (sim_output, _final_db) = simulate_transaction(
            tx_env,
            block_env,
            cfg_env,
            cache_db,
        )?;

        // Extract internal transfers if call tracing is enabled
        let internal_transfers = if self.config.enable_call_tracing {
            // TODO: Integrate with CallTracer
            vec![]
        } else {
            vec![]
        };

        Ok((sim_output, internal_transfers))
    }
}