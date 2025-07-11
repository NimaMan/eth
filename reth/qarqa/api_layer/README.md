# QARQA API Layer

## 🎯 **Overview**

The API Layer provides command-line interfaces and API endpoints for the QARQA analytics system. It offers intuitive access to blockchain analysis capabilities through CLI commands, REST APIs, and batch processing pipelines.

## 🏗️ **Architecture**

### **Module Structure**
```
api_layer/
├── src/
│   ├── lib.rs         # Public API exports
│   ├── commands.rs    # CLI command definitions
│   ├── pipeline.rs    # Data processing pipelines
│   └── bin/
│       └── qarqa.rs   # Main CLI binary
├── tests/             # Integration tests
└── Cargo.toml         # API dependencies
```

### **Core Principles**
- **User-Friendly**: Intuitive CLI commands with helpful error messages
- **Performance**: Fast response times for interactive analysis
- **Flexibility**: Support for both interactive and batch operations
- **Integration**: Easy integration with other tools and scripts

## 🖥️ **Command-Line Interface**

### **Installation**
```bash
cd /home/nima/code/crypto/rust/qarqa
cargo build --release
./target/release/qarqa --help
```

### **Core Commands**

#### **1. Fund Flow Analysis**
```bash
# Analyze fund flows for a transaction
qarqa fund-flow 0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae

# Output format options
qarqa fund-flow <tx_hash> --format json
qarqa fund-flow <tx_hash> --format table
qarqa fund-flow <tx_hash> --format cytoscape

# Save results to file
qarqa fund-flow <tx_hash> --output fund_flow_analysis.json
```

**Example Output:**
```
Fund Flow Analysis for 0xf7bd63f7b673...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📊 Network Summary:
  • Total Addresses: 8 meaningful participants
  • Fund Flow Edges: Complex multi-participant network
  • Total ETH Volume: 23.055 ETH equivalent
  • Transaction Status: ✅ Success

🔄 Direct Fund Transfers:
┌─────────────────────┬─────────────────────┬───────────┬──────────────┬──────────────┐
│ From                │ To                  │ ETH       │ Stablecoins  │ Other Tokens │
├─────────────────────┼─────────────────────┼───────────┼──────────────┼──────────────┤
│ 👤 User             │ ⛏️ Network          │ 0.0039    │ -            │ -            │
│ 🪙 WETH Contract    │ 🏊 V4 Pool Manager  │ 16.3254   │ -            │ -            │
│ 🏊 V4 Pool Manager  │ 🏊 V3 USDT Pool     │ -         │ 40,930 USDC  │ -            │
│ 🏊 V3 USDT Pool     │ 🪙 WETH Contract    │ 6.7296    │ -            │ -            │
└─────────────────────┴─────────────────────┴───────────┴──────────────┴──────────────┘
```

#### **2. Address Analysis**
```bash
# Analyze an address's transaction history
qarqa address 0x742d35Cc6032C0532c6FAEFaa0A8e8A91bb7e4a7

# Time range analysis
qarqa address <address> --from 2024-01-01 --to 2024-01-31

# Transaction count and volume
qarqa address <address> --stats-only

# Export to CSV
qarqa address <address> --export transactions.csv
```

#### **3. Batch Processing**
```bash
# Process multiple transactions
qarqa batch-analyze transactions.txt --output results/

# Address batch analysis
qarqa batch-addresses addresses.txt --format json

# Pipeline processing
qarqa pipeline --config analysis_config.yaml
```

#### **4. Network Visualization**
```bash
# Generate network visualization
qarqa network <tx_hash> --output network.json

# Different layout algorithms
qarqa network <tx_hash> --layout force-directed
qarqa network <tx_hash> --layout hierarchical
qarqa network <tx_hash> --layout circular

# Export to different formats
qarqa network <tx_hash> --format graphviz --output network.dot
qarqa network <tx_hash> --format cytoscape --output network.json
```

#### **5. Configuration and Setup**
```bash
# Set database connection
qarqa config set-db "postgresql://user:pass@localhost/eth_db"

# Configure API endpoints
qarqa config set-rpc "https://eth-mainnet.g.alchemy.com/v2/..."

# View current configuration
qarqa config show

# Test connectivity
qarqa config test
```

## 🌐 **API Endpoints (Future)**

### **REST API Design**
```rust
// Future REST API implementation
use qarqa_api_layer::RestServer;

let server = RestServer::new()
    .with_port(8080)
    .with_cors_enabled(true)
    .with_rate_limiting(100) // 100 requests per minute
    .build();

server.start().await?;
```

**Planned Endpoints:**
```
GET  /api/v1/transaction/{hash}/fund-flow
GET  /api/v1/address/{address}/transactions
GET  /api/v1/address/{address}/stats
POST /api/v1/batch/analyze
GET  /api/v1/network/{hash}
GET  /api/v1/health
```

## 🔧 **Data Processing Pipelines**

### **Pipeline Architecture**
```rust
use qarqa_api_layer::Pipeline;

let pipeline = Pipeline::new()
    .add_stage(FetchTransactionStage::new())
    .add_stage(SimulateTransactionStage::new())
    .add_stage(BuildNetworkStage::new())
    .add_stage(ExportResultsStage::new())
    .with_parallel_processing(true)
    .with_error_recovery(true);

// Execute pipeline
let results = pipeline
    .execute(&input_data)
    .await?;
```

### **Built-in Pipelines**

#### **1. Transaction Analysis Pipeline**
```rust
pub struct TransactionAnalysisPipeline {
    data_fetcher: AddressFetcher,
    simulator: DevelopmentSimulator,
    network_builder: NetworkBuilder,
}

impl TransactionAnalysisPipeline {
    pub async fn analyze_transaction(
        &self, 
        tx_hash: &TransactionHash
    ) -> QarqaResult<AnalysisResult> {
        // Fetch transaction data
        let transaction = self.data_fetcher
            .get_transaction_complete(tx_hash)
            .await?;
        
        // Simulate transaction
        let simulation = self.simulator
            .simulate_transaction(&transaction)
            .await?;
        
        // Build network
        let network = self.network_builder
            .build_from_simulation(&simulation)
            .await?;
        
        Ok(AnalysisResult {
            transaction,
            simulation,
            network,
            metadata: self.generate_metadata().await?,
        })
    }
}
```

#### **2. Address Portfolio Pipeline**
```rust
pub struct AddressPortfolioPipeline {
    address_fetcher: AddressFetcher,
    state_analyzer: StateChangeAnalyzer,
}

impl AddressPortfolioPipeline {
    pub async fn analyze_address_portfolio(
        &self,
        address: &Address,
        time_range: Option<(u64, u64)>
    ) -> QarqaResult<PortfolioAnalysis> {
        // Fetch transaction history
        let transactions = match time_range {
            Some((start, end)) => {
                self.address_fetcher
                    .get_transactions_for_address_in_range(address, start, end)
                    .await?
            }
            None => {
                self.address_fetcher
                    .get_transactions_for_address(address)
                    .await?
            }
        };
        
        // Analyze portfolio changes
        let portfolio_changes = self.state_analyzer
            .analyze_portfolio_evolution(&transactions)
            .await?;
        
        Ok(PortfolioAnalysis {
            address: *address,
            transaction_count: transactions.len(),
            portfolio_changes,
            risk_metrics: self.calculate_risk_metrics(&portfolio_changes),
            performance_metrics: self.calculate_performance(&portfolio_changes),
        })
    }
}
```

## 🎨 **Usage Examples**

### **Basic CLI Usage**
```bash
# Quick transaction analysis
qarqa fund-flow 0xabc123... --format table

# Address deep dive
qarqa address 0x742d35Cc... --from 2024-01-01 --stats-only

# Batch processing
echo "0xabc123...
0xdef456...
0x789ghi..." > transactions.txt

qarqa batch-analyze transactions.txt --output ./results/
```

### **Programmatic Usage**
```rust
use qarqa_api_layer::*;
use qarqa_core_types::*;

#[tokio::main]
async fn main() -> QarqaResult<()> {
    // Initialize API layer
    let api = QarqaApi::new()
        .with_database_url("postgresql://localhost/eth_db")
        .with_cache_enabled(true)
        .build()
        .await?;
    
    // Analyze a transaction
    let tx_hash = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"
        .parse()?;
    
    let analysis = api
        .analyze_transaction(&tx_hash)
        .await?;
    
    println!("Analysis complete:");
    println!("- Network nodes: {}", analysis.network.nodes.len());
    println!("- Fund flows: {}", analysis.simulation.fund_flows.len());
    println!("- Gas used: {}", analysis.simulation.gas_used);
    
    // Export results
    api.export_analysis(&analysis, "./results/transaction_analysis.json")
        .await?;
    
    Ok(())
}
```

### **Custom Pipeline Creation**
```rust
use qarqa_api_layer::*;

async fn create_custom_analysis_pipeline() -> QarqaResult<()> {
    let pipeline = Pipeline::new()
        .add_stage(Box::new(CustomFetchStage::new()))
        .add_stage(Box::new(EnhancedSimulationStage::new()))
        .add_stage(Box::new(RiskAnalysisStage::new()))
        .add_stage(Box::new(CustomExportStage::new()));
    
    let input = PipelineInput::TransactionHash(
        "0xabc123...".parse()?
    );
    
    let results = pipeline
        .execute_with_monitoring(&input)
        .await?;
    
    println!("Pipeline executed successfully");
    println!("Stages completed: {}", results.stages_completed);
    println!("Total execution time: {:?}", results.total_duration);
    
    Ok(())
}
```

### **Configuration Management**
```rust
use qarqa_api_layer::Config;

// Load configuration from file
let config = Config::load_from_file("~/.qarqa/config.yaml")?;

// Or create programmatically
let config = Config::new()
    .with_database_url("postgresql://localhost/eth_db")
    .with_rpc_url("https://eth-mainnet.g.alchemy.com/v2/...")
    .with_cache_size(10000)
    .with_output_format(OutputFormat::Json)
    .with_parallel_workers(4);

// Apply configuration
let api = QarqaApi::with_config(config).await?;
```

## 🧪 **Testing**

### **Running Tests**
```bash
cd /home/nima/code/crypto/rust/qarqa/api_layer
cargo test

# Test CLI commands
cargo test --bin qarqa

# Integration tests
cargo test --features integration-tests
```

### **CLI Testing**
```bash
# Test basic commands
./target/debug/qarqa --help
./target/debug/qarqa fund-flow --help

# Test with real data (requires database)
./target/debug/qarqa config test
./target/debug/qarqa fund-flow 0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae
```

### **Performance Testing**
```rust
#[tokio::test]
async fn test_pipeline_performance() {
    let pipeline = TransactionAnalysisPipeline::new().await;
    let tx_hash = "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae"
        .parse()
        .unwrap();
    
    let start = Instant::now();
    let _result = pipeline
        .analyze_transaction(&tx_hash)
        .await
        .unwrap();
    let duration = start.elapsed();
    
    assert!(duration < Duration::from_secs(5)); // < 5 seconds
}
```

## 📊 **Output Formats**

### **Table Format**
```
┌─────────────────────┬─────────────────────┬───────────┬──────────────┐
│ From                │ To                  │ ETH       │ Tokens       │
├─────────────────────┼─────────────────────┼───────────┼──────────────┤
│ 👤 User             │ ⛏️ Network          │ 0.0039    │ -            │
│ 🪙 WETH Contract    │ 🏊 V4 Pool Manager  │ 16.3254   │ -            │
└─────────────────────┴─────────────────────┴───────────┴──────────────┘
```

### **JSON Format**
```json
{
  "transaction_hash": "0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae",
  "network": {
    "nodes": [
      {
        "address": "0x5B43453FCE04b92E190f391a83136bfBeCEDEFd1",
        "entity_type": "User",
        "net_eth_change": -0.003908755780679457,
        "transaction_count": 1
      }
    ],
    "edges": [
      {
        "from": "0x5B43453FCE04b92E190f391a83136bfBeCEDEFd1",
        "to": "0x0000000000000000000000000000000000000000",
        "eth_amount": 0.003908755780679457,
        "edge_type": "GasPayment"
      }
    ]
  },
  "stats": {
    "total_addresses": 8,
    "total_eth_volume": 23.055,
    "execution_time_ms": 45
  }
}
```

### **Cytoscape Format**
```json
{
  "elements": {
    "nodes": [
      {
        "data": {
          "id": "0x5B43453FCE04b92E190f391a83136bfBeCEDEFd1",
          "label": "User",
          "type": "user",
          "net_change": -0.0039
        }
      }
    ],
    "edges": [
      {
        "data": {
          "source": "0x5B43453FCE04b92E190f391a83136bfBeCEDEFd1",
          "target": "0x0000000000000000000000000000000000000000",
          "amount": 0.0039,
          "type": "gas"
        }
      }
    ]
  }
}
```

## 🔧 **Configuration**

### **Configuration File (`~/.qarqa/config.yaml`)**
```yaml
database:
  url: "postgresql://localhost:5432/eth_db"
  pool_size: 10
  timeout: 30

rpc:
  mainnet_url: "https://eth-mainnet.g.alchemy.com/v2/..."
  fallback_urls:
    - "https://mainnet.infura.io/v3/..."
    - "https://cloudflare-eth.com"

output:
  default_format: "table"
  default_output_dir: "./qarqa_results"
  timestamp_files: true

performance:
  cache_size: 10000
  parallel_workers: 4
  max_memory_usage: "2GB"

analysis:
  min_eth_threshold: 0.001
  min_token_threshold: 1.0
  significance_threshold: 0.1
```

## 📈 **Performance Characteristics**

| Operation | Typical Performance | Notes |
|-----------|-------------------|-------|
| Simple Transaction Analysis | <1 second | Cached results |
| Complex DeFi Transaction | <5 seconds | Full analysis |
| Address Portfolio Analysis | <10 seconds | 100 transactions |
| Batch Processing | ~1 second per transaction | Parallel processing |
| Network Visualization Export | <2 seconds | JSON/Cytoscape format |

## 🔗 **Integration with Other Modules**

### **Dependencies**
- **qarqa_core_types**: Data structures and error handling
- **qarqa_data_access**: Database operations
- **qarqa_tx_simulation**: Transaction simulation
- **qarqa_network_building**: Network construction

### **External Integration**
- **CLI Tools**: Easy integration with shell scripts
- **CI/CD Pipelines**: Automated analysis workflows
- **Jupyter Notebooks**: Python integration via CLI calls
- **Web Applications**: REST API endpoints (future)

## 🔄 **Development Guidelines**

### **Adding New Commands**
1. Define command structure in `commands.rs`
2. Implement command logic with proper error handling
3. Add comprehensive help text and examples
4. Include unit tests and integration tests
5. Update documentation and CLI help

### **Pipeline Development**
1. Implement `PipelineStage` trait for new stages
2. Add error recovery and monitoring
3. Test with various input types
4. Optimize for performance and memory usage
5. Document stage inputs/outputs

### **API Design Principles**
1. Consistent error messages and status codes
2. Comprehensive input validation
3. Rate limiting and security considerations
4. Versioned API endpoints
5. OpenAPI/Swagger documentation

This module provides user-friendly access to all QARQA analytics capabilities, making blockchain analysis accessible through both interactive CLI commands and programmatic interfaces.