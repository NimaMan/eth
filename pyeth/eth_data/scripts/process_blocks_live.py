from eth_block_processor.blockchain.live_block_processor import LiveBlockProcessor
import asyncio


async def main():
    # Initialize the processor
    processor = LiveBlockProcessor()
    
    try:
        # Connect to WebSocket
        await processor.w3.provider.connect()
        
        # Start monitoring blocks
        await processor.monitor_new_blocks()
    finally:
        # Ensure proper cleanup
        if hasattr(processor.w3, 'provider'):
            await processor.w3.provider.disconnect()

        
if __name__ == "__main__":
    asyncio.run(main())