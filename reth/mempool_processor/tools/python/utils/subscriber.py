"""
Simple ZeroMQ subscriber for receiving Ethereum mempool alerts.

This script connects to the ZeroMQ publisher socket and receives FlatBuffer-encoded
alerts about transactions in the Ethereum mempool.
"""

import sys
import time
import zmq
import os
import flatbuffers
import struct
import datetime

def get_string_at_offset(buf, offset):
    """Extract a string from a FlatBuffer at the given offset."""
    if offset == 0:
        return None
    
    # Get vector length (uint32 at offset position)
    length = struct.unpack('<I', buf[offset:offset+4])[0]
    
    # String data starts after the length field
    string_data = buf[offset+4:offset+4+length]
    return string_data

def parse_alert(buf):
    """Manually parse the FlatBuffer alert format."""
    # The root table starts at offset 4 (after the uint32 header)
    table_start = 4 + struct.unpack('<I', buf[4:8])[0]
    
    # Read vtable
    vtable_offset = struct.unpack('<h', buf[table_start:table_start+2])[0]
    vtable_pos = table_start - vtable_offset
    vtable_size = struct.unpack('<h', buf[vtable_pos:vtable_pos+2])[0]
    
    # Field offsets (assuming our table structure from Rust)
    field_offsets = []
    for i in range(4):  # We have 4 fields (hash, value, from, to)
        if vtable_size > 4 + i*2:
            field_offsets.append(struct.unpack('<h', buf[vtable_pos+4+i*2:vtable_pos+6+i*2])[0])
        else:
            field_offsets.append(0)  # Field not present
    
    # Extract fields
    result = {}
    
    # Hash (byte array)
    if field_offsets[0] > 0:
        hash_offset = table_start + field_offsets[0]
        uoffset = struct.unpack('<I', buf[hash_offset:hash_offset+4])[0]
        hash_bytes = get_string_at_offset(buf, hash_offset + uoffset)
        result['hash'] = hash_bytes.hex() if hash_bytes else None
    
    # Value (uint64)
    if field_offsets[1] > 0:
        value_offset = table_start + field_offsets[1]
        result['value'] = struct.unpack('<Q', buf[value_offset:value_offset+8])[0]
    else:
        result['value'] = 0
    
    # From address (byte array)
    if field_offsets[2] > 0:
        from_offset = table_start + field_offsets[2]
        uoffset = struct.unpack('<I', buf[from_offset:from_offset+4])[0]
        from_bytes = get_string_at_offset(buf, from_offset + uoffset)
        result['from'] = from_bytes.hex() if from_bytes else None
    
    # To address (byte array)
    if field_offsets[3] > 0:
        to_offset = table_start + field_offsets[3]
        uoffset = struct.unpack('<I', buf[to_offset:to_offset+4])[0]
        to_bytes = get_string_at_offset(buf, to_offset + uoffset)
        result['to'] = to_bytes.hex() if to_bytes else None
    
    return result

def format_wei(wei_value):
    """Format wei value to various denominations."""
    if wei_value == 0:
        return "0 ETH"
    
    denominations = [
        (10**18, "ETH"),
        (10**15, "finney"),
        (10**12, "szabo"),
        (10**9, "gwei"),
        (10**6, "mwei"),
        (10**3, "kwei"),
        (1, "wei")
    ]
    
    for denom_value, denom_name in denominations:
        if wei_value >= denom_value:
            value = wei_value / denom_value
            return f"{value:.6f} {denom_name}"
    
    return f"{wei_value} wei"

def main():
    # Set up ZeroMQ subscriber
    ctx = zmq.Context()
    socket = ctx.socket(zmq.SUB)
    socket.connect("ipc:///tmp/mempool_feed")
    socket.setsockopt(zmq.SUBSCRIBE, b"")  # Subscribe to all messages
    
    print("Listening for mempool alerts...")
    start_time = time.time()
    msg_count = 0
    eth_total = 0
    
    try:
        while True:
            # Wait for FlatBuffer message
            message = socket.recv()
            msg_count += 1
            
            try:
                # Parse the FlatBuffer message
                alert = parse_alert(message)
                
                # Extract and accumulate values
                value = alert.get('value', 0)
                eth_total += value
                
                # Print the alert
                timestamp = datetime.datetime.now().strftime("%H:%M:%S.%f")[:-3]
                
                # Regular transaction details
                print(f"[{timestamp}] ALERT #{msg_count}: TX {alert.get('hash', 'unknown')}")
                print(f"  From: {alert.get('from', 'unknown')}")
                print(f"  To:   {alert.get('to', 'unknown')}")
                print(f"  Value: {format_wei(value)}")
                
                # Log statistics every 10 transactions
                if msg_count % 10 == 0:
                    elapsed = time.time() - start_time
                    print(f"\n--- STATS AFTER {msg_count} TRANSACTIONS ---")
                    print(f"  Runtime: {elapsed:.2f} seconds")
                    print(f"  Avg processing time: {elapsed/msg_count*1000:.2f} ms per transaction")
                    print(f"  Total value observed: {format_wei(eth_total)}")
                    print("-" * 50)
                else:
                    print("-" * 30)
                
            except Exception as e:
                print(f"Error parsing message: {e}")
            
    except KeyboardInterrupt:
        elapsed = time.time() - start_time
        print("\nShutting down...")
        print(f"Processed {msg_count} transactions in {elapsed:.2f} seconds")
        print(f"Average throughput: {msg_count/elapsed:.2f} tx/s")
        print(f"Total value: {format_wei(eth_total)}")
    finally:
        socket.close()
        ctx.term()

if __name__ == "__main__":
    main() 