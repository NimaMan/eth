use std::time::Instant;
use std::sync::Arc;
use eyre::Result;

use ethers_providers::{Provider, Http, Middleware};
use ethers_core::types::{H256, U256 as EthersU256, Transaction, TransactionReceipt, Block};

use revm::{Evm, db::CacheDB};
use revm_primitives::{
    B256, Address, U256, Bytes, SpecId,
    ExecutionResult, HaltReason, SuccessReason,
};
use revm_context::{
    TxEnv, BlockEnv, CfgEnv, TransactTo,
};

use crate::{
    simulate_transaction, SimulationOutput,
    conversions::{ethers_to_revm_address, ethers_to_revm_u256, h256_to_b256},
    call_tracer::CallTracer,
};
use super::types::{ProcessedTransaction, InternalTransfer, ProcessingMetrics};

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
        let block = self.provider.get_block(receipt.block_number.unwrap()).await?
            .ok_or_else(|| eyre::eyre!("Block not found"))?;
        let fetch_time = fetch_start.elapsed().as_secs_f64() * 1000.0;
        
        // Simulate transaction
        let sim_start = Instant::now();
        let result = self.simulate_tx(&tx, &receipt, &block).await?;
        let sim_time = sim_start.elapsed().as_secs_f64() * 1000.0;
        
        let total_time = total_start.elapsed().as_secs_f64() * 1000.0;
        
        Ok(ProcessedTransaction {
            hash: h256_to_b256(tx_hash),
            block_number: block.number.unwrap().as_u64(),
            from: ethers_to_revm_address(tx.from),
            to: tx.to.map(ethers_to_revm_address),
            value: ethers_to_revm_u256(tx.value),
            gas_used: receipt.gas_used.unwrap().as_u64(),
            success: receipt.status.unwrap().as_u64() == 1,
            internal_transfers: result.internal_transfers,
            state_changes: Default::default(), // TODO: Extract from simulation
            logs: vec![], // TODO: Extract from receipt
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
        receipt: &TransactionReceipt,
        block: &Block<Transaction>,
    ) -> Result<SimulationResult> {
        // Prepare transaction environment
        let mut tx_env = TxEnv::default();
        tx_env.caller = ethers_to_revm_address(tx.from);
        tx_env.gas_limit = tx.gas.as_u64();
        tx_env.gas_price = ethers_to_revm_u256(tx.gas_price.unwrap_or_default()).to::<u128>();
        tx_env.transact_to = match tx.to {
            Some(to) => TransactTo::Call(ethers_to_revm_address(to)),
            None => TransactTo::Create,
        };
        tx_env.value = ethers_to_revm_u256(tx.value);
        tx_env.data = Bytes(tx.input.0.clone());
        tx_env.nonce = Some(tx.nonce.as_u64());
        tx_env.chain_id = Some(1);

        // Block environment
        let mut block_env = BlockEnv::default();
        block_env.number = U256::from(block.number.unwrap().as_u64());
        block_env.timestamp = ethers_to_revm_u256(block.timestamp);
        block_env.basefee = ethers_to_revm_u256(block.base_fee_per_gas.unwrap_or_default());

        // Config
        let mut cfg_env = CfgEnv::default();
        cfg_env.chain_id = 1;
        cfg_env.spec = SpecId::SHANGHAI;

        // TODO: Use our existing simulate_transaction function
        // For now, return empty result
        Ok(SimulationResult {
            internal_transfers: vec![],
        })
    }

    /// Process multiple transactions in parallel
    pub async fn process_transactions(&self, tx_hashes: Vec<H256>) -> Vec<Result<ProcessedTransaction>> {
        let mut handles = vec![];
        
        for hash in tx_hashes {
            let provider = self.provider.clone();
            let handle = tokio::spawn(async move {
                // TODO: Implement parallel processing
                Err(eyre::eyre!("Parallel processing not yet implemented"))
            });
            handles.push(handle);
        }
        
        let mut results = vec![];
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(eyre::eyre!("Task failed: {}", e))),
            }
        }
        
        results
    }
}

struct SimulationResult {
    internal_transfers: Vec<InternalTransfer>,
}