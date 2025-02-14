from typing import Dict, Any, List
from web3 import Web3
from eth_block_processor.contracts.function_signatures import EVENT_TOPICS
from eth_block_processor.data_models.receipt_models import *
from eth_block_processor.data_models.uniswap_v3_models import *


class TransactionLogProcessor:
    def __init__(self, w3: Web3):
        self.w3 = w3

    def process_logs(self, logs: List[Dict[str, Any]]) -> Dict[str, List[Any]]:
        result = {
            'erc20_transfers': [],
            'erc721_transfers': [],
            'erc1155_transfers': [],
            'uniswap_v2_syncs': [],
            'uniswap_v2_swaps': [],
            'mints': [],
            'burns': [],
            'deposits': [],
            'withdraws': [],
            'contract_creation_events': [],
            'pair_events': [],
            'approvals': [],
            'owner_events': [],
            'trading_enabled_events': [],
            'trading_disabled_events': [],
            'other_events': [],
            'unique_addresses': set(),
            'erc20_contracts': set(),
            'uniswap_v3_pools': [],
            'uniswap_v3_initializations': [],
            'uniswap_v3_mints': [],
            'uniswap_v3_swaps': [],
            'uniswap_v3_positions': [],
            'uniswap_v3_increases': [],
            'uniswap_v3_decreases': [],
        }
        
        for log in logs:
            try:
                event = self.classify_and_parse_log(log)
            except Exception as e:
                # TODO: log if you want to get the unparsed log
                continue
            if isinstance(event, ERC20Transfer):
                result['erc20_transfers'].append(event)
                result['unique_addresses'].add(event.from_address)
                result['unique_addresses'].add(event.to_address)
                result['erc20_contracts'].add(event.token_address)
            elif isinstance(event, ERC721Transfer):
                result['erc721_transfers'].append(event)
                result['unique_addresses'].add(event.from_address)
                result['unique_addresses'].add(event.to_address)
            elif isinstance(event, ERC1155Transfer):
                result['erc1155_transfers'].append(event)
                result['unique_addresses'].add(event.from_address)
                result['unique_addresses'].add(event.to_address)
            elif isinstance(event, UniswapV2Sync):
                result['uniswap_v2_syncs'].append(event)
                result['unique_addresses'].add(event.pair_address)
            elif isinstance(event, UniswapV2Swap):
                result['uniswap_v2_swaps'].append(event)
                result['unique_addresses'].add(event.sender)
                result['unique_addresses'].add(event.to)
                result['unique_addresses'].add(event.pair_address)
            elif isinstance(event, MintAction):
                result['mints'].append(event)
                result['unique_addresses'].add(event.pair_address)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, BurnAction):
                result['burns'].append(event)
                result['unique_addresses'].add(event.pair_address)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, DepositAction):
                result['deposits'].append(event)
                result['unique_addresses'].add(event.pair_address)
                if self.w3.is_address(event.sender):
                    result['unique_addresses'].add(event.sender)
            elif isinstance(event, WithdrawAction):
                result['withdraws'].append(event)
                result['unique_addresses'].add(event.pair_address)
                result['unique_addresses'].add(event.sender)
            elif isinstance(event, PairAction):
                result['pair_events'].append(event)
                result['unique_addresses'].add(event.pair_address)
                result['unique_addresses'].add(event.token0)
                result['unique_addresses'].add(event.token1)
            elif isinstance(event, ERC20Approval):
                result['approvals'].append(event)
                result['unique_addresses'].add(event.token_address)
                result['unique_addresses'].add(event.owner)
                result['unique_addresses'].add(event.spender)
                result['erc20_contracts'].add(event.token_address)
            elif isinstance(event, ERC721Approval):
                result['approvals'].append(event)
                result['unique_addresses'].add(event.token_address)
                result['unique_addresses'].add(event.owner)
                result['unique_addresses'].add(event.approved_address)
            elif isinstance(event, OwnerEvent):
                result['owner_events'].append(event)
                result['unique_addresses'].add(event.contract_address)
                result['unique_addresses'].add(event.previous_owner)
                result['unique_addresses'].add(event.new_owner)
            elif isinstance(event, TradingEnabledEvent):
                result['trading_enabled_events'].append(event)
                result['unique_addresses'].add(event.token_address)
                result['erc20_contracts'].add(event.token_address)
            elif isinstance(event, TradingDisabledEvent):
                result['trading_disabled_events'].append(event)
                result['unique_addresses'].add(event.token_address)
                result['erc20_contracts'].add(event.token_address)
            elif isinstance(event, UniswapV3PoolCreated):
                result['uniswap_v3_pools'].append(event)
                result['erc20_contracts'].add(event.token0)
                result['erc20_contracts'].add(event.token1)
                result['unique_addresses'].add(event.pool)
                result['unique_addresses'].add(event.token0)
                result['unique_addresses'].add(event.token1)
            elif isinstance(event, UniswapV3Initialize):
                result['uniswap_v3_initializations'].append(event)
                result['unique_addresses'].add(event.pool_address)
            elif isinstance(event, UniswapV3Mint):
                result['uniswap_v3_mints'].append(event)
                result['unique_addresses'].add(event.pool_address)
                result['unique_addresses'].add(event.sender)
                result['unique_addresses'].add(event.owner)
            elif isinstance(event, UniswapV3Position):
                result['uniswap_v3_positions'].append(event)
                result['unique_addresses'].add(event.pool_address)
                result['unique_addresses'].add(event.owner)
            elif isinstance(event, UniswapV3IncreaseLiquidity):
                result['uniswap_v3_increases'].append(event)
                result['unique_addresses'].add(event.pool_address)
            elif isinstance(event, UniswapV3DecreaseLiquidity):
                result['uniswap_v3_decreases'].append(event)
                result['unique_addresses'].add(event.pool_address)
            elif isinstance(event, UniswapV3Swap):
                result['uniswap_v3_swaps'].append(event)
                result['unique_addresses'].add(event.pool_address)
                result['unique_addresses'].add(event.sender)
                result['unique_addresses'].add(event.recipient)
            else:
                result['other_events'].append(event)
        return result

    def classify_and_parse_log(self, log: Dict[str, Any]) -> Any:
        if not log['topics']:
            return None
        topic = log['topics'][0].hex() if isinstance(log['topics'][0], bytes) else log['topics'][0]
        if topic.startswith('0x'):
            topic = topic[2:]
        try:
            if topic == EVENT_TOPICS['Transfer']:
                return self.parse_transfer(log)
            elif topic == EVENT_TOPICS['TransferSingle']:
                return self.parse_erc1155_single_transfer(log)
            elif topic == EVENT_TOPICS['TransferBatch']:
                return self.parse_erc1155_batch_transfer(log)
            elif topic == EVENT_TOPICS['Sync']:
                return self.parse_uniswap_v2_sync(log)
            elif topic == EVENT_TOPICS['Swap']:
                return self.parse_uniswap_v2_swap(log)
            elif topic == EVENT_TOPICS['Mint']:
                return self.parse_mint(log)
            elif topic == EVENT_TOPICS['Approval']:
                return self.parse_approve(log)
            elif topic == EVENT_TOPICS['Burn']:
                return self.parse_burn(log)
            elif topic == EVENT_TOPICS['Deposit']:
                return self.parse_deposit(log)
            elif topic == EVENT_TOPICS['Withdraw']:
                return self.parse_withdraw(log)
            elif topic == EVENT_TOPICS['PairCreated']:
                return self.parse_pair(log)
            elif topic == EVENT_TOPICS['OwnershipTransferred']:
                return self.parse_owner(log)
            elif topic == EVENT_TOPICS['TradingEnabled']:
                return self.parse_trading_enabled(log)
            elif topic == EVENT_TOPICS['TradingDisabled']:
                return self.parse_trading_disabled(log)
            # Uniswap V3
            elif topic == EVENT_TOPICS['PoolCreatedV3']:
                return self.parse_uniswap_v3_pool_created(log)
            elif topic == EVENT_TOPICS['InitializeV3']:
                return self.parse_uniswap_v3_initialize(log)
            elif topic == EVENT_TOPICS['MintV3']:
                return self.parse_uniswap_v3_mint(log)
            elif topic == EVENT_TOPICS['BurnV3']:
                return self.parse_uniswap_v3_burn(log)
            elif topic == EVENT_TOPICS['SwapV3']:
                return self.parse_uniswap_v3_swap(log)
            elif topic == EVENT_TOPICS['IncreaseLiquidityV3']:
                return self.parse_uniswap_v3_increase_liquidity(log)
            elif topic == EVENT_TOPICS['DecreaseLiquidityV3']:
                return self.parse_uniswap_v3_decrease_liquidity(log)
            else:
                return self.parse_other_event(log)
        except Exception as e:
            raise e 

    def _ensure_hex_string(self, data) -> str:
        """Convert bytes or string data to a hex string with '0x' prefix"""
        if isinstance(data, bytes):
            data = data.hex()
        if not data.startswith('0x'):
            data = '0x' + data
        return data


    def _process_address(self, value: Union[str, bytes]) -> str:
        """Convert address to checksum format"""
        if isinstance(value, bytes):
            value = value.hex()
        address = value[-40:] if len(value) > 40 else value
        return self.w3.to_checksum_address('0x' + address if not address.startswith('0x') else address)

    def _process_integer(self, value: Union[str, bytes, int]) -> int:
        """Convert hex string or bytes to integer, handling empty cases"""
        if not value or value == '0x':
            return None
        if isinstance(value, int):
            return value
        if isinstance(value, bytes):
            value = value.hex()
        if isinstance(value, str):
            value = value[2:] if value.startswith('0x') else value
        return int(value, 16)

    def parse_transfer(self, log: Dict[str, Any]) -> ERC20Transfer:
        """Parse ERC20 Transfer event log Event signature: Transfer(address indexed from, address indexed to, uint256 value)
        Topic[0]: Event signature hash
        Topic[1]: from address (indexed)
        Topic[2]: to address (indexed)
        Data: value (uint256)
        """
        # Handle both hex string and bytes data formats
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return ERC20Transfer(
            token_address=self.w3.to_checksum_address(log['address']),
            from_address=self.w3.to_checksum_address(topics[1][-40:]),
            to_address=self.w3.to_checksum_address(topics[2][-40:]),
            amount=self._process_integer(data),
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_erc721_transfer(self, log: Dict[str, Any]) -> ERC721Transfer:
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return ERC721Transfer(
            token_address=self.w3.to_checksum_address(log['address']),
            from_address=self.w3.to_checksum_address(topics[1][-40:]) if len(topics) > 1 else None,
            to_address=self.w3.to_checksum_address(topics[2][-40:]) if len(topics) > 2 else None,
            token_id=self._process_integer(topics[3]) if len(topics) > 3 and topics[3] != '' else 0,
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_erc1155_single_transfer(self, log: Dict[str, Any]) -> ERC1155Transfer:
        data = self._ensure_hex_string(log['data'])
        return ERC1155Transfer(
            token_address=self.w3.to_checksum_address(log['address']),
            operator=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            from_address=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            to_address=self.w3.to_checksum_address(log['topics'][3].hex()[-40:]),
            token_ids=[self._process_integer(data[2:66])],
            amounts=[self._process_integer(data[66:130])],
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_erc1155_batch_transfer(self, log: Dict[str, Any]) -> ERC1155Transfer:
        data = log['data'].hex()
        ids_offset = int(data[:66], 16) * 2
        amounts_offset = int(data[66:130], 16) * 2
        ids_length = int(data[ids_offset:ids_offset+64], 16) if ids_offset+64 < len(data) else 0
        amounts_length = int(data[amounts_offset:amounts_offset+64], 16) if amounts_offset+64 < len(data) else 0
        
        ids = [int(data[i:i+64], 16) for i in range(ids_offset+64, ids_offset+64+(ids_length*64), 64)] if ids_length > 0 else []
        amounts = [int(data[i:i+64], 16) for i in range(amounts_offset+64, amounts_offset+64+(amounts_length*64), 64)] if amounts_length > 0 else []
        
        return ERC1155Transfer(
            token_address=self.w3.to_checksum_address(log['address']),
            operator=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            from_address=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            to_address=self.w3.to_checksum_address(log['topics'][3].hex()[-40:]),
            token_ids=ids,
            amounts=amounts,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v2_sync(self, log: Dict[str, Any]) -> UniswapV2Sync:
        data = self._ensure_hex_string(log['data'])
        return UniswapV2Sync(
            pair_address=self.w3.to_checksum_address(log['address']),
            reserve0=self._process_integer(data[2:66]),
            reserve1=self._process_integer(data[66:130]),
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v2_swap(self, log: Dict[str, Any]) -> UniswapV2Swap:
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return UniswapV2Swap(
            pair_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(topics[1][-40:]),
            to=self.w3.to_checksum_address(topics[2][-40:]),
            amount0In=self._process_integer(data[2:66]),
            amount1In=self._process_integer(data[66:130]),
            amount0Out=self._process_integer(data[130:194]),
            amount1Out=self._process_integer(data[194:]),
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_approve(self, log: Dict[str, Any]) -> ERC20Approval:
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return ERC20Approval(
            token_address=self.w3.to_checksum_address(log['address']),
            owner=self.w3.to_checksum_address(topics[1][-40:]) if len(topics) > 1 else None,
            spender=self.w3.to_checksum_address(topics[2][-40:]) if len(topics) > 2 else None,
            amount=self._process_integer(data),
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_erc721_approval(self, log: Dict[str, Any]) -> ERC721Approval:
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return ERC721Approval(
            token_address=self.w3.to_checksum_address(log['address']),
            owner=self.w3.to_checksum_address(topics[1][-40:]) if len(topics) > 1 else None,
            approved=self.w3.to_checksum_address(topics[2][-40:]) if len(topics) > 2 else None,
            token_id=self._process_integer(topics[3]) if len(topics) > 3 and topics[3] != '' else 0,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_mint(self, log: Dict[str, Any]) -> MintAction:
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return MintAction(
            pair_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(topics[1][-40:]),
            amount0=self._process_integer(data[2:66]),
            amount1=self._process_integer(data[66:130]),
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_burn(self, log: Dict[str, Any]) -> BurnAction:
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return BurnAction(
            pair_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(topics[1][-40:]),
            amount=self._process_integer(data),
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_deposit(self, log: Dict[str, Any]) -> DepositAction:
        """Parse deposit event log
        
        Handles two types of deposits:
        1. Complex deposit: Deposit(uint256 id, address indexed tokenAddress, address indexed withdrawalAddress, uint256 amount, uint256 unlockTime)
        2. WETH deposit: Deposit(address indexed dst, uint256 wad)
        
        Topics[0]: Event signature hash
        For complex deposits:
            Topics[1]: tokenAddress (indexed)
            Topics[2]: withdrawalAddress (indexed)
            Data: id, amount, unlockTime
        For WETH deposits:
            Topics[1]: dst address (indexed)
            Data: wad (amount)
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        
        # Check if this is a complex deposit (with id, token, withdrawal)
        if len(topics) > 2:  # Complex deposit has more topics
            id = self._process_integer(data[2:66])
            amount = self._process_integer(data[66:130])
            unlock_time = self._process_integer(data[130:])
            
            return DepositAction(
                id=id,
                token_address=self.w3.to_checksum_address(topics[1][-40:]),
                withdrawal_address=self.w3.to_checksum_address(topics[2][-40:]),
                amount=amount,
                unlock_time=unlock_time,
                log_index=self._process_integer(log['logIndex'])
            )
        else:  # Simple deposit (like WETH)
            # For WETH deposit, data contains only the amount (wad)
            sender = self.w3.to_checksum_address(topics[1][-40:]) if len(topics) > 1 else None
            return DepositAction(
                pair_address=self.w3.to_checksum_address(log['address']),
                sender=sender,
                amount= self._process_integer(data),
                log_index=self._process_integer(log['logIndex'])
            )
    
    def parse_withdraw(self, log: Dict[str, Any]) -> WithdrawAction:
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        if len(topics) > 1:
            return WithdrawAction(
                pair_address=self.w3.to_checksum_address(log['address']),
                sender=self.w3.to_checksum_address(topics[1][-40:]),
                amount=self._process_integer(data),
                log_index=self._process_integer(log['logIndex'])
            )
        else:
            return WithdrawAction(
                pair_address=self.w3.to_checksum_address(log['address']),
                sender=None,
                amount=self._process_integer(data),
                log_index=self._process_integer(log['logIndex'])
            )
    
    def parse_pair(self, log: Dict[str, Any]) -> PairAction:
        """Parse Uniswap pair creation event
        
        PairCreated(address token0, address token1, address pair, uint)
        Topic[0]: Event signature
        Topic[1]: token0 address (indexed)
        Topic[2]: token1 address (indexed)
        Data: [pair address (32 bytes), uint256]
        """
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return PairAction(
            pair_address=self.w3.to_checksum_address(data[26:66]),
            token0=self.w3.to_checksum_address(topics[1][-40:]),
            token1=self.w3.to_checksum_address(topics[2][-40:]),
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_owner(self, log: Dict[str, Any]) -> OwnerEvent:
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return OwnerEvent(
            contract_address=self.w3.to_checksum_address(log['address']),
            previous_owner=self.w3.to_checksum_address(topics[1][-40:]),
            new_owner=self.w3.to_checksum_address(topics[2][-40:]),
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_trading_enabled(self, log: Dict[str, Any]) -> TradingEnabledEvent:
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return TradingEnabledEvent(
            token_address=self.w3.to_checksum_address(log['address']),
            block_number=self._process_integer(topics[1]),
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_trading_disabled(self, log: Dict[str, Any]) -> TradingDisabledEvent:  
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return TradingDisabledEvent(
            token_address=self.w3.to_checksum_address(log['address']),
            block_number=self._process_integer(topics[1]),
            log_index=self._process_integer(log['logIndex'])
        )
    
    # --------------------------------------------------------------------------
    # Uniswap V3 Pool Events 
    # --------------------------------------------------------------------------

    def parse_uniswap_v3_pool_created(self, log: Dict[str, Any]) -> UniswapV3PoolCreated:
        """Parse Uniswap V3 pool creation event
        Event: PoolCreated(token0, token1, fee, tickSpacing, pool)
        
        Topics[0]: Event signature
        Topics[1]: token0 address (indexed)
        Topics[2]: token1 address (indexed)
        Topics[3]: fee (indexed)
        Data: tickSpacing (int24), pool (address)
        """
        data = self._ensure_hex_string(log['data'])
        # Handle both hex string and bytes formats for topics
        token0 = self.w3.to_checksum_address(log['topics'][1].hex()[-40:]) if isinstance(log['topics'][1], bytes) else self.w3.to_checksum_address(log['topics'][1][-40:])
        token1 = self.w3.to_checksum_address(log['topics'][2].hex()[-40:]) if isinstance(log['topics'][2], bytes) else self.w3.to_checksum_address(log['topics'][2][-40:])
        fee = int(log['topics'][3].hex(), 16) if isinstance(log['topics'][3], bytes) else int(log['topics'][3], 16)    
        
        tick_spacing = int(data[2:66], 16)  # First 32 bytes
        pool = self.w3.to_checksum_address('0x' + data[90:130])  # Last 40 bytes of second 32-byte chunk
        
        return UniswapV3PoolCreated(
            token0=token0,
            token1=token1,
            fee=fee,
            tick_spacing=tick_spacing,
            pool=pool,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v3_initialize(self, log: Dict[str, Any]) -> UniswapV3Initialize:
        """Parse Uniswap V3 pool initialization
        Event: Initialize(uint160 sqrtPriceX96, int24 tick)
        
        Topics[0]: Event signature
        Data: sqrtPriceX96 (uint160), tick (int24)
        """
        data = self._ensure_hex_string(log['data'])
        
        # Parse sqrtPriceX96 as uint160 (first 32 bytes)
        sqrt_price_x96 = int(data[2:66], 16)
        
        # Parse tick as uint24 (next 32 bytes)
        tick = int(data[66:130], 16)
        
        return UniswapV3Initialize(
            pool_address=self.w3.to_checksum_address(log['address']),
            sqrt_price_x96=sqrt_price_x96,
            tick=tick,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v3_mint(self, log: Dict[str, Any]) -> UniswapV3Mint:
        """Parse Uniswap V3 mint event
        Event: Mint(address sender, address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)
        
        Topics[0]: Event signature
        Topics[1]: owner address (indexed)
        Topics[2]: tickLower (indexed, int24)
        Topics[3]: tickUpper (indexed, int24)
        Data: 
            - sender (address, 32 bytes with padding)
            - amount (uint128, 32 bytes)
            - amount0 (uint256, 32 bytes)
            - amount1 (uint256, 32 bytes)
        """
        data = self._ensure_hex_string(log['data'])
        
        # Parse sender from first 32 bytes (padded address)
        sender = '0x' + data[26:66]  # Take last 40 chars of first 32 bytes
        
        # Parse ticks as signed integers
        tick_lower = int.from_bytes(bytes.fromhex(log['topics'][2].hex()[2:]), byteorder='big', signed=True)
        tick_upper = int.from_bytes(bytes.fromhex(log['topics'][3].hex()[2:]), byteorder='big', signed=True)
        
        return UniswapV3Mint(
            pool_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(sender),
            owner=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]) if isinstance(log['topics'][1], bytes) else self.w3.to_checksum_address(log['topics'][1][-40:]),
            tick_lower=tick_lower,
            tick_upper=tick_upper,
            amount=int(data[66:130], 16),    # Second 32 bytes
            amount0=int(data[130:194], 16),  # Third 32 bytes
            amount1=int(data[194:258], 16),  # Fourth 32 bytes
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v3_swap(self, log: Dict[str, Any]) -> UniswapV3Swap:
        """Parse Uniswap V3 swap event
        Event: Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
        
        Topics[0]: Event signature
        Topics[1]: sender address (indexed)
        Topics[2]: recipient address (indexed)
        Data: amount0 (int256), amount1 (int256), sqrtPriceX96 (uint160), liquidity (uint128), tick (int24)
        """
        data = self._ensure_hex_string(log['data'])
        
        return UniswapV3Swap(
            pool_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]) if isinstance(log['topics'][1], bytes) else self.w3.to_checksum_address(log['topics'][1][-40:]),
            recipient=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]) if isinstance(log['topics'][2], bytes) else self.w3.to_checksum_address(log['topics'][2][-40:]),
            amount0=int(data[2:66], 16) if data[2:66] else 0,  # First 32 bytes
            amount1=int(data[66:130], 16) if data[66:130] else 0,  # Next 32 bytes
            sqrt_price_x96=int(data[130:194], 16) if data[130:194] else 0,  # Next 32 bytes
            liquidity=int(data[194:258], 16) if data[194:258] else 0,  # Next 32 bytes
            tick=int(data[258:322], 16) if data[258:322] else 0,  # Last 32 bytes
            log_index=self._process_integer(log['logIndex'])
        )
        
    def parse_uniswap_v3_position(self, log: Dict[str, Any]) -> UniswapV3Position:
        """Parse Uniswap V3 position event (IncreaseLiquidity)
        Event: IncreaseLiquidity(uint256 indexed tokenId, uint128 liquidity, uint256 amount0, uint256 amount1)
        
        Topics[0]: Event signature
        Topics[1]: tokenId (indexed)
        Data: liquidity (uint128), amount0 (uint256), amount1 (uint256)
        """
        data = self._ensure_hex_string(log['data'])
            
        return UniswapV3Position(
            token_id=int(log['topics'][1].hex(), 16) if isinstance(log['topics'][1], bytes) else int(log['topics'][1], 16),
            liquidity=int(data[2:66], 16),  # First 32 bytes
            amount0=int(data[66:130], 16),  # Next 32 bytes
            amount1=int(data[130:194], 16),  # Last 32 bytes
            pool_address=self.w3.to_checksum_address(log['address']),
            owner=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]) if len(log['topics']) > 2 else None,
            tick_lower=int(log['topics'][3].hex(), 16) if len(log['topics']) > 3 else 0,
            tick_upper=int(log['topics'][4].hex(), 16) if len(log['topics']) > 4 else 0,
            log_index=self._process_integer(log['logIndex'])
        )
    
    def parse_uniswap_v3_increase_liquidity(self, log: Dict[str, Any]) -> UniswapV3IncreaseLiquidity:
        """Parse Uniswap V3 increase liquidity event
        Event: IncreaseLiquidity(uint256 indexed tokenId, uint128 liquidity, uint256 amount0, uint256 amount1)
        
        Topics[0]: Event signature
        Topics[1]: tokenId (indexed)
        Data: liquidity (uint128), amount0 (uint256), amount1 (uint256)
        """
        data = self._ensure_hex_string(log['data'])
            
        return UniswapV3IncreaseLiquidity(
            token_id=int(log['topics'][1].hex(), 16) if isinstance(log['topics'][1], bytes) else int(log['topics'][1], 16),
            liquidity=int(data[2:66], 16) if data[2:66] else 0,
            amount0=int(data[66:130], 16) if data[66:130] else 0,
            amount1=int(data[130:194], 16) if data[130:194] else 0,
            pool_address=self.w3.to_checksum_address(log['address']),
            log_index=self._process_integer(log['logIndex'])
        )


    def parse_uniswap_v3_decrease_liquidity(self, log: Dict[str, Any]) -> UniswapV3DecreaseLiquidity:
        """Parse Uniswap V3 decrease liquidity event
        Event: DecreaseLiquidity(uint256 indexed tokenId, uint128 liquidity, uint256 amount0, uint256 amount1)
        
        Topics[0]: Event signature
        Topics[1]: tokenId (indexed)
        Data: liquidity (uint128), amount0 (uint256), amount1 (uint256)
        """
        data = self._ensure_hex_string(log['data'])
              
        return UniswapV3DecreaseLiquidity(
            token_id=int(log['topics'][1].hex(), 16) if isinstance(log['topics'][1], bytes) else int(log['topics'][1], 16),
            liquidity=int(data[2:66], 16) if data[2:66] else 0,
            amount0=int(data[66:130], 16) if data[66:130] else 0,
            amount1=int(data[130:194], 16) if data[130:194] else 0,
            pool_address=self.w3.to_checksum_address(log['address']),
            log_index=self._process_integer(log['logIndex'])
        )

    def parse_uniswap_v3_burn(self, log: Dict[str, Any]) -> UniswapV3Burn:
        """Parse Uniswap V3 burn event
        Event: Burn(address indexed sender, address indexed owner, uint256 amount0, uint256 amount1)

        Topics[0]: Event signature
        Topics[1]: sender address (indexed)
        Topics[2]: owner address (indexed)
        Data: amount0 (uint256), amount1 (uint256)
        """
        pass
    
    def parse_other_event(self, log: Dict[str, Any]) -> Dict[str, Any]:
        data = self._ensure_hex_string(log['data'])
        topics = [self._ensure_hex_string(topic) for topic in log['topics']]
        return {
            'address': self.w3.to_checksum_address(log['address']),
            'topics': topics,
            'data': data,
            'log_index': self._process_integer(log['logIndex']),
        }
    