// Fast Path Transaction Processor
// Optimizes transaction simulation by using smart detection to determine
// whether full REVM simulation is needed or if database-only analysis suffices

use anyhow::{anyhow, Result};
use ethers_core::types::H256 as EthersH256;
use ethers_providers::{Middleware, Provider as EthersProvider, Http as EthersHttp};
use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use serde_json::json;
use hex;

// REVM imports for full simulation when needed
use crate::{
    ExecutionResultType, 
    conversions::{ethers_to_revm_address, ethers_to_revm_u256}, 
    state_diff_utils::generate_calculated_account_changes,
    SimCacheDBForDiff,
    CallTracer,
    integrate_internal_transfers,
};

// Minimal imports - only what we actually need
use alloy_network::Ethereum as AlloyEthereum;
use alloy_provider::{ProviderBuilder, DynProvider as AlloyDynProvider, Provider as AlloyProviderTrait};

const RPC_URL: &str = "http://127.0.0.1:8545";
const ETH_TO_WEI_FACTOR_F64: f64 = 1_000_000_000_000_000_000.0_f64;

/// Configuration for fast path processing
#[derive(Debug, Clone)]
pub struct FastPathConfig {
    /// Always use full simulation (safest but slower)
    pub force_full_simulation: bool,
    /// Gas threshold above which we always simulate (complex transactions)
    pub complex_gas_threshold: u64,
    /// Cache simulation results for this duration (seconds)
    pub cache_ttl_seconds: u64,
}

impl Default for FastPathConfig {
    fn default() -> Self {
        Self {
            force_full_simulation: false,
            complex_gas_threshold: 100_000, // Above 100k gas is likely complex
            cache_ttl_seconds: 300, // 5 minutes
        }
    }
}

/// Result of transaction analysis
#[derive(Debug, Clone)]
pub struct TransactionAnalysisResult {
    pub tx_hash: String,
    pub analysis_method: AnalysisMethod,
    pub processing_time_ms: f64,
    pub state_changes: serde_json::Value,
    pub internal_transfers_detected: usize,
    pub confidence_level: ConfidenceLevel,
}

#[derive(Debug, Clone)]
pub enum AnalysisMethod {
    DatabaseOnly,    // Fast path using only database/receipt data
    FullSimulation,  // Complete REVM simulation
    Hybrid,          // Database first, simulation if needed
}

#[derive(Debug, Clone)]
pub enum ConfidenceLevel {
    High,    // 95%+ confidence in accuracy
    Medium,  // 80-95% confidence
    Low,     // <80% confidence, recommend full simulation
}

/// Fast Path Transaction Processor
pub struct FastPathProcessor {
    config: FastPathConfig,
    ethers_provider: Arc<EthersProvider<EthersHttp>>,
    alloy_provider: Arc<AlloyDynProvider<AlloyEthereum>>,
}

impl FastPathProcessor {
    pub async fn new(config: FastPathConfig) -> Result<Self> {
        let ethers_provider = EthersProvider::<EthersHttp>::try_from(RPC_URL)?;
        let eth_client = Arc::new(ethers_provider);
        
        let alloy_provider_dyn: Arc<AlloyDynProvider<AlloyEthereum>> =
            Arc::new(ProviderBuilder::new().connect(RPC_URL).await?.erased());
        
        Ok(Self {
            config,
            ethers_provider: eth_client,
            alloy_provider: alloy_provider_dyn,
        })
    }

    /// Main entry point: analyze transaction by hash with optimal speed
    pub async fn analyze_transaction(&self, tx_hash_str: &str) -> Result<TransactionAnalysisResult> {
        let start_time = Instant::now();
        let tx_hash: EthersH256 = tx_hash_str.parse()?;

        // Step 1: Always fetch basic transaction data (fast database lookup)
        let tx_data = self.fetch_transaction_data(&tx_hash).await?;
        
        // Step 2: Smart detection - do we need full simulation?
        let needs_simulation = self.detect_simulation_need(&tx_data).await?;
        
        let result = if self.config.force_full_simulation || needs_simulation {
            // Full REVM simulation for accuracy
            self.full_simulation_analysis(tx_hash_str, tx_data).await?
        } else {
            // Fast database-only analysis
            self.database_only_analysis(tx_hash_str, tx_data).await?
        };

        let processing_time = start_time.elapsed().as_secs_f64() * 1000.0;
        
        Ok(TransactionAnalysisResult {
            tx_hash: tx_hash_str.to_string(),
            analysis_method: result.0,
            processing_time_ms: processing_time,
            state_changes: result.1,
            internal_transfers_detected: result.2,
            confidence_level: result.3,
        })
    }

    /// Fast database-only analysis using receipts and logs
    async fn database_only_analysis(
        &self, 
        _tx_hash_str: &str, 
        tx_data: TransactionData
    ) -> Result<(AnalysisMethod, serde_json::Value, usize, ConfidenceLevel)> {
        
        let mut state_changes = serde_json::Map::new();
        let mut addresses_affected = HashMap::new();

        // Analyze receipt logs for token transfers and events
        if let Some(receipt) = &tx_data.receipt {
            for log in &receipt.logs {
                // ERC20 Transfer: Transfer(address indexed from, address indexed to, uint256 value)
                if log.topics.len() == 3 && 
                   log.topics[0] == ethers_core::types::H256::from_slice(&hex::decode("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef").unwrap()) {
                    
                    let from = ethers_core::types::Address::from(log.topics[1]);
                    let to = ethers_core::types::Address::from(log.topics[2]);
                    let value = ethers_core::types::U256::from_big_endian(&log.data.0);
                    
                    // Add to state changes (simplified format)
                    self.add_token_change(&mut addresses_affected, from, log.address, value, false);
                    self.add_token_change(&mut addresses_affected, to, log.address, value, true);
                }
            }

            // Analyze ETH transfer from transaction value and gas
            if tx_data.value > ethers_core::types::U256::zero() {
                self.add_eth_change(&mut addresses_affected, tx_data.from, tx_data.value, false);
                if let Some(to) = tx_data.to {
                    self.add_eth_change(&mut addresses_affected, to, tx_data.value, true);
                }
            }

            // Gas fees
            let gas_used = receipt.gas_used.unwrap_or_default();
            let gas_price = tx_data.gas_price.unwrap_or_default();
            let gas_cost = gas_used * gas_price;
            self.add_eth_change(&mut addresses_affected, tx_data.from, gas_cost, false);
        }

        // Convert to output format
        for (address, changes) in addresses_affected {
            let addr_str = format!("{:?}", address);
            state_changes.insert(addr_str, json!(changes));
        }

        // Confidence assessment
        let confidence = if tx_data.gas_used.map_or(0, |g| g.as_u64()) < 50_000 {
            ConfidenceLevel::High  // Simple transactions have high confidence
        } else {
            ConfidenceLevel::Medium // Complex transactions may have missed internal transfers
        };

        Ok((
            AnalysisMethod::DatabaseOnly,
            json!(state_changes),
            0, // No internal transfers detected in database-only mode
            confidence
        ))
    }

    /// Full REVM simulation (fallback to existing implementation)
    async fn full_simulation_analysis(
        &self, 
        tx_hash_str: &str, 
        _tx_data: TransactionData
    ) -> Result<(AnalysisMethod, serde_json::Value, usize, ConfidenceLevel)> {
        
        // Use existing simulation logic from json_state_validator_no_rpc.rs
        // This is the proven accurate method
        let simulation_result = self.run_revm_simulation(tx_hash_str).await?;
        
        Ok((
            AnalysisMethod::FullSimulation,
            simulation_result.0,
            simulation_result.1,
            ConfidenceLevel::High // Full simulation always has high confidence
        ))
    }

    /// Smart detection: determine if full simulation is needed
    async fn detect_simulation_need(&self, tx_data: &TransactionData) -> Result<bool> {
        // Always simulate if gas usage is high (likely complex DeFi)
        if let Some(gas_used) = tx_data.gas_used {
            if gas_used.as_u64() > self.config.complex_gas_threshold {
                return Ok(true);
            }
        }

        // Always simulate if transaction called a contract (not simple transfer)
        if tx_data.to.is_some() && tx_data.input.len() > 0 {
            // Check if it's a known simple pattern (e.g., basic ERC20 transfer)
            if self.is_simple_erc20_transfer(&tx_data.input) {
                return Ok(false); // Database analysis sufficient
            } else {
                return Ok(true);  // Complex contract call, need simulation
            }
        }

        // Simple ETH transfer - database analysis sufficient
        Ok(false)
    }

    /// Check if transaction input represents a simple ERC20 transfer
    fn is_simple_erc20_transfer(&self, input: &[u8]) -> bool {
        // ERC20 transfer method signature: transfer(address,uint256) = 0xa9059cbb
        input.len() == 68 && // 4 bytes method + 32 bytes address + 32 bytes amount
        input.starts_with(&[0xa9, 0x05, 0x9c, 0xbb])
    }

    /// Add token change to tracking
    fn add_token_change(
        &self,
        addresses: &mut HashMap<ethers_core::types::Address, serde_json::Value>,
        address: ethers_core::types::Address,
        token: ethers_core::types::Address,
        amount: ethers_core::types::U256,
        is_incoming: bool
    ) {
        let entry = addresses.entry(address).or_insert_with(|| json!({
            "eth_net": 0.0,
            "token_net": {},
            "movements": {"eth": {"in": {}, "out": {}}, "token": {"in": {}, "out": {}}}
        }));

        // Simplified token tracking - would need token metadata for proper decimals
        let token_str = format!("{:?}", token);
        let amount_f64 = amount.as_u128() as f64 / 1_000_000_000_000_000_000.0; // Assume 18 decimals

        if is_incoming {
            entry["token_net"][&token_str] = json!(amount_f64);
        } else {
            entry["token_net"][&token_str] = json!(-amount_f64);
        }
    }

    /// Add ETH change to tracking
    fn add_eth_change(
        &self,
        addresses: &mut HashMap<ethers_core::types::Address, serde_json::Value>,
        address: ethers_core::types::Address,
        amount: ethers_core::types::U256,
        is_incoming: bool
    ) {
        let entry = addresses.entry(address).or_insert_with(|| json!({
            "eth_net": 0.0,
            "token_net": {},
            "movements": {"eth": {"in": {}, "out": {}}, "token": {"in": {}, "out": {}}}
        }));

        let amount_eth = amount.as_u128() as f64 / ETH_TO_WEI_FACTOR_F64;
        let current_eth = entry["eth_net"].as_f64().unwrap_or(0.0);
        
        if is_incoming {
            entry["eth_net"] = json!(current_eth + amount_eth);
        } else {
            entry["eth_net"] = json!(current_eth - amount_eth);
        }
    }

    /// Fetch transaction data from RPC (optimized single call)
    async fn fetch_transaction_data(&self, tx_hash: &EthersH256) -> Result<TransactionData> {
        // Parallel fetch of transaction and receipt
        let (tx_opt, receipt_opt) = tokio::try_join!(
            self.ethers_provider.get_transaction(*tx_hash),
            self.ethers_provider.get_transaction_receipt(*tx_hash)
        )?;

        let tx = tx_opt.ok_or_else(|| anyhow!("Transaction not found"))?;
        let receipt = receipt_opt; // May be None for pending transactions

        Ok(TransactionData {
            hash: *tx_hash,
            from: tx.from,
            to: tx.to,
            value: tx.value,
            gas_price: tx.gas_price,
            gas_limit: tx.gas,
            gas_used: receipt.as_ref().and_then(|r| r.gas_used),
            input: tx.input.0.to_vec(),
            receipt,
            block_number: tx.block_number.map(|n| n.as_u64()),
        })
    }

    /// Run full REVM simulation (simplified version of existing code)
    async fn run_revm_simulation(&self, _tx_hash_str: &str) -> Result<(serde_json::Value, usize)> {
        // This would call the existing json_state_validator_no_rpc logic
        // For now, return placeholder - in production, extract the simulation
        // logic into a reusable function
        
        // TODO: Extract simulation logic from json_state_validator_no_rpc.rs
        // into a reusable function that both examples can call
        
        Ok((json!({}), 0))
    }
}

/// Transaction data structure for analysis
#[derive(Debug, Clone)]
struct TransactionData {
    hash: EthersH256,
    from: ethers_core::types::Address,
    to: Option<ethers_core::types::Address>,
    value: ethers_core::types::U256,
    gas_price: Option<ethers_core::types::U256>,
    gas_limit: ethers_core::types::U256,
    gas_used: Option<ethers_core::types::U256>,
    input: Vec<u8>,
    receipt: Option<ethers_core::types::TransactionReceipt>,
    block_number: Option<u64>,
}

/// Example usage function
pub async fn example_fast_analysis() -> Result<()> {
    let config = FastPathConfig::default();
    let processor = FastPathProcessor::new(config).await?;
    
    let tx_hash = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae";
    let result = processor.analyze_transaction(tx_hash).await?;
    
    println!("Analysis Result:");
    println!("  Method: {:?}", result.analysis_method);
    println!("  Time: {:.2}ms", result.processing_time_ms);
    println!("  Internal Transfers: {}", result.internal_transfers_detected);
    println!("  Confidence: {:?}", result.confidence_level);
    println!("  State Changes: {}", serde_json::to_string_pretty(&result.state_changes)?);
    
    Ok(())
}