from typing import Any, Dict, Optional
import pandas as pd
from eth_token.erc20_token.data.erc20_token_data import ERC20TokenData
from eth_token.erc20_token.network.token_network import LiveTokenNetwork
from eth_token.erc20_token.token_health.token_health_predictor import TokenHealthPredictor


class ERC20Token:
    """
    This class contains all the data and information of a token.

    Args:
        contract_address (str): The contract address of the token.
        token_data (Optional[ERC20TokenData]): The ERC20 token data object.

    """

    def __init__(
        self,
        contract_address: str,
        *,
        name: Optional[str] = None,
        symbol: Optional[str] = None,
        decimals: Optional[int] = None,
        total_supply: Optional[int] = None,
    ):
        self.contract_address = contract_address
        normalized_decimals = int(decimals) if decimals is not None else None
        normalized_supply = int(total_supply) if total_supply is not None else None
        self.token_data = ERC20TokenData(
            contract_address=contract_address,
            name=name,
            symbol=symbol,
            decimals=normalized_decimals,
            total_supply=normalized_supply,
        )
        self.token_network = LiveTokenNetwork(live_token=self)
        self.token_health_predictor = TokenHealthPredictor()

    def update_from_transaction(self, transaction: Dict):
        if not transaction["status"]:
            return
        self.token_data.update_from_transaction(transaction)
        self.token_network.update_from_transaction(transaction)
        self.latest_token_assessment = self.token_health_predictor.update_from_transaction(transaction, self)

    @property
    def token_creation_age_blocks(self):
        return self.token_data.latest_block_number - self.token_data.creation_block

    @property
    def token_creation_age_hours(self):
        return (self.token_data.latest_block_timestamp - self.token_data.creation_timestamp) / 3600

    def __getitem__(self, item: str):
        """Get attribute from token_data"""
        try:
            return self.token_data[item]
        except AttributeError:
            # Try to get it from self directly
            try:
                return self.__dict__[item]
            except KeyError:
                raise KeyError(f"'{self.__class__.__name__}' has no attribute '{item}'")
    
    def __getattr__(self, name: str):
        """
        Provide convenient access to token_data attributes.
        
        This allows accessing token_data fields directly on the ERC20Token object,
        e.g., token.creator_address instead of token.token_data.creator_address
        """
        # Avoid infinite recursion by checking if token_data exists
        if name == 'token_data':
            raise AttributeError(f"'{self.__class__.__name__}' has no attribute '{name}'")
        
        # Try to get the attribute from token_data
        try:
            return getattr(self.token_data, name)
        except AttributeError:
            raise AttributeError(f"'{self.__class__.__name__}' has no attribute '{name}'")
    
    def to_dict(self):
        return {
            'token_data': self.token_data.to_dict(),
            'latest_token_assessment': self.token_health_predictor.to_dict(),
            'is_scam': self.is_scam,
            'scam_reason': self.scam_reason,
        }

    @property
    def pools(self):
        """Expose pool collection directly on the token."""
        return self.token_data.pools

    @property
    def liquidity_matrix(self):
        """Expose pool liquidity matrix directly on the token."""
        return self.token_data.liquidity_matrix

    @property
    def liquidity_analyzer(self):
        """Backwards-compatible alias for older callers."""
        return self.token_data.liquidity_analyzer
    
