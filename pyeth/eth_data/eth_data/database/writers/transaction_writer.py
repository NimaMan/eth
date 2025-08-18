"""
Provides the TransactionWriter class, responsible for efficiently writing processed Ethereum transaction data,
associated addresses, and block information to the 'eth_db' relational database.

Core Functionality:
- Takes batches of ProcessedTransaction objects.
- Identifies unique addresses and block numbers within each batch.
- Retrieves existing or creates new entries in the 'eth_db.addresses' table, mapping address strings to integer IDs (`address_id`), identifying contract addresses, and classifying contract types (ERC20/ERC721) into `entity_category` if initially unknown.
- Retrieves existing or fetches new block timestamps from an Ethereum node (via w3) and inserts them into the 'eth_db.blocks' table.
- Inserts core transaction data into the 'eth_db.transactions' table, linking to addresses using their IDs.
- Inserts records into the 'eth_db.tx_participants' table to link each transaction to all addresses involved (found in the `unique_addresses` attribute of the ProcessedTransaction).
- Utilizes bulk inserts with "ON CONFLICT DO NOTHING" clauses for resilience against duplicate data.

Database Schema Interaction:
- Writes to `eth_db.addresses` (address, is_contract) -> gets address_id. Updates `entity_category` separately for new contracts.
- Writes to `eth_db.blocks` (block_number, block_timestamp).
- Writes to `eth_db.transactions` (tx_hash, block_number, from_address_id, to_address_id, value, status).
- Writes to `eth_db.tx_participants` (tx_hash, address_id).
"""
from sqlalchemy.orm import sessionmaker, Session
from sqlalchemy import text, select
import traceback

# Import the whole module instead of individual classes
import eth_data.database.schema.eth_db_data_models as models
from eth_data.database.eth_db_conn import get_db_engine
from eth_data.chain_utils.address_type_labeler import AddressTypeLabeler
from eth_data.chain_utils.contract_type import classify_contract


class TransactionWriter:
    def __init__(self, w3, engine=None, logger=None):
        """
        Initialize the TransactionWriter with a database engine and logger.
        
        Args:
            w3: Web3 instance.
            engine: SQLAlchemy engine (default: creates new engine for eth_db).
            logger: Logger instance (default: creates new logger).
        """
        self.w3 = w3
        self.address_labeler = AddressTypeLabeler(w3)
        self.engine = engine or get_db_engine(db='eth_db')
        self.SessionLocal = sessionmaker(bind=self.engine)
        self.logger = logger 
    
    def _get_or_create_address_ids(self, session: Session, address_strings: set) -> dict[str, int]:
        """
        Retrieves existing address IDs, creates entries for new addresses,
        classifies new contracts, and returns a mapping from address string to address_id.

        Args:
            session: SQLAlchemy session.
            address_strings: A set of address strings (ChecksumAddress format preferred).

        Returns:
            A dictionary mapping address strings to their corresponding address_id.
        """
        address_map = {}
        if not address_strings:
            return address_map

        # Ensure input addresses are checksummed for consistency
        checksummed_addresses = set()
        invalid_addresses = set()
        for addr in address_strings:
            try:
                checksummed_addresses.add(self.w3.to_checksum_address(addr))
            except ValueError:
                invalid_addresses.add(addr)
        if invalid_addresses:
            if self.logger:
                self.logger.warning(f"Skipping invalid address formats: {invalid_addresses}")

        if not checksummed_addresses:
            return address_map

        # 1. Find existing addresses
        try:
            existing_query = select(models.Address.address, models.Address.address_id).where(models.Address.address.in_(list(checksummed_addresses)))
            existing_results = session.execute(existing_query).fetchall()
            for addr_str, addr_id in existing_results:
                address_map[addr_str] = addr_id
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error querying existing addresses: {e}", exc_info=True)
            raise # Propagate error if we can't query existing addresses

        # 2. Identify addresses to potentially insert
        addresses_to_insert_set = checksummed_addresses - set(address_map.keys())
        if not addresses_to_insert_set:
            if self.logger:
                self.logger.debug("No new addresses to insert in this batch.")
            return address_map # All addresses already existed

        # 3. Prepare data for insertion and classification
        insert_params = []
        classified_contracts = {} # Store {address: category} for update later
        for addr_str in addresses_to_insert_set:
            params = {"address": addr_str}
            is_contract = False # Default
            try:
                is_contract = self.address_labeler.is_contract(addr_str)
                params["is_contract"] = is_contract
                if is_contract:
                    try:
                        contract_category = classify_contract(addr_str, self.w3)
                        if contract_category in ["ERC20", "ERC721"]:
                            # Store for later update, don't put in insert_params yet
                            classified_contracts[addr_str] = contract_category
                            if self.logger:
                                self.logger.debug(f"Identified new contract {addr_str} as {contract_category} for potential category update.")
                    except Exception as class_err:
                        if self.logger:
                            self.logger.warning(f"Could not classify contract {addr_str}: {class_err}. Category will remain null.")

            except ValueError as val_err: # Should not happen if checksum worked, but safety check
                if self.logger:
                    self.logger.warning(f"Invalid address format during is_contract check: {addr_str}. Error: {val_err}")
                continue # Skip this address
            except Exception as label_err:
                if self.logger:
                    self.logger.error(f"Error checking if {addr_str} is contract: {label_err}. Defaulting is_contract to False.")
                params["is_contract"] = False # Ensure key exists even on error

            insert_params.append(params)

        # 4. Perform Bulk Insert with ON CONFLICT DO NOTHING
        if insert_params:
            try:
                # Only insert address and is_contract initially
                insert_sql = text("""
                    INSERT INTO eth_db.addresses (address, is_contract)
                    VALUES (:address, :is_contract)
                    ON CONFLICT (address) DO NOTHING
                """)
                result = session.execute(insert_sql, insert_params)
                if self.logger:
                    self.logger.debug(f"Attempted insert for {len(insert_params)} addresses. Result rowcount (may be 0 if all conflicted): {result.rowcount}")
            except Exception as e:
                if self.logger:
                    self.logger.error(f"Error during bulk address insert: {e}", exc_info=True)
                    session.rollback() # Rollback if insert fails
                    raise # Propagate error

        # 5. Re-query ALL address IDs needed for the batch (critical step for correctness)
        try:
            final_query = select(models.Address.address, models.Address.address_id).where(models.Address.address.in_(list(checksummed_addresses)))
            final_results = session.execute(final_query).fetchall()
            # Overwrite address_map completely with the definitive results after insert attempt
            address_map = {addr_str: addr_id for addr_str, addr_id in final_results}
            if self.logger:
                self.logger.debug(f"Fetched {len(address_map)} address IDs after insert attempt.")
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error re-querying address IDs after insert: {e}", exc_info=True)
                # If this fails, we cannot proceed reliably
                session.rollback()
            raise Exception("Failed to retrieve address IDs after insertion attempt.") from e

        # Check if all original valid addresses were found
        if len(address_map) != len(checksummed_addresses):
            missing_ids = checksummed_addresses - set(address_map.keys())
            self.logger.error(f"CRITICAL: Failed to find/create IDs for addresses after insert/re-query: {missing_ids}")
            # Decide how critical this is. Raising an error might be safest.
            session.rollback()
            raise Exception(f"Could not resolve IDs for all required addresses: {missing_ids}")

        # 6. Perform Conditional Bulk Update for Entity Category
        update_params = []
        if classified_contracts:
            for addr_str, category in classified_contracts.items():
                if addr_str in address_map:
                    update_params.append({
                        "address_id": address_map[addr_str],
                        "category": category
                    })
                else:
                    if self.logger:
                        self.logger.error(f"Cannot update category for {addr_str}: address_id not found in final map.")

        if update_params:
            try:
                # Use text() construct for the UPDATE statement
                update_sql = text("""
                    UPDATE eth_db.addresses
                    SET entity_category = :category
                    WHERE address_id = :address_id
                      AND entity_category IS NULL
                """)

                # Execute with text() object and execution_options
                result = session.execute(update_sql, update_params, execution_options={"synchronize_session": None})
                if self.logger:
                    self.logger.debug(f"Attempted category update for {len(update_params)} addresses. Rows updated: {result.rowcount}")
            except Exception as e:
                if self.logger:
                    self.logger.error(f"Error during bulk category update: {e}", exc_info=True)

        return address_map

    def save_transactions(self, processed_txns):
        """
        Save a single transaction or list of transactions.
        
        Args:
            processed_txns: Single ProcessedTransaction or list of ProcessedTransactions
        """
        if not processed_txns:
            return

        if isinstance(processed_txns, list):
            return self.bulk_save_transactions(processed_txns)
        # Wrap single transaction in a list for bulk processing
        return self.bulk_save_transactions([processed_txns])

    def bulk_save_transactions(self, processed_txns: list):
        """
        Save multiple transactions and their associated blocks in bulk.
        
        Args:
            processed_txns: List of ProcessedTransaction objects
        """
        session = self.SessionLocal()
        try:
            # 1. Collect unique addresses and block numbers (CHECKSUMMED)
            all_address_strings_checksummed = set()
            unique_block_numbers = set()
            invalid_format_addresses = set() # Track invalid formats found

            for txn in processed_txns:
                try:
                    if txn.from_address: all_address_strings_checksummed.add(self.w3.to_checksum_address(txn.from_address))
                    if txn.to_address: all_address_strings_checksummed.add(self.w3.to_checksum_address(txn.to_address))
                    
                    unique_addresses_attr = getattr(txn, 'unique_addresses', [])
                    if unique_addresses_attr:
                        for addr in unique_addresses_attr:
                            if addr: all_address_strings_checksummed.add(self.w3.to_checksum_address(addr))
                            
                    if txn.block_number is not None: unique_block_numbers.add(txn.block_number)
                    
                except ValueError as e:
                    # Log specific invalid address format error
                    original_address = None
                    if txn.from_address and not self.w3.is_checksum_address(txn.from_address): original_address = txn.from_address
                    elif txn.to_address and not self.w3.is_checksum_address(txn.to_address): original_address = txn.to_address
                    # Find the first invalid address in unique_addresses if applicable
                    elif unique_addresses_attr:
                        for addr in unique_addresses_attr:
                            if addr and not self.w3.is_checksum_address(addr):
                                original_address = addr
                                break
                    
                    log_addr = original_address if original_address else "(unknown)"
                    if self.logger:
                        self.logger.warning(f"Invalid address format encountered in tx {txn.hash} (e.g., '{log_addr}'): {e}. Address skipped.")
                    if original_address: invalid_format_addresses.add(original_address)
                except Exception as e:
                    if self.logger:
                        self.logger.error(f"Error processing addresses/block for transaction {txn.hash}: {e}", exc_info=True)
            
            # Exclude addresses that consistently failed checksumming from DB operations
            addresses_for_db = all_address_strings_checksummed
            if not addresses_for_db:
                if self.logger:
                    self.logger.warning("No valid checksum addresses found in transaction batch. Skipping DB operations for addresses.")
                address_id_map = {}
            else:
                # 2. Get or create address IDs (includes contract classification)
                address_id_map = self._get_or_create_address_ids(session, addresses_for_db)
                # Address_id_map keys are now guaranteed to be checksummed

            # 3. Fetch block timestamps for unique blocks (Optimized)
            block_params = []
            if unique_block_numbers:
                existing_blocks_q = select(models.Block.block_number).where(models.Block.block_number.in_(list(unique_block_numbers)))
                existing_blocks = set(res[0] for res in session.execute(existing_blocks_q).fetchall())
                blocks_to_fetch = unique_block_numbers - existing_blocks
                if self.logger:
                    self.logger.debug(f"Need to fetch timestamp for {len(blocks_to_fetch)} blocks.")
                
                fetch_errors = 0
                for block_num in blocks_to_fetch:
                    try:
                        block_data = self.w3.eth.get_block(block_num)
                        if block_data and 'timestamp' in block_data:
                            block_params.append({"block_number": block_num, "block_timestamp": block_data['timestamp']})
                        else:
                            if self.logger:
                                self.logger.warning(f"Could not retrieve valid block data for block {block_num}")
                            fetch_errors += 1
                    except Exception as e:
                        if self.logger:
                            self.logger.error(f"Failed to get block info for {block_num}: {e}")
                        fetch_errors += 1
                if fetch_errors > 0 and len(blocks_to_fetch) > 0:
                    # Decide how critical missing block timestamps are
                    if self.logger:
                        self.logger.warning(f"Failed to fetch metadata for {fetch_errors}/{len(blocks_to_fetch)} requested blocks.")
                    # Might raise an error here if timestamps are essential

            # 4. Prepare data for transaction and participant bulk insertion
            txn_params = []
            tx_participant_params = []

            for processed_txn in processed_txns:
                # Basic data validation
                if not all([processed_txn.hash,
                            processed_txn.block_number is not None,
                            processed_txn.from_address,
                            processed_txn.status is not None]):
                    if self.logger:
                        self.logger.warning(f"Skipping transaction with missing essential base data: hash={processed_txn.hash}, block={processed_txn.block_number}, from={processed_txn.from_address}, status={processed_txn.status}")
                    continue
                
                # Get IDs using CHECKSUMMED addresses
                try:
                    from_checksum = self.w3.to_checksum_address(processed_txn.from_address)
                    to_checksum = self.w3.to_checksum_address(processed_txn.to_address) if processed_txn.to_address else None
                except ValueError:
                    # This address should have been caught earlier, but log defensively
                    if self.logger:
                        self.logger.error(f"CRITICAL: Invalid from/to address format in tx {processed_txn.hash} made it past initial checks.")
                    continue # Skip if core addresses are invalid

                from_id = address_id_map.get(from_checksum)
                to_id = address_id_map.get(to_checksum) if to_checksum else None

                if from_id is None:
                    # If from_address was valid format but ID is missing, something went wrong in _get_or_create_address_ids
                    if self.logger:
                        self.logger.error(f"CRITICAL: Skipping transaction {processed_txn.hash} because from_address_id could not be resolved for valid address {from_checksum}.")
                    continue 

                # Prepare transaction parameters
                txn_params.append({
                    "tx_hash": processed_txn.hash,
                    "block_number": processed_txn.block_number,
                    "from_address_id": from_id,
                    "to_address_id": to_id,
                    "value": processed_txn.value,
                    "status": processed_txn.status
                })

                # Prepare participant parameters (using checksummed addresses)
                processed_participants = set()
                unique_txn_addresses_attr = getattr(processed_txn, 'unique_addresses', [])
                txn_participant_candidates_checksummed = set()
                
                # Add from/to addresses first
                if from_checksum: txn_participant_candidates_checksummed.add(from_checksum)
                if to_checksum: txn_participant_candidates_checksummed.add(to_checksum)
                
                # Add others from unique_addresses attribute
                if unique_txn_addresses_attr:
                    for addr_str in unique_txn_addresses_attr:
                        try:
                            if addr_str: txn_participant_candidates_checksummed.add(self.w3.to_checksum_address(addr_str))
                        except ValueError:
                            # Already logged during initial collection
                            pass 

                # Add participants if ID exists in our map
                for checksum_addr in txn_participant_candidates_checksummed:
                    addr_id = address_id_map.get(checksum_addr)
                    if addr_id is not None:
                        if (processed_txn.hash, addr_id) not in processed_participants:
                            tx_participant_params.append({"tx_hash": processed_txn.hash, "address_id": addr_id})
                            processed_participants.add((processed_txn.hash, addr_id))
                    else:
                        # Address was valid checksum format but ID is missing
                        if self.logger:
                            self.logger.error(f"Could not find address_id for participant {checksum_addr} in transaction {processed_txn.hash} (Should exist after _get_or_create_address_ids)")

            # 5. Execute bulk inserts with ON CONFLICT DO NOTHING
            # Insert Blocks first
            if block_params:
                insert_block_sql = text("""
                    INSERT INTO eth_db.blocks (block_number, block_timestamp)
                    VALUES (:block_number, :block_timestamp)
                    ON CONFLICT (block_number) DO NOTHING
                """)
                try:
                    session.execute(insert_block_sql, block_params)
                    if self.logger:
                        self.logger.debug(f"Inserted {len(block_params)} new blocks.")
                except Exception as e:
                    if self.logger:
                        self.logger.error(f"Error bulk inserting blocks: {e}", exc_info=True)
                        session.rollback()
                        raise Exception("Failed to insert blocks, rolling back transaction.") from e

            # Insert Transactions
            if txn_params:
                insert_txn_sql = text("""
                    INSERT INTO eth_db.transactions (tx_hash, block_number, from_address_id, to_address_id, value, status)
                    VALUES (:tx_hash, :block_number, :from_address_id, :to_address_id, :value, :status)
                    ON CONFLICT (tx_hash) DO NOTHING
                """)
                try:
                    result = session.execute(insert_txn_sql, txn_params)
                    if self.logger:
                        self.logger.debug(f"Attempted insert for {len(txn_params)} transactions. Affected rows (via DO NOTHING): {result.rowcount}")
                except Exception as e:
                    if self.logger:
                        self.logger.error(f"Error bulk inserting transactions: {e}", exc_info=True)
                        session.rollback()
                        raise Exception("Failed to insert transactions, rolling back transaction.") from e

            # Insert Participants
            if tx_participant_params:
                insert_txp_sql = text("""
                    INSERT INTO eth_db.tx_participants (tx_hash, address_id)
                    VALUES (:tx_hash, :address_id)
                    ON CONFLICT (tx_hash, address_id) DO NOTHING
                """)
                try:
                    result = session.execute(insert_txp_sql, tx_participant_params)
                    if self.logger:
                        self.logger.debug(f"Attempted insert for {len(tx_participant_params)} participants. Affected rows (via DO NOTHING): {result.rowcount}")
                except Exception as e:
                    if self.logger:
                        self.logger.error(f"Error bulk inserting participants: {e}", exc_info=True)
                        session.rollback()
                        raise Exception("Failed to insert participants, rolling back transaction.") from e

            session.commit()
        except Exception as e:
            session.rollback()
            if self.logger:
                self.logger.error(f"Error in bulk_save_transactions: {str(e)}")
                self.logger.error(traceback.format_exc())
            raise
        finally:
            session.close()

    def get_processed_blocks(self):
        query = text("""
        SELECT DISTINCT block_number FROM eth_db.transactions
        ORDER BY block_number
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
            result = conn.execute(query).scalar()
            return result if result is not None else 0