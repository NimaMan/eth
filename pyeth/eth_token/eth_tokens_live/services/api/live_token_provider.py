"""
LiveTokenProvider: Access interface for LiveERC20Token objects

Objective:
---------
1. Provide direct access to LiveERC20Token objects
2. Abstract cache/storage access
3. Support async token retrieval
"""

from typing import Optional, List
from eth_tokens_live.live_erc20_token.live_token import LiveERC20Token
from eth_tokens_live.services.cache.token_cache_service import TokenCacheService
from eth_tokens_live.utils.logger import get_logger


class LiveTokenProvider:
    """
    Access provider for LiveERC20Token objects
    
    Usage:
        provider = LiveTokenProvider()
        token: LiveERC20Token = await provider.get_token("0x123...")
        print(token.metrics)  # Access token metrics
        print(token.is_scam)  # Check scam status
    """
    
    def __init__(self, redis_url: str = "redis://localhost:6379/0"):
        self._cache_service = TokenCacheService(redis_url=redis_url)
        self.logger = get_logger(name="live_token_provider", log_folder="tokens_live")
        
    async def get_token(self, address: str) -> Optional[LiveERC20Token]:
        """Get LiveERC20Token object for given address"""
        try:
            token_data = await self._cache_service.get_token(address)
            if not token_data:
                return None
                
            token = LiveERC20Token(address)
            token.token_data.update_from_dict(token_data)
            return token
            
        except Exception as e:
            self.logger.error(f"Error getting token {address}: {e}")
            return None
        
    async def get_active_tokens(self, limit: int = 100) -> List[LiveERC20Token]:
        """Get recently active LiveERC20Token objects"""
        try:
            tokens = []
            token_data_list = await self._cache_service.get_recent_tokens(limit)
            
            for token_data in token_data_list:
                address = token_data['contract_address']
                token = LiveERC20Token(address)
                token.token_data.update_from_dict(token_data)
                tokens.append(token)
                
            return tokens
            
        except Exception as e:
            self.logger.error(f"Error getting active tokens: {e}")
            return []
