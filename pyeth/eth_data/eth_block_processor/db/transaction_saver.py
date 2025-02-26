from sqlalchemy.orm import sessionmaker
from sqlalchemy import text
from eth_block_processor.utils.logger import get_logger
from sarigoz.data.db.schema.models import Address, Transaction, TxParticipant
from sarigoz.data.db.conn import get_engine
from eth_block_processor.utils.address_type_labeler import AddressTypeLabeler

class TransactionSaver:
    def __init__(self, w3, engine=None, logger=None):
        """
        Initialize the TransactionSaver with a database engine and logger.
        
        Args:
            engine: SQLAlchemy engine (default: creates new engine for eth_db)
            logger: Logger instance (default: creates new logger)
        """
        self.w3 = w3
        self.address_labeler = AddressTypeLabeler(w3)
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
                            is_contract=self.address_labeler.is_contract(addr)
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
                # Use ON CONFLICT for addresses too
                address_params = []
                for addr_obj in new_address_objs:
                    try:
                        param = {
                            "address": addr_obj.address,
                            "is_contract": addr_obj.is_contract
                        }
                        address_params.append(param)
                    except AttributeError as e:
                        self.logger.error(f"AttributeError in address object: {e}")
                        continue
                
                session.execute(
                    text("""
                    INSERT INTO eth_db.addresses (address, is_contract)
                    VALUES (:address, :is_contract)
                    ON CONFLICT (address) DO NOTHING
                    """),
                    address_params
                )
            
            if txn_objs:
                try:
                    # Add debug logging
                    self.logger.debug(f"Inserting {len(txn_objs)} transactions")
                    
                    # Create parameters list with proper error checking
                    params = []
                    for txn in txn_objs:
                        try:
                            param = {
                                "tx_hash": txn.tx_hash,
                                "block_number": txn.block_number,
                                "from_address": txn.from_address,
                                "to_address": txn.to_address,
                                "value": txn.value,
                                "status": txn.status
                            }
                            params.append(param)
                        except AttributeError as e:
                            # Log the specific attribute that's missing
                            self.logger.error(f"AttributeError in transaction object: {e}")
                            continue
                    
                    session.execute(
                        text("""
                        INSERT INTO eth_db.transactions (tx_hash, block_number, from_address, to_address, value, status)
                        VALUES (:tx_hash, :block_number, :from_address, :to_address, :value, :status)
                        ON CONFLICT (tx_hash) DO NOTHING
                        """),
                        params
                    )
                except Exception as e:
                    self.logger.error(f"Error in transaction insertion: {str(e)}")
                    # Continue with other operations
            
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
            # Use the same ON CONFLICT approach for single transactions
            session.execute(
                text("""
                INSERT INTO eth_db.transactions (tx_hash, block_number, from_address, to_address, value, status)
                VALUES (:tx_hash, :block_number, :from_address, :to_address, :value, :status)
                ON CONFLICT (tx_hash) DO NOTHING
                """),
                {
                    "tx_hash": processed_txn.hash,
                    "block_number": processed_txn.block_number,
                    "from_address": processed_txn.from_address,
                    "to_address": processed_txn.to_address,
                    "value": processed_txn.value,
                    "status": processed_txn.status
                }
            )
            
            # Process addresses
            for addr in processed_txn.unique_addresses:
                # Use ON CONFLICT for addresses too
                session.execute(
                    text("""
                    INSERT INTO eth_db.addresses (address, is_contract)
                    VALUES (:address, :is_contract)
                    ON CONFLICT (address) DO NOTHING
                    """),
                    {
                        "address": addr,
                        "is_contract": self.address_labeler.is_contract(addr)
                    }
                )
                
                # Check if address exists
                address = session.query(Address).filter_by(address=addr).first()
                if not address:
                    address = Address(address=addr, is_contract=False)
                    session.add(address)
                
                # Use ON CONFLICT for transaction participants too
                session.execute(
                    text("""
                    INSERT INTO eth_db.tx_participants (tx_hash, address)
                    VALUES (:tx_hash, :address)
                    ON CONFLICT (tx_hash, address) DO NOTHING
                    """),
                    {"tx_hash": processed_txn.hash, "address": addr}
                )

            session.commit()
            
        except Exception as e:
            session.rollback()
            self.logger.error(f"Error in save_single_transaction: {str(e)}")
            raise
        finally:
            session.close()

    def get_processed_blocks(self):
        query = text("""
        SELECT DISTINCT block_number FROM eth_db.transactions
        """)

        with self.engine.connect() as conn:
            result = conn.execute(query).fetchall()
            processed_blocks = set(row[0] for row in result)
        return processed_blocks
    
    def get_max_processed_block(self):
        query = text("""
        SELECT MAX(block_number) FROM eth_db.transactions
        """)

        with self.engine.connect() as conn:
            result = conn.execute(query).fetchall()
            return result[0][0]