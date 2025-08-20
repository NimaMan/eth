from sqlalchemy import create_engine, Engine, event
from sqlalchemy.orm import sessionmaker, Session
from sqlalchemy.pool import QueuePool
import os
from typing import Optional
from baygus.utils.logger import get_logger

# Global engine cache for connection reuse
_engines = {}


logger = get_logger(name="db_conn")


def get_db_engine(db: str = 'eth_db', pool_size: int = 10, max_overflow: int = 20) -> Engine:
    """
    Connects to a local Postgres database with optimized connection pooling.
    
    :param db: Database name (default: 'eth_db')
    :param pool_size: Number of connections to maintain (default: 10)
    :param max_overflow: Maximum overflow connections (default: 20)
    :return engine: SQLAlchemy Engine with connection pooling
    """
    # Use cached engine if available
    if db in _engines:
        return _engines[db]
    
    # Database connection parameters from environment or defaults
    db_config = {
        'user': os.getenv('DB_USER', 'postgres'),
        'password': os.getenv('DB_PASSWORD', 'postgres'),
        'host': os.getenv('DB_HOST', 'localhost'),
        'port': os.getenv('DB_PORT', '5432'),
        'db': db,
        'name': 'baygus'
    }
    
    connection_url = 'postgresql+psycopg2://{user}:{password}@{host}:{port}/{db}?application_name={name}'.format(**db_config)
    
    # Create engine with optimized settings
    engine = create_engine(
        connection_url,
        poolclass=QueuePool,
        pool_size=pool_size,
        max_overflow=max_overflow,
        pool_pre_ping=True,  # Verify connections before use
        pool_recycle=3600,   # Recycle connections every hour
        connect_args={
            'client_encoding': 'utf8',
            'application_name': 'baygus_address_profile',
            'connect_timeout': 10,
        },
        echo=False,  # Set to True for SQL debugging
    )
    
    # Add connection event listeners for logging
    @event.listens_for(engine, "connect")
    def receive_connect(dbapi_connection, connection_record):
        logger.debug(f"Connected to database: {db}")
    
    @event.listens_for(engine, "checkout")
    def receive_checkout(dbapi_connection, connection_record, connection_proxy):
        logger.debug("Connection checked out from pool")
    
    # Cache the engine
    _engines[db] = engine
    
    return engine


def get_db_session_maker(db: str = "eth_db") -> sessionmaker:
    """
    Creates a SQLAlchemy Session maker with optimized settings.
    
    :param db: Database name (default: 'eth_db')
    :return session_maker: SQLAlchemy SessionMaker
    """
    engine = get_db_engine(db=db)
    session_maker = sessionmaker(
        bind=engine,
        autoflush=False,  # Don't auto-flush for performance
        autocommit=False,
        expire_on_commit=False  # Keep objects accessible after commit
    )
    return session_maker


def test_database_connection(db: str = "eth_db") -> bool:
    """
    Test database connectivity and verify schema.
    
    :param db: Database name to test
    :return: True if connection successful, False otherwise
    """
    try:
        from sqlalchemy import text
        
        engine = get_db_engine(db=db)
        
        with engine.connect() as conn:
            # Test basic connectivity
            result = conn.execute(text("SELECT 1"))
            assert result.fetchone()[0] == 1
            
            # Verify eth_db schema exists
            result = conn.execute(text("""
                SELECT schema_name FROM information_schema.schemata 
                WHERE schema_name = 'eth_db'
            """))
            
            if not result.fetchone():
                logger.error("eth_db schema not found")
                return False
            
            # Verify required tables exist
            required_tables = ['addresses', 'trades', 'tokens']
            for table in required_tables:
                result = conn.execute(text(f"""
                    SELECT table_name FROM information_schema.tables 
                    WHERE table_schema = 'eth_db' AND table_name = '{table}'
                """))
                
                if not result.fetchone():
                    logger.error(f"Required table {table} not found in eth_db schema")
                    return False
            
            logger.info(f"Database connection to {db} verified successfully")
            return True
            
    except Exception as e:
        logger.error(f"Database connection test failed: {e}")
        return False


def get_connection_info(db: str = "eth_db") -> dict:
    """
    Get information about the current database connection.
    
    :param db: Database name
    :return: Dictionary with connection information
    """
    try:
        from sqlalchemy import text
        
        engine = get_db_engine(db=db)
        
        with engine.connect() as conn:
            # Get database version
            result = conn.execute(text("SELECT version()"))
            db_version = result.fetchone()[0]
            
            # Get table counts
            table_counts = {}
            for table in ['addresses', 'trades', 'tokens']:
                result = conn.execute(text(f"SELECT COUNT(*) FROM eth_db.{table}"))
                table_counts[table] = result.fetchone()[0]
            
            return {
                'database': db,
                'version': db_version,
                'pool_size': engine.pool.size(),
                'checked_out_connections': engine.pool.checkedout(),
                'table_counts': table_counts,
                'status': 'connected'
            }
            
    except Exception as e:
        return {
            'database': db,
            'status': 'error',
            'error': str(e)
        }
