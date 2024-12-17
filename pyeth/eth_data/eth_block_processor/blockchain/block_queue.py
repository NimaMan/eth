import asyncio
from web3.types import BlockData

# Initialize a global queue for processed blocks
processed_block_queue = asyncio.Queue()
