# Ethereum Kartal: Development Plan

## Project Overview
Ethereum Kartal is a high-performance Ethereum mempool monitoring and response system designed to execute Tx upon receiving alerts. The system focuses on optimizing response actions.

## Current State

### Core Components
1. **Alert Processor**
   - Will get real time alerts from other services.
2. **Tx Execution**
   - Tx execution
3. **Tx Simulator**


### Project Structure
The project follows a modular architecture:
- `alert_processor`: Components for alert processing
- `tx_execution`: Components for tx execution
- `tx_simulator`: Components for tx simulation

## Transaction Execution Module Development

### Objective
Design and implement a transaction execution module that receives transaction data, simulates it to verify success, and then executes it on the Ethereum network with proper monitoring and error handling.

### Core Requirements
1. **Transaction Simulation**
   - Leverage REVM for pre-execution simulation
   - Verify transaction validity and expected outcomes
   - Estimate gas costs accurately
   - Predict state changes before actual execution

2. **Transaction Execution**
   - Submit transactions to Ethereum network
   - Handle nonce management
   - Implement gas price strategies (base fee + priority fee)
   - Support EIP-1559 transaction format
   
3. **Transaction Monitoring**
   - Track transaction status (pending, confirmed, failed)
   - Implement confirmation tracking with configurable confirmation blocks
   - Provide resubmission strategies for stuck transactions
   
4. **Error Handling**
   - Detect and handle simulation failures
   - Account for network issues
   - Recover from transaction failures
   - Implement circuit breakers for unsafe conditions

### Architecture

```
┌─────────────────────────┐     1. Alert with tx data     ┌─────────────────────┐
│                         │                               │                     │
│  Alert Processor        ├──────────────────────────────►│  Transaction        │
│                         │                               │  Execution Engine   │
└─────────────────────────┘                               └─────────┬───────────┘
                                                                    │
                                                                    │ 2. Simulate 
                                                                    │ transaction
                                                                    ▼
┌─────────────────────────┐     3. Simulation results    ┌─────────────────────┐
│                         │                               │                     │
│  Transaction            │◄──────────────────────────────┤  Simulation        │
│  Execution Engine       │                               │  Engine (REVM)     │
│                         │                               │                     │
└─────────────┬───────────┘                               └─────────────────────┘
              │
              │ 4. If simulation successful,
              │ execute transaction
              ▼
┌─────────────────────────┐     5. Submit tx             ┌─────────────────────┐
│                         │                               │                     │
│  Transaction            ├──────────────────────────────►│  Ethereum          │
│  Dispatcher             │                               │  Node              │
│                         │                               │                     │
└─────────────┬───────────┘                               └─────────┬───────────┘
              │                                                     │
              │                                                     │ 6. Transaction
              │                                                     │ status updates
              │                                                     ▼
              │                                           ┌─────────────────────┐
              │                                           │                     │
              │                                           │  Transaction        │
              │                                           │  Monitor            │
              │                                           │                     │
              │                                           └─────────┬───────────┘
              │                                                     │
              │ 8. Final status                                     │ 7. Status 
              │ (success/failure)                                   │ updates
              ▼                                                     ▼
┌─────────────────────────┐                               ┌─────────────────────┐
│                         │                               │                     │
│  Result                 │◄──────────────────────────────┤  Status             │
│  Reporter               │                               │  Cache              │
│                         │                               │                     │
└─────────────────────────┘                               └─────────────────────┘
```

### Implementation Plan

#### Phase 1: Transaction Simulation
1. Create the `TxSimulator` struct to handle transaction simulation using REVM
   - Implement gas estimation
   - Track state changes during simulation
   - Validate transaction success criteria
   - Return simulation results with detailed feedback

2. Implement configurability for simulation environments
   - Forked mainnet state
   - Custom state modifications for testing
   - Gas price adjustment for profitability calculations

#### Phase 2: Transaction Execution
1. Create `TxExecutor` struct to handle transaction execution
   - Implement wallet management (private key storage)
   - Support multiple execution strategies (fast, normal, economic)
   - Handle nonce tracking and management
   - Implement gas price strategies with dynamic adjustment

2. Develop the `TxDispatcher` to submit transactions to the Ethereum network
   - Support multiple RPC endpoints for redundancy
   - Implement retry mechanisms
   - Handle network-specific parameters

#### Phase 3: Transaction Monitoring
1. Create `TxMonitor` struct to track transaction status
   - Subscribe to transaction receipts
   - Track confirmation counts
   - Detect stuck or dropped transactions

2. Implement `StatusCache` for efficient status tracking
   - In-memory cache with time-based expiration
   - Persistent storage for critical transactions
   - Event-based notification system

#### Phase 4: Integration and Testing
1. Connect all components into the `TxExecutionEngine`
   - Implement the main execution flow
   - Add circuit breakers for unsafe conditions
   - Create comprehensive error handling

2. Develop test suite
   - Unit tests for each component
   - Integration tests for the full execution flow
   - Simulation tests against forked mainnet
   - Testnets for live execution testing

### Deliverables
1. Complete transaction execution module with simulation-first approach
2. Configuration system for gas strategies, confirmations, and safety parameters
3. Comprehensive test suite
4. Documentation for integration with alert system
5. Performance metrics tracking

### Success Criteria
1. Transactions execute reliably with >99.5% success rate
2. Simulation accurately predicts execution outcomes
3. Failed transactions are properly handled with clear error reporting
4. System can handle concurrent transaction requests
5. Performance meets latency requirements (<2 seconds from alert to execution)

## Implementation Progress

### Completed
- ✅ Project structure setup
- ✅ Basic types for transaction execution module
- ✅ Configuration system for gas strategies and monitoring
- ✅ Transaction simulation using REVM
- ✅ State change tracking during simulation

### In Progress
- 🔄 Transaction execution implementation
- 🔄 Transaction monitoring system

### Next Steps
1. Implement the `executor.rs` module for transaction signing and submission
2. Implement the `monitor.rs` module for transaction status tracking
3. Create the `dispatcher.rs` for handling transaction submission with retry logic
4. Implement the `status_cache.rs` for efficient status tracking
5. Create the `engine.rs` that ties everything together
6. Add tests for the transaction execution flow
