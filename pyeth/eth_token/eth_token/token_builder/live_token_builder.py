"""
Live Token Builder

Objective:
---------
Build a complete token with all its transaction history and pool states.
This is useful for:
1. Testing token states against blockchain
2. Building tokens for analysis
3. Verifying pool reserves match blockchain state

Usage:
------
builder = LiveTokenBuilder()
token = await builder.build_token(contract_address)
"""

import asyncio
from dataclasses import asdict, is_dataclass
from typing import Optional, List, Dict, Any

from eth_data.utils.pyreth_client import PyrethClient
from eth_token.erc20_token.erc20_token import ERC20Token


class LiveTokenBuilder:
    """Builds tokens by processing their complete transaction history."""

    def __init__(self, logger=None, w3=None, processed_tx_provider=None):
        self.logger = logger
        # w3 retained for backwards compatibility with callers that inspect it
        self.w3 = w3

        client = PyrethClient.instance()
        if processed_tx_provider is not None:
            self.processed_tx_provider = processed_tx_provider
        else:
            self.processed_tx_provider = client.processed_tx_provider()

        self.token_provider = self.processed_tx_provider.token_provider()

    @staticmethod
    def _to_transaction_dict(tx: Any) -> Dict:
        """Normalize processed transactions to plain dicts."""
        if isinstance(tx, dict):
            return tx
        if hasattr(tx, "to_dict"):
            return tx.to_dict()
        if is_dataclass(tx):
            return asdict(tx)
        if hasattr(tx, "__dict__"):
            return dict(vars(tx))
        raise TypeError(f"Unsupported transaction type: {type(tx)!r}")
        
    async def build_token(
        self, 
        contract_address: str,
        start_block: Optional[int] = None,
        end_block: Optional[int] = None,
        max_blocks: Optional[int] = None
    ) -> ERC20Token:
        """
        Build a token by processing all its historical transactions.
        
        Args:
            contract_address: Token contract address
            start_block: Optional starting block (defaults to token creation)
            end_block: Optional ending block (defaults to latest)
            max_blocks: Optional maximum number of blocks to process
            
        Returns:
            ERC20Token: Fully built token with transaction history
        """        
        # Create token instance
        token = ERC20Token(contract_address=contract_address)
        
        latest_block = self.processed_tx_provider.get_latest_block()

        if end_block is None:
            end_block = latest_block
        end_block = int(end_block)

        if start_block is None:
            if max_blocks:
                start_block = max(0, end_block + 1 - max_blocks)
            else:
                start_block = 0
        else:
            start_block = int(start_block)
            if max_blocks:
                start_block = max(start_block, end_block + 1 - max_blocks)

        if start_block > end_block:
            start_block = end_block

        await asyncio.to_thread(
            self.token_provider.load_blocks_for_token,
            contract_address,
            start_block,
            end_block,
        )

        raw_transactions = await asyncio.to_thread(
            self.token_provider.transactions_for,
            contract_address,
        )

        filtered_transactions: List[Dict] = []
        for raw_tx in raw_transactions:
            tx_dict = self._to_transaction_dict(raw_tx)
            block_number = int(tx_dict.get("block_number", 0))
            if start_block <= block_number <= end_block:
                filtered_transactions.append(tx_dict)

        filtered_transactions.sort(key=lambda tx: (tx.get("block_number", 0), tx.get("tx_index", 0)))

        for tx in filtered_transactions:
            token.update_from_transaction(tx)

        return token
    
    async def build_token_from_transactions(
        self,
        contract_address: str,
        transactions: List[Dict]
    ) -> ERC20Token:
        """
        Build a token from a provided list of transactions.
        
        Args:
            contract_address: Token contract address
            transactions: List of ProcessedTransaction objects
            
        Returns:
            ERC20Token: Token built from provided transactions
        """
        if self.logger:
            self.logger.info(f"Building token from {len(transactions)} transactions")
        
        # Create token instance
        token = ERC20Token(contract_address=contract_address)
        
        # Process each transaction
        for tx in transactions:
            tx_dict = self._to_transaction_dict(tx)
            token.update_from_transaction(tx_dict)
        
        if self.logger:
            self.logger.info("Token building complete")
        return token
    
    async def close(self):
        """Close resources."""
        # No explicit resources to close; PyReth manages the shared connection.
        return


async def build_token_from_address(
    contract_address: str,
    start_block: Optional[int] = None,
    end_block: Optional[int] = None,
    max_blocks: Optional[int] = None
) -> ERC20Token:
    """
    Convenience function to build a token.
    
    Args:
        contract_address: Token contract address
        start_block: Optional starting block
        end_block: Optional ending block
        max_blocks: Optional cap on the number of blocks to process
        
    Returns:
        ERC20Token: Built token
    """
    builder = LiveTokenBuilder()
    try:
        token = await builder.build_token(
            contract_address, 
            start_block=start_block,
            end_block=end_block,
            max_blocks=max_blocks
        )
        return token
    finally:
        await builder.close()


if __name__ == "__main__":
    # Example usage
    import sys
    
    if len(sys.argv) < 2:
        print("Usage: python live_token_builder.py <token_address>")
        sys.exit(1)
    
    token_address = sys.argv[1]
    
    async def main():
        token = await build_token_from_address(token_address, max_blocks=100)
        print(f"Built token: {token.contract_address}")
        print(f"Total supply: {token.total_supply}")
        print(f"Number of ERC20 transfers: {len(token.erc20_transfers)}")
    
    asyncio.run(main())
