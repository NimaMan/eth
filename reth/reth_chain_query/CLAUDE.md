# Reth Chain Query

## Purpose
Direct blockchain data access via Reth's MDBX database, achieving 100-10,000x performance improvements over RPC by eliminating network overhead and JSON serialization.

## Dependencies
- **tx_simulator**: Core simulation and database access layer (required)
- **reth**: Database and provider infrastructure (required)
- **tx_processor**: Uses our provider for transaction data (optional integration)

## Exports
- **RethQueryProvider**: Central data access layer with transaction/block queries
- **BalanceModule**: Unified ETH and token balance operations  
- **BlockTransactions**: Efficient block-level data fetching with optional traces
- **RethIndex**: Entity-centric indexing for address→transaction mappings
- **EntityTracking**: CEX, ETF, stablecoin monitoring and flow analysis

## Architecture

### Core Design: Direct Database Access
Traditional RPC: `Application → HTTP → RPC Node → JSON Parse → Execute → JSON Serialize → HTTP Response (50-500ms)`

Our approach: `Application → Memory-Mapped Read → MDBX B+ Tree → Direct Memory Access (0.001-10ms)`

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

1. **MDBX Over PostgreSQL**
   - 86% storage reduction (726GB → 100GB)
   - Same technology as Reth
   - Zero-copy memory-mapped access
   - B+ tree O(log n) lookups

2. **View Functions Over Slot Calculation**
   - Universal token compatibility
   - Handles proxy contracts automatically
   - No manual slot position tracking
   - Works with all implementations

3. **Shared Provider Pattern**
   - Single TxSimulator instance across all modules
   - Avoids multiple database connections
   - Thread-safe Arc<TxSimulator> sharing
   - Resource-efficient architecture

4. **Batch Operations**
   - Parallel processing for multi-item queries
   - Streaming for large blocks
   - Optimized for bulk analysis

## Module Structure

### Core Modules
- **provider/** - Central data access layer (see README.md)
- **balance/** - Balance and portfolio operations (see README.md)
- **reth_index/** - Entity-centric indexing (see README.md)

### Feature Modules
- **entities/** - Entity tracking (CEX, ETF, stablecoins) (see README.md)
- **storage/** - Direct storage slot access
- **block/** - Block metadata and queries
- **time_utils/** - Block↔timestamp conversions

### Legacy (Being Refactored)
- **account/** - Being merged into balance/
- **token/** - Being merged into balance/
- **postgres_db/** - Being replaced by RethIndex

## See Also
- src/provider/README.md - Provider implementation and transaction/block fetching
- src/balance/README.md - Balance calculation and portfolio management
- src/reth_index/README.md - MDBX indexing architecture and table schemas
- src/entities/README.md - Entity tracking and flow analysis systems
- examples/ - Usage examples organized by functionality