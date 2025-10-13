"""
SQL-Based Address Metrics Updater Module

Objective:
---------
Update address metrics directly at the database level using efficient SQL operations.
This approach minimizes data transfer between application and database while enabling
custom update logic for different types of metrics.

Update Strategy:
--------------
1. Selective Metric Updates:
   - Currency-dependent metrics (profit, volume): Updated using only ETH currency trades
   - Trade count metrics: Updated incrementally across all currencies
   - Ratio metrics: Recalculated using weighted formulas that incorporate new data

2. SQL-Based Calculation:
   - Direct database-level calculations using raw SQL
   - Transaction-based updates to ensure data integrity
   - Optimized queries to minimize database load

3. Incremental Logic:
   - Use existing metrics as base values
   - Apply incremental formulas: (old_value * old_count + new_value) / new_count
   - Special handling for ratio metrics to maintain statistical validity

Key Features:
-----------
- Efficient batch updates
- Transactional integrity
- Fine-grained control over which metrics to update
- Support for different aggregation strategies per metric
"""

from sqlalchemy import text
from eth_data.database.eth_db_conn import get_db_session_maker


class AddressMetricsUpdater:
    """
    Updates address metrics directly at the database level using SQL.
    
    This class performs efficient database-level updates of address metrics
    based on trade data, without requiring data transfer to the application.
    """
    
    def __init__(self, logger=None):
        """
        Initialize the AddressMetricsUpdater.
        
        Args:
            logger: Logger instance for tracking operations
        """
        self.logger = logger
        self.Session = get_db_session_maker(db='eth_db')
    
    def update_metrics_from_token(self, token):
        """
        Update address metrics based on trades for a specific token.
        
        Args:
            token: Token instance containing trade data
            
        Returns:
            bool: True if successful, False otherwise
        """
        if not self.Session:
            return False
            
        try:
            token_address = token.contract_address
            is_scam = token.token_data.is_scam
            
            # Get all addresses that traded this token
            with self.Session() as session:
                addresses = session.execute(
                    text("""
                    SELECT DISTINCT address 
                    FROM eth_db.trades 
                    WHERE token_address = :token_address
                    """),
                    {"token_address": token_address}
                ).fetchall()
                
                # Update metrics for each address
                for addr_row in addresses:
                    address = addr_row[0]
                    self._update_address_metrics(session, address, token_address, is_scam)
                    
                session.commit()
                
            return True
            
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error updating metrics from token {token_address}: {str(e)}", exc_info=True)
            return False
    
    def _update_address_metrics(self, session, address, token_address, is_scam):
        """
        Update metrics for a single address based on its trades with a specific token.
        
        Args:
            session: Database session
            address: Address to update
            token_address: Token address
            is_scam: Whether the token is flagged as a scam
        """
        try:
            # Ensure address exists in addresses table
            self._ensure_address_exists(session, address)
            
            # Update trade count incrementally
            self._update_trade_count(session, address)
            
            # Update scam ratio incrementally
            self._update_scam_ratio(session, address, is_scam)
            
            # Update profit metrics from ETH trades only
            self._update_profit_metrics(session, address, token_address)
            
            # Update related address count
            self._update_related_addresses_count(session, address, token_address)
            
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error updating metrics for address {address} with token {token_address}: {str(e)}")
            raise
    
    def _ensure_address_exists(self, session, address):
        """
        Ensure address exists in the addresses table.
        
        Args:
            session: Database session
            address: Ethereum address to check/create
        """
        result = session.execute(
            text("SELECT address FROM eth_db.addresses WHERE address = :addr"),
            {"addr": address}
        ).fetchone()
        
        if not result:
            # Insert with default values
            session.execute(
                text("""
                INSERT INTO eth_db.addresses 
                (address, is_contract, total_erc20_tx, total_erc20_trades)
                VALUES (:addr, FALSE, 0, 0)
                """),
                {"addr": address}
            )
    
    def _update_trade_count(self, session, address):
        """
        Update total_erc20_trades incrementally.
        
        Args:
            session: Database session
            address: Address to update
        """
        session.execute(
            text("""
            UPDATE eth_db.addresses
            SET total_erc20_trades = total_erc20_trades + 1
            WHERE address = :address
            """),
            {"address": address}
        )
    
    def _update_scam_ratio(self, session, address, is_scam):
        """
        Update scam_ratio incrementally based on existing ratio and whether the new token is a scam.
        
        Args:
            session: Database session
            address: Address to update
            is_scam: Whether the new token is a scam
        """
        # Get current scam_ratio and total_erc20_trades
        result = session.execute(
            text("""
            SELECT scam_ratio, total_erc20_trades 
            FROM eth_db.addresses
            WHERE address = :address
            """),
            {"address": address}
        ).fetchone()
        
        if not result:
            return
            
        current_ratio, total_trades = result
        
        # Calculate scam interactions based on current ratio
        scam_interactions = round(current_ratio * (total_trades - 1)) if total_trades > 1 else 0
        
        # Increment if new token is a scam
        if is_scam:
            scam_interactions += 1
            
        # Calculate new ratio
        new_ratio = scam_interactions / total_trades if total_trades > 0 else 0
        
        # Update scam_ratio
        session.execute(
            text("""
            UPDATE eth_db.addresses
            SET scam_ratio = :new_ratio
            WHERE address = :address
            """),
            {"address": address, "new_ratio": new_ratio}
        )
    
    def _update_profit_metrics(self, session, address, token_address):
        """
        Update profit and volume metrics based on ETH currency trades only.
        
        Args:
            session: Database session
            address: Address to update
            token_address: Token address
        """
        # Get trade data for this token (ETH currency only)
        trade = session.execute(
            text("""
            SELECT 
                realized_profit,
                total_denom_spent,
                total_denom_received,
                denom_received_spent_ratio,
                bribe_amount,
                agg_denom_balance
            FROM eth_db.trades
            WHERE address = :address
              AND token_address = :token_address
              AND currency = 'ETH'
            """),
            {"address": address, "token_address": token_address}
        ).fetchone()
        
        if not trade:
            return  # No ETH trade found
            
        # Update profit metrics
        if trade.realized_profit is not None:
            session.execute(
                text("""
                UPDATE eth_db.addresses
                SET total_profit = total_profit + :realized_profit,
                    total_realized_profit = total_realized_profit + :realized_profit
                WHERE address = :address
                """),
                {"address": address, "realized_profit": trade.realized_profit}
            )
        
        # Update volume metrics
        if trade.total_denom_spent is not None and trade.total_denom_received is not None:
            trade_volume = trade.total_denom_spent + trade.total_denom_received
            session.execute(
                text("""
                UPDATE eth_db.addresses
                SET total_volume = total_volume + :trade_volume
                WHERE address = :address
                """),
                {"address": address, "trade_volume": trade_volume}
            )
        
        # Update received/spent ratio incrementally
        if trade.denom_received_spent_ratio is not None:
            self._update_mean_received_spent_ratio(session, address, trade.denom_received_spent_ratio)
        
        # Update bribe amount
        if trade.bribe_amount is not None:
            session.execute(
                text("""
                UPDATE eth_db.addresses
                SET total_bribe_amount = total_bribe_amount + :bribe_amount
                WHERE address = :address
                """),
                {"address": address, "bribe_amount": trade.bribe_amount}
            )
            
            # Update avg_bribe_amount
            self._update_avg_bribe_amount(session, address, trade.bribe_amount)
        
        # Update balance (most recent value)
        if trade.agg_denom_balance is not None:
            session.execute(
                text("""
                UPDATE eth_db.addresses
                SET total_denom_balance = :balance
                WHERE address = :address
                """),
                {"address": address, "balance": trade.agg_denom_balance}
            )
    
    def _update_mean_received_spent_ratio(self, session, address, new_ratio):
        """
        Update mean_received_spent_ratio using weighted average.
        
        Args:
            session: Database session
            address: Address to update
            new_ratio: New ratio value
        """
        result = session.execute(
            text("""
            SELECT mean_received_spent_ratio, total_erc20_trades 
            FROM eth_db.addresses
            WHERE address = :address
            """),
            {"address": address}
        ).fetchone()
        
        if not result:
            return
            
        current_ratio, total_trades = result
        
        # Calculate new weighted average
        if total_trades > 1:
            # (old_value * (old_count-1) + new_value) / old_count
            new_mean = ((current_ratio * (total_trades - 1)) + new_ratio) / total_trades
        else:
            new_mean = new_ratio
        
        # Update mean ratio
        session.execute(
            text("""
            UPDATE eth_db.addresses
            SET mean_received_spent_ratio = :new_mean
            WHERE address = :address
            """),
            {"address": address, "new_mean": new_mean}
        )
    
    def _update_avg_bribe_amount(self, session, address, new_bribe):
        """
        Update avg_bribe_amount using weighted average.
        
        Args:
            session: Database session
            address: Address to update
            new_bribe: New bribe amount
        """
        result = session.execute(
            text("""
            SELECT avg_bribe_amount, total_erc20_trades 
            FROM eth_db.addresses
            WHERE address = :address
            """),
            {"address": address}
        ).fetchone()
        
        if not result:
            return
            
        current_avg, total_trades = result
        
        # Calculate new weighted average
        if total_trades > 1:
            # (old_value * (old_count-1) + new_value) / old_count
            new_avg = ((current_avg * (total_trades - 1)) + new_bribe) / total_trades
        else:
            new_avg = new_bribe
        
        # Update average bribe
        session.execute(
            text("""
            UPDATE eth_db.addresses
            SET avg_bribe_amount = :new_avg
            WHERE address = :address
            """),
            {"address": address, "new_avg": new_avg}
        )
    
    def _update_related_addresses_count(self, session, address, token_address):
        """
        Update num_related_addresses based on related_addresses table.
        
        Args:
            session: Database session
            address: Address to update
            token_address: Token address
        """
        # Get related addresses for this token
        result = session.execute(
            text("""
            SELECT COUNT(DISTINCT related_address) 
            FROM eth_db.related_addresses ra
            JOIN eth_db.trades t ON t.id = ra.trade_id
            WHERE t.address = :address
              AND t.token_address = :token_address
            """),
            {"address": address, "token_address": token_address}
        ).fetchone()
        
        if not result or result[0] == 0:
            return
            
        # Get current count
        current_result = session.execute(
            text("""
            SELECT num_related_addresses 
            FROM eth_db.addresses
            WHERE address = :address
            """),
            {"address": address}
        ).fetchone()
        
        if not current_result:
            return
            
        current_count = current_result[0] or 0
        new_related_count = result[0]
        
        # Update if new count is higher (approximation since we don't track unique addresses globally)
        if new_related_count > current_count:
            session.execute(
                text("""
                UPDATE eth_db.addresses
                SET num_related_addresses = :new_count
                WHERE address = :address
                """),
                {"address": address, "new_count": new_related_count}
            )
    
    def update_all_metrics_from_eth_trades(self):
        """
        Update all metrics for all addresses using ETH currency trades only.
        
        Returns:
            bool: True if successful, False otherwise
        """
        if not self.Session:
            return False
            
        try:
            with self.Session() as session:
                # Execute database-level update for all addresses
                session.execute(
                    text("""
                    -- First create a table with aggregated metrics per address
                    WITH address_metrics AS (
                        SELECT 
                            address,
                            COUNT(*) as total_trades,
                            SUM(CASE WHEN tok.is_scam THEN 1 ELSE 0 END) as scam_interactions,
                            SUM(realized_profit) as total_realized_profit,
                            SUM(total_denom_spent + total_denom_received) as total_volume,
                            AVG(denom_received_spent_ratio) as avg_received_spent_ratio,
                            SUM(bribe_amount) as total_bribe,
                            AVG(bribe_amount) as avg_bribe
                        FROM eth_db.trades t
                        JOIN eth_db.tokens tok ON t.token_address = tok.contract_address
                        WHERE t.currency = 'ETH'
                        GROUP BY address
                    )
                    
                    -- Now update the addresses table
                    UPDATE eth_db.addresses a
                    SET 
                        total_erc20_trades = am.total_trades,
                        scam_ratio = CASE WHEN am.total_trades > 0 THEN am.scam_interactions::float / am.total_trades ELSE 0 END,
                        total_profit = am.total_realized_profit,
                        total_realized_profit = am.total_realized_profit,
                        total_volume = am.total_volume,
                        mean_received_spent_ratio = am.avg_received_spent_ratio,
                        avg_bribe_amount = am.avg_bribe,
                        total_bribe_amount = am.total_bribe
                    FROM address_metrics am
                    WHERE a.address = am.address
                    """)
                )
                
                session.commit()
                
            return True
                
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error updating all metrics: {str(e)}", exc_info=True)
            return False
    
    def update_metrics_by_currency_filter(self, currency='ETH', batch_size=1000):
        """
        Update address metrics using trades filtered by currency.
        
        Args:
            currency: Currency to filter trades by
            batch_size: Number of addresses to process in each batch
            
        Returns:
            bool: True if successful, False otherwise
        """
        if not self.Session:
            return False
            
        try:
            with self.Session() as session:
                # Get addresses with trades in the specified currency
                addresses = session.execute(
                    text("""
                    SELECT DISTINCT address 
                    FROM eth_db.trades 
                    WHERE currency = :currency
                    """),
                    {"currency": currency}
                ).fetchall()
                
                total_addresses = len(addresses)
                
                # Process in batches
                for i in range(0, total_addresses, batch_size):
                    batch_addresses = addresses[i:i+batch_size]
                    for addr_row in batch_addresses:
                        address = addr_row[0]
                        self._update_metrics_by_currency(session, address, currency)
                        
                    # Commit after each batch
                    session.commit()
                    
                    if self.logger:
                        self.logger.info(f"Updated metrics for {min(i + batch_size, total_addresses)}/{total_addresses} addresses")
                
            return True
                
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error updating metrics by currency: {str(e)}", exc_info=True)
            return False
    
    def _update_metrics_by_currency(self, session, address, currency):
        """
        Update metrics for a single address using trades filtered by currency.
        
        Args:
            session: Database session
            address: Address to update
            currency: Currency to filter trades by
        """
        try:
            # Execute SQL to update metrics based on currency-filtered trades
            session.execute(
                text("""
                WITH address_currency_metrics AS (
                    SELECT 
                        COUNT(*) as currency_trades,
                        SUM(CASE WHEN tok.is_scam THEN 1 ELSE 0 END) as currency_scam_interactions,
                        SUM(realized_profit) as currency_realized_profit,
                        SUM(total_denom_spent + total_denom_received) as currency_volume,
                        AVG(denom_received_spent_ratio) as currency_received_spent_ratio,
                        SUM(bribe_amount) as currency_total_bribe,
                        AVG(bribe_amount) as currency_avg_bribe,
                        MAX(agg_denom_balance) as latest_balance
                    FROM eth_db.trades t
                    JOIN eth_db.tokens tok ON t.token_address = tok.contract_address
                    WHERE t.address = :address AND t.currency = :currency
                )
                
                UPDATE eth_db.addresses a
                SET 
                    total_profit = acm.currency_realized_profit,
                    total_realized_profit = acm.currency_realized_profit,
                    total_volume = acm.currency_volume,
                    mean_received_spent_ratio = acm.currency_received_spent_ratio,
                    avg_bribe_amount = acm.currency_avg_bribe,
                    total_bribe_amount = acm.currency_total_bribe,
                    total_denom_balance = acm.latest_balance
                FROM address_currency_metrics acm
                WHERE a.address = :address
                """),
                {"address": address, "currency": currency}
            )
            
        except Exception as e:
            if self.logger:
                self.logger.error(f"Error updating metrics for address {address} with currency {currency}: {str(e)}")
            raise
