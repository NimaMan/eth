from web3 import AsyncWeb3
from web3.providers import WebSocketProvider
from web3.types import BlockData
from hexbytes import HexBytes
import asyncio

from ethblockprocessor.blockchain.block_processor import BlockProcessor
from ethblockprocessor.utils.logger import get_logger

logger = get_logger("live_block_processor")

class LiveBlockProcessor:
    def __init__(self, websocket_url: str = "ws://127.0.0.1:8546",
                 http_url: str = "http://127.0.0.1:8545",
                 save_alerts: bool = True,
                 save_erc20_txns: bool = True):
        """Initialize the LiveBlockProcessor with WebSocket connection.
        
        Args:
            websocket_url: WebSocket endpoint for real-time monitoring
            http_url: HTTP endpoint for detailed data fetching
            save_alerts: Whether to save alerts to database
            save_erc20_txns: Whether to save ERC20 transactions
        """
        # Initialize WebSocket provider and web3 instance
        self.provider = WebSocketProvider(websocket_url)
        self.w3 = AsyncWeb3(self.provider)
        
        # Initialize BlockProcessor with HTTP connection for detailed data fetching
        self.block_processor = BlockProcessor(
            node_url=http_url,
            save_alert_db=save_alerts,
            save_erc20_txn_to_db=save_erc20_txns,
        )

    async def monitor_new_blocks(self):
        """Monitor new blocks in real-time using WebSocket subscription."""
        try:
            async with self.w3:  # Properly manage WebSocket lifecycle
                if not await self.w3.is_connected():
                    raise ConnectionError("Failed to connect to WebSocket")
                
                subscription_id = await self.w3.eth.subscribe("newHeads")
                logger.info(f"Subscribed to newHeads with ID: {subscription_id}")
                
                async for message in self.w3.socket.process_subscriptions():
                    try:
                        block_data = message.get("result", {})
                        if not block_data or "hash" not in block_data:
                            continue
                            
                        # Fetch full block with transactions once
                        block = await self.w3.eth.get_block(block_data["hash"], full_transactions=True)
                        if not block:
                            logger.warning(f"Could not fetch block with hash {block_data['hash'].hex()}")
                            continue
                        
                        # Pass the full block directly to process_block
                        block_number, transactions, alerts = await self.block_processor.process_block(block)
                        await self.block_processor.save_alerts(alerts)
                        
                        logger.info(f"Processed block {block_number} with "
                                  f"{len(transactions)} transactions and {len(alerts)} alerts")
                        
                    except Exception as e:
                        logger.error(f"Error processing block message: {e}")
                        continue
                        
        except Exception as e:
            logger.error(f"WebSocket subscription error: {e}")
            # Add delay before reconnection attempt
            await asyncio.sleep(5)
            # Recursive call to restart monitoring
            await self.monitor_new_blocks()
            
        finally:
            # Cleanup WebSocket connection
            if hasattr(self, 'w3'):
                await self.w3.provider.disconnect()
