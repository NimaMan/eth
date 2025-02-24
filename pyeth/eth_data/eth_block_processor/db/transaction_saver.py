# File: eth_block_processor/eth_block_processor/db/transaction_saver.py

"""
Transaction Saver Module

Algorithmic Description:
-----------------------
This module defines a TransactionSaver class that handles saving processed transactions
to the database. It supports both individual and bulk transaction saving operations.

Key Components:
1. Session Management: Maintains a session factory for database operations
2. Bulk Operations: Efficiently saves multiple transactions in a single database transaction
3. Caching: Uses address caching to minimize database queries
4. Error Handling: Provides comprehensive error handling and logging
"""

from sqlalchemy.orm import sessionmaker
from sarigoz.data.db.schema.models import Address, Transaction, TxParticipant
from sarigoz.data.db.conn import get_engine
from eth_block_processor.utils.logger import get_logger
from sqlalchemy import text


class TransactionSaver:
    def __init__(self, engine=None, logger=None):
        """
        Initialize the TransactionSaver with a database engine and logger.
        
        Args:
            engine: SQLAlchemy engine (default: creates new engine for eth_db)
            logger: Logger instance (default: creates new logger)
        """
        self.engine = engine or get_engine(db='eth_db')
        self.SessionLocal = sessionmaker(bind=self.engine)
        self.logger = logger or get_logger(name="transaction_saver")
        
    def save_transactions(self, processed_txns):
        """
        Save a single transaction or list of transactions.
        Automatically determines whether to use bulk save based on input type.
        
        Args:
            processed_txns: Single ProcessedTransaction or list of ProcessedTransactions
        """
        if isinstance(processed_txns, list):
            return self.bulk_save_transactions(processed_txns)
        return self.save_single_transaction(processed_txns)

    def bulk_save_transactions(self, processed_txns):
        """
        Save multiple transactions in bulk, relying on database constraints 
        to handle duplicates.
        
        Args:
            processed_txns: List of ProcessedTransaction objects
        """
        session = self.SessionLocal()
        try:
            txn_objs = []
            tx_participant_objs = []
            address_cache = {}

            # Preload existing addresses to minimize database queries
            all_addresses = set()
            for txn in processed_txns:
                all_addresses |= set(txn.unique_addresses)
            
            if all_addresses:
                existing_addresses = session.query(Address).filter(
                    Address.address.in_(list(all_addresses))
                ).all()
                for addr in existing_addresses:
                    address_cache[addr.address] = addr

            # Process each transaction
            for processed_txn in processed_txns:
                txn = Transaction(
                    tx_hash=processed_txn.hash,
                    block_number=processed_txn.block_number,
                    from_address=processed_txn.from_address,
                    to_address=processed_txn.to_address,
                    value=processed_txn.value,
                    status=processed_txn.status
                )
                txn_objs.append(txn)

                # Process addresses and create associations
                for addr in processed_txn.unique_addresses:
                    if addr not in address_cache:
                        new_address = Address(
                            address=addr,
                            is_contract=False
                        )
                        address_cache[addr] = new_address

                    txp = TxParticipant(
                        tx_hash=processed_txn.hash,
                        address=addr
                    )
                    tx_participant_objs.append(txp)

            # Get new addresses that need to be saved
            new_address_objs = [
                obj for addr, obj in address_cache.items() 
                if not hasattr(obj, 'id') or obj.id is None
            ]

            # Bulk save all objects, letting DB constraints handle duplicates
            if new_address_objs:
                session.bulk_save_objects(new_address_objs)
            if txn_objs:
                session.bulk_save_objects(txn_objs)
            if tx_participant_objs:
                session.execute(
                    text("""
                    INSERT INTO eth_db.tx_participants (tx_hash, address)
                    VALUES (:tx_hash, :address)
                    ON CONFLICT (tx_hash, address) DO NOTHING
                    """),
                    [{"tx_hash": tp.tx_hash, "address": tp.address} 
                     for tp in tx_participant_objs]
                )

            session.commit()
            self.logger.info(
                f"Processed block: {len(txn_objs)} transactions, "
                f"{len(new_address_objs)} new addresses, "
                f"{len(tx_participant_objs)} participant associations"
            )
            
        except Exception as e:
            session.rollback()
            self.logger.error(f"Error in bulk_save_transactions: {str(e)}")
            raise
        finally:
            session.close()

    def save_single_transaction(self, processed_txn):
        """
        Save a single transaction and its related data.
        
        Args:
            processed_txn: Single ProcessedTransaction object
        """
        session = self.SessionLocal()
        try:
            # Similar to bulk save but for a single transaction
            txn = Transaction(
                tx_hash=processed_txn.hash,
                block_number=processed_txn.block_number,
                from_address=processed_txn.from_address,
                to_address=processed_txn.to_address,
                value=processed_txn.value,
                status=processed_txn.status
            )
            session.add(txn)
            
            # Process addresses
            for addr in processed_txn.unique_addresses:
                address = session.query(Address).filter_by(address=addr).first()
                if not address:
                    address = Address(address=addr, is_contract=False)
                    session.add(address)
                
                txp = TxParticipant(tx_hash=processed_txn.hash, address=addr)
                session.add(txp)

            session.commit()
            self.logger.info(f"Saved transaction {processed_txn.hash}")
            
        except Exception as e:
            session.rollback()
            self.logger.error(f"Error in save_single_transaction: {str(e)}")
            raise
        finally:
            session.close()