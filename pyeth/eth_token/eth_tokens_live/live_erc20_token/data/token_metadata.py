from datetime import datetime, timezone
from web3 import Web3
import pandas as pd


class ERC20TokenMetadata:

    def __init__(self, live_token_data):

        self.contract_address = live_token_data.contract_address
        self.txn_hash_creation = live_token_data.creation_txn
        self.txn_hash_ownership_renouncement = live_token_data.renouncement_txn
        self.symbol = live_token_data.symbol
        self.decimals = int(live_token_data.decimals)
        self.total_supply = int(live_token_data.total_supply) / 10 ** self.decimals

        self.creation_block = live_token_data.creation_block
        self.creation_datetime = datetime.fromtimestamp(live_token_data.creation_time).replace(tzinfo=timezone.utc)
        self.renouncement_block = live_token_data.renouncement_block if live_token_data.renouncement_block else None
        self.creator = live_token_data.creator_address
        self.creator_bytes = live_token_data.creator_nonce

        self.lp_address = self.uni_v2_pair_address = live_token_data.pair_db_obj.address_hex()
        self.uni_v2_pair_creation_txn_hash = live_token_data.pair_db_obj.txn_hash_hex()
        self.uni_v2_pair_creation_block = live_token_data.metadata.pair_db_obj.block_number
        self.uni_v2_pair_creation_datetime = datetime.fromtimestamp(live_token_data.pair_db_obj.block.timestamp).replace(tzinfo=timezone.utc)
        self.token0_contract_address = live_token_data.pair_db_obj.token0_contract_address_hex()
        self.token1_contract_address = live_token_data.pair_db_obj.token1_contract_address_hex()
        self.denom_contract_address = live_token_data.metadata.pair_db_obj.denominator_token_contract_address_hex()
        self.denom_decimals = int(live_token_data.pair_db_obj.denominator_token().decimals)
        self.denom_symbol = live_token_data.pair_db_obj.denominator_token().symbol  

        self.trading_enabled_block = None
        self.trading_enabled_timestamp = None
        self.trading_enabled_datetime = None
        self.trading_enabled = self.trading_enabled_block is not None
        self.tax_address = None
        self.buy_tax_rate = None
        self.sell_tax_rate = None
        
        self.max_buy_limit = None
        self.max_buy_ratio = None
    
    @property 
    def tax_rates(self):
        return self.buy_tax_rate, self.sell_tax_rate
    
    def to_dataframe(self) -> pd.DataFrame:
        return pd.DataFrame([self.to_dict()])

    def __getitem__(self, item):
        val = getattr(self, item)
        if isinstance(val, bytes):
            val = Web3.to_hex(val)
        return val
    
    def to_dict(self) -> dict: 
        """
        Converts the token metadata to a dictionary.
        """
        return {
            'contract_address': self.contract_address,
            'symbol': self.symbol,
            'decimals': self.decimals,
            'total_supply': self.total_supply,
            'creation_block': self.creation_block,
            'creation_datetime': self.creation_datetime,
            'renouncement_block': self.renouncement_block,
            'creator': self.creator,
            'trading_enabled_block': self.trading_enabled_block,
            'trading_enabled_timestamp': self.trading_enabled_timestamp,
            'trading_enabled_datetime': self.trading_enabled_datetime,
            'tax_address': self.tax_address,
        }
        