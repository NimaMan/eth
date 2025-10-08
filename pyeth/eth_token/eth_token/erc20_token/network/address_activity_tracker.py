import numpy as np
from collections import defaultdict
from eth_token.erc20_token.data.erc20_token_data import ERC20TokenData

# ignore warnings
import warnings
warnings.filterwarnings("ignore")


class AddressTokenActivityTracker:
    def __init__(self, init_denom_balance=0, 
                 init_token_balance=0, 
                 entry_block=None, 
                 entry_index=None,
                 entry_log_index=None, 
                 latest_block=None,
                 token_data:ERC20TokenData=None,
                 address=None,
                 address_type=None,
                 is_fee_source=None,
                 fee_source=None, 
                 init_tx_fee=None):
        self.token_data = token_data
        self.entry_block = entry_block
        self.latest_block = latest_block
        self.entry_index = entry_index
        self.entry_log_index = entry_log_index
        self.token_in_dict = {} if init_token_balance <= 0 else {(entry_block, entry_index, entry_log_index): init_token_balance}
        self.token_out_dict = {} if init_token_balance >= 0 else {(entry_block, entry_index, entry_log_index): -init_token_balance}
        self.denom_in_dict = {} if init_denom_balance <= 0 else {(entry_block, entry_index, entry_log_index): init_denom_balance}
        self.denom_out_dict = {} if init_denom_balance >= 0 else {(entry_block, entry_index, entry_log_index): -init_denom_balance}
        
        self.got_from = []
        self.sent_to = []
        self.bribe_amount = 0
        self.tx_fees = [] if init_tx_fee is None else [init_tx_fee]

        self.address = address
        self.address_type = address_type
        self.user_is_fee_source = is_fee_source
        self.fee_source = fee_source
        self.associated_addresses = {}

    def __getitem__(self, address):
        """
        Get the associated user activity by address using dictionary-like access.
        
        :param address: The address of the associated user
        :return: The associated user activity or None if not found
        """
        return self.associated_addresses.get(address, None)

    def __repr__(self):
        """
        Return a string representation of the UserTokenActivity object.
        """
        return (
            f"denom_balance={self.denom_balance:.2f}, "
            f"token_balance={self.token_balance:.2f}, "
            f"realized_profit={self.realized_profit:.2f}, "
            f"num_tx={self.num_tx}, "
            f"address_type={self.address_type}, "
        )
    
    @property
    def all_token_movements(self):
        '''
        Returns a list of all token movements. 
        Add the token_in_dict and negative of token_out_dict
        '''
        all_movements = self.token_in_dict.copy()
        for key, val in self.token_out_dict.copy().items():            
            if key in all_movements:
                all_movements[key] -= val
            else:
                all_movements[key] = -val
        return all_movements
    
    @property
    def all_denom_movements(self):
        '''
        Returns a list of all denom movements.  Add the denom_in_dict and negative of denom_out_dict
        '''
        all_movements = self.denom_in_dict.copy()
        for key, val in self.denom_out_dict.copy().items():
            if key in all_movements:
                all_movements[key] -= val
            else:
                all_movements[key] = -val
        return  all_movements
    
    def _net_movements(self, movements):
        net = defaultdict(float)
        for (block, tx_index, _), val in movements.items():
            net[(block, tx_index)] += val
        return net    
    
    @property
    def token_got_list(self):
        grouped_movements = self._net_movements(self.all_token_movements)
        return [val for val in grouped_movements.values() if val > 0]

    @property
    def token_sent_list(self):
        grouped_movements = self._net_movements(self.all_token_movements)
        return [-val for val in grouped_movements.values() if val < 0]

    @property
    def denom_got_list(self):
        grouped_movements = self._net_movements(self.all_denom_movements)
        return [val for val in grouped_movements.values() if val > 0]

    @property
    def denom_sent_list(self):
        grouped_movements = self._net_movements(self.all_denom_movements)
        return [-val for val in grouped_movements.values() if val < 0]
    
    @property
    def token_latest_price(self):
        pool_snapshot, best_price = self.token_data.liquidity_analyzer.get_best_price(for_buy=True)
        return best_price if best_price is not None else 0.0
        
    @property
    def token_balance(self):
        balance = np.sum(self.token_got_list) - np.sum(self.token_sent_list)
        #round it to zero if it is less than 0.000001
        return 0 if abs(balance) < 0.000001 else balance
      
    @property
    def num_buys(self):
        return len(self.denom_sent_list)

    @property
    def num_sells(self):
        return len(self.denom_got_list)
    
    @property
    def num_tx(self):
        return self.num_buys + self.num_sells
    
    @property
    def total_denom_spent(self):
        return np.sum(self.denom_sent_list)
    
    @property
    def total_denom_received(self):
        return np.sum(self.denom_got_list)
    
    @property
    def denom_balance(self):
        return self.total_denom_received - self.total_denom_spent
  
    @property
    def total_token_bought(self):
        return np.sum(self.token_got_list)
    
    @property
    def total_token_sold(self):
        return np.sum(self.token_sent_list)
    
    @property
    def token_sell_buy_ratio(self):
        return self.total_token_sold / self.total_token_bought if self.total_token_bought > 0 else np.nan
    
    @property
    def received_spent_ratio(self):
        return self.total_denom_received / self.total_denom_spent if self.total_denom_spent > 0 else np.nan
    
    @property
    def mean_buy(self):
        return self.total_denom_spent / self.num_buys if self.num_buys > 0 else np.nan
    
    @property
    def buy_volatility(self):
        if self.num_buys > 1:
            return np.std(self.denom_got_list)
        return np.nan
    
    @property
    def mean_sell(self):
        return self.total_denom_received / self.num_sells if self.num_sells > 0 else np.nan
    
    @property
    def sell_volatility(self):
        if self.num_sells > 1:
            return np.std(self.denom_got_list)
        return np.nan
    
    @property
    def token_holdings(self):
        return self.token_balance
    
    @property
    def token_holdings_to_total_supply_ratio(self):
        return self.token_holdings / self.token_data.total_supply

    @property  
    def token_holdings_ratio(self):
        return self.token_holdings / self.total_token_bought if self.total_token_bought > 0 else np.nan
    
    @property
    def holdings_value(self):
        return self.token_holdings * self.token_latest_price

    @property
    def unrealized_profit(self):
        return self.holdings_value
    
    @property
    def realized_profit(self):
        return self.denom_balance - self.bribe_amount - self.total_tx_fees
    
    @property
    def total_profit(self):
        return self.realized_profit + self.unrealized_profit
    
    @property
    def total_tx_fees(self):
        return np.sum(self.tx_fees)
    
    def get_user_features(self):
        return {
            'entry_block': self.entry_block,
            'latest_block': self.latest_block,
            'denom_balance': self.denom_balance,
            "bribe_amount": self.bribe_amount,
            'denom_received_spent_ratio': self.received_spent_ratio,
            'token_sell_buy_ratio': self.token_sell_buy_ratio,
            'realized_profit': self.realized_profit,
            'total_denom_spent': self.total_denom_spent,
            'total_denom_received': self.total_denom_received,
            'unrealized_profit': self.unrealized_profit,
            'total_profit': self.total_profit,
            'num_buys': self.num_buys,
            'num_sells': self.num_sells,
            'num_tx': self.num_tx,
            'mean_buy': self.mean_buy,
            #'buy_volatility': self.buy_volatility,
            'mean_sell': self.mean_sell,
            #'sell_volatility': self.sell_volatility,            
            'total_token_bought': self.total_token_bought,
            'total_token_sold': self.total_token_sold,
            'token_holdings': self.token_holdings,
            'token_holdings_to_total_supply_ratio': self.token_holdings_to_total_supply_ratio,
            'token_holdings_ratio': self.token_holdings_ratio,
            'is_fee_source': self.user_is_fee_source,
            #'fee_source': self.fee_source,
            'address_type': self.address_type,
            'address': self.address,
            'is_scam': self.token_data.is_scam,
            'scam_label': self.token_data.scam_label,
            'total_tx_fees': self.total_tx_fees,
            'contract_address': self.token_data.contract_address,
        }

    def merge(self, other):
        """
        Merge another UserTokenActivity into this one.

        :param other: Another UserTokenActivity instance
        """
        if not isinstance(other, AddressTokenActivityTracker):
            raise ValueError("Can only merge with another UserTokenActivity instance.")

        # Merge token movements
        self.token_in_dict.update(other.token_in_dict)
        self.token_out_dict.update(other.token_out_dict)
        self.denom_in_dict.update(other.denom_in_dict)
        self.denom_out_dict.update(other.denom_out_dict)

        # Merge other attributes
        self.got_from.extend(other.got_from)
        self.sent_to.extend(other.sent_to)
        self.bribe_amount += other.bribe_amount
        self.tx_fees.extend(other.tx_fees)

        # Merge associated addresses
        for addr, activity in other.associated_addresses.items():
            if addr in self.associated_addresses:
                self.associated_addresses[addr].merge(activity)
            else:
                self.associated_addresses[addr] = activity

    def __add__(self, other):
        """
        Override the + operator to merge two UserTokenActivity instances.

        :param other: Another UserTokenActivity instance
        :return: A new merged UserTokenActivity instance
        """
        if not isinstance(other, AddressTokenActivityTracker):
            return NotImplemented
        
        merged = AddressTokenActivityTracker(
            init_denom_balance=self.denom_balance + other.denom_balance,
            init_token_balance=self.token_balance + other.token_balance,
            entry_block=min(self.entry_block, other.entry_block),
            entry_index=None,
            entry_log_index=None,
            token_data=self.token_data,
            address_type="Aggregated",
            associated_addresses=self.associated_addresses | other.associated_addresses
        )

        merged.merge(self)
        merged.merge(other)
        return merged