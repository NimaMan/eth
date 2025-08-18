from collections import defaultdict, OrderedDict
from eth_data.chain_utils.common_addresses import fee_recipients_set, DENOM_ADDRESSES, ERC20_TOKEN_DECIMALS


class ProcessedTxStateDiffCalculator:
    """
    ProcessedTxStateDiffCalculator: Tracks and analyzes state changes in Ethereum transactions.

    This class calculates net state changes for addresses involved in transactions by:
    1. Tracking individual token movements per token contract address
    2. Tracking ETH/WETH movements 
    3. Handling special cases like WETH conversions and bribes
    4. Filtering out insignificant state changes based on thresholds

    Key Features:
    - Maintains chronological order of transfers using OrderedDict
    - Tracks individual tokens separately with known symbols (USDC, USDT, DAI) or contract addresses
    - Only applies decimal conversion for tokens with known decimals (no RPC calls)
    - Tracks both incoming and outgoing movements for tokens and ETH
    - Handles special addresses (WETH, null, dead addresses)
    - Identifies significant state changes based on configurable thresholds
    - Excludes WETH conversions from ETH movements
    - Tracks bribe payments to known fee recipients
    - Returns token_net as dictionary with symbols (stablecoins) or addresses (others)
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
        # Track movements per address per token contract
        # Structure: movements[address][token_address] = {"in": OrderedDict(), "out": OrderedDict()}
        self.movements = {
            "denom": defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()}),
            "tokens": defaultdict(lambda: defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()})),
        }

    # ------------------------------------------------------------------ #
    # Internal helpers
    # ------------------------------------------------------------------ #
    def _track_movement(
        self,
        movement_type: str,
        from_addr: str,
        to_addr: str,
        amount: float,
        transfer_id: tuple,
        token_address: str = None,
    ):
        """Track a movement between addresses and handle special cases."""
        # WETH conversions are denomination-neutral
        if to_addr == self.WETH_ADDRESS or from_addr == self.WETH_ADDRESS:
            return
        # Note: Removed incorrect deployer tx filter that was blocking log_index=0

        if movement_type == "denom":
            # Outgoing from `from_addr`
            self.movements["denom"][from_addr]["out"][transfer_id] = amount
            # Incoming to `to_addr` (unless denom bribe to fee recipient)
            if not (to_addr in fee_recipients_set):
                self.movements["denom"][to_addr]["in"][transfer_id] = amount
        elif movement_type == "token" and token_address:
            # Track per token address
            # Outgoing from `from_addr`
            self.movements["tokens"][from_addr][token_address]["out"][transfer_id] = amount
            # Incoming to `to_addr`
            self.movements["tokens"][to_addr][token_address]["in"][transfer_id] = amount

    # ------------------------------------------------------------------ #
    # Public API
    # ------------------------------------------------------------------ #
    def get_net_changes(self, from_address: str) -> dict:
        """
        Aggregate net changes for every address touched in this tx.

        Returns
        -------
        dict[address] -> {
            token_net : dict[token_symbol_or_address] -> float,
            eth_net : float,
            movements : {tokens:{token_addr:{in/out}}, denom:{in/out}}
        }
        """
        net = {}
        
        # Get all addresses that had any movements
        denom_addrs = set(self.movements["denom"].keys())
        token_addrs = set(self.movements["tokens"].keys())
        all_addrs = denom_addrs | token_addrs
        
        for addr in all_addrs:
            # Calculate denom (ETH) net change
            denom_in = sum(self.movements["denom"][addr]["in"].values())
            denom_out = sum(self.movements["denom"][addr]["out"].values())
            eth_net = denom_in - denom_out
            
            # Calculate token net changes per token
            token_net = {}
            total_token_movement = 0.0
            
            for token_addr, movements in self.movements["tokens"][addr].items():
                token_in = sum(movements["in"].values())
                token_out = sum(movements["out"].values())
                net_change = token_in - token_out
                
                if abs(net_change) > self.token_state_change_threshold:
                    # Use symbol for known stablecoins, address for others
                    if token_addr in DENOM_ADDRESSES:
                        token_key = DENOM_ADDRESSES[token_addr]  # Use symbol
                        # Apply decimal conversion only for known tokens
                        if token_key in ERC20_TOKEN_DECIMALS:
                            decimals = ERC20_TOKEN_DECIMALS[token_key]
                            net_change_formatted = net_change / (10 ** decimals)
                        else:
                            net_change_formatted = net_change  # Raw amount for unknown decimals
                    else:
                        token_key = token_addr  # Use contract address
                        net_change_formatted = net_change  # Raw amount for unknown tokens
                    
                    token_net[token_key] = net_change_formatted
                    total_token_movement += abs(net_change_formatted)
            
            # Include address if it meets thresholds or is the sender
            if (
                total_token_movement > 0
                or abs(eth_net) > self.eth_state_change_threshold
                or addr == from_address
            ):
                net[addr] = {
                    "token_net": token_net,
                    "eth_net": eth_net,
                    "movements": {
                        "tokens": dict(self.movements["tokens"][addr]),
                        "denom": self.movements["denom"][addr],
                    },
                }
        return net

    # ------------------------------------------------------------------ #
    # Convenience entry points
    # ------------------------------------------------------------------ #
    def calculate_state_changes(
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
            "denom": defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()}),
            "tokens": defaultdict(lambda: defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()})),
        }

        # ERC-20 transfers
        for tr in erc20_transfers:
            tid = (block_number, txn_index, tr["log_index"])
            is_weth = tr["token_address"] == self.WETH_ADDRESS
            if is_weth:
                # WETH transfers treated as denom (ETH)
                amount = tr["amount"] / 1e18
                self._track_movement(
                    "denom", tr["from_address"], tr["to_address"], amount, tid
                )
            else:
                # Regular token transfers
                self._track_movement(
                    "token", tr["from_address"], tr["to_address"], tr["amount"], tid, tr["token_address"]
                )

        # ETH (internal) transfers
        for tr in eth_transfers:
            log_index_or_depth = tr.get("log_index") or tr.get('depth')
            tid = (block_number, txn_index, log_index_or_depth)
            self._track_movement(
                "denom", tr["from_address"], tr["to_address"], tr["amount"], tid
            )

        try:
            return self.get_net_changes(from_address)
        except Exception as exc:
            if self.logger:
                self.logger.error(
                    f"State diff error for {txn_hash}: {exc}", exc_info=True
                )
            return {}

    def calculate_state_changes_from_processed_tx(self, processed_tx) -> dict:
        """
        Preferred path: use a `ProcessedTransaction` object that already contains
        parsed ERC-20 and internal transfers.
        """
        self.movements = {
            "denom": defaultdict(lambda: {"in": OrderedDict(), "out": OrderedDict()}),
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
                "denom", 
                processed_tx.from_address, 
                processed_tx.to_address, 
                float(processed_tx.value), 
                basic_transfer_id
            )

        # 1️⃣ ERC-20 transfers
        for tr in processed_tx.erc20_transfers:
            tid = (bn, txi, tr.log_index)
            is_weth = tr.token_address == self.WETH_ADDRESS
            if is_weth:
                # WETH transfers treated as denom (ETH)
                amount = float(tr.amount) / 1e18
                self._track_movement(
                    "denom", tr.from_address, tr.to_address, amount, tid
                )
            else:
                # Regular token transfers
                self._track_movement(
                    "token", tr.from_address, tr.to_address, float(tr.amount), tid, tr.token_address
                )

        # 2️⃣ Internal ETH transfers
        for internal_index, it in enumerate(processed_tx.internal_transactions):
            tid = (bn, txi, f"internal_{internal_index}")  # Use sequential index for uniqueness
            self._track_movement(
                "denom", it.from_address, it.to_address, float(it.value), tid
            )

        try:
            return self.get_net_changes(processed_tx.from_address)
        except Exception as exc:
            if self.logger:
                self.logger.error(
                    f"State diff calc failed for {processed_tx.hash}: {exc}",
                    exc_info=True,
                )
            return {}