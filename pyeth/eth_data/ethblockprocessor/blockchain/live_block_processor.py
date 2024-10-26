from web3 import Web3
import asyncio

from ethblockprocessor.blockchain.block_processor import BlockProcessor
from ethblockprocessor.utils.logger import get_logger


logger = get_logger("live_block_processor")



class LiveBlockProcessor:
    def __init__(self, w3: Web3, 
                 save_alerts: bool = True, 
                 save_erc20_txns: bool = True,
                 max_workers: int = 4):
        
        self.w3 = w3
        self.block_processor = BlockProcessor(
            node_url=w3.provider.endpoint_uri,
            save_alert_db=save_alerts,
            save_erc20_txn_to_db=save_erc20_txns,
            max_workers=max_workers
        )

    async def monitor_new_blocks(self):
        while True:
            try:
                block_filter = self.w3.eth.filter('latest')
                while True:
                    for event in block_filter.get_new_entries():
                        block = self.w3.eth.get_block(event, full_transactions=True)
                        await self.block_processor.process_block(block)
                    await asyncio.sleep(1)
            except Exception as e:
                logger.error(f"Error: {e}")
                