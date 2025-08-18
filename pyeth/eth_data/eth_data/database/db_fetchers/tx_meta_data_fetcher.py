# py/sarigoz/sarigoz/data/fetchers/tx_meta_data_fetcher.py

"""
Postgres Transaction Data Fetcher
---------------------------------

This module provides a class to fetch transaction-related data specifically
from the PostgreSQL database defined by the schema in eth_db_data_models.py.
It encapsulates SQLAlchemy queries (using text() for potential performance)
for retrieving transaction hashes associated with addresses and block numbers
associated with transaction hashes.
"""

import logging
from typing import List, Dict, Optional, Tuple

# Use text for raw SQL execution
from sqlalchemy import select, text
from sqlalchemy.orm import Session, sessionmaker
from sqlalchemy.engine import Engine

# Assuming your models are accessible like this
from eth_data.database.schema.eth_db_data_models import Address # Still needed for address_id lookup
from eth_data.database.eth_db_conn import get_db_engine, get_db_session_maker # Added get_db_session


class TxMetaDataFetcher: # Renamed class
    """
    Fetches transaction data from the PostgreSQL database (eth_db) using raw SQL via text().
    """

    def __init__(self, logger=None):
        """
        Initializes the fetcher.

        Args:
            logger: An optional logger instance.
        """
        # Use get_db_session for session management
        self.Session = get_db_session_maker(db='eth_db')
        self.logger = logger or logging.getLogger(__name__)

    def get_tx_hashes_and_blocks_for_address(
        self,
        address_str: str,
        start_block: Optional[int] = None,
        end_block: Optional[int] = None,
        num_blocks: Optional[int] = None
    ) -> List[Tuple[str, int]]:
        """
        Retrieves a list of (transaction hash, block number) tuples associated
        with a given address. Optionally filters by block range and/or limits
        to the latest 'num_blocks' containing transactions for the address.

        Args:
            address_str: The checksummed Ethereum address string.
            start_block: Optional starting block number (inclusive).
            end_block: Optional ending block number (inclusive).
            num_blocks: Optional. If provided, limit results to transactions
                        within the 'num_blocks' most recent blocks associated
                        with the address (respecting start/end block filters).

        Returns:
            A list of (transaction_hash, block_number) tuples, ordered by block number.
            Returns an empty list if the address is not found or on error.
        """
        tx_data = []
        try:
            with self.Session() as session:
                # 1. Find the address_id
                address_id_query = select(Address.address_id).where(Address.address == address_str)
                address_id_result = session.execute(address_id_query).scalar_one_or_none()

                if address_id_result is None:
                    self.logger.warning(f"Address {address_str} not found in the database.")
                    return []
                address_id = address_id_result

                # 2. Build the main SQL query and parameters
                params = {"address_id": address_id}
                sql_query = """
                    SELECT t.tx_hash, t.block_number
                    FROM eth_db.transactions t
                    JOIN eth_db.tx_participants tp ON t.tx_hash = tp.tx_hash
                    WHERE tp.address_id = :address_id
                """

                # Add block range conditions
                block_conditions = "" # For use in subquery
                if start_block is not None:
                    block_conditions += " AND sub_t.block_number >= :start_block" # For subquery
                    sql_query += " AND t.block_number >= :start_block" # For main query
                    params["start_block"] = start_block
                if end_block is not None:
                    block_conditions += " AND sub_t.block_number <= :end_block" # For subquery
                    sql_query += " AND t.block_number <= :end_block" # For main query
                    params["end_block"] = end_block

                # Add subquery to filter by latest N blocks IF num_blocks is provided
                if num_blocks is not None and num_blocks > 0:
                    sql_query += f"""
                        AND t.block_number IN (
                            SELECT DISTINCT sub_t.block_number
                            FROM eth_db.transactions sub_t
                            JOIN eth_db.tx_participants sub_tp ON sub_t.tx_hash = sub_tp.tx_hash
                            WHERE sub_tp.address_id = :address_id
                              {block_conditions} -- Apply range filters here too
                            ORDER BY sub_t.block_number DESC
                            LIMIT :num_blocks
                        )
                    """
                    params["num_blocks"] = num_blocks
                    # Note: start/end block params are already in 'params' if needed

                sql_query += " ORDER BY t.block_number" # Final ordering

                # Execute using text()
                results = session.execute(text(sql_query), params).fetchall()
                # Create list of tuples directly from results
                tx_data = [(row[0], row[1]) for row in results if row[1] is not None] # Ensure block number is not null

        except Exception as e:
            self.logger.error(f"Error fetching tx hashes and blocks for address {address_str}: {e}", exc_info=True)
            # Return empty list on error

        return tx_data

    def get_block_number_for_tx_hashes(
        self,
        tx_hashes: List[str]
    ) -> Dict[str, int]:
        """
        Retrieves the block number for each transaction hash in the provided list
        using a raw SQL query.

        Args:
            tx_hashes: A list of transaction hash strings.

        Returns:
            A dictionary mapping transaction hashes to their block numbers.
            Hashes not found in the database will be omitted.
        """
        if not tx_hashes:
            return {}

        tx_to_block_map: Dict[str, int] = {}
        try:
            with self.Session() as session:
                sql_query = """
                    SELECT tx_hash, block_number
                    FROM eth_db.transactions
                    WHERE tx_hash IN :tx_hashes
                """
                params = {"tx_hashes": tuple(tx_hashes)}
                results = session.execute(text(sql_query), params).fetchall()
                for tx_hash, block_number in results:
                    if block_number is not None:
                        tx_to_block_map[tx_hash] = block_number
        except Exception as e:
            self.logger.error(f"Error fetching block numbers for transactions: {e}", exc_info=True)
        return tx_to_block_map

    # --- Add back the other methods from the original file --- 
    def get_transaction_by_hash(self, tx_hash):
        """
        Get details about a specific transaction.
        
        Args:
            tx_hash: Transaction hash
            
        Returns:
            dict: Transaction details or None
        """
        # Ensure Session is imported and self.Session is initialized correctly
        from datetime import datetime
        with self.Session() as session:
            query = text("""
                SELECT 
                    t.tx_hash,
                    t.block_number,
                    a_from.address as from_address, -- Join to get address string
                    a_to.address as to_address,     -- Join to get address string
                    t.value,
                    t.status,
                    b.block_timestamp
                FROM 
                    eth_db.transactions t
                LEFT JOIN eth_db.addresses a_from ON t.from_address_id = a_from.address_id
                LEFT JOIN eth_db.addresses a_to ON t.to_address_id = a_to.address_id 
                LEFT JOIN eth_db.blocks b ON t.block_number = b.block_number -- Assuming block timestamp is in blocks table
                WHERE 
                    t.tx_hash = :tx_hash
            """)

            result = session.execute(query, {"tx_hash": tx_hash})
            row = result.fetchone()
            if row:
                 # Using _mapping for potential robustness with different result types
                row_map = row._mapping if hasattr(row, '_mapping') else dict(zip(result.keys(), row))
                return {
                    "tx_hash": row_map.get('tx_hash'),
                    "block_number": row_map.get('block_number'),
                    "from_address": row_map.get('from_address'),
                    "to_address": row_map.get('to_address'),
                    "value": row_map.get('value'),
                    "status": row_map.get('status'),
                    "timestamp": datetime.fromtimestamp(row_map['block_timestamp']) if row_map.get('block_timestamp') else None
                }
            return None

    def get_transaction_participants(self, tx_hash):
        """
        Get all participants involved in a transaction.
        
        Args:
            tx_hash: Transaction hash
            
        Returns:
            list: Addresses involved in the transaction with their details
        """
        # Ensure TxParticipant and Address models are imported
        with self.Session() as session:
            query = text("""
                SELECT 
                    a.address,
                    a.is_contract,
                    a.total_realized_profit,
                    a.total_volume,
                    a.scam_ratio
                FROM 
                    eth_db.tx_participants tp
                JOIN
                    eth_db.addresses a ON tp.address_id = a.address_id -- Join based on address_id
                WHERE 
                    tp.tx_hash = :tx_hash
            """)

            result = session.execute(query, {"tx_hash": tx_hash})
            participants = []
            for row in result.fetchall():
                row_map = row._mapping if hasattr(row, '_mapping') else dict(zip(result.keys(), row))
                participants.append({
                    "address": row_map.get('address'),
                    "is_contract": row_map.get('is_contract'),
                    "total_realized_profit": row_map.get('total_realized_profit'),
                    "total_volume": row_map.get('total_volume'),
                    "scam_ratio": row_map.get('scam_ratio')
                })
            return participants

    def get_transactions_by_block(self, block_number, limit=100, offset=0):
        """
        Get transactions in a specific block.
        
        Args:
            block_number: Block number
            limit: Maximum number of transactions to return
            offset: Number of transactions to skip
            
        Returns:
            pandas.DataFrame: Transactions in the block
        """
        import pandas as pd # Import pandas locally
        from datetime import datetime
        with self.Session() as session:
            query = text("""
                SELECT 
                    t.tx_hash,
                    a_from.address as from_address,
                    a_to.address as to_address,
                    t.value,
                    t.status,
                    b.block_timestamp
                FROM 
                    eth_db.transactions t
                LEFT JOIN eth_db.addresses a_from ON t.from_address_id = a_from.address_id
                LEFT JOIN eth_db.addresses a_to ON t.to_address_id = a_to.address_id
                LEFT JOIN eth_db.blocks b ON t.block_number = b.block_number
                WHERE 
                    t.block_number = :block_number
                ORDER BY 
                    t.tx_index -- Assuming tx_index exists for ordering within block
                LIMIT :limit OFFSET :offset
            """)

            result = session.execute(
                query, 
                {"block_number": block_number, "limit": limit, "offset": offset}
            )
            
            columns = ['tx_hash', 'from_address', 'to_address', 'value', 'status', 'timestamp']
            df = pd.DataFrame(result.fetchall(), columns=columns)
            
            if 'timestamp' in df.columns and not df.empty:
                df['timestamp'] = pd.to_datetime(df['timestamp'], unit='s', errors='coerce')
                
            return df

    def get_transactions_by_timeframe(self, start_date=None, end_date=None, limit=100, offset=0):
        """
        Get transactions within a specific time period.
        
        Args:
            start_date: Optional start date (datetime)
            end_date: Optional end date (datetime)
            limit: Maximum number of transactions to return
            offset: Number of transactions to skip
            
        Returns:
            pandas.DataFrame: Transactions in the time period
        """
        import pandas as pd
        from datetime import datetime
        start_timestamp = int(start_date.timestamp()) if start_date else None
        end_timestamp = int(end_date.timestamp()) if end_date else None
        
        with self.Session() as session:
            query = text("""
                SELECT 
                    t.tx_hash,
                    t.block_number,
                    a_from.address as from_address,
                    a_to.address as to_address,
                    t.value,
                    t.status,
                    b.block_timestamp
                FROM 
                    eth_db.transactions t
                LEFT JOIN eth_db.addresses a_from ON t.from_address_id = a_from.address_id
                LEFT JOIN eth_db.addresses a_to ON t.to_address_id = a_to.address_id
                LEFT JOIN eth_db.blocks b ON t.block_number = b.block_number
                WHERE 
                    (:start_timestamp IS NULL OR b.block_timestamp >= :start_timestamp)
                    AND (:end_timestamp IS NULL OR b.block_timestamp <= :end_timestamp)
                ORDER BY 
                    b.block_timestamp DESC, t.tx_index -- Added tx_index for consistent ordering
                LIMIT :limit OFFSET :offset
            """)

            result = session.execute(
                query, 
                {"start_timestamp": start_timestamp, "end_timestamp": end_timestamp, "limit": limit, "offset": offset}
            )
            
            columns = ['tx_hash', 'block_number', 'from_address', 'to_address', 'value', 'status', 'timestamp']
            df = pd.DataFrame(result.fetchall(), columns=columns)
            
            if 'timestamp' in df.columns and not df.empty:
                df['timestamp'] = pd.to_datetime(df['timestamp'], unit='s', errors='coerce')
                
            return df

    def get_transactions_by_address_pair(self, from_address, to_address, limit=100, offset=0):
        """
        Get transactions between two addresses.
        
        Args:
            from_address: Sender address string
            to_address: Recipient address string
            limit: Maximum number of transactions to return
            offset: Number of transactions to skip
            
        Returns:
            pandas.DataFrame: Transactions between the two addresses
        """
        import pandas as pd
        from datetime import datetime
        with self.Session() as session:
             # Find address IDs first
            from_id_q = select(Address.address_id).where(Address.address == from_address)
            to_id_q = select(Address.address_id).where(Address.address == to_address)
            from_id = session.execute(from_id_q).scalar_one_or_none()
            to_id = session.execute(to_id_q).scalar_one_or_none()

            if from_id is None or to_id is None:
                 self.logger.warning(f"Could not find DB entry for one or both addresses: {from_address}, {to_address}")
                 return pd.DataFrame(columns=['tx_hash', 'block_number', 'value', 'status', 'timestamp'])

            query = text("""
                SELECT 
                    t.tx_hash,
                    t.block_number,
                    t.value,
                    t.status,
                    b.block_timestamp
                FROM 
                    eth_db.transactions t
                LEFT JOIN eth_db.blocks b ON t.block_number = b.block_number
                WHERE 
                    t.from_address_id = :from_id 
                    AND t.to_address_id = :to_id
                ORDER BY 
                    b.block_timestamp DESC, t.tx_index
                LIMIT :limit OFFSET :offset
            """)

            result = session.execute(
                query, 
                {"from_id": from_id, "to_id": to_id, "limit": limit, "offset": offset}
            )
            
            columns = ['tx_hash', 'block_number', 'value', 'status', 'timestamp']
            df = pd.DataFrame(result.fetchall(), columns=columns)
            
            if 'timestamp' in df.columns and not df.empty:
                df['timestamp'] = pd.to_datetime(df['timestamp'], unit='s', errors='coerce')
                
            return df