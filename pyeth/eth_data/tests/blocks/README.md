# Broadcast Messaging Tests

This directory contains test scripts to verify that the broadcast messaging system works correctly for Ethereum block processing.

## Broadcast Messaging Tests

### 1. `test_multiple_quick.py` (Recommended)
**Quick verification test that runs multiple consumers in one process**

```bash
python3 test_multiple_quick.py
```

- Runs 3 consumers (A, B, C) for 15 seconds
- Verifies all consumers receive the same blocks
- Shows clear success/failure results
- Best for quick verification

### 2. `test_broadcast_simple.py`
**Individual consumer test for manual verification**

```bash
# Run individual consumers in separate terminals
python3 test_broadcast_simple.py consumer_1
python3 test_broadcast_simple.py consumer_2
python3 test_broadcast_simple.py consumer_3
```

- Each consumer runs independently
- Good for debugging individual consumer behavior
- Shows real-time block reception with timestamps

### 3. `run_broadcast_test.sh`
**Automated script to run multiple consumers**

```bash
./run_broadcast_test.sh
```

- Automatically starts 3 consumers in background
- Shows output from all consumers simultaneously
- Handles cleanup on Ctrl+C

## Other Performance & Integration Tests

- `test_block_processor_performance.py` - Block processing performance testing
- `test_block_fetcher_performance.py` - Block fetching performance testing  
- `test_block_processor.py` - Core block processor functionality
- `test_block_rabbitmq_encoding.py` - Message encoding/serialization testing
- `test_monitor_new_blocks.py` - WebSocket monitoring testing
- `test_batch_data_processor.py` - Batch processing testing

## Prerequisites

1. **RabbitMQ running**: Make sure RabbitMQ server is running on localhost:5672
2. **LiveBlockProcessor running**: Start the block publisher first:
   ```bash
   cd ../../scripts
   python3 process_blocks_live.py
   ```

## Expected Results

When working correctly, you should see:

```
[17:57:01.166] Consumer C received block 22560869
[17:57:01.167] Consumer B received block 22560869  
[17:57:01.168] Consumer A received block 22560869

✅ SUCCESS: All consumers received the same blocks!
```

## Troubleshooting

### No blocks received
- Check if LiveBlockProcessor is running and publishing blocks
- Verify RabbitMQ is running: `sudo systemctl status rabbitmq-server`
- Check RabbitMQ logs for connection issues

### Connection errors
- Verify RabbitMQ URL: `amqp://guest:guest@127.0.0.1/`
- Check firewall settings
- Ensure RabbitMQ guest user has permissions

## How It Works

Each broadcast test creates **exclusive, auto-delete queues** with unique names:
- `test_consumer_A_abc123` (Consumer A)
- `test_consumer_B_def456` (Consumer B)  
- `test_consumer_C_ghi789` (Consumer C)

All queues are bound to the same `blocks_exchange` (FANOUT), so every consumer receives every published block.

The `x-max-length: 1` setting ensures only the latest block is kept in each queue. 