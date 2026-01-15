"""
Minimal token update cache.

This cache now tracks only which token addresses are known overall and
which were updated in the most recent block so the publisher can emit a
lightweight notification. All detailed state lives in Redis snapshots
maintained by the token manager.
"""

import asyncio
from typing import Dict, Any, List, Set


class TokenUpdateCache:
    def __init__(self, logger=None):
        self.logger = logger
        self._lock = asyncio.Lock()
        self._all_tokens: Set[str] = set()
        self._updated_tokens_current_block: Set[str] = set()
        self._latest_block_by_token: Dict[str, int] = {}

    async def update_from_token_objects(
        self,
        updated_tokens: Dict[str, Any],
        block_number: int,
    ) -> None:
        """
        Record the set of token addresses touched in the current block.
        """
        if not updated_tokens:
            return

        async with self._lock:
            addresses = set(updated_tokens.keys())
            self._updated_tokens_current_block = addresses
            self._all_tokens.update(addresses)
            for addr in addresses:
                self._latest_block_by_token[addr] = block_number

    async def get_all_token_addresses(self) -> List[str]:
        async with self._lock:
            return list(self._all_tokens)

    async def get_updated_token_addresses(self) -> List[str]:
        async with self._lock:
            return list(self._updated_tokens_current_block)

    def get_stats(self) -> Dict[str, int]:
        """
        Lightweight stats used by the REQ/REP API.
        """
        return {
            "total_tokens": len(self._all_tokens),
            "updated_tokens_current_block": len(self._updated_tokens_current_block),
        }
