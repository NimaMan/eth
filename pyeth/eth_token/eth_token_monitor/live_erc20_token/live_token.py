
from typing import Any, Dict
from eth_token_monitor.live_erc20_token.data.live_token_data import LiveTokenData
from eth_token_monitor.live_erc20_token.network.live_token_network import LiveTokenNetwork
from eth_token_analyzer.erc20_token.token_metrics import UniV2PairSyncInfo
from eth_token_analyzer.erc20_token.token_metrics import ScamInfo
from eth_token_monitor.live_erc20_token.token_health.token_health_predictor import TokenHealthPredictor
from eth_token_monitor.utils.logger import get_logger


logger = get_logger(name="live_token", log_folder="token_manager")


class LiveERC20Token:
    """
    This class contains all the data and information of a token.

    Args:
        contract_address (str): The contract address of the token.
        token_data (Optional[ERC20TokenData]): The ERC20 token data object.

    """

    def __init__(self, contract_address):
        self.contract_address = contract_address
        self.token_data = LiveTokenData(contract_address=contract_address, logger=logger)
        self.token_network = LiveTokenNetwork(token_data=self.token_data)
        self.token_health_predictor = TokenHealthPredictor()

    def update_from_transaction(self, transaction: Dict):
        self.token_data.update_from_transaction(transaction)
        self.token_network.update_from_transaction(transaction)
        #if not self.token_health_predictor.is_scam:
        self.latest_token_assessment = self.token_health_predictor.update_from_transaction(transaction, self)
    
    async def update_from_transaction_async(self, transaction: Dict):
        self.token_data.update_from_transaction(transaction)
        self.token_network.update_from_transaction(transaction)
        #if not self.token_health_predictor.is_scam:
        self.latest_token_assessment = self.token_health_predictor.update_from_transaction(transaction, self)

    @property
    def sync_info(self):
        return UniV2PairSyncInfo(token_obj=self)

    @property
    def scam_info(self):
        return ScamInfo(self.token_data)
    
    @property
    def is_scam(self):
        scam_label = self.scam_info.scam_label
        if scam_label == "Other":
            return False
        return True

    @property
    def scam_label(self):
        return self.scam_info.scam_label
    
    @property
    def scam_block(self):
        return self.scam_info.scam_block
    
    @property
    def scam_txn(self):
        return self.scam_info.txn
    
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
        return (self.token_data.latest_block_timestamp - self.token_data.trading_enabled_timestamp) / 3600
    
    @property
    def metrics(self) -> Dict[str, Any]:
        """Combine all metrics into a single dictionary"""
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
            raise e

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
    
