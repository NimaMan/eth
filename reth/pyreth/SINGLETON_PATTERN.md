# PyReth Singleton Pattern

## Problem Solved

PyReth components (TxProcessor, Simulator, ChainQuery) previously created their own database connections with file watchers. This caused "too many file watches" errors when multiple components were created:

```
thread '<unnamed>' panicked: failed to watch path: Error { kind: MaxFilesWatch...
```

## Solution: Singleton Pattern

PyReth now uses a singleton pattern to ensure only ONE database connection is created and shared across all components.

## Usage

### ✅ Correct Usage (Shared Instance)

```python
import pyreth

# Create main instance (opens database once)
reth = pyreth.PyReth()

# Get components that share the database
processor = reth.tx_processor()
simulator = reth.simulator()
query = reth.chain_query()

# All components share the same database connection
# No file watcher exhaustion!
```

### ❌ Deprecated Usage (Standalone)

```python
# DEPRECATED - Will show warnings
processor = pyreth.TxProcessor()  # Creates new DB connection
simulator = pyreth.Simulator()    # Creates another DB connection
query = pyreth.ChainQuery()       # Creates yet another DB connection
# This causes file watcher exhaustion!
```

## Implementation Details

1. **Singleton Database**: `PyRethInstance` manages a global singleton `TxProcessor` instance
2. **Shared Components**: All components use the shared `TxProcessor`:
   - `PyTxProcessor` uses the processor directly
   - `PySimulator` uses the processor's simulator
   - `PyChainQuery` uses the processor's chain_query
3. **Thread Safety**: Uses `Arc<Mutex<>>` for safe concurrent access
4. **Lazy Initialization**: Database opens on first `PyReth()` call

## Benefits

- ✅ Only one database connection
- ✅ No file watcher exhaustion
- ✅ Better performance (shared caches)
- ✅ Lower resource usage
- ✅ Thread-safe sharing

## Testing

Run the test to verify singleton behavior:

```bash
python examples/test_singleton.py
```

## Migration Guide

Update your code from:
```python
processor = pyreth.TxProcessor()
simulator = pyreth.Simulator()
```

To:
```python
reth = pyreth.PyReth()
processor = reth.tx_processor()
simulator = reth.simulator()
```

## Technical Note

The first `PyReth()` creation will still show ONE panic about file watchers - this is from the initial database opening. All subsequent components reuse this connection without additional panics.