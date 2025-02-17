#!/usr/bin/env python3
import psycopg2
from psycopg2 import sql
from psycopg2.extensions import ISOLATION_LEVEL_AUTOCOMMIT
from sqlalchemy import create_engine
from eth_portfolio_manager.backtesting.db.models import Base, TokenPosition, StrategyRun


def drop_database():
    """Drop the backtest database if it exists"""
    try:
        conn = psycopg2.connect(
            user="postgres",
            password="postgres",
            host="localhost",
            port=5432
        )
        conn.set_isolation_level(ISOLATION_LEVEL_AUTOCOMMIT)
        cursor = conn.cursor()
        
        # Terminate all connections to the database
        cursor.execute("""
            SELECT pg_terminate_backend(pg_stat_activity.pid)
            FROM pg_stat_activity
            WHERE pg_stat_activity.datname = 'backtest'
            AND pid <> pg_backend_pid();
        """)
        
        cursor.execute("DROP DATABASE IF EXISTS backtest")
        print("Dropped existing database")
        
    except Exception as e:
        print(f"Database drop failed: {e}")
        raise
    finally:
        if 'conn' in locals():
            conn.close()


def create_backtest_database():
    """Create backtest database if not exists"""
    try:
        # Connect to default postgres DB
        conn = psycopg2.connect(
            user="postgres",
            password="postgres",
            host="localhost",
            port=5432
        )
        conn.set_isolation_level(ISOLATION_LEVEL_AUTOCOMMIT)
        cursor = conn.cursor()
        
        # Check if database exists
        cursor.execute("SELECT 1 FROM pg_database WHERE datname='backtest'")
        exists = cursor.fetchone()
        
        if not exists:
            cursor.execute(sql.SQL("CREATE DATABASE backtest"))
            print("Created backtest database")
        else:
            print("Backtest database already exists")
            
    except Exception as e:
        print(f"Database creation failed: {e}")
        raise
    finally:
        if 'conn' in locals():
            conn.close()


def create_tables():
    """Create all tables defined in models"""
    try:
        # Create SQLAlchemy engine
        engine = create_engine(
            "postgresql+psycopg2://postgres:postgres@localhost:5432/backtest"
        )
        
        # Create all tables
        Base.metadata.create_all(engine)
        print("Created database tables")
        
    except Exception as e:
        print(f"Table creation failed: {e}")
        raise


if __name__ == "__main__":
    drop_database()
    create_backtest_database()
    create_tables() 