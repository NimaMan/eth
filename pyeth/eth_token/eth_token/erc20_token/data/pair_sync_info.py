import pandas as pd
from typing import Optional, Dict, Any

# Constants for block range filtering
MIN_UINT256 = 0
MAX_UINT256 = 2**256 - 1


class UniV2PairSyncInfo:
    """
    A class to represent the synchronization information of a Uniswap V2 pair.

    Args:
        token_data (ERC20TokenData): The data of the token.
    """

    def __init__(self, token_obj=None, price_jump_anomaly_threshold: float=10):
        self.price_jump_anomaly_threshold = price_jump_anomaly_threshold
        self.token_obj = token_obj
        self.total_supply = token_obj.token_data.total_supply
        self.price_df = token_obj.token_data.price_df
        self.remove_anomalies_from_price_df()
    
    def remove_price_jumps(self, threshold: float = 10.0) -> pd.DataFrame:
        """
        Get price jumps where price increased by more than threshold ratio compared to previous price.        
        Args:
            threshold: Minimum ratio between consecutive prices to be considered a jump.
                    E.g., 2.0 means price doubled, 3.0 means price tripled
        Returns:
            DataFrame containing price jump events
        """
        if self.price_df.empty:
            return pd.DataFrame()
            
        # Calculate price ratios between consecutive trades
        price_ratios = self.price_df['relative_price'] / self.price_df['relative_price'].shift(1)
        
        # Keep first row (usually Add Liquidity) and rows with acceptable price ratios
        mask = price_ratios.isna() | (price_ratios <= threshold)
        clean_price_jumps_df = self.price_df[mask].copy()
        
        return clean_price_jumps_df.sort_values(by=['block_number', 'tx_index', 'log_index'], ascending=True)

    def remove_price_drops(self, threshold: float = 0.1) -> pd.DataFrame:
        """
        Get price drops where price decreased by more than threshold ratio compared to previous price.
        
        Args:
            threshold: Maximum ratio between consecutive prices to be considered a drop.
                      E.g., 0.5 means price halved, 0.33 means price dropped to 1/3
    
        Returns:
            DataFrame containing price drop events
        """
        if self.price_df.empty:
            return pd.DataFrame()
            
        # Calculate price ratios between consecutive trades
        price_ratios = self.price_df['relative_price'] / self.price_df['relative_price'].shift(1)
        # Get clean price drops
        mask = price_ratios.isna() | (price_ratios >= threshold)
        clean_price_drops_df = self.price_df[mask].copy()
        return clean_price_drops_df.sort_values(by=['block_number', 'tx_index', 'log_index'], ascending=True)
    
    def remove_anomalies_from_price_df(self):
        """Remove anomalies from the price dataframe using the relative price jump threshold.
        """
        if self.price_df.empty:
            return
        self.price_df = self.remove_price_jumps(threshold=self.price_jump_anomaly_threshold)
        self.price_df = self.remove_price_drops(threshold=1/self.price_jump_anomaly_threshold)

    # region Price
    @property
    def max_price_ratio(self) -> Optional[float]:
        """Calculate the maximum price ratio"""
        if self.price_df.empty:
            return None
        return self.price_df['relative_price'].max()
    
    @property
    def min_price_ratio(self) -> Optional[float]:
        """Calculate the minimum price ratio"""
        if self.price_df.empty:
            return None
        return self.price_df['relative_price'].min()
    
    @property
    def current_price_ratio(self) -> Optional[float]:
        """Calculate the current price ratio"""
        if self.token_obj and self.token_obj.is_scam:
            return 0
        if self.price_df.empty:
            return None
        return self.price_df.iloc[-1]['relative_price']
    
    @property
    def current_price(self) -> Optional[float]:
        """Returns the most recent price of the token as recorded in the DataFrame."""
        if not self.price_df.empty:
            return self.price_df['price'].iloc[-1]
                
    def relative_price_at_block(self, block: int) -> Optional[float]:
        """Returns the relative price of the token at the end of the block range."""
        if not self.price_df.empty:
            filtered_df = self.price_df[self.price_df['block'] <= block]
            if not filtered_df.empty:
                return filtered_df['relative_price'].iloc[-1]

    # endregion
    # region Liquidity
    @property
    def current_liquidity(self) -> Optional[float]:
        """Current liquidity"""
        if not self.price_df.empty:
            return self.price_df.iloc[-1]['denom_reserve']

    @property
    def funders(self) -> Optional[str]:
        """Returns the address of the initial liquidity provider."""
        if not self.price_df.empty:
            tmp = self.price_df[self.price_df['action'] == 'Add Liquidity']
            if not tmp.empty:
                return set(tmp['from_address'].unique())
    
    @property
    def initial_liquidity_add_block(self) -> Optional[int]:
        """Returns the block number of the initial liquidity add."""
        if not self.price_df.empty: 
            tmp = self.price_df.iloc[0]
            if tmp['action'] == 'Add Liquidity':
                return tmp['block']
    
    @property
    def initial_liquidity_add_timestamp(self) -> Optional[int]:
        """Returns the timestamp of the initial liquidity add."""
        if not self.price_df.empty:
            tmp = self.price_df.iloc[0]
            if tmp['action'] == 'Add Liquidity':
                return tmp['timestamp']

    @property
    def initial_liquidity_add_token_amount(self) -> Optional[float]:
        """Returns the initial token amount added to the liquidity pool."""
        if not self.price_df.empty:
            tmp = self.price_df.iloc[0]
            if tmp['action'] == 'Add Liquidity':
                return tmp['token_reserve']
    
    @property
    def initial_liquidity_add_denom_amount(self) -> Optional[float]:
        """Returns the initial denomination amount added to the liquidity pool."""
        if not self.price_df.empty:
            tmp = self.price_df.iloc[0]
            if tmp['action'] == 'Add Liquidity':
                return tmp['denom_reserve']
    
    @property
    def initial_liquidity(self) -> Optional[float]:
        """
        Liquidity at launch: defined as the last liquidity value before trading was enabled
        """
        trading_enabled_block = self.token_obj.trading_enabled_block
        liquidity_series = self.price_df[self.price_df['block_number'] <= trading_enabled_block]
        liquidity_series = liquidity_series[liquidity_series["action"] == "Add Liquidity"]['denom_reserve']
        if liquidity_series.empty:
            return None
        return liquidity_series.iloc[-1]
    
    @property
    def initial_liquidity_add_token_fraction(self) -> Optional[float]:
        """Returns the fraction of the total token supply that was added to the liquidity pool."""
        if (initial_liquidity_add_token_amount := self.initial_liquidity_add_token_amount):
            if self.total_supply > 0:
                return initial_liquidity_add_token_amount / self.total_supply

    # endregion
    # region Transactions 
    def unique_addresses_block_range(self, from_block: int=MIN_UINT256, to_block: int=MAX_UINT256) -> int:
        """Returns the number of unique traders within a specified block range."""
        if not self.price_df.empty:
            filtered_df = self.price_df[~self.price_df['sandwich_attack']
                                    & (self.price_df['block_number'] >= from_block)
                                    & (self.price_df['block_number'] <= to_block)
                                    & (self.price_df['action'].isin(['Sell', 'Buy']))]
            if not filtered_df.empty:
                return filtered_df['from_address'].nunique()
        return 0
    
    def num_transactions_block_range(self, from_block: int=MIN_UINT256, to_block: int=MAX_UINT256) -> int:
        """Returns the number of interactions with the Uniswap V2 pool within a specified block range."""
        if not self.price_df.empty:
            filtered_df = self.price_df[~self.price_df['sandwich_attack']
                                    & (self.price_df['block_number'] >= from_block)
                                    & (self.price_df['block_number'] <= to_block)
                                    & (self.price_df['action'].isin(['Sell', 'Buy']))]
            if not filtered_df.empty:
                return filtered_df.index.nunique()
        return 0
    
    @property
    def num_transactions(self) -> int:
        if not self.price_df.empty:
            return self.price_df.shape[0]
        return 0
    
    @property
    def num_buys(self) -> int:
        return self.price_df[self.price_df['action'] == 'Buy'].shape[0]
    
    @property
    def num_sells(self) -> int:
        return self.price_df[self.price_df['action'] == 'Sell'].shape[0]
    
    @property
    def num_buys_to_sell_ratio(self) -> float:
        return self.num_buys / self.num_sells if self.num_sells > 0 else 0
    
    # region Addresses
    @property
    def unique_addresses(self) -> set:
        # Check if token_obj has unique_addresses property
        try:
            return self.token_obj.unique_addresses
        except AttributeError:
            # If not available or price_df is empty, return empty set
            if self.price_df.empty:
                return set()
            return set(self.price_df['from_address'].unique())
    
    @property
    def num_unique_addresses(self) -> int:
        return len(self.unique_addresses)
    
    @property
    def buyer_addresses(self) -> set:
        return set(self.price_df[self.price_df['action'] == 'Buy']['from_address'].unique())
    
    @property
    def seller_addresses(self) -> set:
        return set(self.price_df[self.price_df['action'] == 'Sell']['from_address'].unique())
    
    @property
    def num_unique_buyers(self) -> int:
        return len(self.buyer_addresses)
    
    @property
    def num_unique_sellers(self) -> int:
        return len(self.seller_addresses)
    
    # endregion
    # region Volume
    def denom_volume_block_range(self, from_block: int=MIN_UINT256, to_block: int=MAX_UINT256) -> float:
        """Returns the total volume of the denomination token within a specified block range."""
        if not self.price_df.empty:
            filtered_df = self.price_df[~self.price_df['sandwich_attack']
                                    & (self.price_df['block_number'] >= from_block)
                                    & (self.price_df['block_number'] <= to_block)
                                    & (self.price_df['action'].isin(['Sell', 'Buy']))]
            if not filtered_df.empty:
                return (filtered_df['denom_spent'].sum() + filtered_df['denom_received'].sum())
        return 0.0
    
    @property
    def denom_volume(self) -> float:
        return self.denom_volume_block_range(from_block=MIN_UINT256, to_block=MAX_UINT256)
    
    def denom_in_block_range(self, from_block: int=MIN_UINT256, to_block: int=MAX_UINT256) -> float:
        """Returns the total amount of the denomination token spent within a specified block range."""
        if not self.price_df.empty:
            filtered_df = self.price_df[~self.price_df['sandwich_attack']
                                    & (self.price_df['block_number'] >= from_block)
                                    & (self.price_df['block_number'] <= to_block)
                                    & (self.price_df['action'].isin(['Buy']))]
            if not filtered_df.empty:
                return filtered_df['denom_spent'].sum()
        return 0.0
    
    @property
    def total_denom_in(self) -> float:
        if not self.price_df.empty:
            return self.price_df[self.price_df['action'] == 'Buy']['denom_spent'].sum()
        return 0.0
    
    def avg_denom_in_block_range(self, from_block: int=MIN_UINT256, to_block: int=MAX_UINT256) -> float:
        """Returns the average amount of the denomination token spent within a specified block range."""
        if not self.price_df.empty:
            filtered_df = self.price_df[~self.price_df['sandwich_attack']
                                    & (self.price_df['block_number'] >= from_block)
                                    & (self.price_df['block_number'] <= to_block)
                                    & (self.price_df['action'].isin(['Buy']))]
            if not filtered_df.empty:
                return filtered_df['denom_spent'].mean()
        return 0.0
    
    @property
    def avg_denom_in(self) -> float:
        return self.total_denom_in / self.num_buys if self.num_buys > 0 else 0
    
    def denom_out_block_range(self, from_block: int=MIN_UINT256, to_block: int=MAX_UINT256) -> float:
        """Returns the total amount of the denomination token received within a specified block range."""
        if not self.price_df.empty:
            filtered_df = self.price_df[~self.price_df['sandwich_attack']
                                    & (self.price_df['block_number'] >= from_block)
                                    & (self.price_df['block_number'] <= to_block)
                                    & (self.price_df['action'].isin(['Sell']))]
            if not filtered_df.empty:
                return filtered_df['denom_received'].sum()
        return 0.0
    
    @property
    def total_denom_out(self) -> float:
        if not self.price_df.empty:
            return self.price_df[self.price_df['action'] == 'Sell']['denom_received'].sum()
        return 0.0

    @property
    def avg_denom_out(self) -> float:
        return self.total_denom_out / self.num_sells if self.num_sells > 0 else 0
    
    @property
    def total_denom_in_to_total_denom_out_ratio(self) -> float:
        return self.total_denom_in / self.total_denom_out if self.total_denom_out > 0 else 0
    
    # endregion
    # region Fully diluted value
    def fdv_at_block(self, block: int) -> Optional[float]:
        """Returns the fully diluted value (FDV) of the token."""
        if (price := self.current_price) is not None:
            return price * self.total_supply

    # endregion
    # region Buy/Sell ratio
    def buy_sell_ratio_block_range(self, from_block: int=MIN_UINT256, to_block: int=MAX_UINT256) -> float:
        if not self.price_df.empty:
            filtered_df = self.price_df[~self.price_df['sandwich_attack']
                                    & (self.price_df['block_number'] >= from_block)
                                    & (self.price_df['block_number'] <= to_block)
                                    & (self.price_df['action'].isin(['Sell', 'Buy']))]
            if not filtered_df.empty:
                denom_spent_total = filtered_df['denom_spent'].sum()
                denom_received_total = filtered_df['denom_received'].sum()
                if denom_spent_total > 0 or denom_received_total > 0:
                    return (denom_spent_total - denom_received_total) / max(denom_spent_total, denom_received_total) * 100
                
    @property
    def buy_sell_ratio(self) -> float:
        return self.buy_sell_ratio_block_range(from_block=MIN_UINT256, to_block=MAX_UINT256)
    
    # endregion
    ####

    def to_dict(self) -> Dict[str, Any]:
        """Convert all metrics to dictionary"""
        return {
            'num_unique_addresses': self.num_unique_addresses,
            'num_buys': self.num_buys,
            'num_sells': self.num_sells,
            'num_buys_to_sell_ratio': self.num_buys_to_sell_ratio,
            'num_unique_buyers': self.num_unique_buyers,
            'num_unique_sellers': self.num_unique_sellers,
            'total_denom_in': self.total_denom_in,
            'total_denom_out': self.total_denom_out,
            'avg_denom_in': self.avg_denom_in,
            'avg_denom_out': self.avg_denom_out,
            'denom_in_out_ratio': self.total_denom_in_to_total_denom_out_ratio,
            'max_price_ratio': self.max_price_ratio,
            'min_price_ratio': self.min_price_ratio,
            'current_price_ratio': self.current_price_ratio,
            'current_liquidity': self.current_liquidity,
            'init_liquidity': self.initial_liquidity,
            'init_liquidity_add_token_amount': self.initial_liquidity_add_token_amount,
            'init_liquidity_add_token_fraction': self.initial_liquidity_add_token_fraction,
            'funders': self.funders,            
        }