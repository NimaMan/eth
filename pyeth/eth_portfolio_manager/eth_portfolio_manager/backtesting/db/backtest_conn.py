from sqlalchemy import create_engine, Engine
from sqlalchemy.orm import sessionmaker, Session


def get_backtest_engine() -> Engine:
    """Get database engine with correct connection parameters"""
    return create_engine(
        "postgresql+psycopg2://postgres:postgres@localhost:5432/backtest",
        # Remove search_path as it might be causing issues
    )


def get_backtest_session() -> Session:
    """Get a session with proper connection parameters"""
    engine = get_backtest_engine()
    Session = sessionmaker(bind=engine)
    return Session()


def get_connection_string() -> str:
    """Get standardized connection string"""
    return "postgresql+psycopg2://postgres:postgres@localhost:5432/backtest" 