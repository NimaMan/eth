# Reth Chain Query

## Purpose
Direct blockchain data access via Reth's MDBX database.

## Dependencies
- **tx_simulator**: Core simulation and database access layer (required)
- **reth**: Database and provider infrastructure (required)
- **tx_processor**: Uses our provider for transaction data

## Exports
- **RethQueryProvider**: Central data access layer with transaction/block queries
- **BalanceModule**: Unified ETH and token balance operations  
- **BlockTransactions**: Efficient block-level data fetching with optional traces
- **RethIndex**: Entity-centric indexing for address→transaction mappings
- **EntityTracking**: CEX, ETF, stablecoin monitoring and flow analysis

## Architecture
### Two-Database Strategy

1. **Reth DB (Sequential)**
   - Optimized for blockchain operation
   - Block and transaction data in sequential order
   - Great for: syncing, validation, consensus
   - Terrible for: address queries, entity tracking

2. **RethIndex (Entity-Centric)**  
   - Complementary indexing layer
   - Address → Transaction mappings
   - Pre-computed aggregations
   - Only stores what Reth lacks

### Key Architectural Decisions

1. **Contract  Function Simulaiton Over Slot Calculation**
   - Universal token compatibility
   - Handles proxy contracts automatically
   - No manual slot position tracking
   - Works with all implementations

2. **Shared Provider Pattern**
   - Single TxSimulator instance across all modules
   - Avoids multiple database connections
   - Thread-safe Arc<TxSimulator> sharing
   - Resource-efficient architecture

3. **Batch Operations**
   - Parallel processing for multi-item queries
   - Streaming for large blocks
   - Optimized for bulk analysis

## Module Structure

### Core Modules
- **provider/** - Central data access layer (see README.md)
- **reth_index/** - Entity-centric indexing (see README.md)

### Feature Modules
- **entities/** - Entity tracking (CEX, ETF, stablecoins) (see README.md)
- **time_utils/** - Block↔timestamp conversions


## See Also
- src/provider/README.md - Provider implementation and transaction/block fetching
- src/reth_index/README.md - MDBX indexing architecture and table schemasr
- src/entities/README.md - Entity tracking and flow analysis systems
- examples/ - Usage examples organized by functionality