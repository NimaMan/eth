import numpy as np
import pandas as pd
from eth_data.chain_utils.common_addresses import addresses_by_name


class UniV2PairSyncData:

    def __init__(self, syncs):
        self.syncs = syncs

    def to_dataframe(self) -> pd.DataFrame:
        syncs_df = pd.DataFrame(self.syncs)
        if not syncs_df.empty:
            syncs_df = syncs_df.sort_values(by=['block_number', 'tx_index', 'log_index']).reset_index(drop=True)
            syncs_df = self._add_additional_columns(syncs_df)
            syncs_df = syncs_df.set_index('tx_hash')
        return syncs_df

    def _add_additional_columns(self, syncs_df: pd.DataFrame, ) -> pd.DataFrame:
        """
        Adds additional columns to the sync data DataFrame.
        The columns are shifted by 1 such that the price represents the price at the previous block, i.e., the price for which the buy/sell happened.

        Args:
            price_df (pd.DataFrame): The DataFrame containing the sync data.

        Returns:
            pd.DataFrame: The price DataFrame.
        """
        
        # Calculate price and relative price
        syncs_df['price'] = syncs_df['denom_reserve'] / syncs_df['token_reserve']
        syncs_df['relative_price'] = syncs_df['price'] / syncs_df['price'].iloc[0]

        # Shift price and relative price columns
        syncs_df['price'] = syncs_df['price'].shift(1)
        syncs_df['relative_price'] = syncs_df['relative_price'].shift(1)
        # syncs_df['price'].shift(1, inplace=True)
        # syncs_df['relative_price'].shift(1, inplace=True)

        tmp = -syncs_df['denom_reserve'].diff()
        syncs_df['denom_spent'] = tmp.clip(upper=0).abs()   # Denom to pair
        syncs_df['denom_received'] = tmp.clip(lower=0)      # Denom from pair

        tmp = -syncs_df['token_reserve'].diff()
        syncs_df['tokens_bought'] = tmp.clip(lower=0)        # Token from pair
        syncs_df['tokens_sold'] = tmp.clip(upper=0).abs()    # Token to pair

        # Fill the first row
        first_row = syncs_df.iloc[0]
        denom_reserve = first_row['denom_reserve']
        token_reserve = first_row['token_reserve']
        price = denom_reserve / token_reserve if token_reserve != 0 and not np.isnan(token_reserve) else np.nan

        syncs_df.at[0, 'price'] = price
        syncs_df.at[0, 'relative_price'] = 1
        syncs_df.at[0, 'denom_spent'] = denom_reserve
        syncs_df.at[0, 'denom_received'] = 0
        syncs_df.at[0, 'tokens_bought'] = 0
        syncs_df.at[0, 'tokens_sold'] = token_reserve

        # Add labels based on transaction types
        conditions = [
            (syncs_df['denom_spent'] > 0) & (syncs_df['tokens_bought'] > 0),     # Buy (denom to pair, token from pair)
            (syncs_df['denom_received'] > 0) & (syncs_df['tokens_sold'] > 0),    # Sell (denom from pair, token to pair)
            (syncs_df['denom_spent'] > 0) & (syncs_df['tokens_sold'] > 0),       # Add liquidity (denom to pair, token to pair)
            (syncs_df['denom_received'] > 0) & (syncs_df['tokens_bought'] > 0)   # Remove liquidity (denom from pair, token from pair)
        ]
        labels = ['Buy', 'Sell', 'Add Liquidity', 'Remove Liquidity']
        syncs_df['action'] = np.select(conditions, labels, default='Unknown')

        def contains_buy_and_sell(actions: pd.Series):
            unique_actions = actions.unique()
            return 'Buy' in unique_actions and 'Sell' in unique_actions

        # syncs_df['sandwich_attack'] = syncs_df.groupby(['block', 'from_address'])['action'].transform(contains_buy_and_sell) & syncs_df.groupby(['block', 'from_address'])['tx_hash'].transform(lambda x: x.nunique() > 1)

        # Compute the groupby once
        grouped = syncs_df.groupby(['block_number', 'from_address'])

        # Compute the unique actions and tx counts once
        unique_actions = grouped['action'].apply(contains_buy_and_sell).reset_index(name='unique_actions')
        tx_counts = grouped['tx_hash'].nunique().reset_index(name='tx_counts')

        # Merge these new dataframes with the original dataframe
        syncs_df = pd.merge(syncs_df, unique_actions, on=['block_number', 'from_address'])
        syncs_df = pd.merge(syncs_df, tx_counts, on=['block_number', 'from_address'])

        # Compute the sandwich_attack_ column
        syncs_df['sandwich_attack'] = syncs_df['unique_actions'] & (syncs_df['tx_counts'] > 1)

        # Drop the intermediate columns
        syncs_df.drop(columns=['unique_actions', 'tx_counts'], inplace=True)

        return syncs_df
    

class ERC20TokenApprovalData:
    def __init__(self, approvals):
        self.approvals = approvals

    def to_dataframe(self) -> pd.DataFrame:
        df = pd.DataFrame(self.approvals)
        if not df.empty:
            df = df.sort_values(by=['block_number', 'index', 'log_index']).reset_index(drop=True)
            df = df.set_index('tx_hash')
        return df
    
    def get_spender_label(self, spender: str) -> str:
        if spender == addresses_by_name['UniswapV2Router02']:
            return 'Uniswap'
        if spender == addresses_by_name['Banana Gun']:
            return 'Banana'
        if spender == addresses_by_name['Maestro: Router 2']:
            return 'Maestro'
        if spender == addresses_by_name['Unibot']:
            return 'Unibot'
        if spender == addresses_by_name['Sigma / Alphaman']:
            return 'Sigma / Alphaman'
        return 'Other'
    
