import pytest
from unittest.mock import MagicMock
from web3 import Web3

# Adjust the path to import from the project's root
import sys
import os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from eth_token.erc20_token.data.pools.pool_manager import PoolManager
from eth_token.erc20_token.data.pools.uniswap_v2_pool import UniswapV2Pool
from eth_token.erc20_token.data.pools.uniswap_v3_pool import UniswapV3Pool
from eth_token.erc20_token.data.pools.uniswap_v4_pool import UniswapV4Pool, PoolKey
from eth_block_processor.data_models.txn_models import ProcessedTransaction

# --- Test Fixtures ---

@pytest.fixture
def logger():
    """Mock logger fixture."""
    return MagicMock()

@pytest.fixture
def w3():
    """Web3 instance fixture."""
    # This should be configured to connect to your test node
    return Web3(Web3.HTTPProvider('http://127.0.0.1:8545'))

@pytest.fixture
def mock_transaction():
    """Fixture for a mock ProcessedTransaction."""
    tx = ProcessedTransaction(
        hash="0xdeadbeef",
        block_number=1,
        block_timestamp=1622544000,
        from_address="0x1",
        to_address="0x2",
        value=0,
        gas_used=0,
        gas_price=0
    )
    return tx

# --- Test Cases ---

def test_pool_manager_creation(logger):
    """Test that the PoolManager can be created successfully."""
    pm = PoolManager("0xToken", logger)
    assert pm is not None
    assert pm.token_address == Web3.to_checksum_address("0xToken")
    assert pm.logger is not None

def test_v2_pool_creation_and_processing(logger, mock_transaction):
    """
    Tests end-to-end V2 pool discovery and processing.
    It simulates a `PairCreated` event and then a `Sync` event.
    """
    token_address = "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"  # USDC
    denom_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"  # WETH
    pool_address = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
    
    pm = PoolManager(token_address, logger)
    
    # 1. Test Pool Creation
    creation_event = {
        'pair_address': pool_address,
        'token0': token_address,
        'token1': denom_address
    }
    mock_transaction.pair_events = [creation_event]
    
    pm.process_transaction(mock_transaction)
    
    assert pool_address in pm.pools
    pool = pm.get_pool(pool_address)
    assert isinstance(pool, UniswapV2Pool)
    assert pool.get_protocol() == "V2"
    
    # 2. Test Sync Event Processing
    sync_event = {
        'pair_address': pool_address,
        'reserve0': 1000 * (10**6), # USDC has 6 decimals
        'reserve1': 2 * (10**18)    # WETH has 18 decimals
    }
    mock_transaction.pair_events = []
    mock_transaction.uniswap_v2_syncs = [sync_event]
    
    # Mock the decimal fetching for this test
    pool.get_token_decimals = MagicMock(return_value=6)
    pool.get_denom_decimals = MagicMock(return_value=18)
    
    pm.process_transaction(mock_transaction)
    
    assert pool.state.reserve0 == 1000.0
    assert pool.state.reserve1 == 2.0
    assert pytest.approx(pool.get_price()) == 2.0 / 1000.0

def test_v3_pool_creation_and_processing(logger, mock_transaction):
    """
    Tests end-to-end V3 pool discovery and event processing,
    verifying that concentrated liquidity logic is handled.
    """
    token_address = "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"  # USDC
    denom_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"  # WETH
    pool_address = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"
    
    pm = PoolManager(token_address, logger)
    
    # 1. Test Pool Creation
    creation_event = {
        'pool_address': pool_address,
        'token0': token_address,
        'token1': denom_address,
        'fee': 500
    }
    mock_transaction.uniswap_v3_pools = [creation_event]
    pm.process_transaction(mock_transaction)
    
    assert pool_address in pm.pools
    pool = pm.get_pool(pool_address)
    assert isinstance(pool, UniswapV3Pool)
    
    # 2. Test Mint Event Processing (Concentrated Liquidity)
    mint_event = {
        'pool_address': pool_address,
        'tick_lower': -200000,
        'tick_upper': -190000,
        'amount': 10**15 # some liquidity amount
    }
    mock_transaction.uniswap_v3_pools = []
    mock_transaction.univ3_mints = [mint_event]
    
    pm.process_transaction(mock_transaction)
    
    # Assert that the tick was updated correctly
    assert -200000 in pool.ticks
    assert -190000 in pool.ticks
    assert pool.ticks[-200000].liquidity_net == 10**15
    assert pool.ticks[-190000].liquidity_net == -10**15
    
    # 3. Test Swap Event to trigger reserve calculation
    swap_event = {
        'pool_address': pool_address,
        'tick': -195000,
        'sqrt_price_x96': 5602277099426993177874900000000000, # Corresponds to a price
        'amount0': 1000,
        'amount1': 0.5
    }
    mock_transaction.univ3_mints = []
    mock_transaction.univ3_swaps = [swap_event]
    
    # Mock the decimal fetching for this test
    pool.get_token_decimals = MagicMock(return_value=6)
    pool.get_denom_decimals = MagicMock(return_value=18)
    
    pm.process_transaction(mock_transaction)
    
    # Check that active liquidity and reserves were calculated
    assert pool.current_tick == -195000
    assert pool.current_liquidity == 10**15
    assert pool.state.reserve0 > 0
    assert pool.state.reserve1 > 0

def test_v4_pool_creation_and_processing(logger, mock_transaction):
    """
    Tests end-to-end V4 pool discovery and event processing.
    """
    token_address = "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"  # USDC
    denom_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"  # WETH
    
    pool_key = PoolKey(
        currency0=token_address,
        currency1=denom_address,
        fee=3000,
        tick_spacing=60,
        hooks="0x0000000000000000000000000000000000000000"
    )
    # A dummy pool_id for testing purposes
    pool_id = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"

    pm = PoolManager(token_address, logger)

    # 1. Test Pool Creation (via Initialize event)
    init_event = {
        'pool_id': pool_id,
        'currency0': token_address,
        'currency1': denom_address,
        'fee': 3000,
        'tick_spacing': 60,
        'hooks': "0x0000000000000000000000000000000000000000"
    }
    mock_transaction.uniswap_v4_initializes = [init_event]
    pm.process_transaction(mock_transaction)

    assert pool_id in pm.v4_pools
    pool = pm.v4_pools[pool_id]
    assert isinstance(pool, UniswapV4Pool)

    # 2. Test ModifyLiquidity Event
    modify_event = {
        'pool_id': pool_id,
        'tick_lower': 80000,
        'tick_upper': 90000,
        'liquidity_delta': 10**18
    }
    mock_transaction.uniswap_v4_initializes = []
    mock_transaction.uniswap_v4_modifies = [modify_event]
    
    pm.process_transaction(mock_transaction)

    # Assert that tick liquidity was updated correctly
    assert 80000 in pool.ticks
    assert pool.ticks[80000].liquidity_net == 10**18

    # 3. Test Swap Event
    swap_event = {
        'pool_id': pool_id,
        'tick': 85000,
        'sqrt_price_x96': 5602277099426993177874900000000000,
        'amount0': 500,
        'amount1': 0.2
    }
    mock_transaction.uniswap_v4_modifies = []
    mock_transaction.uniswap_v4_swaps = [swap_event]

    # Mock the decimal fetching for this test
    pool.get_token_decimals = MagicMock(return_value=6)
    pool.get_denom_decimals = MagicMock(return_value=18)
    
    pm.process_transaction(mock_transaction)
    
    assert pool.current_tick == 85000
    assert pool.state.reserve0 > 0
    assert pool.state.reserve1 > 0


# --- E2E Test against Real Chain Data ---

# Using the well-known USDC/WETH V2 pool for our E2E test
USDC_WETH_V2_POOL = "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc"
USDC_ADDRESS = "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"

# A recent block number to test against. Using a specific historical block
# ensures the test is deterministic and repeatable.
TEST_BLOCK_NUMBER = 18000000

@pytest.mark.e2e
def test_v2_pool_against_real_chain(logger, w3):
    """
    Tests the V2 pool against real historical data from the blockchain.

    This test fetches the actual reserves of a pool at a specific block,
    simulates the processing of the corresponding Sync event, and asserts
    that our calculated state matches the on-chain ground truth.
    """
    pm = PoolManager(USDC_ADDRESS, logger)

    # 1. Discover the pool via a mock creation event
    creation_event = {
        'pair_address': USDC_WETH_V2_POOL,
        'token0': USDC_ADDRESS,
        'token1': WETH_ADDRESS
    }
    mock_creation_tx = ProcessedTransaction(hash="0xcreate", block_number=TEST_BLOCK_NUMBER - 1)
    mock_creation_tx.pair_events = [creation_event]
    pm.process_transaction(mock_creation_tx)
    
    pool = pm.get_pool(USDC_WETH_V2_POOL)
    assert pool is not None

    # 2. Get the "ground truth" reserves from the blockchain at the specific block
    v2_abi = [{"constant":True,"inputs":[],"name":"getReserves","outputs":[{"internalType":"uint112","name":"_reserve0","type":"uint112"},{"internalType":"uint112","name":"_reserve1","type":"uint112"},{"internalType":"uint32","name":"_blockTimestampLast","type":"uint32"}],"stateMutability":"view","type":"function"}]
    pool_contract = w3.eth.contract(address=pool.pool_address, abi=v2_abi)
    
    try:
        ground_truth_reserves = pool_contract.functions.getReserves().call(block_identifier=TEST_BLOCK_NUMBER)
    except Exception as e:
        pytest.fail(f"Failed to get reserves from blockchain node. Is it running and synced? Error: {e}")

    # 3. Simulate processing a Sync event with the ground truth data
    sync_event = {
        'pair_address': USDC_WETH_V2_POOL,
        'reserve0': ground_truth_reserves[0],
        'reserve1': ground_truth_reserves[1]
    }
    mock_sync_tx = ProcessedTransaction(hash="0xsync", block_number=TEST_BLOCK_NUMBER)
    mock_sync_tx.uniswap_v2_syncs = [sync_event]

    # Mock the decimal fetching to avoid another chain call in the test
    pool.get_token_decimals = MagicMock(return_value=6)
    pool.get_denom_decimals = MagicMock(return_value=18)

    pm.process_transaction(mock_sync_tx)

    # 4. Assert that our final state matches the ground truth
    expected_reserve0 = ground_truth_reserves[0] / (10**6)  # USDC decimals
    expected_reserve1 = ground_truth_reserves[1] / (10**18) # WETH decimals
    
    assert pytest.approx(pool.state.reserve0) == expected_reserve0
    assert pytest.approx(pool.state.reserve1) == expected_reserve1

    print(f"\nE2E Test Passed for V2 Pool {USDC_WETH_V2_POOL} at block {TEST_BLOCK_NUMBER}")
    print(f"  Calculated Reserve 0 (USDC): {pool.state.reserve0}")
    print(f"  Ground Truth Reserve 0 (USDC): {expected_reserve0}")
    print(f"  Calculated Reserve 1 (WETH): {pool.state.reserve1}")
    print(f"  Ground Truth Reserve 1 (WETH): {expected_reserve1}") 