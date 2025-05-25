# Ethereum Kartal: Event Sequence Documentation

## Overview
This document details the exact sequence of events, data flows, and timing requirements for the Ethereum Kartal frontrunning system. The system is designed to receive scam detection alerts and execute protective transactions faster than malicious actors.

## System Context

### Input Source
- **Mempool Processor Scam Detection Service**: Monitors Ethereum mempool and detects potential scam transactions
- **Alert Format**: JSON messages via ZMQ containing scam transaction details
- **Alert Frequency**: Variable, typically 0-10 alerts per minute during normal conditions

### Output Target
- **Ethereum Network**: Mainnet transaction submission via multiple RPC endpoints
- **Transaction Type**: Token sell orders to frontrun scam transactions
- **Success Criteria**: Transaction confirmed before scammer's transaction

## Detailed Event Sequence

### Phase 1: Alert Reception (Target: <50ms)

#### Event 1.1: ZMQ Message Reception
```
Timestamp: T+0ms
Source: Mempool Processor (tcp://localhost:5557)
Action: Kartal Alert Receiver subscribes to ZMQ socket
Data Format:
{
  "alert_type": "scam_detection",
  "timestamp": 1748194045.350,
  "token_address": "0x6Ae82F23C593b520f90822D6A0bA29ce5f7b06f8",
  "pool_address": "0xBCac3A7cA9385F141469f2dE2bfFb1d18C7A67d8",
  "current_eth_reserve": 0.15,
  "simulated_eth_reserve": 0.02,
  "eth_threshold": 0.3,
  "percentage_change": -0.87,
  "scammer_tx_hash": "0x1234...abcd",
  "detection_block": 22561106,
  "confidence_score": 0.95
}
```

#### Event 1.2: Message Parsing & Validation
```
Timestamp: T+5ms
Action: Parse JSON and validate required fields
Validation Checks:
- ✓ Valid Ethereum addresses (token_address, pool_address)
- ✓ Numeric values within expected ranges
- ✓ Confidence score above threshold (>0.8)
- ✓ Alert timestamp is recent (<30 seconds old)
- ✓ Token is in our supported token list
```

#### Event 1.3: Alert Enrichment
```
Timestamp: T+15ms
Action: Enrich alert with additional context
Data Added:
- Current token price from DEX
- Our wallet's token balance
- Current gas prices (base fee + priority fee)
- Pool liquidity depth
- Historical volatility data
```

### Phase 2: Risk Assessment & Strategy Selection (Target: <100ms)

#### Event 2.1: Profitability Analysis
```
Timestamp: T+25ms
Action: Calculate potential profit/loss
Calculations:
- Estimated sell price after frontrun
- Gas costs for transaction execution
- Slippage impact on large orders
- MEV competition risk assessment
- Minimum profit threshold check (>$50)
```

#### Event 2.2: Strategy Selection
```
Timestamp: T+40ms
Action: Choose optimal response strategy
Options:
1. IMMEDIATE_SELL: Sell all tokens immediately
2. PARTIAL_SELL: Sell portion to minimize loss
3. ADD_LIQUIDITY: Counter-attack by adding liquidity
4. NO_ACTION: Risk/reward ratio unfavorable
```

#### Event 2.3: Transaction Construction
```
Timestamp: T+60ms
Action: Build transaction parameters
Transaction Data:
- To: Uniswap V2/V3 Router address
- Data: Encoded swap function call
- Value: 0 (ERC-20 token swap)
- Gas Limit: Estimated + 20% buffer
- Gas Price: Current base fee + aggressive priority fee
- Nonce: Next available nonce for our wallet
```

### Phase 3: Transaction Simulation (Target: <200ms)

#### Event 3.1: REVM Simulation Setup
```
Timestamp: T+80ms
Action: Initialize REVM with current blockchain state
Setup:
- Fork latest block state
- Load our wallet balance
- Load token contract state
- Load pool contract state
- Set gas price and limits
```

#### Event 3.2: Transaction Execution Simulation
```
Timestamp: T+120ms
Action: Simulate transaction execution
Simulation Results:
- Gas used: 180,000 (within 200,000 limit)
- Token balance change: -1000 tokens
- ETH balance change: +0.45 ETH
- Transaction success: true
- Revert reason: null
- State changes: [wallet, token_contract, pool_contract]
```

#### Event 3.3: Simulation Validation
```
Timestamp: T+150ms
Action: Validate simulation results
Checks:
- ✓ Transaction succeeds without revert
- ✓ Gas usage within acceptable limits
- ✓ Expected token/ETH balance changes
- ✓ No unexpected state changes
- ✓ Profit after gas costs > threshold
```

### Phase 4: Transaction Execution (Target: <500ms)

#### Event 4.1: Transaction Signing
```
Timestamp: T+180ms
Action: Sign transaction with private key
Security:
- Private key loaded from secure storage
- Transaction hash calculated
- ECDSA signature generated
- Signed transaction serialized
```

#### Event 4.2: Multi-RPC Submission
```
Timestamp: T+200ms
Action: Submit to multiple RPC endpoints simultaneously
Endpoints:
1. Primary: Alchemy (eth_sendRawTransaction)
2. Secondary: Infura (eth_sendRawTransaction)
3. Tertiary: QuickNode (eth_sendRawTransaction)
4. Backup: Local Geth node (eth_sendRawTransaction)
```

#### Event 4.3: Submission Confirmation
```
Timestamp: T+250ms
Action: Receive transaction hash confirmations
Responses:
- Alchemy: "0xabcd1234..." (success)
- Infura: "0xabcd1234..." (success)
- QuickNode: "0xabcd1234..." (success)
- Local: "nonce too low" (expected duplicate)
```

### Phase 5: Transaction Monitoring (Target: <30 seconds)

#### Event 5.1: Mempool Tracking
```
Timestamp: T+300ms - T+15s
Action: Monitor transaction in mempool
Tracking:
- Transaction appears in mempool
- Gas price competitiveness
- Position in pending transactions
- Competing transactions detected
```

#### Event 5.2: Block Inclusion
```
Timestamp: T+12s (example)
Action: Transaction included in block
Block Data:
- Block number: 22561107
- Transaction index: 15
- Gas used: 178,432
- Gas price: 25.5 gwei
- Status: success (1)
```

#### Event 5.3: Confirmation Tracking
```
Timestamp: T+12s, T+24s, T+36s
Action: Track confirmation blocks
Confirmations:
- Block 22561107: 1 confirmation
- Block 22561108: 2 confirmations  
- Block 22561109: 3 confirmations (considered final)
```

### Phase 6: Result Analysis & Reporting (Target: <60 seconds)

#### Event 6.1: Outcome Calculation
```
Timestamp: T+45s
Action: Calculate final profit/loss
Results:
- Tokens sold: 1000 SCAM
- ETH received: 0.447 ETH
- Gas cost: 0.0045 ETH
- Net profit: 0.4425 ETH ($1,106.25)
- Execution time: 12.2 seconds
- Frontrun success: true
```

#### Event 6.2: Performance Metrics
```
Timestamp: T+50s
Action: Record performance data
Metrics:
- Alert to submission: 250ms ✓
- Submission to inclusion: 11.75s ✓
- Total execution time: 12.2s ✓
- Success rate: 100% ✓
- Profit margin: 98.9% ✓
```

#### Event 6.3: Logging & Alerting
```
Timestamp: T+60s
Action: Log results and send alerts
Outputs:
- Structured JSON log entry
- Prometheus metrics update
- Slack notification (if configured)
- Database record insertion
```

## Timing Requirements & SLAs

### Critical Path Timing
```
Alert Reception:     0ms →   50ms  (50ms budget)
Risk Assessment:    50ms →  150ms  (100ms budget)
Simulation:        150ms →  350ms  (200ms budget)
Execution:         350ms →  850ms  (500ms budget)
Monitoring:        850ms → 30000ms (29s budget)
Total Target:      0ms → 850ms (0.85 seconds)
```

### Performance Targets
- **P50 Response Time**: <500ms (alert to submission)
- **P95 Response Time**: <1000ms (alert to submission)
- **P99 Response Time**: <2000ms (alert to submission)
- **Success Rate**: >99% (successful transaction execution)
- **Uptime**: >99.9% (system availability)

## Error Handling & Recovery

### Failure Scenarios

#### Scenario 1: Simulation Failure
```
Trigger: REVM simulation returns revert
Action: Log failure reason, skip execution
Recovery: Continue monitoring for next alert
Timing: No execution delay
```

#### Scenario 2: RPC Endpoint Failure
```
Trigger: All RPC endpoints return errors
Action: Retry with exponential backoff
Recovery: Switch to backup endpoints
Timing: +200ms delay maximum
```

#### Scenario 3: Gas Price Spike
```
Trigger: Gas price exceeds profitability threshold
Action: Recalculate with new gas prices
Recovery: Adjust strategy or abort
Timing: +100ms delay for recalculation
```

#### Scenario 4: Nonce Conflict
```
Trigger: "nonce too low" or "nonce too high" error
Action: Refresh nonce from blockchain
Recovery: Rebuild and resubmit transaction
Timing: +300ms delay for nonce refresh
```

## Integration Points

### Input Integration: Mempool Processor
```
Protocol: ZMQ SUB socket
Address: tcp://localhost:5557
Message Format: JSON
Reliability: At-least-once delivery
Backpressure: Drop old alerts if queue full
```

### Output Integration: Ethereum Network
```
Protocol: JSON-RPC over HTTPS
Endpoints: Multiple providers (Alchemy, Infura, etc.)
Method: eth_sendRawTransaction
Reliability: Retry with exponential backoff
Rate Limiting: Respect provider limits
```

### Monitoring Integration: Observability Stack
```
Metrics: Prometheus format
Logs: Structured JSON to stdout
Tracing: OpenTelemetry spans
Alerting: Webhook notifications
```

## Security Considerations

### Private Key Management
- Keys stored in encrypted format
- Memory protection against dumps
- Automatic key rotation capability
- Hardware security module support

### Transaction Security
- Simulation before execution prevents failed transactions
- Gas limit caps prevent excessive costs
- Profit thresholds prevent unprofitable trades
- Circuit breakers stop execution during anomalies

### Network Security
- TLS encryption for all RPC communications
- API key rotation for provider access
- Rate limiting to prevent abuse
- Input validation for all external data

## Performance Optimization

### Critical Path Optimizations
- Pre-compiled transaction templates
- Cached token contract ABIs
- Persistent RPC connections
- Parallel simulation and validation

### Memory Management
- Object pooling for frequent allocations
- Bounded queues to prevent memory leaks
- Periodic garbage collection tuning
- Memory-mapped files for large data

### Network Optimizations
- Connection pooling for RPC clients
- Request pipelining where possible
- Geographic distribution of RPC endpoints
- Local caching of blockchain data

---

*This event sequence will be validated and refined during implementation and testing phases.* 