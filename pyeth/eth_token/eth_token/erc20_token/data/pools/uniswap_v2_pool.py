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

LP Token Tracking:
- `LPTokenTracker` keeps ERC20 LP balances, approvals, and history isolated
  from the pool's AMM state transitions.
- Router approvals surface as early warnings for liquidity exits—key for
  scam detection workflows.
- The pool delegates LP transfer/approval events to the tracker, keeping AMM
  logic focused on reserves, pricing, and trading enablement.

Blockchain Interface:
- getReserves(): Current reserve amounts and last update timestamp
- token0()/token1(): Token addresses in deterministic order
- Direct reserve queries for real-time validation

Price Calculation:
- price = reserve1 / reserve0 (token1 per token0)
- Tracks price history for analytics and validation
- Price impact calculations using constant product formula
"""

from typing import Optional, Tuple, Dict, List, Iterable, Any
from dataclasses import dataclass, field
import pyreth
from .base_pool import BasePool
from .pool_chain_data_fetcher import PoolChainDataFetcher
from ..token_chain_data_fetcher import TokenChainDataFetcher
from eth_data.utils.type_converter import convert_scaled_amount
from eth_data.chain_utils.common_addresses import (
    ROUTER_ADDRESSES as KNOWN_ROUTERS,
    canonicalize_dex_pool_type,
)
UNISWAP_V2_PROTOCOL = canonicalize_dex_pool_type('UNISWAP-V2')


@dataclass
class ApprovalInfo:
    """Stores information about an LP token approval"""
    amount: float
    tx_hash: str
    block_number: int
    timestamp: Optional[int] = None
    

@dataclass
class LPHolderInfo:
    """Stores LP holder balance and approval information"""
    balance: float = 0.0
    approvals: Dict[str, ApprovalInfo] = field(default_factory=dict)  # spender -> approval info


class LPTokenTracker:
    """Tracks ERC20 LP balances, approvals, and related events."""

    ZERO_ADDRESS = '0x0000000000000000000000000000000000000000'
    BALANCE_EPSILON = 1e-18

    def __init__(
        self,
        *,
        lp_decimals: int = 18,
        known_routers: Optional[Iterable[str]] = None,
        history_limit: int = 100,
    ):
        self.lp_decimals = lp_decimals
        self.known_routers = set(known_routers or [])
        self.history_limit = history_limit

        self._holders: Dict[str, LPHolderInfo] = {}
        self._total_supply: float = 0.0

        # Event history (bounded)
        self._transfers: List[Dict[str, Any]] = []
        self._mint_events: List[Dict[str, Any]] = []
        self._burn_events: List[Dict[str, Any]] = []
        self._approval_events: List[Dict[str, Any]] = []

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------
    def _append_event(self, collection: List[Dict[str, Any]], entry: Dict[str, Any]) -> None:
        collection.append(entry)
        if len(collection) > self.history_limit:
            del collection[: len(collection) - self.history_limit]

    def _get_or_create_holder(self, address: str) -> LPHolderInfo:
        if address not in self._holders:
            self._holders[address] = LPHolderInfo()
        return self._holders[address]

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------
    def record_transfer(self, transfer: Dict[str, Any]) -> None:
        """
        Track an LP token transfer, updating balances and supply.

        Args:
            transfer: Raw ERC20 transfer event dict.
        """
        amount = float(transfer['amount']) / (10 ** self.lp_decimals)
        event = {
            'block_number': transfer.get('block_number'),
            'tx_hash': transfer.get('tx_hash'),
            'from_address': transfer['from_address'],
            'to_address': transfer['to_address'],
            'amount': amount,
            'log_index': transfer.get('log_index'),
        }
        self._append_event(self._transfers, event)

        from_address = transfer['from_address']
        to_address = transfer['to_address']

        if from_address != self.ZERO_ADDRESS:
            holder = self._get_or_create_holder(from_address)
            holder.balance -= amount
            if abs(holder.balance) < self.BALANCE_EPSILON and not holder.approvals:
                del self._holders[from_address]
        else:
            self._total_supply += amount
            # Keep the original shape for downstream consumers
            self._append_event(self._mint_events, dict(transfer))

        if to_address != self.ZERO_ADDRESS:
            holder = self._get_or_create_holder(to_address)
            holder.balance += amount
        else:
            self._total_supply = max(0.0, self._total_supply - amount)
            self._append_event(self._burn_events, dict(transfer))

    def record_approval(self, approval: Dict[str, Any]) -> ApprovalInfo:
        """
        Track an LP token approval, returning normalized approval info.

        Args:
            approval: Approval event dict with owner, spender, value, etc.

        Returns:
            ApprovalInfo instance for the approval.
        """
        owner = approval['owner']
        spender = approval['spender']
        raw_value = approval.get('value', approval.get('amount', 0))
        amount = convert_scaled_amount(raw_value, self.lp_decimals)

        holder = self._get_or_create_holder(owner)
        approval_info = ApprovalInfo(
            amount=amount,
            tx_hash=approval.get('tx_hash', ''),
            block_number=approval.get('block_number', 0),
            timestamp=approval.get('block_timestamp'),
        )
        holder.approvals[spender] = approval_info

        event = {
            'block_number': approval.get('block_number'),
            'tx_hash': approval.get('tx_hash'),
            'owner': owner,
            'spender': spender,
            'amount': amount,
            'is_router': spender in self.known_routers,
            'timestamp': approval.get('block_timestamp'),
        }
        self._append_event(self._approval_events, event)
        return approval_info

    # ------------------------------------------------------------------
    # Accessors
    # ------------------------------------------------------------------
    @property
    def total_supply(self) -> float:
        return self._total_supply

    def get_balances(self) -> Dict[str, float]:
        """Return current LP balances by address."""
        return {
            address: holder.balance
            for address, holder in self._holders.items()
        }

    def get_share(self, address: str) -> float:
        """Return the percentage ownership for an address."""
        if self._total_supply == 0:
            return 0.0
        holder = self._holders.get(address)
        if not holder:
            return 0.0
        return (holder.balance / self._total_supply) * 100

    def get_holder_snapshots(self) -> Dict[str, Dict[str, Any]]:
        """Return balance, share, and approval details for each holder."""
        result: Dict[str, Dict[str, Any]] = {}
        for address, holder in self._holders.items():
            if holder.balance <= 0 and not holder.approvals:
                continue
            approvals = {
                spender: {
                    'amount': info.amount,
                    'tx_hash': info.tx_hash,
                    'block_number': info.block_number,
                    'is_router': spender in self.known_routers,
                }
                for spender, info in holder.approvals.items()
            }
            result[address] = {
                'balance': holder.balance,
                'share': self.get_share(address),
                'approvals': approvals,
            }
        return dict(
            sorted(result.items(), key=lambda item: item[1]['balance'], reverse=True)
        )

    def get_total_approved_to_routers(self) -> float:
        """Return the aggregate LP amount approved to known routers."""
        total_approved = 0.0
        for holder in self._holders.values():
            for spender, approval in holder.approvals.items():
                if spender in self.known_routers:
                    effective = min(approval.amount, holder.balance)
                    total_approved += effective
        return total_approved

    def get_approved_percentage(self) -> float:
        """Return percentage of total supply approved to known routers."""
        if self._total_supply == 0:
            return 0.0
        return (self.get_total_approved_to_routers() / self._total_supply) * 100

    def get_last_approval_block(self) -> Optional[int]:
        if not self._approval_events:
            return None
        return self._approval_events[-1].get('block_number')

    def get_last_approval_event(self) -> Optional[Dict[str, Any]]:
        if not self._approval_events:
            return None
        return dict(self._approval_events[-1])

    def get_holders_with_approvals(self) -> List[str]:
        return [
            address
            for address, holder in self._holders.items()
            if holder.approvals
        ]

    # ------------------------------------------------------------------
    # Event history accessors
    # ------------------------------------------------------------------
    @property
    def transfers(self) -> List[Dict[str, Any]]:
        return self._transfers

    @property
    def mint_events(self) -> List[Dict[str, Any]]:
        return self._mint_events

    @property
    def burn_events(self) -> List[Dict[str, Any]]:
        return self._burn_events

    @property
    def approval_events(self) -> List[Dict[str, Any]]:
        return self._approval_events


class UniswapV2Pool(BasePool):
    """
    Uniswap V2 pool that processes its own events.
    
    V2 pools emit Sync events on every state change, making them
    easy to track.
    """
    
    def __init__(
        self,
        pool_address: str,
        token_address: str,
        denom_address: str,
        *,
        token_decimals: Optional[int] = None,
        denom_decimals: Optional[int] = None,
        token1_is_denom: bool = True,
        pool_chain_fetcher: Optional[PoolChainDataFetcher] = None,
        token_chain_fetcher: Optional[TokenChainDataFetcher] = None,
        history_limit: int = 1000,
    ):
        """
        Initialize V2 pool.
        
        Args:
            pool_address: The pool contract address
            token_address: Our token's address
            denom_address: The paired token address (WETH, USDC, etc.)
            token1_is_denom: Whether token1 is the denomination token
        """
        super().__init__(
            pool_address=pool_address,
            token_address=token_address,
            denom_address=denom_address,
            token_decimals=token_decimals,
            denom_decimals=denom_decimals,
            token1_is_denom=token1_is_denom,
            pool_chain_fetcher=pool_chain_fetcher,
            token_chain_fetcher=token_chain_fetcher,
            history_limit=history_limit,
        )
            
        # LP Token tracking (V2 pools ARE ERC20 LP tokens)
        self.lp_decimals = 18  # V2 LP tokens always have 18 decimals
        self.lp_tracker = LPTokenTracker(
            lp_decimals=self.lp_decimals,
            known_routers=KNOWN_ROUTERS,
            history_limit=self.history_limit,
        )

    def get_protocol(self) -> str:
        return UNISWAP_V2_PROTOCOL

    def evaluate_trading_status(self, transaction: Dict) -> None:
        if self.can_buy and self.can_sell:
            return
        
        config = pyreth.PoolBuySellParameters()
        config.test_amount_eth = float(self.test_buy_amount_eth)
        config.token_decimals = int(self.get_token_decimals())
        config.block_number = int(transaction['block_number'])
        block_header = transaction.get('block_header')
        if block_header:
            config.set_block_header_json(block_header)

        # The tranaction is already mined, so we dont need to include it as a prior tx 
        simulator = self.pyreth_client.pool_buy_sell_simulator()
        result = simulator.check_uniswap_v2_pool(
            self.token_address,
            self.pool_address,
            config,
        )

        if result.can_buy and not self.can_buy:
            self.can_buy = True
            self.can_buy_block = transaction['block_number']
            self.can_buy_tx = transaction['hash']
            self.can_buy_timestamp = transaction.get('block_timestamp', 0)

        self.can_sell = bool(result.can_sell)
        self.buy_tax = result.buy_tax_percentage
        self.sell_tax = result.sell_tax_percentage
        self.tax_check_block = transaction['block_number']
        self.tax_check_tx = transaction['hash']
    
    def process_transaction(self, transaction: Dict):
        """Process V2 events from a transaction.        
        - univ2_syncs: Reserve updates
        - univ2_swaps: Trade events
        - univ2_mints: Liquidity additions
        - univ2_burns: Liquidity removals
        """
        # Process sync events
        if transaction.get('uniswap_v2_syncs'):
            for sync in transaction['uniswap_v2_syncs']:
                if sync.get('pair_address', '') == self.pool_address:
                    self._process_sync(sync, transaction)
                
        # Process swap events  
        if transaction.get('uniswap_v2_swaps'):
            for swap in transaction['uniswap_v2_swaps']:
                if swap.get('pair_address', '') == self.pool_address:
                    self._process_swap(swap, transaction)
                
        # Process mint events
        mint_events = transaction.get('mints', [])
        if not mint_events and 'uniswap_v2_mints' in transaction:
            mint_events = transaction['uniswap_v2_mints']
        
        for mint in mint_events:
            if mint.get('token_address', mint.get('pair_address', '')) == self.pool_address:
                self._process_mint(mint, transaction)
                
        # Process burn events
        # Check both 'burns' and 'uniswap_v2_burns' attributes
        burn_events = transaction.get('burns', [])
        if not burn_events and 'uniswap_v2_burns' in transaction:
            burn_events = transaction['uniswap_v2_burns']
            
        for burn in burn_events:
            if burn.get('token_address', burn.get('pair_address', '')) == self.pool_address:
                self._process_burn(burn, transaction)
        
        # Check if trading is enabled after processing all events
        self.check_and_update_trading_status(transaction)

    def _process_sync(self, sync: dict, transaction: Dict):
        """Process a sync event to update reserves."""
        # Get decimals for proper conversion
        token0_decimals = self._get_token0_decimals()
        token1_decimals = self._get_token1_decimals()
        
        # Update reserves from sync event (convert from raw values)
        reserve0_raw = float(sync['reserve0'])
        reserve1_raw = float(sync['reserve1'])
        
        # Handle cases where decimals might be None
        if token0_decimals is not None and token1_decimals is not None:
            reserve0 = reserve0_raw / (10 ** token0_decimals)
            reserve1 = reserve1_raw / (10 ** token1_decimals)
        else:
            raise RuntimeError(
                "UniswapV2Pool._process_sync: "
                f"missing decimals (token0={token0_decimals}, token1={token1_decimals}) for pool {self.pool_address}"
            )
            
        self.update_reserves(
            reserve0=reserve0, 
            reserve1=reserve1, 
            block_number=transaction['block_number'],
            timestamp=transaction["block_timestamp"],
            tx_hash=transaction['hash']
        )
        
        # Store sync event
        self._append_event(self.sync_events, {
            'block': transaction['block_number'],
            'tx_hash': transaction['hash'],
            'reserve0': reserve0,
            'reserve1': reserve1,
            'timestamp': transaction["block_timestamp"],
        })

    def _process_swap(self, swap: dict, transaction: Dict):
        """Process a V2 swap event."""
        # Mark token as buyable from first swap event
        self.mark_can_buy_from_event(transaction, event_type='swap')
        
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
        self._append_event(self.swap_events, {
            'block': transaction['block_number'],
            'tx_hash': transaction['hash'],
            'sender': sender,
            'to': to,
            'amount0_in': amount0_in,
            'amount1_in': amount1_in,
            'amount0_out': amount0_out,
            'amount1_out': amount1_out,
            'is_buy': is_buy,
            'is_sell': is_sell,
            'timestamp': transaction["block_timestamp"]
        })

    def _process_mint(self, mint: dict, transaction: Dict):
        """Process a V2 mint (add liquidity) event."""
        to = mint.get('to_address', mint.get('to'))
        amount = float(mint.get('amount', 0))
        
        self.state.total_liquidity += amount
        self.state.total_mints += 1
        
        # Store mint event
        self._append_event(self.mint_events, {
            'block': transaction['block_number'],
            'tx_hash': transaction['hash'],
            'to': to,
            'amount': amount,
            'timestamp': transaction["block_timestamp"]
        })

    def _process_burn(self, burn: dict, transaction: Dict):
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
        self._append_event(self.burn_events, {
            'block': transaction['block_number'],
            'tx_hash': transaction['hash'],
            'from': from_address,
            'amount': amount,
            'timestamp': transaction["block_timestamp"],
        })

    def get_latest_sync(self) -> Optional[dict]:
        return self.sync_events[-1] if self.sync_events else None
        
    def get_recent_swaps(self, count: int = 10) -> list:
        return list(self.swap_events)[-count:]
        
    def _get_token0_decimals(self) -> int:
        return self.get_token_decimals() if self.token1_is_denom else self.get_denom_decimals()

    def _get_token1_decimals(self) -> int:
        return self.get_denom_decimals() if self.token1_is_denom else self.get_token_decimals()
    
    def process_lp_transfer(self, transfer: Dict):
        self.lp_tracker.record_transfer(transfer)
    
    def process_lp_approval(self, approval: Dict):
        self.lp_tracker.record_approval(approval)
    
    def get_lp_share(self, address: str) -> float:
        return self.lp_tracker.get_share(address)
    
    def get_lp_holders(self) -> Dict[str, Dict]:
        return self.lp_tracker.get_holder_snapshots()
    
    def get_total_approved_to_routers(self) -> float:
        return self.lp_tracker.get_total_approved_to_routers()
    
    def get_lp_approved_percentage(self) -> float:
        return self.lp_tracker.get_approved_percentage()
    
    def get_last_lp_approval_block(self) -> Optional[int]:
        return self.lp_tracker.get_last_approval_block()

    def get_last_lp_approval_event(self) -> Optional[Dict[str, Any]]:
        return self.lp_tracker.get_last_approval_event()
    
    def get_holders_with_approvals(self) -> List[str]:
        return self.lp_tracker.get_holders_with_approvals()
    
    # ------------------------------------------------------------------
    # LP tracker views (for backwards compatibility with callers that
    # expect attributes on the pool instance)
    # ------------------------------------------------------------------
    @property
    def lp_total_supply(self) -> float:
        return self.lp_tracker.total_supply

    @property
    def lp_holders(self) -> Dict[str, float]:
        return self.lp_tracker.get_balances()

    @property
    def lp_transfers(self) -> List[Dict[str, Any]]:
        return self.lp_tracker.transfers

    @property
    def lp_mint_events(self) -> List[Dict[str, Any]]:
        return self.lp_tracker.mint_events

    @property
    def lp_burn_events(self) -> List[Dict[str, Any]]:
        return self.lp_tracker.burn_events

    @property
    def lp_approval_events(self) -> List[Dict[str, Any]]:
        return self.lp_tracker.approval_events
    
    def get_pool_data_for_publishing(self) -> Dict:
        """
        Get pool data with LP approval percentage for publishing.
        
        Returns:
            Dict with pool data including lp_tokens_approved_percentage
        """
        pool_data = {
            'pool_address': self.pool_address,
            'token_reserve': self.state.reserve_token,
            'denom_reserve': self.state.reserve_denom,
            'trading_enabled': self.trading_enabled,
            'trading_enabled_block': self.trading_enabled_block,
            'trading_enabled_tx': self.trading_enabled_tx,
            'lp_tokens_approved_percentage': self.get_lp_approved_percentage()
        }
        return pool_data
    
