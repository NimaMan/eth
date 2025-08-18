"""
Token Data Retriever Module

Objective:
---------
Provide methods for retrieving token-related data from the database,
including token metadata, token trades, creation information,
and scam detection metrics.

Key Features:
-----------
1. Token Metadata: Get basic information about a token
2. Token Trades: Find all trades for a specific token
3. Creation Data: Access information about token creation events
4. Scam Analysis: Retrieve scam status and related metrics
5. PnL Rankings: Get top profitable/unprofitable traders for a token
6. Token Launches: Track newly created tokens and their initial metrics
"""

from sqlalchemy import text, func
from eth_data.database.eth_db_conn import get_db_session_maker
import pandas as pd


class TokenDataFetcher:
    
    def __init__(self):
        self.Session = get_db_session_maker(db='eth_db')
    
    def get_token_metadata(self, token_address):
        """
        Get basic token information.
        
        Args:
            token_address: Token contract address
            
        Returns:
            dict: Token metadata including creator, creation txn, scam status
        """
        with self.Session() as session:
            query = text("""
                SELECT 
                    t.contract_address,
                    a.address as creator_address,
                    t.is_scam,
                    t.scam_label,
                    t.creation_txn,
                    t.trading_enabled_txn
                FROM 
                    eth_db.tokens t
                LEFT JOIN 
                    eth_db.addresses a ON t.creator_address_id = a.address_id
                WHERE 
                    t.contract_address = :token_address
            """)

            result = session.execute(
                query, 
                {"token_address": token_address}
            )
            
            row = result.fetchone()
            if row:
                return {
                    "contract_address": row[0],
                    "creator_address": row[1],
                    "is_scam": row[2],
                    "scam_label": row[3],
                    "creation_txn": row[4],
                    "trading_enabled_txn": row[5]
                }
            return None
    
    def get_token_trades(self, token_address, limit=100, offset=0):
        """
        Get all trades for a specific token.
        
        Args:
            token_address: Token contract address
            limit: Maximum number of trades to return
            offset: Number of trades to skip
            
        Returns:
            pandas.DataFrame: Trade data for the token
        """
        with self.Session() as session:
            query = text("""
                SELECT 
                    t.id,
                    t.address,
                    t.token_address,
                    t.currency,
                    t.entry_block,
                    t.total_denom_spent,
                    t.total_denom_received,
                    t.denom_received_spent_ratio,
                    t.bribe_amount,
                    t.realized_profit,
                    t.unrealized_profit,
                    t.num_buys,
                    t.num_sells,
                    t.token_holdings_ratio,
                    t.token_sell_buy_ratio,
                    t.agg_denom_balance,
                    t.agg_token_balance,
                    t.tx_fee,
                    a.is_contract
                FROM 
                    eth_db.trades t
                LEFT JOIN
                    eth_db.addresses a ON t.address = a.address
                WHERE 
                    t.token_address = :token_address
                ORDER BY 
                    t.entry_block DESC
                LIMIT :limit OFFSET :offset
            """)

            result = session.execute(
                query, 
                {
                    "token_address": token_address,
                    "limit": limit,
                    "offset": offset
                }
            )
            
            columns = ['id', 'address', 'token_address', 'currency', 'entry_block', 
                       'total_denom_spent', 'total_denom_received', 
                       'denom_received_spent_ratio', 'bribe_amount',
                       'realized_profit', 'unrealized_profit', 
                       'num_buys', 'num_sells', 'token_holdings_ratio',
                       'token_sell_buy_ratio', 'agg_denom_balance', 'agg_token_balance',
                       'tx_fee', 'is_contract']
            
            df = pd.DataFrame(result.fetchall(), columns=columns)
            return df
    