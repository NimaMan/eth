# ethtx Python Examples

Clean, focused examples demonstrating the high-performance Ethereum transaction toolkit.

## 📁 Structure (7 Essential Examples)

```
python/
├── processor/              # Transaction processing
│   ├── performance_benchmark.py  # Speed comparison: 91.5x faster
│   └── batch_optimization.py     # Batch processing techniques
│
├── simulator/              # Transaction simulation
│   ├── basic_simulation.py       # ETH transfers, state changes
│   └── mev_analysis.py          # MEV detection, sandwich attacks
│
├── builder/                # Transaction building
│   └── transaction_building.py   # Build without ABI knowledge
│
└── integration/            # Complete demonstrations
    ├── complete_demo.py          # All features demonstration
    └── test_all_features.py      # Comprehensive test suite
```

## 🚀 Quick Start

### 1. Build Module
```bash
cd /home/nima/code/crypto/rust/tx_processor
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/eth_db"
maturin develop --release --features python
```

### 2. Test Installation
```bash
python3 integration/test_all_features.py
```

### 3. Run Key Examples

#### See Everything Work Together
```bash
python3 integration/complete_demo.py
```

#### Build Transactions Easily
```bash
python3 builder/transaction_building.py
```

#### Simulate Transactions
```bash
python3 simulator/basic_simulation.py
```

## 📊 Components

### TxProcessor
- **Purpose**: Process historical transactions from Reth DB
- **Speed**: ~1825 tx/sec (91.5x faster than Python)
- **Examples**: `processor/performance_benchmark.py`

### Simulator
- **Purpose**: Test transactions before sending
- **Features**: Gas prediction, state changes, MEV detection
- **Examples**: `simulator/basic_simulation.py`, `simulator/mev_analysis.py`

### TxBuilder
- **Purpose**: Build transactions without ABI knowledge
- **Features**: Token symbols, smart decimals, protocol names
- **Example**: `builder/transaction_building.py`

## 💡 Usage Examples

### Process Transactions
```python
import ethtx
processor = ethtx.TxProcessor()
tx = processor.process_transaction("0xabc...")
```

### Simulate Before Sending
```python
simulator = ethtx.Simulator()
result = simulator.simulate_transaction({
    "from": "0x...",
    "to": "0x...",
    "value": "1000000000000000000"
})
```

### Build Transactions
```python
builder = ethtx.TxBuilder.mainnet()
tx = builder.erc20_transfer("USDC", from_addr, to_addr, "1000.0")
```

## 📈 Performance

| Component | Speed | Use Case |
|-----------|-------|----------|
| TxProcessor | 1825 tx/sec | Historical analysis |
| Simulator | ~1ms/tx | Pre-flight checks |
| TxBuilder | Instant | Transaction creation |

## 🔧 Requirements

- Python 3.9+
- Reth database at `/home/nima/.local/share/reth/mainnet`
- PostgreSQL (for some features)