# Fund Flow Network Tests

## Overview

This test suite validates the fund flow network analysis for blockchain transactions, specifically focusing on removing intermediary addresses and showing only meaningful fund transfers.

## Test Case: Multi-Pool Arbitrage Transaction

**Transaction Hash**: `0xf7bd63f7b673646734cf259824bf2c0fa698b3474dff1fcce410acd86bdbd1ae`

This is a complex multi-pool arbitrage transaction involving:
- **Uniswap V4 Pool Manager**
- **Uniswap V3 USDT Pool** 
- **WETH Contract**
- **Multiple intermediary routing addresses**

## Key Testing Requirements

### 1. **Intermediary Filtering**
The network must exclude addresses with zero net change:
- `0x6bDf3535...8f9d59e9d` (pass-through)
- `0x3177F690...99FA1C359` (pass-through)
- `0xfBd4cdB4...67E794C37` (routing contract)

### 2. **WETH = ETH Treatment** 
WETH transfers must be combined with ETH amounts, not shown separately.

### 3. **Entity Classification**
Proper classification of:
- 👤 Users (EOAs)
- 🏊 Liquidity Pools  
- 🪙 Token Contracts
- ⛏️ Network (gas recipients)

### 4. **Edge Classification**
Correct coloring and categorization:
- 🔵 ETH transfers
- 🟢 Stablecoin flows
- 🟣 Token transfers
- 🔘 Gas payments

## Expected Final Network

### **Nodes (5 total)**
1. **👤 User** - Transaction initiator (pays gas)
2. **⛏️ Network** - Gas recipient (miners/validators)
3. **🪙 WETH Contract** - ETH ↔ WETH conversion
4. **🏊 Uniswap V4** - Pool Manager (liquidity provider)
5. **🏊 Uniswap V3** - USDT Pool (exchange pool)

### **Edges (4 total)**
1. **User → Network**: 0.0039 ETH (gas payment)
2. **WETH → V4 Pool**: 16.3254 ETH (conversion)
3. **V4 Pool → V3 Pool**: 27,158 USDC + 13,772 USDT (stablecoin exchange)
4. **V3 Pool → WETH**: 6.7296 WETH (token return)

## Running Tests

### **Quick Test**
```bash
cd /home/nima/code/crypto/py/sarigoz/tests/test_network
python3 run_network_tests.py
```

### **Individual Test Files**
```bash
python3 -m unittest test_transaction_network.py -v
```

### **Expected Output**
```
🔍 FUND FLOW NETWORK ANALYSIS TESTS
Testing transaction: 0xf7bd63...bd1ae
Expected network: 5 nodes, 4 edges (excluding intermediaries)

test_edge_classification_gas_payment ... ok
test_edge_classification_stablecoin ... ok  
test_edge_classification_weth ... ok
test_transaction_network_structure ... ok
test_expected_final_edges ... ok
test_expected_final_nodes ... ok

✅ ALL NETWORK TESTS PASSED!
```

## Test Files

- **`TRANSACTION_NETWORK_ANALYSIS.md`** - Detailed analysis of the test transaction
- **`test_transaction_network.py`** - Main test cases for network generation
- **`run_network_tests.py`** - Test runner script
- **`README.md`** - This documentation

## Integration with Frontend

The tests verify both:
- **Backend API** - Correct transaction parsing and edge classification
- **Frontend Logic** - Proper filtering of intermediaries and WETH treatment

## Validation Criteria

✅ **Backend Tests**
- Transaction parsing from Web3
- Edge classification (gas, ETH, stablecoins, tokens)
- Node entity detection
- Proper metadata extraction

✅ **Frontend Tests** 
- Intermediary address filtering (zero net change)
- WETH combination with ETH amounts
- Meaningful transfer extraction
- Network visualization data structure

## Common Issues to Test

1. **Intermediary Leakage** - Ensuring zero-change addresses are filtered
2. **WETH Separation** - WETH should not appear as separate tokens
3. **Gas Payment Exclusion** - Gas flows should be separate from fund flows
4. **Edge Classification** - Proper colors and types for all transfer types
5. **Entity Recognition** - Correct identification of pools, contracts, users

This test suite ensures the fund flow network provides a clean, accurate representation of actual economic flows without intermediary noise.