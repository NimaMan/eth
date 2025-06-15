# Mempool Performance Measurement Results

## Initial Mempool State
- **Pending transactions**: 50
- **Queued transactions**: 19,993
- **Total in mempool**: 20,043 transactions

## Detection Performance (60 seconds)
- **Total transactions detected**: 632 
- **Detection rate**: 10.5 tx/s
- **Note**: IPC subscriptions only show NEW transactions entering mempool (not existing ones)

## Latency Statistics

### With Optimized IPC Socket
Using socket options to minimize buffering (SO_RCVBUF=4KB, SO_RCVLOWAT=1):

| Metric | Value |
|--------|-------|
| **Min latency** | 175.863μs |
| **Average** | 36.85ms |
| **Median (P50)** | 29.20ms |
| **P95** | 93.05ms |
| **P99** | 100.08ms |
| **Max** | 101.27ms |

### Latency Distribution
- **<100μs**: 0.0% (0/632)
- **<1ms**: 1.7% (11/632)
- **<10ms**: 21.4% (135/632)

### Socket Optimization Impact
Comparing optimized vs baseline IPC:
- **Average improvement**: 36.5%
- **P50 improvement**: 64.1%
- **P95 improvement**: 20.4%

## Key Findings

1. **We did NOT achieve <1ms detection** despite socket optimizations
   - Median latency is ~29ms, not <1ms as initially claimed
   - Even with optimizations, only 1.7% of detections are <1ms

2. **Unix socket optimizations have limited impact**
   - Reducing buffer sizes helps somewhat (36% average improvement)
   - But Unix domain sockets are already highly optimized by the kernel
   - TCP-specific options (TCP_NODELAY) don't apply to Unix sockets

3. **The real bottleneck appears to be elsewhere**
   - Possibly in Reth's internal processing before publishing to IPC
   - Or in the JSON-RPC protocol overhead
   - Network propagation time (~50ms) still dominates total latency

4. **Initial mempool capture challenge**
   - 20,043 transactions exist in mempool
   - IPC subscriptions only show NEW transactions
   - Cannot capture existing mempool state via subscription

## Conclusion

The objective of <1ms detection from TX arrival at Reth to our detection was **not achieved** with current methods. The median detection latency is ~29ms, which is still reasonable for most use cases but far from the <1ms target.

To achieve true <1ms detection would likely require:
1. Direct integration into Reth process (ExEx)
2. Bypassing JSON-RPC protocol overhead
3. Direct memory access to Reth's transaction pool

However, given that network propagation is ~50ms, optimizing from 29ms to <1ms provides limited practical benefit.