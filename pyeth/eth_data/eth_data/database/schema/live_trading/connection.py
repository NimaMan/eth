"""
Live Trading Database Connection Manager

Handles database connections and session management for the live trading database.
"""

import os
from contextlib import contextmanager
from typing import Optional

from sqlalchemy import create_engine, event
from sqlalchemy.orm import sessionmaker, Session
from sqlalchemy.pool import NullPool

from .models import Base


class LiveTradingDB:
    """
    Database connection manager for live trading operations.
    """
    
    def __init__(self, connection_string: Optional[str] = None):
        """
        Initialize database connection.
        
        Args:
            connection_string: PostgreSQL connection string. 
                             If not provided, uses LIVE_TRADING_DB_URL environment variable.
        """
        if connection_string is None:
            connection_string = os.environ.get(
                'LIVE_TRADING_DB_URL',
                'postgresql://postgres:postgres@localhost:5432/live_trading_db'
            )
        
        # Create engine with connection pooling disabled for async compatibility
        self.engine = create_engine(
            connection_string,
            poolclass=NullPool,  # Disable pooling for async safety
            echo=False,  # Set to True for SQL debugging
            future=True  # Use SQLAlchemy 2.0 style
        )
        
        # Create session factory
        self.SessionLocal = sessionmaker(
            bind=self.engine,
            autocommit=False,
            autoflush=False,
            expire_on_commit=False
        )
        
        # Add listener for setting search_path if using schemas
        @event.listens_for(self.engine, "connect", insert=True)
        def set_search_path(dbapi_connection, connection_record):
            existing_autocommit = dbapi_connection.autocommit
            dbapi_connection.autocommit = True
            cursor = dbapi_connection.cursor()
            cursor.execute("SET search_path TO public")
            cursor.close()
            dbapi_connection.autocommit = existing_autocommit
    
    def create_tables(self):
        """Create all tables in the database."""
        Base.metadata.create_all(bind=self.engine)
    
    def drop_tables(self):
        """Drop all tables in the database. USE WITH CAUTION!"""
        Base.metadata.drop_all(bind=self.engine)
    
    @contextmanager
    def get_session(self) -> Session:
        """
        Context manager for database sessions.
        
        Usage:
            with db.get_session() as session:
                # Use session here
                session.add(position)
                session.commit()
        """
        session = self.SessionLocal()
        try:
            yield session
        except Exception:
            session.rollback()
            raise
        finally:
            session.close()
    
    def get_new_session(self) -> Session:
        """
        Get a new session instance.
        Caller is responsible for closing the session.
        """
        return self.SessionLocal()
    
    def close(self):
        """Close the database connection."""
        self.engine.dispose()


# Global instance (optional - can be initialized elsewhere)
_db_instance: Optional[LiveTradingDB] = None


def get_db() -> LiveTradingDB:
    """Get or create the global database instance."""
    global _db_instance
    if _db_instance is None:
        _db_instance = LiveTradingDB()
    return _db_instance


def init_db(connection_string: Optional[str] = None) -> LiveTradingDB:
    """Initialize the global database instance with a specific connection string."""
    global _db_instance
    _db_instance = LiveTradingDB(connection_string)
    return _db_instance


# Convenience functions
def create_all_tables():
    """Create all tables in the database."""
    db = get_db()
    db.create_tables()
    print("All tables created successfully in live_trading_db")


def get_session() -> Session:
    """Get a new database session."""
    db = get_db()
    return db.get_new_session()


@contextmanager
def session_scope():
    """Provide a transactional scope for database operations."""
    db = get_db()
    with db.get_session() as session:
        yield session