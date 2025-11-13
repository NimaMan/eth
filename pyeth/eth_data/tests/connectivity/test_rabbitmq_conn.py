#!/usr/bin/env python3
"""
TROUBLESHOOTING GUIDE: RabbitMQ Connection Issues
=================================================

ERROR ENCOUNTERED:
-----------------
AMQPInternalError: ("one of ['Connection.OpenOk']", <Connection.Close object>)
ProbableAccessDeniedError: ConnectionClosedByBroker: (541) "INTERNAL_ERROR - access to vhost '/' refused for user 'guest': vhost '/' is down"

ROOT CAUSE:
----------
Corruption in the RabbitMQ message store files for the default virtual host ('/').
This corruption prevented the virtual host from starting properly, causing all connection
attempts to be rejected with "vhost '/' is down" errors.

SOLUTION:
--------
Complete removal and reinstallation of RabbitMQ:

1. Stop RabbitMQ:
   sudo systemctl stop rabbitmq-server

2. Remove RabbitMQ completely:
   sudo apt-get remove --purge rabbitmq-server
   sudo apt-get autoremove

3. Delete all data directories:
   sudo rm -rf /var/lib/rabbitmq
   sudo rm -rf /var/log/rabbitmq
   sudo rm -rf /etc/rabbitmq

4. Reinstall RabbitMQ:
   sudo apt-get update
   sudo apt-get install rabbitmq-server

5. Start RabbitMQ:
   sudo systemctl start rabbitmq-server
   sudo systemctl enable rabbitmq-server

PREVENTION:
----------
1. Ensure proper shutdown of RabbitMQ when stopping the server
2. Monitor disk space to prevent running out of space
3. Consider setting up periodic backups of RabbitMQ configuration
4. Implement robust error handling in applications to gracefully handle RabbitMQ outages

This test script can be used to verify RabbitMQ connectivity using both synchronous (pika)
and asynchronous (aio_pika) libraries.
"""

# Original docstring follows
"""
RabbitMQ Connection Test Script

This script tests connectivity to RabbitMQ using different connection parameters
and both synchronous (pika) and asynchronous (aio_pika) libraries.
"""

import sys
import asyncio
import socket
from datetime import datetime
import pytest

# Try to import the required libraries
try:
    import pika
    import aio_pika
except ImportError:
    print("Error: Required libraries not found. Please install them with:")
    print("pip install pika aio_pika")
    sys.exit(1)

# Connection parameters to test
CONNECTION_URLS = [
    "amqp://guest:guest@localhost/",
    "amqp://guest:guest@127.0.0.1/",
    f"amqp://guest:guest@{socket.gethostname()}/",
    "amqp://guest:guest@localhost:5672/"
]

HOSTS = ["localhost", "127.0.0.1", socket.gethostname()]

def log(message):
    """Log a message with timestamp"""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(f"[{timestamp}] {message}")

@pytest.mark.parametrize("url", CONNECTION_URLS)
def test_sync_connection(url):
    """Test synchronous connection using pika"""
    try:
        # Parse the URL to get connection parameters
        params = pika.URLParameters(url)
        
        # Set a shorter connection timeout
        params.socket_timeout = 5
        
        # Try to establish a connection
        log(f"Attempting synchronous connection to {url}")
        connection = pika.BlockingConnection(params)
        
        # If we get here, connection was successful
        log(f"✅ Synchronous connection successful to {url}")
        
        # Close the connection
        connection.close()
        return True
        
    except Exception as e:
        log(f"❌ Synchronous connection failed: {type(e).__name__}: {e}")
        assert False, f"Synchronous connection failed for {url}"

@pytest.mark.asyncio
@pytest.mark.parametrize("url", CONNECTION_URLS)
async def test_async_connection(url):
    """Test asynchronous connection using aio_pika"""
    try:
        # Try to establish a connection
        log(f"Attempting asynchronous connection to {url}")
        connection = await aio_pika.connect_robust(
            url,
            timeout=5.0  # 5 second timeout
        )
        
        # If we get here, connection was successful
        log(f"✅ Asynchronous connection successful to {url}")
        
        # Create a channel
        channel = await connection.channel()
        
        # Close the connection
        await connection.close()
        return True
        
    except Exception as e:
        log(f"❌ Asynchronous connection failed: {type(e).__name__}: {e}")
        assert False, f"Asynchronous connection failed for {url}"

@pytest.mark.asyncio
@pytest.mark.parametrize("host", HOSTS)
async def test_network_connectivity(host, port=5672):
    """Test basic network connectivity to the host:port"""
    try:
        # Create a future that will be completed when the connection is made
        future = asyncio.open_connection(host, port)
        
        # Wait for the connection with a timeout
        reader, writer = await asyncio.wait_for(future, timeout=5.0)
        
        log(f"✅ Network connectivity to {host}:{port} successful")
        
        # Close the connection
        writer.close()
        await writer.wait_closed()
        return True
        
    except asyncio.TimeoutError:
        log(f"❌ Network connectivity to {host}:{port} timed out")
        assert False, f"Network connectivity to {host}:{port} timed out"
        
    except Exception as e:
        log(f"❌ Network connectivity to {host}:{port} failed: {type(e).__name__}: {e}")
        assert False, f"Network connectivity to {host}:{port} failed"