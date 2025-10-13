#!/usr/bin/env python3
"""
Backtest Database Creation Script

Objective:
----------
This script sets up the backtest database by:
  1. Dropping the existing "backtest" database if it exists.
  2. Creating a fresh "backtest" database.
  3. Creating all tables defined in our SQLAlchemy models.

Each token position is stored as a JSONB column containing the complete
aggregated token position (both static data and dynamic history) as produced via TokenPosition.to_full_dict().
This enables full historical traceability and analysis directly from the JSON field.

Usage:
------
Run this script from the command line to recreate the backtest database with the latest schema.
"""

import psycopg2
from psycopg2 import sql
from psycopg2.extensions import ISOLATION_LEVEL_AUTOCOMMIT
from sqlalchemy import create_engine
from eth_data.database.token_position_db_models import Base


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
    """Create all tables defined in models using SQLAlchemy"""
    try:
        # Create SQLAlchemy engine
        engine = create_engine(
            "postgresql+psycopg2://postgres:postgres@localhost:5432/backtest"
        )

        # Create all tables as defined in the Base's metadata.
        # Note: The TokenPosition table now includes a column "token_position" of JSONB type,
        # which stores the complete aggregated token position (static and dynamic) as a JSON blob.
        Base.metadata.create_all(engine)
        print("Created database tables")
    except Exception as e:
        print(f"Table creation failed: {e}")
        raise


if __name__ == "__main__":
    drop_database()
    create_backtest_database()
    create_tables() 