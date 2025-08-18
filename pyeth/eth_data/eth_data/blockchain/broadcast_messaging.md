# Broadcast Messaging for Ethereum Block Processing

## Objective

Enable **multiple consumers to receive the same published block data** from RabbitMQ, where each consumer gets access to the latest block until a new one is published.

## Problem Solved

### Previous Issue
- Multiple consumers connecting to the **same named queue** resulted in round-robin message distribution
- Only one consumer would receive each published block
- Other consumers would miss blocks entirely

### Root Cause
```python
# OLD: Named queues caused round-robin distribution
queue = await self.channel.declare_queue("shared_queue_name", durable=True)
```

## Solution: Exclusive Broadcast Queues

### Algorithm Overview

1. **Publisher** (LiveBlockProcessor):
   - Uses FANOUT exchange to broadcast to all queues
   - No longer pre-creates consumer queues
   - Simply publishes blocks to the exchange

2. **Consumers** (BlockSubscriber):
   - Each creates its own **exclusive, auto-delete queue** with unique name
   - Queue configured to keep only **1 message** (latest block)
   - All consumers receive the same message via FANOUT broadcast

3. **Latest Block Only**:
   - `x-max-length: 1` ensures only latest block is kept
   - `x-overflow: drop-head` drops older blocks when new ones arrive

### Key Changes

#### LiveBlockProcessor
```python
# REMOVED: Pre-creation of named consumer queues
# OLD CODE:
# consumer_queues = ["jupyter_consumer", "eth_block_tokens_consumer", ...]
# for queue_name in consumer_queues:
#     queue = await self.channel.declare_queue(queue_name, durable=True)

# NEW: Only declare exchanges, let consumers create their own queues
self.blocks_exchange = await self.channel.declare_exchange(
    "blocks_exchange",
    aio_pika.ExchangeType.FANOUT,
    durable=True
)
```

#### BlockSubscriber
```python
# NEW: Unique queue name per consumer instance
self.queue_name = f"token_analyzer_blocks_{uuid.uuid4().hex[:8]}"

# NEW: Exclusive, auto-delete queue with latest-only policy
self.queue = await self.channel.declare_queue(
    self.queue_name,
    exclusive=True,      # Only this connection can access
    auto_delete=True,    # Deleted when connection closes
    arguments={
        'x-max-length': 1,           # Keep only 1 message
        'x-overflow': 'drop-head'    # Drop oldest when new arrives
    }
)
```

## Benefits

### 1. True Broadcast Messaging
- **Every consumer receives every block**
- No more round-robin distribution
- Multiple processes can run simultaneously

### 2. Latest Block Only
- Consumers always get the most recent block
- No queue buildup or memory issues
- Automatic cleanup of old blocks

### 3. Automatic Resource Management
- Exclusive queues prevent conflicts
- Auto-delete ensures cleanup on disconnect
- No manual queue management needed

### 4. Scalable Architecture
- Add/remove consumers without affecting others
- Each consumer operates independently
- No shared state between consumers

## Usage Examples

### Running Multiple Consumers

```bash
# Terminal 1: Start first consumer
python test_multiple_consumers.py consumer_1

# Terminal 2: Start second consumer  
python test_multiple_consumers.py consumer_2

# Terminal 3: Start third consumer
python test_multiple_consumers.py consumer_3

# All consumers will receive the same blocks!
```

### Integration in Your Code

```python
from eth_token.subscribers.block_subscriber import BlockSubscriber

async def my_block_processor(block_data):
    """Your custom block processing logic"""
    block_number = block_data[0].get("block_number")
    print(f"Processing block {block_number}")
    # Your processing logic here...

# Create subscriber with callback
subscriber = BlockSubscriber(
    rabbitmq_url="amqp://guest:guest@127.0.0.1/",
    callback=my_block_processor
)

# Start consuming (each instance gets its own queue)
await subscriber.start()
```

## Technical Details

### Queue Naming Strategy
- Format: `token_analyzer_blocks_{8_char_uuid}`
- Example: `token_analyzer_blocks_a1b2c3d4`
- Ensures uniqueness across multiple instances

### Message Flow
```
LiveBlockProcessor → FANOUT Exchange → Multiple Exclusive Queues → Multiple Consumers
                                    ├─ Queue_1 (Consumer_1)
                                    ├─ Queue_2 (Consumer_2)  
                                    └─ Queue_3 (Consumer_3)
```

### Error Handling
- Connection failures handled gracefully
- Automatic queue cleanup on disconnect
- Continued processing despite individual consumer failures

## Testing

Use the provided test scripts to verify broadcast functionality:

### Quick Test (Recommended)
```bash
# Navigate to test directory
cd py/eth_data/tests/blocks

# Run quick test with 3 consumers in one process
python3 test_multiple_quick.py
```

This will run 3 consumers for 15 seconds and verify they all receive the same blocks.

### Individual Consumer Test
```bash
# Start the block publisher (in separate terminal)
cd py/eth_data/scripts
python3 process_blocks_live.py

# In separate terminals, start multiple test consumers
cd py/eth_data/tests/blocks
python3 test_broadcast_simple.py consumer_1
python3 test_broadcast_simple.py consumer_2
python3 test_broadcast_simple.py consumer_3
```

### Automated Multi-Consumer Test
```bash
# Navigate to test directory
cd py/eth_data/tests/blocks

# Run the automated test script
./run_broadcast_test.sh
```

This will automatically start 3 consumers and show their output side by side.

### Expected Results
You should see output like:
```
[17:57:01.166] Consumer C received block 22560869
[17:57:01.167] Consumer B received block 22560869  
[17:57:01.168] Consumer A received block 22560869

✅ SUCCESS: All consumers received the same blocks!
```

All consumers should receive the same block numbers, confirming that broadcast messaging is working correctly.

## Migration Notes

### For Existing Consumers
1. Update to use the new `BlockSubscriber` class
2. Remove any manual queue creation code
3. Let the subscriber handle queue management automatically

### For Publishers
1. Remove pre-creation of consumer queues
2. Focus only on exchange declaration
3. Continue publishing to the same exchange

This solution ensures reliable, scalable broadcast messaging while maintaining the "latest block only" requirement. 