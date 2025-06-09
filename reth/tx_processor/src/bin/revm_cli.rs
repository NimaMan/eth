use anyhow::Result;
use clap::{App, Arg};
use revm_tx_simulator_lib::{
    simulate_transaction, SimulationOutput, ExecutionResultType,
    conversions::{ethers_to_revm_u256, ethers_to_revm_address},
    SimCacheDB,
};
use ethers_providers::{Provider, Http, Middleware};
use ethers_core::types::{H256, Address, U256};
use serde::{Serialize, Deserialize};
use std::str::FromStr;

// REVM imports
use revm_primitives::{
    Address as RevmAddress, Bytes as RevmBytes, 
    B256 as RevmB256, 
    hardfork::SpecId as RevmSpecId_primitive,
    U256 as RevmU256,
};
use revm_context::{
    BlockEnv as RevmBlockEnv_ctx, CfgEnv as RevmCfgEnv_ctx, TxEnv as RevmTxEnv_ctx, 
    TransactTo as RevmTransactTo_ctx,
};
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync}; 

// Alloy Imports
use alloy_eips::BlockId as AlloyBlockId;
use alloy_network::Ethereum as AlloyEthereum;
use alloy_provider::{ProviderBuilder, DynProvider as AlloyDynProvider};

#[derive(Serialize, Deserialize, Debug)]
struct SimulationInput {
    pub tx_hash: String,
    pub rpc_url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SimulationResponse {
    pub success: bool,
    pub gas_used: u64,
    pub gas_refunded: u64,
    pub output: String,
    pub result_type: String,
    pub internal_transfers: Vec<SerializableInternalTransfer>,
    pub state_changes: Vec<SerializableAccountSummary>,
    pub error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SerializableInternalTransfer {
    pub from: String,
    pub to: String,
    pub value: String,
    pub depth: u32,
}

#[derive(Serialize, Deserialize, Debug)]  
struct SerializableAccountSummary {
    pub address: String,
    pub balance_change: String,
    pub nonce_change: u64,
    pub modified: bool,
}

impl From<&InternalTransfer> for SerializableInternalTransfer {
    fn from(transfer: &InternalTransfer) -> Self {
        Self {
            from: format!("{:?}", transfer.from),
            to: format!("{:?}", transfer.to),
            value: transfer.value.to_string(),
            depth: transfer.depth,
        }
    }
}

impl From<&AccountStateSummary> for SerializableAccountSummary {
    fn from(summary: &AccountStateSummary) -> Self {
        Self {
            address: format!("{:?}", summary.address),
            balance_change: summary.balance_change.to_string(),
            nonce_change: summary.nonce_change,
            modified: summary.modified,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = App::new("REVM Transaction Simulator CLI")
        .version("1.0")
        .author("Your REVM Simulator")
        .about("Simulates Ethereum transactions using REVM")
        .arg(
            Arg::with_name("tx_hash")
                .long("tx-hash")
                .value_name("HASH")
                .help("Transaction hash to simulate")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("rpc_url")
                .long("rpc-url")
                .value_name("URL")
                .help("RPC URL")
                .default_value("http://127.0.0.1:8545")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("json_input")
                .long("json")
                .help("Expect JSON input from stdin")
                .takes_value(false),
        )
        .get_matches();

    let response = if matches.is_present("json_input") {
        // Read JSON from stdin
        let mut input = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
        let sim_input: SimulationInput = serde_json::from_str(&input)?;
        
        simulate_tx(&sim_input.tx_hash, &sim_input.rpc_url).await
    } else {
        // Use command line arguments
        let tx_hash = matches.value_of("tx_hash").unwrap();
        let rpc_url = matches.value_of("rpc_url").unwrap();
        
        simulate_tx(tx_hash, rpc_url).await
    };

    // Output JSON response
    println!("{}", serde_json::to_string_pretty(&response)?);
    
    if !response.success {
        std::process::exit(1);
    }
    
    Ok(())
}

async fn simulate_tx(tx_hash_str: &str, rpc_url: &str) -> SimulationResponse {
    match simulate_transaction_internal(tx_hash_str, rpc_url).await {
        Ok((sim_output, internal_transfers, state_changes)) => SimulationResponse {
            success: matches!(sim_output.result_type, ExecutionResultType::Success(_)),
            gas_used: sim_output.gas_used,
            gas_refunded: sim_output.gas_refunded,
            output: hex::encode(&sim_output.output_data),
            result_type: format!("{:?}", sim_output.result_type),
            internal_transfers: internal_transfers.iter().map(|t| t.into()).collect(),
            state_changes: state_changes.iter().map(|s| s.into()).collect(),
            error: None,
        },
        Err(e) => SimulationResponse {
            success: false,
            gas_used: 0,
            gas_refunded: 0,
            output: String::new(),
            result_type: "Error".to_string(),
            internal_transfers: Vec::new(),
            state_changes: Vec::new(),
            error: Some(e.to_string()),
        }
    }
}

async fn simulate_transaction_internal(
    tx_hash_str: &str,
    rpc_url: &str,
) -> Result<(SimulationOutput, Vec<InternalTransfer>, Vec<AccountStateSummary>)> {
    // Parse transaction hash
    let tx_hash = H256::from_str(tx_hash_str)?;
    
    // Connect to RPC
    let provider = Provider::<Http>::try_from(rpc_url)?;
    
    // Get transaction and receipt
    let tx = provider.get_transaction(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Transaction not found"))?;
    let receipt = provider.get_transaction_receipt(tx_hash).await?
        .ok_or_else(|| anyhow::anyhow!("Receipt not found"))?;
    
    // Get block for context
    let block_number = receipt.block_number.ok_or_else(|| anyhow::anyhow!("Block number not found"))?;
    let block = provider.get_block(block_number).await?
        .ok_or_else(|| anyhow::anyhow!("Block not found"))?;
    
    // Convert to REVM types
    let tx_env = ethers_tx_to_revm_txenv(&tx)?;
    let block_env = ethers_block_to_revm_blockenv(&block);
    let cfg_env = create_default_cfg_env();
    
    // Create AlloyDB provider for REVM
    let alloy_provider = alloy_provider::ProviderBuilder::new()
        .on_http(rpc_url.parse()?);
    
    // Create cache database 
    let alloy_db = revm::database::AlloyDB::new(alloy_provider, Default::default());
    let cache_db = revm::database::CacheDB::new(alloy_db);
    
    // Run simulation with call tracer for internal transfers
    let mut call_tracer = CallTracer::new();
    
    // This is a simplified version - you would need to integrate the call tracer
    // with your simulation core properly
    let (simulation_output, final_db) = simulate_transaction(tx_env, block_env, cfg_env, cache_db)?;
    
    // Extract internal transfers from call tracer
    let internal_transfers = call_tracer.get_internal_transfers();
    
    // Extract state changes
    let state_changes = extract_final_touched_account_states(&final_db)?;
    
    Ok((simulation_output, internal_transfers, state_changes))
}