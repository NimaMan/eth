# Mempool Processor Test Suite

This directory contains all tests for validating the mempool processor's alert publishing functionality.

## 📁 Directory Structure

```
tests/
├── validation/          # Static code validation tests
│   ├── validate_phase1.py           # Checks code structure and compilation
│   └── validate_alert_integration.sh # Shell-based validation script
│
├── integration/         # End-to-end integration tests
│   ├── test_e2e_alert_flow.py      # Tests complete alert pipeline
│   └── integration_test_pool_levels.py # Pool subscriber integration
│
├── performance/         # Performance and throughput tests
│   ├── test_alert_throughput.py     # Measures alerts/second capability
│   └── measure_publisher_performance.py # Measures serialization overhead
│
└── examples/           # Example scripts and utilities
    └── test_zmq_publisher.py        # Manual alert receiver for testing
```

## 🚀 Quick Start

Run the complete validation suite for Phase 1 (Alert Integration):

```bash
# 1. Static validation - verify code structure
python3 validation/validate_phase1.py

# 2. Integration test - verify alerts are published
python3 integration/test_e2e_alert_flow.py

# 3. Performance test - verify throughput requirements
python3 performance/test_alert_throughput.py
```

## 📋 Test Descriptions

### Validation Tests

#### `validate_phase1.py`
- **Purpose**: Validates code structure, compilation, and static requirements
- **What it tests**:
  - Code compiles without errors
  - Required modules exist (publisher.rs)
  - CLI flags implemented (--enable-publisher)
  - Alert message has all required fields
  - Performance optimizations present (DONTWAIT, HWM)
- **Success criteria**: 11/11 tests pass

#### `validate_alert_integration.sh`
- **Purpose**: Shell-based validation with more system checks
- **What it tests**: Similar to Python version but includes binary startup test
- **Note**: Use Python version for more reliable results

### Integration Tests

#### `test_e2e_alert_flow.py` ⭐ **Most Important**
- **Purpose**: Validates complete alert flow from detection to ZMQ
- **What it tests**:
  - Mempool processor starts with publisher enabled
  - Real scam transactions trigger alerts
  - Alerts are published via ZMQ
  - Alert content matches expected format
  - All required fields are present
- **Success criteria**: At least one valid alert received
- **Note**: Run during active trading hours for best results

#### `integration_test_pool_levels.py`
- **Purpose**: Tests pool subscriber integration
- **What it tests**: Pool state updates via ZMQ
- **Note**: Requires Python pool publisher running

### Performance Tests

#### `test_alert_throughput.py`
- **Purpose**: Validates system meets performance requirements
- **What it tests**:
  - Can handle 100+ alerts per second
  - Sustained load of 10,000 alerts
  - Message delivery reliability (>99%)
  - End-to-end latency measurements
- **Success criteria**: 
  - Throughput >100 alerts/sec
  - Delivery rate >99%
  - Average latency <10ms

#### `measure_publisher_performance.py`
- **Purpose**: Measures JSON serialization overhead
- **What it tests**: Time to serialize alert messages
- **Expected result**: <0.002ms per alert

### Examples

#### `test_zmq_publisher.py`
- **Purpose**: Manual testing utility to receive and display alerts
- **Usage**:
  ```bash
  # Terminal 1: Start receiver
  python3 examples/test_zmq_publisher.py
  
  # Terminal 2: Start mempool processor
  ./target/debug/mempool_signal_detection_full_tx_ipc --enable-publisher
  ```
- **Features**: 
  - Connects to tcp://localhost:5559
  - Pretty-prints received alerts
  - Shows alert fields in readable format

## ✅ Validation Checklist

Phase 1 (Alert Integration) is complete when:

- [ ] **Static Validation**: All 11 tests pass
- [ ] **Integration**: E2E test receives real alerts
- [ ] **Performance**: >100 alerts/second capability
- [ ] **Manual Testing**: Can observe alerts during trading

## 🎯 Success Criteria

From `VALIDATION_CHECKLIST.md`, the alert system must:

1. **Receive ZMQ alerts** ✅ (publisher implemented)
2. **Parse all fields correctly** ✅ (15+ fields per alert)
3. **Handle malformed alerts** ⚠️ (basic error handling)
4. **Maintain connection** ⚠️ (reconnection not tested)
5. **Process 100+ alerts/sec** ✅ (throughput test validates)

## 🔧 Troubleshooting

### No alerts in E2E test
- Run during active trading hours (9am-5pm EST best)
- Verify Reth node is synced: `curl http://localhost:8545`
- Check pool subscriber is running on port 5557
- Lower thresholds: `--percentage-threshold 0.2`

### Throughput test fails
- Check CPU/memory usage during test
- Verify no other process using port 5560
- Try smaller test first: 100 alerts instead of 1000
- Check ZMQ installation: `pip install pyzmq`

### Connection refused
- Verify mempool processor is running
- Check firewall rules for ports 5559/5560
- Try `127.0.0.1` instead of `localhost`
- Ensure `--enable-publisher` flag is used

## 📊 Performance Baselines

Expected results on modern hardware:

| Test | Expected Result | Requirement |
|------|-----------------|-------------|
| Static validation | <5 seconds | Must pass |
| E2E alert rate | 1-5 alerts/min | Depends on mempool |
| Throughput | >1000 alerts/sec | >100 alerts/sec |
| Serialization | <0.002ms | <0.1ms |
| E2E latency | <5ms average | <10ms |

## 🚦 Test Execution Order

1. **First**: Run static validation to ensure code is valid
2. **Second**: Run performance tests (controlled environment)
3. **Third**: Run E2E test during trading hours
4. **Finally**: Manual testing with example receiver

## 📝 Key Findings

The test audit revealed that initial tests only validated code structure, not functionality. The new test suite properly validates:

- **Real alert flow**: From scam detection to ZMQ publishing
- **Performance**: Meets 100+ alerts/second requirement
- **Content accuracy**: All required fields present and valid
- **Integration**: Works with actual mempool processor

## 🔄 Continuous Validation

For production readiness:

1. Run E2E test every 4 hours during trading
2. Monitor alert rate and latency
3. Check for memory leaks over 24 hours
4. Validate alerts match known scam patterns

## 📚 Related Documentation

- Alert message format: `../doc/ALERT_PUBLISHER.md`
- Phase 1 summary: `../doc/PHASE1_VALIDATION_SUMMARY.md`
- System architecture: `../README.md`