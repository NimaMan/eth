
import numpy as np
from functools import cached_property
from collections import defaultdict

# ignore warnings
import warnings
warnings.filterwarnings("ignore")


class UserTokenActivityTracker:
    def __init__(self, init_denom_balance=0, 
                 init_token_balance=0, 
                 entry_block=None, 
                 entry_index=None,
                 entry_log_index=None, 
                 token_data=None,
                 address=None,
                 address_type=None,
                 is_fee_source=None,
                 fee_source=None):
        self.token_data = token_data
        self.entry_block = entry_block
        self.entry_index = entry_index
        self.entry_log_index = entry_log_index
        self.token_in_dict = {} if init_token_balance <= 0 else {(entry_block, entry_index, entry_log_index): init_token_balance}
        self.token_out_dict = {} if init_token_balance >= 0 else {(entry_block, entry_index, entry_log_index): -init_token_balance}
        self.denom_in_dict = {} if init_denom_balance <= 0 else {(entry_block, entry_index, entry_log_index): init_denom_balance}
        self.denom_out_dict = {} if init_denom_balance >= 0 else {(entry_block, entry_index, entry_log_index): -init_denom_balance}
        
        self.got_from = []
        self.sent_to = []
        self.bribe_amount = 0
        self.txn_fees = []

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
            f"num_txn={self.num_txn}, "
            f"address_type={self.address_type}, "
        )
    
    @cached_property
    def all_token_movements(self):
        '''
        Returns a list of all token movements. 
        Add the token_in_dict and negative of token_out_dict
        '''
        all_movements = self.token_in_dict.copy()
        for key, val in self.token_out_dict.items():            
            if key in all_movements:
                all_movements[key] -= val
            else:
                all_movements[key] = -val
        return all_movements
    
    @cached_property
    def all_denom_movements(self):
        '''
        Returns a list of all denom movements.  Add the denom_in_dict and negative of denom_out_dict
        '''
        all_movements = self.denom_in_dict.copy()
        for key, val in self.denom_out_dict.items():
            if key in all_movements:
                all_movements[key] -= val
            else:
                all_movements[key] = -val
        return  all_movements
    
    def _net_movements(self, movements):
        net = defaultdict(float)
        for (block, txn_index, _), val in movements.items():
            net[(block, txn_index)] += val
        return net    

    @cached_property
    def token_got_list(self):
        grouped_movements = self._net_movements(self.all_token_movements)
        return [val for val in grouped_movements.values() if val > 0]

    @cached_property
    def token_sent_list(self):
        grouped_movements = self._net_movements(self.all_token_movements)
        return [-val for val in grouped_movements.values() if val < 0]

    @cached_property
    def denom_got_list(self):
        grouped_movements = self._net_movements(self.all_denom_movements)
        return [val for val in grouped_movements.values() if val > 0]

    @cached_property
    def denom_sent_list(self):
        grouped_movements = self._net_movements(self.all_denom_movements)
        return [-val for val in grouped_movements.values() if val < 0]
    
    @property
    def token_latest_price(self):
        denom_reserve = self.token_data.price_df.iloc[-1].denom_reserve
        token_reserve = self.token_data.price_df.iloc[-1].token_reserve
        # check if token reserve is number or not 
        if self.token_data.is_scam:
            return 0
        return denom_reserve / token_reserve
    
    @property
    def token_balance(self):
        return np.sum(self.token_got_list) - np.sum(self.token_sent_list)
    
    @property
    def denom_balance(self):
        return np.sum(self.denom_got_list) - np.sum(self.denom_sent_list)
    
    @property
    def num_buys(self):
        return len(self.denom_sent_list)

    @property
    def num_sells(self):
        return len(self.denom_got_list)
    
    @property
    def num_txn(self):
        return self.num_buys + self.num_sells
    
    @property
    def total_denom_spent(self):
        return np.sum(self.denom_sent_list)
    
    @property
    def total_denom_received(self):
        return np.sum(self.denom_got_list)
    
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
        return self.token_holdings / self.token_data["total_supply"]

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
        return self.total_denom_received - self.total_denom_spent - self.bribe_amount
    
    @property
    def total_profit(self):
        return self.realized_profit + self.unrealized_profit
    
    def get_user_features(self):
        return {
            'entry_block': self.entry_block - self.token_data.trading_enabled_block,
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
            'num_txn': self.num_txn,
            'mean_buy': self.mean_buy,
            'buy_volatility': self.buy_volatility,
            'mean_sell': self.mean_sell,
            'sell_volatility': self.sell_volatility,            
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
            'contract_address': self.token_data["contract_address"],
        }

    def check_associated_cashflow(self, processed_addresses=None):
        """
        Recursively check the cashflow of associated addresses.
        
        :param processed_addresses: Set of addresses already processed to avoid circular references
        :return: Dictionary with summed attributes of associated addresses
        """
        if processed_addresses is None:
            processed_addresses = set()

        associated_attributes = defaultdict(float)
        for address, activity in self.associated_addresses.items():
            if address in processed_addresses:
                continue
            processed_addresses.add(address)

            # Sum up attributes
            associated_attributes['denom_balance'] += activity.denom_balance
            associated_attributes['token_balance'] += activity.token_balance
            associated_attributes['realized_profit'] += activity.realized_profit
            associated_attributes['unrealized_profit'] += activity.unrealized_profit
            associated_attributes['total_profit'] += activity.total_profit
            associated_attributes['total_denom_spent'] += activity.total_denom_spent
            associated_attributes['total_denom_received'] += activity.total_denom_received
            associated_attributes['denom_received_spent_ratio'] += activity.received_spent_ratio   

            # Recursively check associated addresses of this address
            if activity.associated_addresses:
                nested_attributes = activity.check_associated_cashflow(processed_addresses)
                for key, value in nested_attributes.items():
                    associated_attributes[key] += value

        return dict(associated_attributes)

    def get_total_cashflow(self):
        """
        Get the total cashflow including this address and all associated addresses.
        
        :return: Dictionary with total attributes
        """
        total_attributes = self.get_user_features()
        associated_attributes = self.check_associated_cashflow()

        for key, value in associated_attributes.items():
            if key in total_attributes:
                total_attributes[f'associated_{key}'] += value
            else:
                total_attributes[f'associated_{key}'] = value

        # Calculate ratios and other derived attributes
        if total_attributes['total_denom_spent'] > 0:
            total_attributes['associated_received_spent_ratio'] = total_attributes['total_denom_received'] / total_attributes['total_denom_spent']
        else:
            total_attributes['associated_received_spent_ratio'] = np.nan

        if total_attributes['total_token_bought'] > 0:
            total_attributes['associated_token_sell_buy_ratio'] = total_attributes['total_token_sold'] / total_attributes['total_token_bought']
        else:
            total_attributes['associated_token_sell_buy_ratio'] = np.nan

        return total_attributes

    def is_cashflow_balanced(self, tolerance=1e-10):
        """
        Check if the total cashflow (including associated addresses) is balanced.
        
        :param tolerance: Tolerance for floating-point comparison
        :return: Boolean indicating if cashflow is balanced
        """
        total_cashflow = self.get_total_cashflow()
        total_denom_balance = total_cashflow['denom_balance']
        return np.isclose(total_denom_balance, 0, atol=tolerance, rtol=0)

    def merge(self, other):
        """
        Merge another UserTokenActivity into this one.

        :param other: Another UserTokenActivity instance
        """
        if not isinstance(other, UserTokenActivityTracker):
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
        self.txn_fees.extend(other.txn_fees)

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
        if not isinstance(other, UserTokenActivityTracker):
            return NotImplemented
        
        merged = UserTokenActivityTracker(
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