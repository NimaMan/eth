"""
Initialize Transaction Executor Tables

This script creates the enhanced execution-related tables for ETH Kartal
within the live_trading_db database.

These tables provide more detailed execution tracking and performance metrics
beyond the basic executions table.
"""

import sys
import os
from datetime import datetime

# Add parent directory to path for imports
sys.path.append(os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))))

from sqlalchemy import create_engine, text
from sqlalchemy.orm import sessionmaker
from eth_data.database.schema.live_trading.tx_executor.tx_executor_models import Base
from eth_data.database.schema.live_trading.connection import get_db_url


def create_tx_executor_tables():
    """Create all transaction executor tables in live_trading_db."""
    print("🚀 Initializing Transaction Executor Tables")
    print("=========================================")
    
    # Get database URL
    db_url = get_db_url()
    if not db_url:
        db_url = "postgresql://postgres:password@localhost:5432/live_trading_db"
        print(f"Using default database URL: {db_url}")
    
    # Create engine
    engine = create_engine(db_url)
    
    # Create tables
    print("\n📋 Creating tables...")
    Base.metadata.create_all(engine)
    print("✅ Tables created successfully")
    
    # Show table summary
    print("\n📊 Table Summary:")
    
    tables = [
        'execution_wallets',
        'execution_details',
        'execution_errors'
    ]
    
    with engine.connect() as conn:
        for table in tables:
            try:
                result = conn.execute(text(f"SELECT COUNT(*) FROM {table}"))
                count = result.scalar_one()
                print(f"  - {table}: {count} rows")
            except Exception as e:
                print(f"  - {table}: Not created (error: {str(e)})")
        conn.commit()
    engine.dispose()
    
    print("\n✅ Transaction Executor initialization complete!")
    print("\n📝 Next steps:")
    print("1. Insert rows into execution_wallets by mapping to wallets.id (FK)")
    print("2. Start ETH Kartal signal processor to begin processing")


def drop_tx_executor_tables():
    """Drop all transaction executor tables (USE WITH CAUTION)."""
    print("⚠️  WARNING: This will drop all transaction executor tables!")
    response = input("Are you sure? (yes/no): ")
    
    if response.lower() != 'yes':
        print("Cancelled.")
        return
    
    db_url = get_db_url()
    if not db_url:
        db_url = "postgresql://postgres:password@localhost:5432/live_trading_db"
    
    engine = create_engine(db_url)
    
    # Drop tables in reverse order of dependencies
    tables_to_drop = [
        'execution_errors',
        'execution_details',
        'execution_wallets'
    ]
    
    with engine.connect() as conn:
        for table in tables_to_drop:
            try:
                conn.execute(text(f"DROP TABLE IF EXISTS {table} CASCADE"))
                print(f"Dropped {table}")
            except Exception as e:
                print(f"Error dropping {table}: {str(e)}")
        conn.commit()
    
    engine.dispose()
    print("✅ Tables dropped")


def show_schema_info():
    """Display information about the tx_executor schema."""
    print("\n📚 Transaction Executor Schema Information")
    print("==========================================")
    
    print("\n🗄️  Database: live_trading_db")
    print("\n📋 Tables:")
    
    print("\n1. execution_wallets")
    print("   - Manages wallets for trade execution")
    print("   - Tracks performance and risk limits")
    
    print("\n2. execution_details")
    print("   - Detailed execution records with performance metrics")
    print("   - Tracks gas optimization and MEV protection")
    print("   - Includes gas prices directly (no separate history table)")
    
    print("\n3. execution_errors")
    print("   - Detailed error tracking for debugging")
    
    print("\n🔗 Integration Points:")
    print("   - Links to existing trade_signals table")
    print("   - References live_positions for position tracking")
    print("   - Can reference eth_db.pools for pool data")


if __name__ == "__main__":
    import argparse
    
    parser = argparse.ArgumentParser(description="Initialize Transaction Executor Tables")
    parser.add_argument('--drop', action='store_true', help='Drop all tables first')
    parser.add_argument('--info', action='store_true', help='Show schema information')
    
    args = parser.parse_args()
    
    if args.info:
        show_schema_info()
    elif args.drop:
        drop_tx_executor_tables()
    else:
        create_tx_executor_tables()
