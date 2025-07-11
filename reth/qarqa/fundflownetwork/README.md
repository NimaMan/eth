# Fund Flow Network Module

This module provides fund flow network analysis capabilities for QARQA. It builds interactive, expandable networks showing ETH and token flows between addresses.

## Architecture

The module is structured as follows:

### Core Components

1. **fund_flow_analyzer.rs** - Analyzes and aggregates fund flows from processed transactions
2. **state_changes.rs** - Tracks balance changes for addresses
3. **network_types.rs** - Defines the network graph structures (nodes, edges, metadata)
4. **network_builder.rs** - Builds static fund flow networks from flows
5. **interactive_builder.rs** - Builds networks incrementally using eth_db
6. **visualization.rs** - Exports networks to various formats (Cytoscape, vis.js, GraphML)
7. **tx_processor_integration.rs** - Integrates with tx_processor to convert transactions to flows

### Key Features

- Extract fund flows from processed transactions
- Build networks centered on specific addresses
- Interactive network expansion using database lookups
- Identify zero-net intermediaries (routers)
- Export networks for visualization
- Track ETH and token movements
- Calculate net balance changes

### Usage Example

```rust
use qarqa_fundflownetwork::{
    InteractiveFundFlowNetwork, InteractiveConfig,
    FundFlowAnalyzer, NetworkBuilder,
    CytoscapeExporter,
};

// Create interactive network
let config = InteractiveConfig {
    max_txs_per_address: 100,
    min_eth_flow: 0.1,
    fetch_metadata: true,
    use_cache: true,
};

let mut network = InteractiveFundFlowNetwork::new(
    db_pool,
    tx_processor,
    config,
).await?;

// Initialize from center address
network.initialize_from_address(center_address).await?;

// Expand network
let new_addresses = network.expand_top_candidates(5).await?;

// Export for visualization
let cytoscape_json = CytoscapeExporter::export(network.export_network())?;
```

## Integration with QARQA

This module replaces the old tx_simulation-based fund flow analysis with a cleaner architecture that:
1. Uses tx_processor for transaction data
2. Leverages PostgreSQL eth_db for efficient address lookups
3. Provides interactive network building capabilities
4. Focuses purely on analytics without simulation