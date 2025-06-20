from typing import Any, Dict
import pandas as pd
from eth_token.erc20_token.data.erc20_token_data import ERC20TokenData
from eth_token.erc20_token.network.token_network import LiveTokenNetwork
from eth_token.erc20_token.token_health.token_health_predictor import TokenHealthPredictor
from eth_token.erc20_token.data.pair_sync_info import UniV2PairSyncInfo
from eth_token.utils.logger import get_logger


class ERC20Token:
    """
    This class contains all the data and information of a token.

    Args:
        contract_address (str): The contract address of the token.
        token_data (Optional[ERC20TokenData]): The ERC20 token data object.

    """

    def __init__(self, contract_address, logger=None):
        self.logger = logger or get_logger(name="erc20_token", log_folder="token_manager")   
        self.contract_address = contract_address
        self.token_data = ERC20TokenData(contract_address=contract_address, logger=self.logger)
        self.token_network = LiveTokenNetwork(live_token=self, logger=self.logger)
        self.token_health_predictor = TokenHealthPredictor()

    def update_from_transaction(self, transaction: Dict):
            if not transaction["status"]:
                return
            self.token_data.update_from_transaction(transaction)
            self.token_network.update_from_transaction(transaction)
            self.latest_token_assessment = self.token_health_predictor.update_from_transaction(transaction, self)

    async def update_from_transaction_async(self, transaction: Dict):
            if not transaction["status"]:
                return
            self.token_data.update_from_transaction(transaction)
            self.token_network.update_from_transaction(transaction)
            self.latest_token_assessment = self.token_health_predictor.update_from_transaction(transaction, self)

    @property
    def token_creation_age_blocks(self):
        return self.token_data.latest_block_number - self.token_data.creation_block
    
    @property
    def token_trading_age_blocks(self):
        if self.token_data.trading_enabled_block is None:
            return None
        return self.token_data.latest_block_number - self.token_data.trading_enabled_block

    @property
    def token_creation_age_hours(self):
        return (self.token_data.latest_block_timestamp - self.token_data.creation_timestamp) / 3600
    
    @property
    def token_trading_age_hours(self):
        if self.token_data.trading_enabled_timestamp is None:
            return None
        if self.token_data.trading_enabled_timestamp==0:
            # Calulate the age from the blocks. Zero is returned if the block timestamp is not available.
            return (self.token_data.latest_block_number - self.token_data.trading_enabled_block) / 300
        return (self.token_data.latest_block_timestamp - self.token_data.trading_enabled_timestamp) / 3600

    @property
    def liquidity_token_mint_df(self):
        """Get the liquidity token mint dataframe"""
        return self._sort_df(pd.DataFrame(self.token_data.univ2_mints))

    @property
    def liquidity_token_burn_df(self):
        """Get the liquidity token burn dataframe"""
        return self._sort_df(pd.DataFrame(self.token_data.univ2_burns))

    @property
    def liquidity_token_lock_df(self):
        """Get the liquidity token lock dataframe"""
        lock_df = pd.DataFrame(self.token_data.locks)
        lock_df['duration'] = lock_df['unlock_at'] - lock_df['timestamp']
     
        return self._sort_df(lock_df)

    @property
    def metrics(self) -> Dict[str, Any]:
        """Get metrics with performance logging"""
        self.sync_info = UniV2PairSyncInfo(token_obj=self)

        try:
            if self.has_uni_v2_pair:
                sync_info_dict = {}
                lifetime_info_dict = {}
                scam_info_dict = {}
                if not self.token_data.price_df.empty:
                    sync_info_dict = self.sync_info.to_dict()
                    lifetime_info_dict = self.lifetime_info.to_dict()
                    scam_info_dict = self.scam_info.to_dict()
                
                return {
                    **self.token_data.to_dict(),
                    **scam_info_dict,
                    **sync_info_dict,
                    **lifetime_info_dict,
                }
            else:
                return {
                    **self.token_data.to_dict(),
                }
        except Exception as e:
            self.logger.error(f"[{self.contract_address}] Error computing metrics: {str(e)}")
            raise

    def __getitem__(self, item: str):
        """Get attribute from token_data"""
        try:
            return self.token_data[item]
        except AttributeError:
            return getattr(self, item)
        except KeyError:
            raise KeyError(f"'{self.__class__.__name__}' has no attribute '{item}'")
        
    def __getattribute__(self, name: str) -> Any:
        """Custom attribute access that checks token_data if attribute not found"""
        try:
            return super().__getattribute__(name)
        except AttributeError:
            try:
                token_data = super().__getattribute__('token_data')
                return getattr(token_data, name)
            except AttributeError:
                raise AttributeError(f"'{self.__class__.__name__}' has no attribute '{name}'")

    def to_dict(self):
        return {
            'token_data': self.token_data.to_dict(),
            'latest_token_assessment': self.token_health_predictor.to_dict(),
        }
    
