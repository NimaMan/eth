//! Complete fund flow analysis pipeline

use qarqa_core_types::QarqaResult;
use qarqa_data_access::{DatabaseManager, AddressDataFetcher, TransactionDataFetcher};
use qarqa_tx_simulation::{DevelopmentTransactionSimulator, TransactionSimulator, FundFlowAnalyzer};
use qarqa_network_building::{NetworkBuilder, AnalysisParams, convert_to_visualization_data, VisualizationData, NetworkCurrency};
use alloy_primitives::Address;
use tracing::{info, warn};

/// Complete fund flow analysis pipeline
pub struct FundFlowPipeline {
    address_fetcher: AddressDataFetcher,
    tx_fetcher: TransactionDataFetcher,
    simulator: DevelopmentTransactionSimulator,
    analyzer: FundFlowAnalyzer,
    builder: NetworkBuilder,
}

impl FundFlowPipeline {
    /// Create a new pipeline
    pub async fn new(database_url: String) -> QarqaResult<Self> {
        info!("Initializing fund flow pipeline");
        
        // Connect to database
        let db = DatabaseManager::new(&database_url).await?;
        let pool = db.pool_cloned();
        
        // Create components
        let address_fetcher = AddressDataFetcher::new(pool.clone());
        let tx_fetcher = TransactionDataFetcher::new(pool.clone());
        let mut simulator = DevelopmentTransactionSimulator::with_database(
            TransactionDataFetcher::new(pool)
        );
        
        // Initialize simulator
        simulator.initialize().await?;
        
        let analyzer = FundFlowAnalyzer::new();
        let builder = NetworkBuilder::new();
        
        info!("Fund flow pipeline initialized successfully");
        
        Ok(Self {
            address_fetcher,
            tx_fetcher,
            simulator,
            analyzer,
            builder,
        })
    }
    
    /// Analyze fund flows for an address
    pub async fn analyze_address_fund_flows(
        &self,
        center_address: Address,
        limit: usize,
        min_value: f64,
        currency: NetworkCurrency,
    ) -> QarqaResult<VisualizationData> {
        info!("Starting fund flow analysis for address: {:?}", center_address);
        
        // Step 1: Get transaction hashes for address
        info!("Fetching transaction hashes...");
        let tx_hashes = self.address_fetcher.get_address_transactions(
            center_address,
            None,
            None,
            Some(limit as u64),
        ).await?;
        
        if tx_hashes.is_empty() {
            warn!("No transactions found for address");
            return Ok(create_empty_visualization_data(currency));
        }
        
        info!("Found {} transactions", tx_hashes.len());
        
        // Step 2: Get transaction details
        info!("Fetching transaction details...");
        let transactions = self.tx_fetcher.get_transactions_by_hashes(&tx_hashes).await?;
        
        if transactions.is_empty() {
            warn!("No transaction details found");
            return Ok(create_empty_visualization_data(currency));
        }
        
        info!("Retrieved {} transaction details", transactions.len());
        
        // Step 3: Simulate transactions to get fund flows
        info!("Simulating transactions...");
        let mut all_fund_flows = Vec::new();
        
        // Simulate first few transactions to avoid overwhelming system
        let simulation_limit = std::cmp::min(transactions.len(), 10);
        for tx in transactions.iter().take(simulation_limit) {
            match self.simulator.simulate_transaction(tx).await {
                Ok(flows) => {
                    info!("Simulated tx {}: {} ETH movements, {} token movements", 
                        tx.hash, flows.eth_movements.len(), flows.token_movements.len());
                    all_fund_flows.push(flows);
                }
                Err(e) => {
                    warn!("Failed to simulate tx {}: {}", tx.hash, e);
                    // Continue with other transactions
                }
            }
        }
        
        if all_fund_flows.is_empty() {
            warn!("No transactions could be simulated");
            return Ok(create_empty_visualization_data(currency));
        }
        
        info!("Successfully simulated {} transactions", all_fund_flows.len());
        
        // Step 4: Analyze fund flows
        info!("Analyzing fund flows...");
        let fund_flows = self.analyzer.analyze_fund_flows(&all_fund_flows)?;
        
        if fund_flows.is_empty() {
            warn!("No significant fund flows found");
            return Ok(create_empty_visualization_data(currency));
        }
        
        info!("Extracted {} fund flows", fund_flows.len());
        
        // Step 5: Build network
        info!("Building network...");
        let params = AnalysisParams {
            min_usd_threshold: min_value,
            max_depth: 2,
            include_gas: false,
            treat_weth_as_eth: true,
        };
        
        let builder = self.builder.clone().with_params(params);
        let network = builder.build_from_fund_flows(&fund_flows, Some(center_address))?;
        
        info!("Built network with {} nodes and {} edges", 
              network.stats.node_count, network.stats.edge_count);
        
        // Step 6: Convert to visualization format
        let visualization_data = convert_to_visualization_data(&network, Some(center_address), currency);
        
        info!("Fund flow analysis completed successfully");
        Ok(visualization_data)
    }
}

/// Create empty visualization data for error cases
fn create_empty_visualization_data(currency: NetworkCurrency) -> VisualizationData {
    qarqa_network_building::VisualizationData {
        nodes: Vec::new(),
        edges: Vec::new(),
        stats: qarqa_network_building::VisualizationStats {
            total_addresses: 0,
            total_edges: 0,
            total_eth_volume: "0".to_string(),
            network_density: "0".to_string(),
            currency: format!("{:?}", currency),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::Address;
    use std::str::FromStr;
    
    #[tokio::test]
    #[ignore] // Requires database
    async fn test_pipeline_creation() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:postgres@localhost/eth_db".to_string());
        
        match FundFlowPipeline::new(database_url).await {
            Ok(_) => println!("Pipeline created successfully"),
            Err(e) => println!("Expected error (no database): {}", e),
        }
    }
}