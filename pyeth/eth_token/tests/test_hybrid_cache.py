"""
Test script for HybridLiveTokensCache
"""

import time
import struct
import mmap
from eth_token.token_manager.hybrid_live_tokens_cache import (
    HybridLiveTokensCache, 
    HEADER_SIZE, 
    TOKEN_RECORD_SIZE,
    SharedMemoryHeader
)
from eth_token.erc20_token.erc20_token import ERC20Token
from eth_token.erc20_token.data.erc20_token_data import ERC20TokenData


def create_test_token(address: str, symbol: str = "TEST", total_supply: int = 1000000) -> ERC20Token:
    """Create a test token for testing"""
    token_data = ERC20TokenData(
        contract_address=address,
        name=f"Test Token {symbol}",
        symbol=symbol,
        decimals=18,
        total_supply=total_supply,
        creation_block=20000000,
        creation_timestamp=int(time.time()),
        trading_enabled=True,
        trading_enabled_block=20000001
    )
    
    # Create token instance
    token = ERC20Token(contract_address=address)
    token.token_data = token_data
    
    # Add some pool data
    token.token_data.pool_info = {
        'V2': [{
            'pool_address': '0x' + '1' * 40,
            'reserve0': 100.5,  # ETH
            'reserve1': 50000.0,  # Token
            'pool_type': 'V2'
        }],
        'V3': [{
            'pool_address': '0x' + '2' * 40,
            'reserve0': 200.0,
            'reserve1': 100000.0,
            'pool_type': 'V3'
        }]
    }
    
    return token


def test_hybrid_cache():
    """Test the hybrid cache functionality"""
    print("Testing HybridLiveTokensCache...")
    
    # Create cache instance
    cache = HybridLiveTokensCache(
        max_size=100,
        shm_size=10_000_000,  # 10MB
        enable_shared_memory=True
    )
    
    print(f"Shared memory stats: {cache.get_shm_stats()}")
    
    # Test adding tokens
    print("\nAdding test tokens...")
    test_tokens = []
    for i in range(5):
        addr = f"0x{'0' * 38}{i:02x}"
        token = create_test_token(addr, f"TOKEN{i}", 1000000 * (i + 1))
        cache[addr] = token
        test_tokens.append(token)
        print(f"Added token {addr}")
    
    print(f"\nShared memory stats after adding: {cache.get_shm_stats()}")
    
    # Verify shared memory content
    print("\nVerifying shared memory content...")
    verify_shared_memory(cache.shm_path)
    
    # Test deletion
    print("\nTesting deletion...")
    del_addr = test_tokens[2].token_data.contract_address
    del cache[del_addr]
    print(f"Deleted token {del_addr}")
    print(f"Shared memory stats after deletion: {cache.get_shm_stats()}")
    
    # Test update
    print("\nTesting update...")
    update_token = test_tokens[0]
    update_token.token_data.total_supply = 2000000
    cache[update_token.token_data.contract_address] = update_token
    print(f"Updated token {update_token.token_data.contract_address}")
    
    # Cleanup
    cache.cleanup()
    print("\nTest completed successfully!")


def verify_shared_memory(shm_path: str):
    """Verify the contents of shared memory"""
    try:
        # Open and map the shared memory file
        with open(shm_path, 'rb') as f:
            data = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
            
            # Read header
            header_data = data[:24]
            header = SharedMemoryHeader.unpack(header_data)
            
            print(f"\nShared Memory Header:")
            print(f"  Magic: 0x{header.magic_number:08X}")
            print(f"  Version: {header.version}")
            print(f"  Last Update: {header.last_update_timestamp}")
            print(f"  Num Tokens: {header.num_tokens}")
            print(f"  Record Size: {header.token_record_size}")
            
            # Read first few token records
            print(f"\nToken Records:")
            for i in range(min(3, header.num_tokens)):
                offset = HEADER_SIZE + (i * TOKEN_RECORD_SIZE)
                record = data[offset:offset + TOKEN_RECORD_SIZE]
                
                # Parse token address
                token_addr = '0x' + record[0:20].hex()
                
                # Parse total supply
                total_supply = int.from_bytes(record[20:52], 'big')
                
                # Parse decimals
                decimals = record[52]
                
                # Parse creation block
                creation_block = int.from_bytes(record[53:61], 'big')
                
                print(f"\n  Token {i}:")
                print(f"    Address: {token_addr}")
                print(f"    Total Supply: {total_supply}")
                print(f"    Decimals: {decimals}")
                print(f"    Creation Block: {creation_block}")
                
                # Parse V2 pool data
                v2_pool_addr = '0x' + record[77:97].hex()
                v2_eth_reserve = int.from_bytes(record[97:129], 'big') / 10**18
                v2_token_reserve = int.from_bytes(record[129:161], 'big') / 10**decimals
                
                if v2_pool_addr != '0x' + '0' * 40:
                    print(f"    V2 Pool: {v2_pool_addr}")
                    print(f"      ETH Reserve: {v2_eth_reserve:.2f}")
                    print(f"      Token Reserve: {v2_token_reserve:.2f}")
            
            data.close()
            
    except Exception as e:
        print(f"Error verifying shared memory: {e}")


def test_concurrent_access():
    """Test concurrent access to shared memory"""
    import multiprocessing
    
    def reader_process(shm_path: str):
        """Reader process that reads from shared memory"""
        try:
            with open(shm_path, 'rb') as f:
                data = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
                
                # Read header
                header_data = data[:24]
                header = SharedMemoryHeader.unpack(header_data)
                
                print(f"Reader process - Num tokens: {header.num_tokens}")
                
                # Read a token record
                if header.num_tokens > 0:
                    offset = HEADER_SIZE
                    record = data[offset:offset + 20]
                    token_addr = '0x' + record.hex()
                    print(f"Reader process - First token: {token_addr}")
                
                data.close()
        except Exception as e:
            print(f"Reader process error: {e}")
    
    print("\nTesting concurrent access...")
    
    # Create cache and add token
    cache = HybridLiveTokensCache(enable_shared_memory=True)
    token = create_test_token("0x" + "1" * 40)
    cache[token.token_data.contract_address] = token
    
    # Start reader process
    p = multiprocessing.Process(target=reader_process, args=(cache.shm_path,))
    p.start()
    
    # Add more tokens while reader is active
    for i in range(2, 5):
        addr = f"0x{'0' * 38}{i:02x}"
        token = create_test_token(addr)
        cache[addr] = token
        time.sleep(0.1)
    
    p.join()
    cache.cleanup()
    print("Concurrent access test completed!")


if __name__ == "__main__":
    print("=" * 60)
    print("HybridLiveTokensCache Test Suite")
    print("=" * 60)
    
    # Run basic tests
    test_hybrid_cache()
    
    print("\n" + "=" * 60)
    
    # Run concurrent access test
    test_concurrent_access()
    
    print("\n" + "=" * 60)
    print("All tests completed!")