# Thread Safety and Resource Management

## Overview
The tx_processor Python bindings use parallel processing for optimal performance, but with careful safety controls to prevent system overload.

## Safety Features

### 1. Default Thread Limits
- **Default workers**: 4 threads (or CPU count if less)
- **Maximum workers**: 8 threads (hard cap)
- **Automatic CPU detection**: Never exceeds available CPU cores

### 2. Batch Processing Safety

```python
# Default batch processing - safe 4 workers
results = processor.process_transactions_batch(tx_hashes)

# Detailed with custom workers (will be capped at 8)
results = processor.process_transactions_detailed(
    tx_hashes,
    parallel=True,
    max_workers=6  # Safe: will use min(6, 8, cpu_count)
)
```

### 3. Resource Protection

The module implements multiple layers of protection:

1. **Thread pool limits**: Hard cap at 8 workers
2. **CPU awareness**: Respects system CPU count
3. **GIL release**: Uses `py.allow_threads()` for true parallelism
4. **Memory safety**: Arc<Mutex<>> for thread-safe access

### 4. Configuration

```python
# Configure global thread pool (can only be done once)
processor.configure_thread_pool(4)  # Set to 4 workers

# Check system capabilities
stats = processor.get_stats()
print(f"CPU count: {stats['cpu_count']}")
print(f"Default workers: {stats['default_workers']}")
print(f"Max workers: {stats['max_workers']}")
```

## Best Practices

### For Small Batches (< 10 transactions)
```python
# Use default settings
results = processor.process_transactions_batch(tx_hashes)
```

### For Medium Batches (10-100 transactions)
```python
# Use parallel with default 4 workers
results = processor.process_transactions_detailed(
    tx_hashes,
    parallel=True
)
```

### For Large Batches (100+ transactions)
```python
# Consider chunking to avoid memory issues
chunk_size = 50
for i in range(0, len(tx_hashes), chunk_size):
    chunk = tx_hashes[i:i+chunk_size]
    results = processor.process_transactions_batch(chunk)
    # Process results...
```

### For Production Systems
```python
# Use detailed processing with error tracking
results = processor.process_transactions_detailed(
    tx_hashes,
    parallel=True,
    max_workers=4  # Conservative for production
)

# Check for failures
if results['failed_count'] > 0:
    for failed in results['failed']:
        logger.error(f"Failed: {failed['hash']}: {failed['error']}")
```

## System Requirements

- **Minimum**: 2 CPU cores, 4GB RAM
- **Recommended**: 4+ CPU cores, 8GB RAM
- **Optimal**: 8+ CPU cores, 16GB RAM

## Why These Limits?

1. **4 default workers**: Provides good parallelism without overwhelming most systems
2. **8 maximum workers**: Beyond this, diminishing returns and increased overhead
3. **Automatic CPU detection**: Prevents creating more threads than CPU cores
4. **Thread pool reuse**: Amortizes thread creation overhead across batches

## Monitoring

Monitor system resources during batch processing:

```bash
# CPU usage
top -H

# Memory usage
free -h

# Process details
ps aux | grep python
```

## Troubleshooting

### High CPU Usage
- Reduce max_workers parameter
- Use sequential processing for small batches
- Add delays between large batches

### Memory Issues
- Process in smaller chunks
- Ensure adequate system RAM
- Monitor for memory leaks

### Thread Pool Errors
- The global thread pool can only be configured once
- Restart Python process to reconfigure
- Use per-batch worker limits instead

## Performance Guidelines

| Batch Size | Recommended Workers | Expected Throughput |
|------------|-------------------|---------------------|
| 1-5        | 1-2               | 100-200 tx/sec      |
| 5-20       | 4                 | 200-400 tx/sec      |
| 20-100     | 4-6               | 300-500 tx/sec      |
| 100+       | 6-8               | 400-600 tx/sec      |

Note: Actual performance depends on transaction complexity and system specifications.