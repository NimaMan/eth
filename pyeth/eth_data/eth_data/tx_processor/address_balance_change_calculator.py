from collections import defaultdict, OrderedDict
from eth_data.chain_utils.common_addresses import fee_recipients_set, DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS


class AddressBalanceChangeCalculator:
    """
    AddressBalanceChangeCalculator: Tracks and analyzes address balance changes in Ethereum transactions.

    This class calculates net balance changes for addresses involved in transactions by:
    1. Tracking unknown token movements with contract addresses as keys
    2. Tracking known currency movements (ETH, USDC, USDT, DAI, MKR, etc.) with symbols as keys
    3. Handling special cases like WETH conversions and bribes
    4. Filtering out insignificant state changes based on thresholds

    Key Features:
    - Maintains chronological order of transfers using OrderedDict
    - Clean separation: token_net uses addresses, currency_net uses symbols
    - Automatically recognizes 100+ currencies from DENOM_ADDRESSES
    - Only applies decimal conversion for tokens with known decimals (no RPC calls)
    - Tracks both incoming and outgoing movements for tokens and currencies
    - Handles special addresses (WETH, null, dead addresses)
    - Identifies significant state changes based on configurable thresholds
    - Excludes WETH conversions from ETH movements
    - Tracks bribe payments to known fee recipients
    - Returns token_net with contract addresses only (unknown tokens)
    - Returns currency_net with symbols only (tokens in DENOM_ADDRESSES)
    - No overlap between token_net and currency_net
    """

    def __init__(
        self,
        eth_state_change_threshold: float = 0.0005,
        token_state_change_threshold: float = 0.1,
        logger=None,
    ):
        self.logger = logger
        self.WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        self.eth_state_change_threshold = eth_state_change_threshold
        self.token_state_change_threshold = token_state_change_threshold
        # Track movements per address per currency/token
        # Structure: 
        #   currencies[address][currency_symbol] = {"in": OrderedDict(), "out": OrderedDict()}
        #   tokens[address][token_address] = {"in": OrderedDict(), "out": OrderedDict()}
        self.movements = {
            "currencies": defaultdict(lambda: defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()})),
            "tokens": defaultdict(lambda: defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()})),
        }

    def _track_movement(
        self,
        movement_type: str,
        from_addr: str,
        to_addr: str,
        amount: float,
        transfer_id: tuple,
        token_address: str = None,
        currency: str = None,
    ):
        """Track a movement between addresses and handle special cases.
        
        Args:
            movement_type: "currency" or "token"
            from_addr: Source address
            to_addr: Target address
            amount: Amount transferred
            transfer_id: Unique transfer identifier (block, txn_index, log_index)
            token_address: Token contract address (for unknown token movements)
            currency: Currency symbol (ETH, USDC, USDT, etc.) for currency movements
        """
        # WETH conversions are denomination-neutral
        if to_addr == self.WETH_ADDRESS or from_addr == self.WETH_ADDRESS:
            return
        
        if movement_type == "currency" and currency:
            # Track per currency (ETH, USDC, USDT, DAI, etc.)
            # Outgoing from `from_addr`
            self.movements["currencies"][from_addr][currency]["out"][transfer_id] = amount
            # Incoming to `to_addr` (unless it's a bribe to fee recipient for ETH)
            if not (currency == "ETH" and to_addr in fee_recipients_set):
                self.movements["currencies"][to_addr][currency]["in"][transfer_id] = amount
                    
        elif movement_type == "token" and token_address:
            # Track per token address (for unknown tokens not in DENOM_ADDRESSES)
            # Outgoing from `from_addr`
            self.movements["tokens"][from_addr][token_address]["out"][transfer_id] = amount
            # Incoming to `to_addr`
            self.movements["tokens"][to_addr][token_address]["in"][transfer_id] = amount

    def get_net_balance_changes(self, from_address: str) -> dict:
        """
        Aggregate net changes for every address touched in this tx.

        Returns
        -------
        dict[address] -> {
            token_net : dict[token_address] -> float,  # Unknown tokens only, raw amounts
            currency_net : dict[currency_symbol] -> float,  # Known currencies only, decimal-adjusted
            movements : {
                tokens: {token_addr: {in/out}},  # Raw amounts
                currencies: {currency_symbol: {in/out}}  # Decimal-adjusted amounts
            }
        }
        """
        net = {}
        
        # Get all addresses that had any movements
        token_addrs = set(self.movements["tokens"].keys())
        currency_addrs = set(self.movements["currencies"].keys())
        all_addrs = token_addrs | currency_addrs
        
        for addr in all_addrs:
            # Calculate currency net changes per currency
            currency_net = {}
            total_currency_movement = 0.0
            
            for currency_symbol, movements in self.movements["currencies"][addr].items():
                currency_in = sum(movements["in"].values())
                currency_out = sum(movements["out"].values())
                net_change = currency_in - currency_out
                
                # Apply threshold based on currency type
                if currency_symbol == "ETH":
                    # ETH is now in wei, convert threshold to wei for comparison
                    threshold = self.eth_state_change_threshold * 1e18
                else:
                    threshold = self.token_state_change_threshold
                
                if abs(net_change) > threshold:
                    # For display: convert ETH from wei to ether, keep others as is
                    if currency_symbol == "ETH":
                        # Convert wei to ETH for display (but keep original for movements)
                        currency_net[currency_symbol] = net_change / 1e18  # Display in ETH
                    else:
                        currency_net[currency_symbol] = net_change
                    total_currency_movement += abs(net_change)
            
            # Calculate token net changes per token (only for tokens NOT in DENOM_ADDRESSES)
            token_net = {}
            total_token_movement = 0.0
            
            for token_addr, movements in self.movements["tokens"][addr].items():
                # Skip tokens that are in DENOM_ADDRESSES - they go to currency_net
                if token_addr not in DENOM_ADDRESSES:
                    token_in = sum(movements["in"].values())
                    token_out = sum(movements["out"].values())
                    net_change = token_in - token_out
                    
                    if abs(net_change) > self.token_state_change_threshold:
                        # Always use contract address as key, raw amount as value
                        token_net[token_addr] = net_change
                        total_token_movement += abs(net_change)
            
            # Include address if it meets thresholds or is the sender
            if (
                total_token_movement > 0
                or total_currency_movement > 0
                or addr == from_address
            ):
                net[addr] = {
                    "token_net": token_net,
                    "currency_net": currency_net,
                    "movements": {
                        "tokens": dict(self.movements["tokens"][addr]),
                        "currencies": dict(self.movements["currencies"][addr]),
                    },
                }
        return net

    # ------------------------------------------------------------------ #
    # Convenience entry points
    # ------------------------------------------------------------------ #
    def calculate_address_balance_changes(
        self,
        txn_hash: str,
        from_address: str,
        block_number: int,
        txn_index: int,
        eth_transfers: list,
        erc20_transfers: list,
    ) -> dict:
        """
        Build movements directly from raw transfer lists
        (legacy entry point).
        """
        self.movements = {
            "currencies": defaultdict(lambda: defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()})),
            "tokens": defaultdict(lambda: defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()})),
        }

        # ERC-20 transfers
        for tr in erc20_transfers:
            tid = (block_number, txn_index, tr["log_index"])
            token_addr = tr["token_address"]
            
            if token_addr in DENOM_ADDRESSES:
                # Known currencies/stablecoins - track as currency with decimal conversion
                currency_symbol = DENOM_ADDRESSES[token_addr]
                
                # Special handling for WETH - always treat as ETH
                if currency_symbol == 'WETH':
                    currency_symbol = 'ETH'
                
                # Apply decimal conversion if known
                if currency_symbol in ERC20_TOKEN_DECIMALS:
                    decimals = ERC20_TOKEN_DECIMALS[currency_symbol]
                    amount = tr["amount"] / (10 ** decimals)
                else:
                    amount = tr["amount"]
                    
                self._track_movement(
                    "currency", tr["from_address"], tr["to_address"], amount, tid, currency=currency_symbol
                )
            else:
                # Unknown tokens - track as regular tokens with raw amounts
                self._track_movement(
                    "token", tr["from_address"], tr["to_address"], tr["amount"], tid, token_address=token_addr
                )

        # ETH (internal) transfers
        for tr in eth_transfers:
            log_index_or_depth = tr.get("log_index") or tr.get('depth')
            tid = (block_number, txn_index, log_index_or_depth)
            self._track_movement(
                "currency", tr["from_address"], tr["to_address"], tr["amount"], tid, currency="ETH"
            )

        try:
            return self.get_net_balance_changes(from_address)
        except Exception as exc:
            if self.logger:
                self.logger.error(
                    f"State diff error for {txn_hash}: {exc}", exc_info=True
                )
            return {}

    def calculate_address_balance_changes_from_processed_tx(self, processed_tx) -> dict:
        """
        Preferred path: use a `ProcessedTransaction` object that already contains
        parsed ERC-20 and internal transfers.
        """
        self.movements = {
            "currencies": defaultdict(lambda: defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()})),
            "tokens": defaultdict(lambda: defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()})),
        }

        bn, txi = processed_tx.block_number, processed_tx.txn_index

        # 0️⃣ Handle basic transaction value transfer (if any)
        # Skip if there's a depth-0 internal transaction that already captures this transfer
        has_depth_zero_internal = any(
            it.depth == 0 and 
            it.from_address == processed_tx.from_address and 
            it.to_address == processed_tx.to_address and
            abs(float(it.value) - float(processed_tx.value)) < 1e-18  # Use epsilon for float comparison
            for it in processed_tx.internal_transactions
        )
        
        if processed_tx.value > 0 and processed_tx.to_address and not has_depth_zero_internal:
            # Basic ETH transfer from sender to receiver (only if not already captured internally)
            basic_transfer_id = (bn, txi, "basic_transfer")
            self._track_movement(
                "currency", 
                processed_tx.from_address, 
                processed_tx.to_address, 
                # Store ETH amounts in wei to keep units consistent with internal transfers
                float(processed_tx.value) * 1e18,
                basic_transfer_id,
                currency="ETH"
            )

        # 1️⃣ ERC-20 transfers
        for tr in processed_tx.erc20_transfers:
            tid = (bn, txi, tr.log_index)
            token_addr = tr.token_address
            
            if token_addr in DENOM_ADDRESSES:
                # Known currencies/stablecoins - track as currency with decimal conversion
                currency_symbol = DENOM_ADDRESSES[token_addr]
                
                # Special handling for WETH - always treat as ETH
                if currency_symbol == 'WETH':
                    currency_symbol = 'ETH'
                
                # Apply decimal conversion if known
                if currency_symbol in ERC20_TOKEN_DECIMALS:
                    decimals = ERC20_TOKEN_DECIMALS[currency_symbol]
                    amount = float(tr.amount) / (10 ** decimals)
                else:
                    amount = float(tr.amount)
                    
                self._track_movement(
                    "currency", tr.from_address, tr.to_address, amount, tid, currency=currency_symbol
                )
            else:
                # Unknown tokens - track as regular tokens with raw amounts
                self._track_movement(
                    "token", tr.from_address, tr.to_address, float(tr.amount), tid, token_address=token_addr
                )

        # 2️⃣ Internal ETH transfers
        for internal_index, it in enumerate(processed_tx.internal_transactions):
            tid = (bn, txi, f"internal_{internal_index}")  # Use sequential index for uniqueness
            self._track_movement(
                "currency", it.from_address, it.to_address, float(it.value), tid, currency="ETH"
            )

        try:
            return self.get_net_balance_changes(processed_tx.from_address)
        except Exception as exc:
            if self.logger:
                self.logger.error(
                    f"State diff calc failed for {processed_tx.hash}: {exc}",
                    exc_info=True,
                )
            return {}
