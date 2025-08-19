# Integration Demos

Complete demonstrations and integration tests showing how all ethtx components work together.

## Examples

### 01_complete_demo.py
Comprehensive demonstration of all ethtx features:
- TxProcessor for historical analysis
- Simulator for testing transactions
- TxBuilder for construction
- Complete workflow examples

### 02_comprehensive_workflow.py
End-to-end workflows demonstrating real use cases:
- Build → Simulate → Send → Process flow
- MEV bot implementation example
- DeFi interaction patterns
- Fund flow analysis

### 03_integrated_usage.py
Shows how components integrate:
- Using TxBuilder output with Simulator
- Processing simulated transactions
- Combining all three components

### 04_test_all_features.py
Comprehensive test suite:
- Verifies all components are working
- Tests method availability
- Validates integration points
- Good for troubleshooting

### 05_integration_test.py
Integration testing with real data:
- Tests against actual blockchain data
- Validates processing accuracy
- Performance measurements

## Complete Workflow Example

```python
import ethtx

# Step 1: Build a transaction
builder = ethtx.TxBuilder.mainnet()
tx_params = builder.erc20_transfer("USDC", from_addr, to_addr, "100.0")

# Step 2: Simulate before sending
simulator = ethtx.Simulator()
result = simulator.simulate_transaction(tx_params)

if result.success:
    print(f"✅ Ready to send! Gas: {result.gas_used}")
    
    # Step 3: Send with web3.py (not included in ethtx)
    # tx_hash = web3.eth.send_transaction(tx_params)
    
    # Step 4: After mining, analyze the transaction
    processor = ethtx.TxProcessor()
    processed = processor.process_transaction(tx_hash)
    print(f"Transaction type: {processed.txn_type}")
```

## Running the Demos

```bash
# Test everything is working
python integration_demos/04_test_all_features.py

# See complete demonstration
python integration_demos/01_complete_demo.py

# Run comprehensive workflow
python integration_demos/02_comprehensive_workflow.py
```