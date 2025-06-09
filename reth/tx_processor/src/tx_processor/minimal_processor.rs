use std::time::Instant;
use std::sync::Arc;
use anyhow::Result;

use ethers_providers::{Provider, Http, Middleware};
use ethers_core::types::{H256, Transaction, TransactionReceipt};

use revm_primitives::{Bytes, hardfork::SpecId};
use revm_context::{TxEnv, BlockEnv, CfgEnv, TransactTo};

use crate::{
    simulate_transaction,
    conversions::{ethers_to_revm_address, ethers_to_revm_u256},
};

pub struct MinimalProcessor {
    provider: Arc<Provider<Http>>,
}

#[derive(Debug)]
pub struct MinimalResult {
    pub hash: H256,
    pub gas_used: u64,
    pub success: bool,
    pub fetch_time_ms: f64,
    pub sim_time_ms: f64,
    pub total_time_ms: f64,
}

impl MinimalProcessor {
    pub async fn new(rpc_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        Ok(Self {
            provider: Arc::new(provider),
        })
    }

    pub async fn process_tx_minimal(&self, tx_hash: H256) -> Result<MinimalResult> {
        let total_start = Instant::now();
        
        // Fetch minimal data needed
        let fetch_start = Instant::now();
        let tx = self.provider.get_transaction(tx_hash).await?
            .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
        let receipt = self.provider.get_transaction_receipt(tx_hash).await?
            .ok_or_else(|| anyhow::anyhow!("Receipt not found"))?;
        let block = self.provider.get_block(receipt.block_number.unwrap()).await?
            .ok_or_else(|| anyhow::anyhow!("Block not found"))?;
        let fetch_time = fetch_start.elapsed().as_secs_f64() * 1000.0;
        
        // Simulate with REVM
        let sim_start = Instant::now();
        let gas_used = self.simulate_minimal(&tx, &receipt, block.number.unwrap().as_u64()).await?;
        let sim_time = sim_start.elapsed().as_secs_f64() * 1000.0;
        
        let total_time = total_start.elapsed().as_secs_f64() * 1000.0;
        
        Ok(MinimalResult {
            hash: tx_hash,
            gas_used,
            success: receipt.status.unwrap().as_u64() == 1,
            fetch_time_ms: fetch_time,
            sim_time_ms: sim_time,
            total_time_ms: total_time,
        })
    }

    async fn simulate_minimal(
        &self,
        tx: &Transaction,
        _receipt: &TransactionReceipt,
        block_number: u64,
    ) -> Result<u64> {
        // Setup minimal environments
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

        let mut block_env = BlockEnv::default();
        block_env.number = revm_primitives::U256::from(block_number);

        let mut cfg_env = CfgEnv::default();
        cfg_env.chain_id = 1;
        cfg_env.spec = SpecId::SHANGHAI;

        // For now, just return a dummy gas value to test speed
        // TODO: Hook up actual simulation
        Ok(500_000)
    }
}