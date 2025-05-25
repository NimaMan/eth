#!/bin/bash

# Test script to run multiple broadcast consumers
# This script demonstrates that multiple consumers can receive the same blocks

echo "Starting broadcast messaging test..."
echo "This will start 3 test consumers that should all receive the same blocks"
echo ""
echo "Make sure your LiveBlockProcessor is running first!"
echo "Press Ctrl+C to stop all consumers"
echo ""

# Function to cleanup background processes
cleanup() {
    echo ""
    echo "Stopping all test consumers..."
    jobs -p | xargs -r kill
    wait
    echo "All consumers stopped."
    exit 0
}

# Set trap to cleanup on script exit
trap cleanup SIGINT SIGTERM EXIT

# Start multiple consumers in background
echo "Starting consumer 1..."
python3 test_broadcast_simple.py consumer_1 &

echo "Starting consumer 2..."
python3 test_broadcast_simple.py consumer_2 &

echo "Starting consumer 3..."
python3 test_broadcast_simple.py consumer_3 &

echo ""
echo "All consumers started. Waiting for blocks..."
echo "You should see all 3 consumers receiving the same block numbers."
echo ""

# Wait for all background jobs
wait 