import asyncio
from web3 import AsyncWeb3
from web3.providers import WebSocketProvider
from general_utils.logging.logger import get_logger
import pytest


logger = get_logger(name="test_reth_connection_async", log_folder="tests")


@pytest.mark.asyncio
async def test_websocket_connection():
    # Use explicit localhost IP address
    WS_ENDPOINT = "ws://127.0.0.1:8546"
    
    try:
        # Create provider and web3 instance
        provider = WebSocketProvider(WS_ENDPOINT)
        w3 = AsyncWeb3(provider)
        
        async with w3:  # Properly manage WebSocket lifecycle
            is_connected = await w3.is_connected()
            logger.info(f"Connection established: {is_connected}")
            
            if is_connected:
                block_number = await w3.eth.block_number
                logger.info(f"Current block number: {block_number}")
            else:
                logger.error("Failed to connect to WebSocket endpoint")
                
    except Exception as e:
        logger.error(f"Error in WebSocket connection: {e}")
        
    finally:
        if 'provider' in locals():
            await provider.disconnect()

if __name__ == "__main__":
    asyncio.run(test_websocket_connection())