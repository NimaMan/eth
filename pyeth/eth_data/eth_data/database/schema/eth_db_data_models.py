"""
PostgreSQL Schema for Ethereum PnL Analysis and Address Ranking System
-----------------------------------------------------------------------

Overview:
-----------
This module defines the database schema used for the Sarigoz project—a system that ingests Ethereum blockchain data (from the Reth Node) to track token interactions, calculate profit and loss (PnL) for each user, and determine whether tokens are scams or not. Based on these metrics, addresses are ranked bird-themed universe.

The primary objectives of this schema are to:
  • Store raw and computed on-chain data related to user trades.
  • Maintain metadata on tokens (including scam flags and labels).
  • Capture detailed trade-level information and aggregated metrics.
  • Record network relationships between addresses, which aids in identifying interconnected sub-networks.
  • Provide a foundation for computing ranking metrics and composite scores that map to our bird-themed ranking system.

Schema Design Rationale & Data Flow:
-----------------------------------
1.  **Core Entities:** `blocks`, `transactions`, `addresses`, `tokens` store fundamental blockchain data. `tx_participants` links transactions to participating addresses.
2.  **Trade Aggregation (`trades` table):** Instead of storing every single swap, the `trades` table aggregates all interactions (buys/sells) between a specific `address` and a specific `token_address`. This simplifies PnL calculation at the address-token level.
    *   Metrics like `total_denom_spent`, `total_denom_received`, `num_buys`, `num_sells`, and `tx_fee` summarize these interactions.
    *   `currency` specifies the denomination asset (e.g., WETH) used for value calculations within that trade record.
    *   `realized_profit` and `unrealized_profit` track the PnL for this specific address-token pair.
3.  **Address Profiling (`addresses` table):** Aggregates metrics across all trades and activities for a given address.
    *   Includes overall PnL (`total_profit`, `total_realized_profit`), volume (`total_volume`), scam exposure (`scam_ratio`), activity (`trade_frequency`, `first_seen`, `last_seen`), and total gas costs (`total_tx_fee`).
4.  **Token Metadata (`tokens`):** Stores token-specific information, including creator details and scam status, linking to creation transactions.

This design prioritizes aggregated metrics suitable for PnL analysis, scam detection heuristics, and address ranking, while retaining links to underlying transactions for drill-down analysis.

Tables Overview:
  1. addresses:                         Stores on-chain user (wallet/contract) data and aggregated metrics.
  2. tokens:                            Contains metadata for tokens tracked on Ethereum.
  3. trades:                            Aggregates interactions between an address and a token.
  4. tx_participants:                   Links transactions and participating addresses (many-to-many).
  5. transactions:                      Represents minimal transaction records.
  6. blocks:                            Stores block-level metadata.
"""

from sqlalchemy import Column, Integer, Float, String, Boolean, JSON, ForeignKey, Index, CheckConstraint, Text, BigInteger, UniqueConstraint, DateTime
from sqlalchemy.dialects.postgresql import ARRAY
from sqlalchemy.orm import declarative_base, relationship
from sqlalchemy.sql import func 


Base = declarative_base()


# ---------------------------------------------------------------------------
# Block Model: Tracks Block-level Metadata
# ---------------------------------------------------------------------------
class Block(Base):
    __tablename__ = 'blocks'
    __table_args__ = {'schema': 'eth_db'}

    block_number = Column(Integer, primary_key=True)
    block_timestamp = Column(Integer)  # Unix timestamp for block creation time


# ---------------------------------------------------------------------------
# Transaction Model: Represents Minimal Transaction Records
# ---------------------------------------------------------------------------
class Transaction(Base):
    __tablename__ = 'transactions'

    tx_hash = Column(String(66), primary_key=True)
    block_number = Column(Integer, index=True)
    from_address_id = Column(BigInteger, ForeignKey('eth_db.addresses.address_id'), index=True)
    to_address_id = Column(BigInteger, ForeignKey('eth_db.addresses.address_id'), index=True)
    value = Column(Float)
    status = Column(String(20))

    # Add composite index for block range queries
    __table_args__ = (
        Index('idx_tx_block_tx', 'block_number', 'tx_hash'),
        Index('idx_tx_from_address_id', 'from_address_id'),
        Index('idx_tx_to_address_id', 'to_address_id'),
        {'schema': 'eth_db'}
    )

    # Relationships - Update join conditions
    addresses = relationship(
        "Address",
        secondary="eth_db.tx_participants",
        primaryjoin="Transaction.tx_hash == TxParticipant.tx_hash",
        secondaryjoin="TxParticipant.address_id == Address.address_id",
        back_populates="transactions"
    )
    sender = relationship("Address", foreign_keys=[from_address_id], back_populates="sent_transactions")
    receiver = relationship("Address", foreign_keys=[to_address_id], back_populates="received_transactions")


# ---------------------------------------------------------------------------
# Transaction Participants: Links Transactions to Addresses
# ---------------------------------------------------------------------------
# This table captures the many-to-many relationship between transactions and addresses.
# It allows us to track which addresses participated in each transaction, enabling complex queries on address interactions
# and trade history.
class TxParticipant(Base):
    __tablename__ = 'tx_participants'

    tx_hash = Column(String(66), ForeignKey('eth_db.transactions.tx_hash'), primary_key=True)
    address_id = Column(BigInteger, ForeignKey('eth_db.addresses.address_id'), primary_key=True)

    # Adjust index to use address_id
    __table_args__ = (
        Index('idx_txp_address_id_tx_hash', 'address_id', 'tx_hash'),
        {'schema': 'eth_db'}
    )


# ---------------------------------------------------------------------------
# Address Model: Represents On-Chain User (Wallet/Contract) Data
# ---------------------------------------------------------------------------
class Address(Base):
    __tablename__ = 'addresses'
    __table_args__ = (
        UniqueConstraint('address', name='uq_addresses_address'),
        {'schema': 'eth_db'}
    )

    address_id = Column(BigInteger, primary_key=True)
    address = Column(String(42), nullable=False, index=True)

    is_contract = Column(Boolean, nullable=False, default=False)
    total_erc20_tx = Column(Integer, default=0)
    total_erc20_trades = Column(Integer, default=0)

    # Core Trading Metrics
    scam_ratio = Column(Float, default=0.0)
    total_profit = Column(Float, default=0.0)
    total_volume = Column(Float, default=0.0)
    trade_frequency = Column(Float, default=0.0)
    first_seen = Column(Integer)
    last_seen = Column(Integer)
    total_tx_fee = Column(Float, default=0.0)

    total_denom_balance = Column(Float, default=0.0)
    total_realized_profit = Column(Float, default=0.0)
    mean_received_spent_ratio = Column(Float, default=0.0)
    median_received_spent_ratio = Column(Float, default=0.0)
    avg_bribe_amount = Column(Float, default=0.0)
    total_bribe_amount = Column(Float, default=0.0)

    # Address Naming / Labeling
    name = Column(String(255), nullable=True)
    entity_category = Column(String(100), nullable=True)
    cluster_label = Column(String(20))
    
    # Relationships - Update secondary join condition if necessary for Transaction relationship
    transactions = relationship(
        "Transaction",
        secondary="eth_db.tx_participants",
        primaryjoin="Address.address_id == TxParticipant.address_id",
        secondaryjoin="TxParticipant.tx_hash == Transaction.tx_hash",
        back_populates="addresses"
    )
    # Add backref relationships for foreign keys pointing TO address_id
    created_tokens = relationship("Token", back_populates="creator")
    initiated_trades = relationship("Trade", back_populates="trader")
    sent_transactions = relationship("Transaction", foreign_keys="[Transaction.from_address_id]", back_populates="sender")
    received_transactions = relationship("Transaction", foreign_keys="[Transaction.to_address_id]", back_populates="receiver")


# ---------------------------------------------------------------------------
# Token Model: Represents ERC20 Tokens and Their Metadata
# ---------------------------------------------------------------------------
# This model captures metadata for ERC20 tokens, including their creator, scam status, and trading transactions.
# It also links to the pools where these tokens can be traded across different DEX protocols.
# The `contract_address` serves as the primary key, ensuring uniqueness for each token.
class Token(Base):
    __tablename__ = 'tokens'
    __table_args__ = {'schema': 'eth_db'}
    contract_address = Column(String(42), primary_key=True, index=True)

    creator_address_id = Column(BigInteger, ForeignKey('eth_db.addresses.address_id'), index=True)
    is_scam = Column(Boolean, default=False)
    scam_label = Column(String(50))

    # Transaction references (assuming these remain hashes)
    creation_tx = Column(String(66), ForeignKey('eth_db.transactions.tx_hash'), index=True)

    # Relationships
    creator = relationship("Address", back_populates="created_tokens", foreign_keys=[creator_address_id])
    creation_transaction = relationship("Transaction", foreign_keys=[creation_tx])
    pools = relationship("Pool", back_populates="token")


# ---------------------------------------------------------------------------
# Pool Model: Represents Token Pools Across DEX Protocols
# ---------------------------------------------------------------------------
# This model captures the various pools where tokens can be traded, including Uniswap V2, V3, and V4 pools.
# Each token can have multiple pools across different DEX protocols.
# The pool type indicates the DEX protocol (V2, V3, V4),
# and the pool_id is used for V4 pools where the address may not be available.
class Pool(Base):
    """
    Minimal pool registry for tokens.
    Each token can have multiple pools across different DEX protocols.
    """
    __tablename__ = 'pools'
    __table_args__ = (
        UniqueConstraint('pool_id', name='uq_pool_v4_id'),
        Index('idx_pool_token', 'token_address'),
        Index('idx_pool_type', 'pool_type'),
        CheckConstraint("pool_type IN ('V2', 'V3', 'V4')", name='check_pool_type'),
        CheckConstraint("(pool_address IS NOT NULL) OR (pool_id IS NOT NULL)", 
                       name='check_pool_identifier'),
        {'schema': 'eth_db'}
    )
    
    id = Column(Integer, primary_key=True)
    
    # Pool identity
    pool_address = Column(String(42), unique=True, nullable=True)  # NULL for V4
    pool_id = Column(String(66), nullable=True)  # For V4 pools (bytes32 as hex)
    pool_type = Column(String(10), nullable=False)  # V2, V3, V4
    
    # Token reference
    token_address = Column(String(42), ForeignKey('eth_db.tokens.contract_address'), nullable=False)
    pair_token_address = Column(String(42), nullable=False)  # Usually WETH
    
    # V3/V4 specific
    fee_tier = Column(Integer)  # 100, 500, 3000, 10000 = 0.01%, 0.05%, 0.3%, 1%
    
    # Scam detection fields
    is_scam = Column(Boolean, default=False)
    scam_label = Column(String(255))
    scam_block = Column(Integer)
    scam_tx_hash = Column(String(66))
    
    # Trading enabled fields (tx presence indicates enabled status)
    trading_enabled_block = Column(BigInteger)
    trading_enabled_tx = Column(String(66))
    
    # Relationships
    token = relationship("Token", back_populates="pools")


# ---------------------------------------------------------------------------
# Trade Model: Represents Aggregated Trades Between Addresses and Tokens
# ---------------------------------------------------------------------------
# This model aggregates trades between a specific address and a specific token
# in a specific currency. Each row represents all interactions between an address
# and token when trading against a particular currency (ETH, USDC, USDT, etc.).
class Trade(Base):
    __tablename__ = 'trades'

    id = Column(Integer, primary_key=True)
    address_id = Column(BigInteger, ForeignKey('eth_db.addresses.address_id'), nullable=False, index=True)
    token_address = Column(String(42), ForeignKey('eth_db.tokens.contract_address'), nullable=False, index=True)
    currency = Column(String(10), nullable=False, default='ETH', index=True)

    # Core Transaction Metrics
    entry_block = Column(Integer)
    latest_block = Column(Integer, index=True)
    total_denom_spent = Column(Float, default=0.0)
    total_denom_received = Column(Float, default=0.0)
    denom_received_spent_ratio = Column(Float)

    # Profit and Loss Metrics
    realized_profit = Column(Float, default=0.0)
    unrealized_profit = Column(Float, default=0.0)

    # Behavioral Signals
    num_buys = Column(Integer, default=0)
    num_sells = Column(Integer, default=0)

    # Balance tracking
    total_gas_spent = Column(Float, default=0.0)
    token_balance = Column(Float, default=0.0)
    denom_balance = Column(Float, default=0.0)

    # Metadata
    last_updated = Column(DateTime, server_default=func.now())

    # Relationships
    trader = relationship("Address", back_populates="initiated_trades", foreign_keys=[address_id])
    token = relationship("Token", backref="trades")

    __table_args__ = (
        UniqueConstraint('address_id', 'token_address', 'currency', name='trades_address_token_currency_key'),
        Index('idx_trades_address_id', 'address_id'),
        Index('idx_trades_token_address', 'token_address'),
        Index('idx_trades_currency', 'currency'),
        Index('idx_trades_realized_profit', realized_profit.desc()),
        Index('idx_trades_latest_block', latest_block.desc()),
        Index('idx_trades_updated', last_updated.desc()),
        {'schema': 'eth_db'}
    )

