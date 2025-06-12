/// Simple binary to process a single transaction
/// This avoids all dependency conflicts by being built within revm_tx_simulator
use ethers_core::types::{H256, U256};
use ethers_providers::{Provider, Http, Middleware};
use std::str::FromStr;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use revm_tx_simulator_lib::{
    simulate_transaction, ExecutionResultType, SimCacheDB,
    conversions::{ethers_to_revm_u256, ethers_to_revm_address},
    spec_id_from_block_number,
};
use revm_primitives::{
    Bytes as RevmBytes, hardfork::SpecId,
};
use revm_context::{
    TxEnv as RevmTxEnv, BlockEnv as RevmBlockEnv, CfgEnv as RevmCfgEnv,
    TransactTo as RevmTransactTo,
};
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync};
use alloy_provider::ProviderBuilder;
use alloy_eips::BlockId;

#[derive(Serialize, Deserialize)]
struct ProcessedTx {
    hash: String,
    from: String,
    to: Option<String>,
    value: String,
    gas_used: u64,
    success: bool,
    internal_transfers: Vec<InternalTransfer>,
}

#[derive(Serialize, Deserialize)]
struct InternalTransfer {
    from: String,
    to: String,
    value: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <tx_hash>", args[0]);
        std::process::exit(1);
    }
    
    let tx_hash = H256::from_str(&args[1])?;
    let rpc_url = "http://127.0.0.1:8545";
    
    // Fetch transaction data
    let provider = Provider::<Http>::try_from(rpc_url)?;
    let tx = provider.get_transaction(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
    let receipt = provider.get_transaction_receipt(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Receipt not found"))?;
    let block = provider.get_block(receipt.block_number.unwrap()).await?
        .ok_or_else(|| anyhow::anyhow!("Block not found"))?;
    
    // Prepare environments
    let mut tx_env = RevmTxEnv::default();
    tx_env.caller = ethers_to_revm_address(tx.from);
    tx_env.gas_limit = tx.gas.as_u64();
    tx_env.gas_price = ethers_to_revm_u256(tx.gas_price.unwrap_or_default()).to::<u128>();
    tx_env.kind = match tx.to {
        Some(to) => RevmTransactTo::Call(ethers_to_revm_address(to)),
        None => RevmTransactTo::Create,
    };
    tx_env.value = ethers_to_revm_u256(tx.value);
    tx_env.data = RevmBytes(tx.input.0.clone());
    tx_env.nonce = tx.nonce.as_u64();
    tx_env.chain_id = Some(1);
    if let Some(access_list) = &tx.access_list {
        let items: Vec<_> = access_list.0.iter().map(|item| {
            revm_context::transaction::AccessListItem {
                address: ethers_to_revm_address(item.address),
                storage_keys: item.storage_keys.iter().map(|key| revm_primitives::B256::from_slice(key.as_bytes())).collect(),
            }
        }).collect();
        tx_env.access_list = revm_context::transaction::AccessList(items);
    }
    tx_env.gas_priority_fee = tx.max_priority_fee_per_gas
        .map(|fee| ethers_to_revm_u256(fee).to::<u128>());
    
    let mut block_env = RevmBlockEnv::default();
    block_env.number = ethers_to_revm_u256(U256::from(block.number.unwrap().as_u64()));
    block_env.timestamp = ethers_to_revm_u256(block.timestamp);
    block_env.basefee = ethers_to_revm_u256(block.base_fee_per_gas.unwrap_or_default()).to::<u64>();
    block_env.difficulty = ethers_to_revm_u256(block.difficulty);
    block_env.beneficiary = ethers_to_revm_address(block.author.unwrap_or_default());
    block_env.gas_limit = ethers_to_revm_u256(block.gas_limit).to::<u64>();
    block_env.prevrandao = block.mix_hash.map(|h| revm_primitives::B256::from_slice(h.as_bytes()));
    
    let mut cfg_env = RevmCfgEnv::default();
    cfg_env.chain_id = 1;
    
    // Automatically determine the correct spec based on block number
    let block_num = block.number.unwrap().as_u64();
    cfg_env.spec = spec_id_from_block_number(block_num);
    
    eprintln!("Debug: Block number: {}, Using spec: {:?}", block_num, cfg_env.spec);
    
    // Setup database - Use DynProvider like in the example
    use alloy_provider::{DynProvider, Provider as AlloyProviderTrait};
    use alloy_network::Ethereum;
    
    let alloy_provider = ProviderBuilder::new().connect_http(rpc_url.parse()?);
    let fork_block = BlockId::from(block.number.unwrap().as_u64() - 1);
    // Convert to dyn provider
    let alloy_provider_dyn: Arc<DynProvider<Ethereum>> = Arc::new(DynProvider::new(alloy_provider));
    let alloy_db = AlloyDB::<Ethereum, Arc<DynProvider<Ethereum>>>::new(
        alloy_provider_dyn.clone(),
        fork_block
    );
    let cache_db: SimCacheDB = CacheDB::new(WrapDatabaseAsync::new(alloy_db).unwrap());
    
    // SIMULATE!
    let (sim_output, _final_db) = simulate_transaction(
        tx_env.clone(),
        block_env.clone(),
        cfg_env,
        cache_db
    )?;
    
    // Debug: Print the actual result type
    eprintln!("Debug: Simulation result type: {:?}", sim_output.result_type);
    eprintln!("Debug: Gas used: {}", sim_output.gas_used);
    eprintln!("Debug: Output data length: {}", sim_output.output_data.len());
    
    // Create result
    let result = ProcessedTx {
        hash: format!("{:?}", tx_hash),
        from: format!("{:?}", tx.from),
        to: tx.to.map(|a| format!("{:?}", a)),
        value: tx.value.to_string(),
        gas_used: sim_output.gas_used,
        success: matches!(sim_output.result_type, ExecutionResultType::Success(_)),
        internal_transfers: vec![], // TODO: Extract from CallTracer
    };
    
    // Output as JSON
    println!("{}", serde_json::to_string(&result)?);
    
    Ok(())
}