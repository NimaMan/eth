"""
Transaction Metadata Fetcher
----------------------------

Fetches transaction-related metadata from PostgreSQL eth_db database.
Provides methods to retrieve transaction hashes, block mappings, and 
transaction details for addresses and blocks.

All methods return native Python dictionaries and lists (no pandas).
Logging uses eth_data.utils.logger for error handling only.
"""

from typing import List, Dict, Optional, Tuple
from datetime import datetime
from sqlalchemy import select, text

from eth_data.database.schema.eth_db_data_models import Address
from eth_data.database.eth_db_conn import get_db_session_maker


class TxMetaDataFetcher:
    """
    Fetches transaction metadata from PostgreSQL eth_db database.
    Returns native Python types (dicts/lists) for all operations.
    """

    def __init__(self, logger):
        """Initialize the fetcher with database connection and logger."""
        self.Session = get_db_session_maker(db='eth_db')
        self.logger = logger

    def get_tx_hashes_for_address(
        self,
        address: str,
        start_block: Optional[int] = None,
        end_block: Optional[int] = None,
        num_blocks: Optional[int] = None
    ) -> List[str]:
        """
        Get transaction hashes for an address with optional filters.
        
        Args:
            address: Checksummed Ethereum address
            start_block: Optional starting block number (inclusive)
            end_block: Optional ending block number (inclusive)  
            num_blocks: Optional limit to N most recent blocks
            
        Returns:
            List of transaction hashes
        """
        try:
            with self.Session() as session:
                # Find address_id
                address_id_query = select(Address.address_id).where(Address.address == address)
                address_id = session.execute(address_id_query).scalar_one_or_none()
                
                if address_id is None:
                    return []
                
                # Build query
                params = {"address_id": address_id}
                sql_query = """
                    SELECT DISTINCT t.tx_hash
                    FROM eth_db.transactions t
                    JOIN eth_db.tx_participants tp ON t.tx_hash = tp.tx_hash
                    WHERE tp.address_id = :address_id
                """
                
                # Add block filters
                if start_block is not None:
                    sql_query += " AND t.block_number >= :start_block"
                    params["start_block"] = start_block
                if end_block is not None:
                    sql_query += " AND t.block_number <= :end_block"
                    params["end_block"] = end_block
                    
                # Add num_blocks filter
                if num_blocks is not None and num_blocks > 0:
                    block_conditions = ""
                    if start_block is not None:
                        block_conditions += " AND sub_t.block_number >= :start_block"
                    if end_block is not None:
                        block_conditions += " AND sub_t.block_number <= :end_block"
                        
                    sql_query += f"""
                        AND t.block_number IN (
                            SELECT DISTINCT sub_t.block_number
                            FROM eth_db.transactions sub_t
                            JOIN eth_db.tx_participants sub_tp ON sub_t.tx_hash = sub_tp.tx_hash
                            WHERE sub_tp.address_id = :address_id
                              {block_conditions}
                            ORDER BY sub_t.block_number DESC
                            LIMIT :num_blocks
                        )
                    """
                    params["num_blocks"] = num_blocks
                
                sql_query += " ORDER BY t.block_number, t.tx_hash"
                
                results = session.execute(text(sql_query), params).fetchall()
                return [row[0] for row in results]
                
        except Exception as e:
            self.logger.error(f"Error fetching tx hashes for address {address}: {e}")
            return []

    def get_tx_hashes_from_block_number(self, block_number: int) -> List[str]:
        """
        Get all transaction hashes from a specific block.
        
        Args:
            block_number: Block number
            
        Returns:
            List of transaction hashes in the block
        """
        try:
            with self.Session() as session:
                sql_query = """
                    SELECT tx_hash
                    FROM eth_db.transactions
                    WHERE block_number = :block_number
                    ORDER BY tx_index
                """
                results = session.execute(text(sql_query), {"block_number": block_number}).fetchall()
                return [row[0] for row in results]
                
        except Exception as e:
            self.logger.error(f"Error fetching tx hashes for block {block_number}: {e}")
            return []

    def get_tx_hashes_and_blocks_for_address(
        self,
        address_str: str,
        start_block: Optional[int] = None,
        end_block: Optional[int] = None,
        num_blocks: Optional[int] = None
    ) -> List[Tuple[str, int]]:
        """
        Get (transaction_hash, block_number) tuples for an address.
        
        Args:
            address_str: Checksummed Ethereum address
            start_block: Optional starting block number (inclusive)
            end_block: Optional ending block number (inclusive)
            num_blocks: Optional limit to N most recent blocks
            
        Returns:
            List of (tx_hash, block_number) tuples
        """
        try:
            with self.Session() as session:
                # Find address_id
                address_id_query = select(Address.address_id).where(Address.address == address_str)
                address_id = session.execute(address_id_query).scalar_one_or_none()
                
                if address_id is None:
                    return []
                
                # Build query
                params = {"address_id": address_id}
                sql_query = """
                    SELECT t.tx_hash, t.block_number
                    FROM eth_db.transactions t
                    JOIN eth_db.tx_participants tp ON t.tx_hash = tp.tx_hash
                    WHERE tp.address_id = :address_id
                """
                
                # Add block filters
                if start_block is not None:
                    sql_query += " AND t.block_number >= :start_block"
                    params["start_block"] = start_block
                if end_block is not None:
                    sql_query += " AND t.block_number <= :end_block"
                    params["end_block"] = end_block
                    
                # Add num_blocks filter
                if num_blocks is not None and num_blocks > 0:
                    block_conditions = ""
                    if start_block is not None:
                        block_conditions += " AND sub_t.block_number >= :start_block"
                    if end_block is not None:
                        block_conditions += " AND sub_t.block_number <= :end_block"
                        
                    sql_query += f"""
                        AND t.block_number IN (
                            SELECT DISTINCT sub_t.block_number
                            FROM eth_db.transactions sub_t
                            JOIN eth_db.tx_participants sub_tp ON sub_t.tx_hash = sub_tp.tx_hash
                            WHERE sub_tp.address_id = :address_id
                              {block_conditions}
                            ORDER BY sub_t.block_number DESC
                            LIMIT :num_blocks
                        )
                    """
                    params["num_blocks"] = num_blocks
                
                sql_query += " ORDER BY t.block_number, t.tx_hash"
                
                results = session.execute(text(sql_query), params).fetchall()
                return [(row[0], row[1]) for row in results if row[1] is not None]
                
        except Exception as e:
            self.logger.error(f"Error fetching tx hashes and blocks for address {address_str}: {e}")
            return []

    def get_block_number_for_tx_hashes(self, tx_hashes: List[str]) -> Dict[str, int]:
        """
        Get block numbers for a list of transaction hashes.
        
        Args:
            tx_hashes: List of transaction hash strings
            
        Returns:
            Dict mapping tx_hash -> block_number
        """
        if not tx_hashes:
            return {}
            
        try:
            with self.Session() as session:
                sql_query = """
                    SELECT tx_hash, block_number
                    FROM eth_db.transactions
                    WHERE tx_hash IN :tx_hashes
                """
                params = {"tx_hashes": tuple(tx_hashes)}
                results = session.execute(text(sql_query), params).fetchall()
                return {tx_hash: block_number for tx_hash, block_number in results if block_number is not None}
                
        except Exception as e:
            self.logger.error(f"Error fetching block numbers for transactions: {e}")
            return {}

    def get_tx_meta_data_by_hash(self, tx_hash: str) -> Optional[Dict]:
        """
        Get basic transaction metadata by hash.
        
        Args:
            tx_hash: Transaction hash
            
        Returns:
            Dict with transaction details or None if not found
        """
        try:
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
                        t.tx_hash = :tx_hash
                """)
                
                result = session.execute(query, {"tx_hash": tx_hash})
                row = result.fetchone()
                if row:
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
                
        except Exception as e:
            self.logger.error(f"Error fetching transaction {tx_hash}: {e}")
            return None

    def get_tx_participants(self, tx_hash: str) -> List[Dict]:
        """
        Get all address participants for a transaction.
        
        Args:
            tx_hash: Transaction hash
            
        Returns:
            List of participant address details
        """
        try:
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
                        eth_db.addresses a ON tp.address_id = a.address_id
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
                
        except Exception as e:
            self.logger.error(f"Error fetching participants for transaction {tx_hash}: {e}")
            return []

    def get_tx_by_block(self, block_number: int, limit: int = 100, offset: int = 0) -> List[Dict]:
        """
        Get transactions in a specific block as list of dicts.
        
        Args:
            block_number: Block number
            limit: Maximum number of transactions
            offset: Number of transactions to skip
            
        Returns:
            List of transaction dicts
        """
        try:
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
                        t.tx_index
                    LIMIT :limit OFFSET :offset
                """)
                
                result = session.execute(
                    query, 
                    {"block_number": block_number, "limit": limit, "offset": offset}
                )
                
                transactions = []
                for row in result.fetchall():
                    row_map = row._mapping if hasattr(row, '_mapping') else dict(zip(result.keys(), row))
                    tx_dict = {
                        "tx_hash": row_map.get('tx_hash'),
                        "from_address": row_map.get('from_address'),
                        "to_address": row_map.get('to_address'),
                        "value": row_map.get('value'),
                        "status": row_map.get('status'),
                        "timestamp": datetime.fromtimestamp(row_map['block_timestamp']) if row_map.get('block_timestamp') else None
                    }
                    transactions.append(tx_dict)
                return transactions
                
        except Exception as e:
            self.logger.error(f"Error fetching transactions for block {block_number}: {e}")
            return []

    def get_tx_by_timeframe(
        self, 
        start_date: Optional[datetime] = None, 
        end_date: Optional[datetime] = None, 
        limit: int = 100, 
        offset: int = 0
    ) -> List[Dict]:
        """
        Get transactions within a time period as list of dicts.
        
        Args:
            start_date: Optional start date
            end_date: Optional end date
            limit: Maximum number of transactions
            offset: Number of transactions to skip
            
        Returns:
            List of transaction dicts
        """
        try:
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
                        b.block_timestamp DESC, t.tx_index
                    LIMIT :limit OFFSET :offset
                """)
                
                result = session.execute(
                    query, 
                    {
                        "start_timestamp": start_timestamp, 
                        "end_timestamp": end_timestamp, 
                        "limit": limit, 
                        "offset": offset
                    }
                )
                
                transactions = []
                for row in result.fetchall():
                    row_map = row._mapping if hasattr(row, '_mapping') else dict(zip(result.keys(), row))
                    tx_dict = {
                        "tx_hash": row_map.get('tx_hash'),
                        "block_number": row_map.get('block_number'),
                        "from_address": row_map.get('from_address'),
                        "to_address": row_map.get('to_address'),
                        "value": row_map.get('value'),
                        "status": row_map.get('status'),
                        "timestamp": datetime.fromtimestamp(row_map['block_timestamp']) if row_map.get('block_timestamp') else None
                    }
                    transactions.append(tx_dict)
                return transactions
                
        except Exception as e:
            self.logger.error(f"Error fetching transactions by timeframe: {e}")
            return []

    def get_tx_by_address_pair(
        self, 
        from_address: str, 
        to_address: str, 
        limit: int = 100, 
        offset: int = 0
    ) -> List[Dict]:
        """
        Get transactions between two specific addresses as list of dicts.
        
        Args:
            from_address: Sender address
            to_address: Recipient address
            limit: Maximum number of transactions
            offset: Number of transactions to skip
            
        Returns:
            List of transaction dicts
        """
        try:
            with self.Session() as session:
                # Find address IDs
                from_id_q = select(Address.address_id).where(Address.address == from_address)
                to_id_q = select(Address.address_id).where(Address.address == to_address)
                from_id = session.execute(from_id_q).scalar_one_or_none()
                to_id = session.execute(to_id_q).scalar_one_or_none()
                
                if from_id is None or to_id is None:
                    return []
                
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
                
                transactions = []
                for row in result.fetchall():
                    row_map = row._mapping if hasattr(row, '_mapping') else dict(zip(result.keys(), row))
                    tx_dict = {
                        "tx_hash": row_map.get('tx_hash'),
                        "block_number": row_map.get('block_number'),
                        "value": row_map.get('value'),
                        "status": row_map.get('status'),
                        "timestamp": datetime.fromtimestamp(row_map['block_timestamp']) if row_map.get('block_timestamp') else None
                    }
                    transactions.append(tx_dict)
                return transactions
                
        except Exception as e:
            self.logger.error(f"Error fetching transactions between {from_address} and {to_address}: {e}")
            return []