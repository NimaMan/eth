#!/usr/bin/env python3
"""
Create Live Trading Database Tables

This script creates all necessary tables for the live_trading_db database.
Run this once to set up the database schema.
"""

import psycopg2
from psycopg2.extensions import ISOLATION_LEVEL_AUTOCOMMIT
import os
import sys
from sqlalchemy import create_engine
from live_trading_models import Base

def create_database_if_not_exists():
    """Create live_trading_db database if it doesn't exist."""
    try:
        # Connect to PostgreSQL server
        conn = psycopg2.connect(
            host=os.getenv('PGHOST', 'localhost'),
            port=os.getenv('PGPORT', '5432'),
            user=os.getenv('PGUSER', 'postgres'),
            password=os.getenv('PGPASSWORD', ''),
            database='postgres'  # Connect to default database
        )
        conn.set_isolation_level(ISOLATION_LEVEL_AUTOCOMMIT)
        cur = conn.cursor()
        
        # Check if database exists
        cur.execute("SELECT 1 FROM pg_database WHERE datname = 'live_trading_db'")
        exists = cur.fetchone()
        
        if not exists:
            print("Creating database live_trading_db...")
            cur.execute("CREATE DATABASE live_trading_db")
            print("✅ Database created successfully")
        else:
            print("✅ Database live_trading_db already exists")
        
        cur.close()
        conn.close()
        
    except Exception as e:
        print(f"❌ Error creating database: {e}")
        return False
    
    return True

def create_tables():
    """Create all tables defined in the models."""
    try:
        # Build connection string
        db_url = f"postgresql://{os.getenv('PGUSER', 'postgres')}:{os.getenv('PGPASSWORD', '')}@{os.getenv('PGHOST', 'localhost')}:{os.getenv('PGPORT', '5432')}/live_trading_db"
        
        # Create engine
        engine = create_engine(db_url)
        
        # Create all tables
        print("Creating tables in live_trading_db...")
        Base.metadata.create_all(engine)
        
        print("✅ All tables created successfully")
        
        # List created tables
        with engine.connect() as conn:
            result = conn.execute("""
                SELECT tablename FROM pg_tables 
                WHERE schemaname = 'public' 
                ORDER BY tablename
            """)
            tables = [row[0] for row in result]
            
            print("\n📊 Created tables:")
            for table in tables:
                print(f"  - {table}")
        
    except Exception as e:
        print(f"❌ Error creating tables: {e}")
        return False
    
    return True

def main():
    """Main function to create database and tables."""
    print("🚀 Setting up live_trading_db...")
    
    # Step 1: Create database
    if not create_database_if_not_exists():
        sys.exit(1)
    
    # Step 2: Create tables
    if not create_tables():
        sys.exit(1)
    
    print("\n✅ Live trading database setup complete!")
    print("\n📝 Next steps:")
    print("1. Verify tables were created correctly")
    print("2. Set up proper permissions for your application user")
    print("3. Consider adding initial data (e.g., wallet records)")

if __name__ == "__main__":
    main()