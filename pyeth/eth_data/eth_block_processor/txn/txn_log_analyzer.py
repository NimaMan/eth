from typing import Dict, Any, List
from web3 import Web3
from eth_block_processor.constants.function_signatures import EVENT_TOPICS
from eth_block_processor.data_models.receipt_models import *


class TransactionLogAnalyzer:
    def __init__(self, w3: Web3):
        self.w3 = w3
                # ERC Token Events
        self.erc20_transfer_topic = EVENT_TOPICS['Transfer']
        self.erc721_transfer_topic = EVENT_TOPICS['Transfer']
        self.erc1155_transfer_single_topic = EVENT_TOPICS['TransferSingle']
        self.erc1155_transfer_batch_topic = EVENT_TOPICS['TransferBatch']
        
        # Basic Token Operations
        self.approve_signature = EVENT_TOPICS['Approval']
        self.mint_signature = EVENT_TOPICS['Mint']
        self.burn_signature = EVENT_TOPICS['Burn']
        self.deposit_signature = EVENT_TOPICS['Deposit']
        self.withdraw_signature = EVENT_TOPICS['Withdraw']
        self.weth_withdrawal_topic = EVENT_TOPICS['WETHWithdrawal']
        
        # Uniswap V2 Events
        self.uniswap_v2_sync_topic = EVENT_TOPICS['Sync']
        self.uniswap_v2_swap_topic = EVENT_TOPICS['Swap']
        self.pair_signature = EVENT_TOPICS['PairCreated']
        self.uniswap_v2_collect_topic = EVENT_TOPICS['Collect']
        self.uniswap_v2_flash_topic = EVENT_TOPICS['Flash']
        self.uniswap_v2_increase_observation_cardinality_next_topic = EVENT_TOPICS['IncreaseObservationCardinalityNext']
        self.uniswap_v2_set_fee_protocol_topic = EVENT_TOPICS['SetFeeProtocol']
        self.uniswap_v2_collect_protocol_topic = EVENT_TOPICS['CollectProtocol']
        
        # Token Management Events
        self.owner_signature = EVENT_TOPICS['OwnershipTransferred']
        self.trading_enabled_signature = EVENT_TOPICS['TradingEnabled']
        self.trading_disabled_signature = EVENT_TOPICS['TradingDisabled']
        self.exclude_from_fees_signature = EVENT_TOPICS['ExcludeFromFees']
        self.exclude_from_limits_signature = EVENT_TOPICS['ExcludeFromLimits']
        self.set_max_tx_amount_signature = EVENT_TOPICS['SetMaxTxAmount']
        self.set_max_wallet_token_signature = EVENT_TOPICS['SetMaxWalletToken']
        self.set_max_wallet_signature = EVENT_TOPICS['SetMaxWallet']
        
        # Uniswap V3 Pool Events
        self.uniswap_v3_initialize_topic = EVENT_TOPICS['Initialize']
        self.uniswap_v3_mint_topic = EVENT_TOPICS['UniswapV3Mint']
        self.uniswap_v3_burn_topic = EVENT_TOPICS['UniswapV3Burn']
        self.uniswap_v3_swap_topic = EVENT_TOPICS['UniswapV3Swap']
        
        # Uniswap V3 Factory Events
        self.uniswap_v3_pool_created_topic = EVENT_TOPICS['PoolCreated']
        
        # Uniswap V3 NFT Manager Events
        self.uniswap_v3_increase_liquidity_topic = EVENT_TOPICS['IncreaseLiquidity']
        self.uniswap_v3_decrease_liquidity_topic = EVENT_TOPICS['DecreaseLiquidity']

    def analyze_logs(self, logs: List[Dict[str, Any]]) -> Dict[str, List[Any]]:
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
            'pair_events': [],
            'approvals': [],
            'owner_events': [],
            'trading_enabled_events': [],
            'trading_disabled_events': [],
            'other_events': [],
            'unique_addresses': set(),
            'erc20_contracts': set(),
        }
        
        for log in logs:
            try:
                event = self.classify_and_parse_log(log)
            except Exception as e:
                print(f"Error parsing log: {log}")
                raise e
            if isinstance(event, ERC20Transfer):
                result['erc20_transfers'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.from_address))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.to_address))
                result['erc20_contracts'].add(self.w3.to_checksum_address(event.token_address))
            elif isinstance(event, ERC721Transfer):
                result['erc721_transfers'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.from_address))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.to_address))
            elif isinstance(event, ERC1155Transfer):
                result['erc1155_transfers'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.from_address))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.to_address))
            elif isinstance(event, UniswapV2Sync):
                result['uniswap_v2_syncs'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.pair_address))
            elif isinstance(event, UniswapV2Swap):
                result['uniswap_v2_swaps'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.sender))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.to))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.pair_address))
            elif isinstance(event, MintAction):
                result['mints'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.pair_address))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.sender))
            elif isinstance(event, BurnAction):
                result['burns'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.pair_address))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.sender))
            elif isinstance(event, DepositAction):
                result['deposits'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.pair_address))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.sender))
            elif isinstance(event, WithdrawAction):
                result['withdraws'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.pair_address))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.sender))
            elif isinstance(event, PairAction):
                result['pair_events'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.pair_address))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.token0))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.token1))
            elif isinstance(event, ERC20Approval):
                result['approvals'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.token_address))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.owner))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.spender))
                result['erc20_contracts'].add(self.w3.to_checksum_address(event.token_address))
            elif isinstance(event, ERC721Approval):
                result['approvals'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.token_address))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.owner))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.approved_address))
            elif isinstance(event, OwnerEvent):
                result['owner_events'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.contract_address))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.previous_owner))
                result['unique_addresses'].add(self.w3.to_checksum_address(event.new_owner))
            elif isinstance(event, TradingEnabledEvent):
                result['trading_enabled_events'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.token_address))
                result['erc20_contracts'].add(self.w3.to_checksum_address(event.token_address))
            elif isinstance(event, TradingDisabledEvent):
                result['trading_disabled_events'].append(event)
                result['unique_addresses'].add(self.w3.to_checksum_address(event.token_address))
                result['erc20_contracts'].add(self.w3.to_checksum_address(event.token_address))
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
            if topic == self.erc20_transfer_topic:
                return self.parse_erc20_transfer(log)
            elif topic == self.erc721_transfer_topic:
                return self.parse_erc721_transfer(log)
            elif topic == self.erc1155_transfer_single_topic:
                return self.parse_erc1155_single_transfer(log)
            elif topic == self.erc1155_transfer_batch_topic:
                return self.parse_erc1155_batch_transfer(log)
            elif topic == self.uniswap_v2_sync_topic:
                return self.parse_uniswap_v2_sync(log)
            elif topic == self.uniswap_v2_swap_topic:
                return self.parse_uniswap_v2_swap(log)
            elif topic == self.mint_signature:
                return self.parse_mint(log)
            elif topic == self.approve_signature:
                return self.parse_approve(log)
            elif topic == self.burn_signature:
                return self.parse_burn(log)
            elif topic == self.deposit_signature:
                return self.parse_deposit(log)
            elif topic == self.withdraw_signature or topic == self.weth_withdrawal_topic:
                return self.parse_withdraw(log)
            elif topic == self.pair_signature:
                return self.parse_pair(log)
            elif topic == self.owner_signature:
                return self.parse_owner(log)
            elif topic == self.trading_enabled_signature:
                return self.parse_trading_enabled(log)
            elif topic == self.trading_disabled_signature:
                return self.parse_trading_disabled(log)
            else:
                return self.parse_other_event(log)
        except Exception as e:
            return self.parse_other_event(log)

    def parse_erc20_transfer(self, log: Dict[str, Any]) -> ERC20Transfer:
        """Parse ERC20 Transfer event log Event signature: Transfer(address indexed from, address indexed to, uint256 value)
        Topic[0]: Event signature hash
        Topic[1]: from address (indexed)
        Topic[2]: to address (indexed)
        Data: value (uint256)
        """
        # Handle both hex string and bytes data formats
        data = log['data']
        if isinstance(data, bytes):
            data = data.hex()
        if not data.startswith('0x'):
            data = '0x' + data
        
        return ERC20Transfer(
            token_address=self.w3.to_checksum_address(log['address']),
            from_address=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]) if isinstance(log['topics'][1], bytes) else self.w3.to_checksum_address(log['topics'][1][-40:]),
            to_address=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]) if isinstance(log['topics'][2], bytes) else self.w3.to_checksum_address(log['topics'][2][-40:]),
            amount=int(data, 16) if data != '0x' else 0,
            log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
        )

    def parse_erc721_transfer(self, log: Dict[str, Any]) -> ERC721Transfer:
        return ERC721Transfer(
            token_address=self.w3.to_checksum_address(log['address']),
            from_address=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]) if len(log['topics']) > 1 else None,
            to_address=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]) if len(log['topics']) > 2 else None,
            token_id=int(log['topics'][3].hex(), 16) if len(log['topics']) > 3 and log['topics'][3].hex() != '' else 0,
            log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
        )

    def parse_erc1155_single_transfer(self, log: Dict[str, Any]) -> ERC1155Transfer:
        data = log['data'].hex()
        return ERC1155Transfer(
            token_address=self.w3.to_checksum_address(log['address']),
            operator=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            from_address=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            to_address=self.w3.to_checksum_address(log['topics'][3].hex()[-40:]),
            token_ids=[int(data[:64], 16) if data[:64] else 0],
            amounts=[int(data[64:128], 16) if len(data) >= 128 and data[64:128] else 0],
            log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
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
            log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
        )
    
    def parse_uniswap_v2_sync(self, log: Dict[str, Any]) -> UniswapV2Sync:
        return UniswapV2Sync(
            pair_address=self.w3.to_checksum_address(log['address']),
            reserve0=int(log['data'].hex()[:64], 16) if log['data'].hex()[:64] != '' else 0,
            reserve1=int(log['data'].hex()[64:], 16) if log['data'].hex()[64:] != '' else 0,
            log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
        )
    
    def parse_uniswap_v2_swap(self, log: Dict[str, Any]) -> UniswapV2Swap:
        return UniswapV2Swap(
            pair_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            to=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            amount0In=int(log['data'].hex()[:64], 16) if log['data'].hex()[:64] != '' else 0,
            amount1In=int(log['data'].hex()[64:128], 16) if log['data'].hex()[64:128] != '' else 0,
            amount0Out=int(log['data'].hex()[128:192], 16) if log['data'].hex()[128:192] != '' else 0,
            amount1Out=int(log['data'].hex()[192:], 16) if log['data'].hex()[192:] != '' else 0,
            log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
        )

    def parse_approve(self, log: Dict[str, Any]) -> ERC20Approval:
        return ERC20Approval(
            token_address=self.w3.to_checksum_address(log['address']),
            owner=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]) if len(log['topics']) > 1 else None,
            spender=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]) if len(log['topics']) > 2 else None,
            amount=int(log['data'].hex(), 16) if log['data'].hex() != '' else 0,
            log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
        )
    
    def parse_erc721_approval(self, log: Dict[str, Any]) -> ERC721Approval:
        return ERC721Approval(
            token_address=self.w3.to_checksum_address(log['address']),
            owner=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            approved=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            token_id=int(log['topics'][3].hex(), 16) if log['topics'][3].hex() != '' else 0,
            log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
        )
    
    def parse_mint(self, log: Dict[str, Any]) -> MintAction:
        return MintAction(
            pair_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            amount0=int(log['data'].hex()[:64], 16) if log['data'].hex()[:64] != '' else 0,
            amount1=int(log['data'].hex()[64:], 16) if log['data'].hex()[64:] != '' else 0,
            log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
        )
    
    def parse_burn(self, log: Dict[str, Any]) -> BurnAction:
        return BurnAction(
            pair_address=self.w3.to_checksum_address(log['address']),
            sender=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            amount=int(log['data'].hex(), 16) if log['data'].hex() != '' else 0,
            log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
        )
    
    def parse_deposit(self, log: Dict[str, Any]) -> DepositAction:
        """Parse deposit event log
        Event signature: Deposit(uint256 id, address indexed tokenAddress, address indexed withdrawalAddress, uint256 amount, uint256 unlockTime)
        Topic[0]: Event signature hash
        Topic[1]: tokenAddress (indexed)
        Topic[2]: withdrawalAddress (indexed)
        Data: id (uint256), amount (uint256), unlockTime (uint256)
        """
        if len(log['topics']) > 1:
            data = log['data']
            # Extract values from data field
            id = int(data[:66], 16)  # First 32 bytes
            amount = int(data[66:130], 16)  # Second 32 bytes
            unlock_time = int(data[130:], 16)  # Third 32 bytes
            
            return DepositAction(
                id=id,
                token_address=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
                withdrawal_address=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
                amount=amount,
                unlock_time=unlock_time,
                log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
            )
        else:
            # Fallback for other deposit types
            data = log['data']
            sender = None
            amount = int(data[66:], 16) if data[66:] else None
            return DepositAction(
                pair_address=self.w3.to_checksum_address(log['address']),
                sender=sender,
                amount=amount,
                log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
            )
    
    def parse_withdraw(self, log: Dict[str, Any]) -> WithdrawAction:
        if len(log['topics']) > 1:
            return WithdrawAction(
                pair_address=self.w3.to_checksum_address(log['address']),
                sender=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
                amount=int(log['data'].hex(), 16) if log['data'].hex() != '' else 0,
                log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
            )
        else:
            return WithdrawAction(
                pair_address=self.w3.to_checksum_address(log['address']),
                sender=None,
                amount=int(log['data'].hex(), 16) if log['data'].hex() != '' else 0,
                log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
            )
    
    def parse_pair(self, log: Dict[str, Any]) -> PairAction:
        """Parse Uniswap pair creation event
        
        PairCreated(address token0, address token1, address pair, uint)
        Topic[0]: Event signature
        Topic[1]: token0 address (indexed)
        Topic[2]: token1 address (indexed)
        Data: [pair address (32 bytes), uint256]
        """
        # Extract addresses from topics
        token0 = self.w3.to_checksum_address("0x" + log['topics'][1].hex()[-40:])
        token1 = self.w3.to_checksum_address("0x" + log['topics'][2].hex()[-40:])
        pair_address = self.w3.to_checksum_address("0x" + log['data'].hex()[24:64])
        log_index = log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
        return PairAction(
            pair_address=pair_address,
            token0=token0,
            token1=token1,
            log_index=log_index
        )
    
    def parse_owner(self, log: Dict[str, Any]) -> OwnerEvent:
        return OwnerEvent(
            contract_address=self.w3.to_checksum_address(log['address']),
            previous_owner=self.w3.to_checksum_address(log['topics'][1].hex()[-40:]),
            new_owner=self.w3.to_checksum_address(log['topics'][2].hex()[-40:]),
            log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
        )
    
    def parse_trading_enabled(self, log: Dict[str, Any]) -> TradingEnabledEvent:
        return TradingEnabledEvent(
            token_address=self.w3.to_checksum_address(log['address']),
            block_number=int(log['topics'][1].hex(), 16) if log['topics'][1].hex() != '' else 0,
            log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
        )
    
    def parse_trading_disabled(self, log: Dict[str, Any]) -> TradingDisabledEvent:  
        return TradingDisabledEvent(
            token_address=self.w3.to_checksum_address(log['address']),
            block_number=int(log['topics'][1].hex(), 16) if log['topics'][1].hex() != '' else 0,
            log_index=log['logIndex'] if isinstance(log['logIndex'], int) else int(log['logIndex'], 16)
        )
    
    def parse_other_event(self, log: Dict[str, Any]) -> Dict[str, Any]:
        return {
            'address': self.w3.to_checksum_address(log['address']),
            'topics': [topic.hex() if isinstance(topic, bytes) else topic for topic in log.get('topics', [])],
            'data': log['data'].hex() if isinstance(log['data'], bytes) else log['data'],
            'log_index': log.get('logIndex', None) if isinstance(log['logIndex'], int) else int(log['logIndex'], 16),
        }
