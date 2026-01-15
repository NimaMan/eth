#!/usr/bin/env python3
"""
Setup script for the Transaction Validation Service

This script ensures the Python path is correctly configured so the validation
service can import the eth_data modules from the parent directory.
"""

import sys
import os
from pathlib import Path

def setup_python_path():
    """Add the eth_data package to Python path"""
    # Get the package root (two levels up from scripts/services)
    service_dir = Path(__file__).parent
    package_root = service_dir.parent.parent
    
    # Add to Python path if not already there
    package_root_str = str(package_root)
    if package_root_str not in sys.path:
        sys.path.insert(0, package_root_str)
        print(f"✅ Added {package_root_str} to Python path")
    else:
        print(f"✅ Package root already in Python path")
    
    # Verify imports work
    try:
        from eth_data.tx_processor.tx_processor import TransactionProcessor
        from eth_data.tx_processor.tx_data_fetcher import TransactionDataFetcher
        print("✅ Successfully imported eth_data modules")
        return True
    except ImportError as e:
        print(f"❌ Failed to import eth_data modules: {e}")
        print(f"   Current Python path: {sys.path}")
        print(f"   Package root: {package_root}")
        return False

def check_dependencies():
    """Check if required dependencies are installed"""
    required_packages = [
        'fastapi',
        'uvicorn',
        'web3',
        'aiohttp',
        'pydantic',
        'orjson'
    ]
    
    missing_packages = []
    for package in required_packages:
        try:
            __import__(package)
            print(f"✅ {package} is installed")
        except ImportError:
            missing_packages.append(package)
            print(f"❌ {package} is missing")
    
    if missing_packages:
        print(f"\n📚 Install missing packages with:")
        print(f"   pip install {' '.join(missing_packages)}")
        return False
    
    return True

def main():
    """Main setup function"""
    print("🔧 Setting up Transaction Validation Service...")
    
    # Setup Python path
    if not setup_python_path():
        return False
    
    # Check dependencies
    if not check_dependencies():
        return False
    
    print("\n✅ Validation service setup complete!")
    print("   You can now run: python validation_service.py")
    return True

if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)