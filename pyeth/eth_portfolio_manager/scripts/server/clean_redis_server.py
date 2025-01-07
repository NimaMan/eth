import redis


# Connect to Redis
redis_client = redis.Redis(host='localhost', port=6379, db=0, decode_responses=True)

# Clear all position-related data and history
position_keys = redis_client.keys("position:*")
if position_keys:
    redis_client.delete(*position_keys)
    print(f"Cleared {len(position_keys)} position keys")

# Clear all position history
position_history_keys = redis_client.keys("position:history:*")
if position_history_keys:
    redis_client.delete(*position_history_keys)
    print(f"Cleared {len(position_history_keys)} position history keys")

# Verify it's clean
remaining_keys = redis_client.keys("position:*")
print(f"\nRemaining position keys: {remaining_keys}")
