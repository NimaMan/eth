"""
Uniswap V2 Pool Implementation

Classic AMM Design:
- **Constant Product Formula**: x * y = k invariant
- **Equal Weight Pools**: 50/50 token distribution
- **Sync-based Updates**: Reserves updated via Sync events
- **Simple Interface**: getReserves(), token0(), token1()

Event Processing:
- univ2_syncs: Reserve updates (most frequent, tracks all state changes)
- univ2_swaps: Trade execution with amounts and pricing
- univ2_mints: Liquidity provision with LP token minting
- univ2_burns: Liquidity removal with LP token burning

Blockchain Interface:
- getReserves(): Current reserve amounts and last update timestamp
- token0()/token1(): Token addresses in deterministic order
- Direct reserve queries for real-time validation

Price Calculation:
- price = reserve1 / reserve0 (token1 per token0)
- Tracks price history for analytics and validation
- Price impact calculations using constant product formula
"""

from typing import Optional, Tuple, Dict, List
from .base_pool import BasePool
from eth_block_processor.data_models.txn_models import ProcessedTransaction


# Uniswap V2 Pair contract ABI for getReserves()
UNISWAP_V2_PAIR_ABI = [
    {
        "constant": True,
        "inputs": [],
        "name": "getReserves",
        "outputs": [
            {"internalType": "uint112", "name": "_reserve0", "type": "uint112"},
            {"internalType": "uint112", "name": "_reserve1", "type": "uint112"},
            {"internalType": "uint32", "name": "_blockTimestampLast", "type": "uint32"}
        ],
        "payable": False,
        "stateMutability": "view",
        "type": "function"
    },
    {
        "constant": True,
        "inputs": [],
        "name": "token0",
        "outputs": [{"internalType": "address", "name": "", "type": "address"}],
        "payable": False,
        "stateMutability": "view",
        "type": "function"
    },
    {
        "constant": True,
        "inputs": [],
        "name": "token1",
        "outputs": [{"internalType": "address", "name": "", "type": "address"}],
        "payable": False,
        "stateMutability": "view",
        "type": "function"
    }
]


class UniswapV2Pool(BasePool):
    """
    Uniswap V2 pool that processes its own events.
    
    V2 pools emit Sync events on every state change, making them
    easy to track.
    """
    
    def __init__(self, pool_address: str, token_address: str, 
                 denom_address: str, token1_is_denom: bool = True):
        """
        Initialize V2 pool.
        
        Args:
            pool_address: The pool contract address
            token_address: Our token's address
            denom_address: The paired token address (WETH, USDC, etc.)
            token1_is_denom: Whether token1 is the denomination token
        """
        super().__init__(pool_address, token_address, denom_address, token1_is_denom)
            
        # LP Token tracking (V2 pools ARE ERC20 LP tokens)
        self.lp_decimals = 18  # V2 LP tokens always have 18 decimals
        self.lp_total_supply = 0.0  # Total LP tokens in circulation
        self.lp_holders = {}  # Dict[address, balance] - LP token holders
        self.lp_transfers = []  # List of all LP token transfers
        self.lp_mint_events = []  # Liquidity addition events
        self.lp_burn_events = []  # Liquidity removal events
    
    def get_protocol(self) -> str:
        return "V2"
        
    def process_transaction(self, transaction: ProcessedTransaction):
        """
        Process V2 events from a transaction.
        
        V2 events we care about:
        - univ2_syncs: Reserve updates
        - univ2_swaps: Trade events
        - univ2_mints: Liquidity additions
        - univ2_burns: Liquidity removals
        """
        # Process sync events
        for sync in getattr(transaction, 'uniswap_v2_syncs', []):
            if sync.get('pair_address', '').lower() == self.pool_address.lower():
                self._process_sync(sync, transaction)
                
        # Process swap events  
        for swap in getattr(transaction, 'uniswap_v2_swaps', []):
            if swap.get('pair_address', '').lower() == self.pool_address:
                self._process_swap(swap, transaction)
                
        # Process mint events
        for mint in getattr(transaction, 'mints', getattr(transaction, 'uniswap_v2_mints', [])):
            if mint.get('token_address', mint.get('pair_address', '')).lower() == self.pool_address:
                self._process_mint(mint, transaction)
                
        # Process burn events
        for burn in getattr(transaction, 'burns', getattr(transaction, 'uniswap_v2_burns', [])):
            if burn.get('token_address', burn.get('pair_address', '')).lower() == self.pool_address:
                self._process_burn(burn, transaction)

    
    def _process_sync(self, sync: dict, transaction: ProcessedTransaction):
        """Process a sync event to update reserves."""
        # Get decimals for proper conversion
        token0_decimals = self._get_token0_decimals()
        token1_decimals = self._get_token1_decimals()
        
        # Update reserves from sync event (convert from raw values)
        reserve0_raw = float(sync.get('reserve0', 0))
        reserve1_raw = float(sync.get('reserve1', 0))
        
        # Handle cases where decimals might be None
        if token0_decimals is not None and token1_decimals is not None:
            reserve0 = reserve0_raw / (10 ** token0_decimals)
            reserve1 = reserve1_raw / (10 ** token1_decimals)
        else:
            # Skip processing if we can't get decimals
            if hasattr(self, 'logger') and self.logger:
                self.logger.warning(f"Cannot process sync for pool {self.pool_address}: missing decimals (token0: {token0_decimals}, token1: {token1_decimals})")
            return
        
        # Update pool state with reserve tracker
        timestamp = transaction.block_timestamp
        if hasattr(timestamp, 'timestamp'):
            timestamp = int(timestamp.timestamp())
        else:
            timestamp = int(timestamp) if timestamp else 0
            
        self.update_reserves(
            reserve0=reserve0, 
            reserve1=reserve1, 
            block_number=transaction.block_number,
            timestamp=timestamp,
            tx_hash=transaction.hash
        )
        
        # Store sync event
        self.sync_events.append({
            'block': transaction.block_number,
            'txn_hash': transaction.hash,
            'reserve0': reserve0,
            'reserve1': reserve1,
            'timestamp': transaction.block_timestamp
        })

    def _process_swap(self, swap: dict, transaction: ProcessedTransaction):
        """Process a V2 swap event."""
        # Mark trading enabled
        self._mark_trading_enabled(transaction)
        
        # Extract swap data
        sender = swap.get('sender')
        to = swap.get('to')
        amount0_in = float(swap.get('amount0In', swap.get('amount0_in', 0)))
        amount1_in = float(swap.get('amount1In', swap.get('amount1_in', 0)))
        amount0_out = float(swap.get('amount0Out', swap.get('amount0_out', 0)))
        amount1_out = float(swap.get('amount1Out', swap.get('amount1_out', 0)))
        
        # Update volumes
        self.state.volume0_in += amount0_in
        self.state.volume1_in += amount1_in  
        self.state.volume0_out += amount0_out
        self.state.volume1_out += amount1_out
        self.state.total_swaps += 1
        
        # Determine trade direction
        is_buy = amount0_out > 0 and amount1_in > 0  # Getting token0 for token1
        is_sell = amount0_in > 0 and amount1_out > 0  # Giving token0 for token1
        
        # Store swap event
        self.swap_events.append({
            'block': transaction.block_number,
            'txn_hash': transaction.hash,
            'sender': sender,
            'to': to,
            'amount0_in': amount0_in,
            'amount1_in': amount1_in,
            'amount0_out': amount0_out,
            'amount1_out': amount1_out,
            'is_buy': is_buy,
            'is_sell': is_sell,
            'timestamp': transaction.block_timestamp
        })

    def _process_mint(self, mint: dict, transaction: ProcessedTransaction):
        """Process a V2 mint (add liquidity) event."""
        to = mint.get('to_address', mint.get('to'))
        amount = float(mint.get('amount', 0))
        
        self.state.total_liquidity += amount
        self.state.total_mints += 1
        
        # Store mint event
        self.mint_events.append({
            'block': transaction.block_number,
            'txn_hash': transaction.hash,
            'to': to,
            'amount': amount,
            'timestamp': transaction.block_timestamp
        })

    def _process_burn(self, burn: dict, transaction: ProcessedTransaction):
        """Process a V2 burn (remove liquidity) event."""
        from_address = burn.get('from_address', burn.get('sender'))
        amount = float(burn.get('amount', 0))
        
        # Check for significant liquidity removal
        if self.state.total_liquidity > 0:
            removal_pct = amount / self.state.total_liquidity * 100
            if removal_pct > 50:
                # Significant liquidity removal detected
                pass
                
        self.state.total_liquidity = max(0, self.state.total_liquidity - amount)
        self.state.total_burns += 1
        
        # Store burn event
        self.burn_events.append({
            'block': transaction.block_number,
            'txn_hash': transaction.hash,
            'from': from_address,
            'amount': amount,
            'timestamp': transaction.block_timestamp
        })
                

    def get_latest_sync(self) -> Optional[dict]:
        """Get the latest sync event."""
        return self.sync_events[-1] if self.sync_events else None
        
    def get_recent_swaps(self, count: int = 10) -> list:
        """Get the last N swaps."""
        return list(self.swap_events)[-count:]
        
    def calculate_price_impact(self, amount_in: float, is_token0: bool) -> float:
        """
        Calculate price impact for a given trade size.
        
        Args:
            amount_in: The amount of token being traded in
            is_token0: Whether the input token is token0
            
        Returns:
            Price impact as a percentage (e.g., 1.5 for 1.5%)
        """
        if is_token0:
            reserve_in = self.state.reserve0
            reserve_out = self.state.reserve1
        else:
            reserve_in = self.state.reserve1
            reserve_out = self.state.reserve0
            
        if reserve_in == 0 or reserve_out == 0:
            return 0.0
            
        # Calculate amount out based on constant product formula
        amount_in_with_fee = amount_in * 0.997  # V2 fee is 0.3%
        k = reserve_in * reserve_out
        new_reserve_in = reserve_in + amount_in_with_fee
        new_reserve_out = k / new_reserve_in
        amount_out = reserve_out - new_reserve_out
        
        # Calculate prices
        market_price = reserve_out / reserve_in
        exec_price = amount_out / amount_in
        
        # Calculate price impact
        price_impact = (abs(market_price - exec_price) / market_price) * 100
        return price_impact

    def _get_token0_decimals(self) -> int:
        """Get token0 decimals."""
        if self.token1_is_denom:
            # Our token is token0
            return self.get_token_decimals()
        else:
            # Denom token is token0
            return self.get_denom_decimals()

    def _get_token1_decimals(self) -> int:
        """Get token1 decimals."""
        if self.token1_is_denom:
            # Denom token is token1
            return self.get_denom_decimals()
        else:
            # Our token is token1
            return self.get_token_decimals()
    
    def get_reserves_from_blockchain(self, block_identifier='latest') -> Tuple[float, float, bool]:
        """
        Fetch actual pool reserves from the blockchain using getReserves() call.
        
        Args:
            block_identifier: Block number or 'latest'
            
        Returns:
            Tuple of (denom_reserve, token_reserve, success)
        """
        try:
            if not self.w3.is_connected():
                return 0.0, 0.0, False
                
            pair_contract = self.w3.eth.contract(
                address=self.w3.to_checksum_address(self.pool_address),
                abi=UNISWAP_V2_PAIR_ABI
            )
            
            token0 = pair_contract.functions.token0().call(block_identifier=block_identifier)
            token1 = pair_contract.functions.token1().call(block_identifier=block_identifier)
            reserves = pair_contract.functions.getReserves().call(block_identifier=block_identifier)
            
            reserve0, reserve1, _ = reserves
            
            denom_decimals = self.get_denom_decimals()
            token_decimals = self.get_token_decimals()  # Use actual token decimals, not hardcoded 18
            
            if token0.lower() == self.denom_address.lower():
                denom_reserve = float(reserve0) / (10 ** denom_decimals)
                token_reserve = float(reserve1) / (10 ** token_decimals)
            elif token1.lower() == self.denom_address.lower():
                denom_reserve = float(reserve1) / (10 ** denom_decimals)
                token_reserve = float(reserve0) / (10 ** token_decimals)
            else:
                return 0.0, 0.0, False
                
            return denom_reserve, token_reserve, True
            
        except Exception as e:
            return 0.0, 0.0, False
    
    def process_lp_transfer(self, transfer: Dict):
        """
        Process an LP token transfer for this V2 pool.
        
        V2 pools ARE ERC20 tokens, so transfers of the pool address
        are actually LP token transfers representing liquidity ownership.
        
        Args:
            transfer: Transfer event dict with from_address, to_address, amount, etc.
        """
        amount = float(transfer['amount']) / (10 ** self.lp_decimals)
        zero_address = '0x0000000000000000000000000000000000000000'
        
        # Store the transfer
        self.lp_transfers.append({
            'block_number': transfer.get('block_number'),
            'txn_hash': transfer.get('txn_hash'),
            'from_address': transfer['from_address'],
            'to_address': transfer['to_address'],
            'amount': amount,
            'log_index': transfer.get('log_index')
        })
        
        # Update holder balances
        if transfer['from_address'] != zero_address:
            if transfer['from_address'] not in self.lp_holders:
                self.lp_holders[transfer['from_address']] = 0
            self.lp_holders[transfer['from_address']] -= amount
            # Remove if balance is effectively zero
            if abs(self.lp_holders[transfer['from_address']]) < 1e-18:
                del self.lp_holders[transfer['from_address']]
                
        if transfer['to_address'] != zero_address:
            if transfer['to_address'] not in self.lp_holders:
                self.lp_holders[transfer['to_address']] = 0
            self.lp_holders[transfer['to_address']] += amount
            
        # Track mints and burns
        if transfer['from_address'] == zero_address:
            # Mint - liquidity added
            self.lp_total_supply += amount
            self.lp_mint_events.append(transfer)
        elif transfer['to_address'] == zero_address:
            # Burn - liquidity removed
            self.lp_total_supply -= amount
            self.lp_burn_events.append(transfer)
    
    def get_lp_share(self, address: str) -> float:
        """
        Get the percentage share of the pool owned by an address.
        
        Args:
            address: The address to check
            
        Returns:
            Percentage of pool owned (0-100)
        """
        if self.lp_total_supply == 0:
            return 0
        balance = self.lp_holders.get(address, 0)
        return (balance / self.lp_total_supply) * 100
    
    def get_lp_holders(self) -> List[Tuple[str, float, float]]:
        """
        Get the top N LP token holders.
        
        Args:
            n: Number of top holders to return
            
        Returns:
            List of tuples (address, balance, percentage_share)
        """
        
        result = {}
        for address, balance in self.lp_holders.items():
            if balance > 0:  # Only include positive balances
                share = self.get_lp_share(address)
                result[address] = (balance, share)
        # Sort by balance descending
        return dict(sorted(result.items(), key=lambda x: x[1][0], reverse=True))
    