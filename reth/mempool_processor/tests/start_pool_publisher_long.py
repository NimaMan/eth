#!/usr/bin/env python3
import sys
sys.path.append('.')
from pool_publisher_test import publish_pool_updates

# Run for 5 minutes to give us time to test
print("Starting pool publisher for 5 minutes...")
publish_pool_updates(duration_seconds=300)