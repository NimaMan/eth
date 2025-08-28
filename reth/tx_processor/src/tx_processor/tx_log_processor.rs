use alloy_primitives::{Address, B256, U256, Log as AlloyLog};
use super::data_models::events::*;
use eyre::Result;
use tx_simulator::CallFrame;

/// Event signatures for common token standards and DEX protocols
pub struct EventSignatures {
    // ERC20
    pub transfer: B256,
    pub approval: B256,
    
    // ERC721
    pub transfer_erc721: B256,
    pub approval_erc721: B256,
    pub approval_for_all: B256,
    
    // ERC1155
    pub transfer_single: B256,
    pub transfer_batch: B256,
    
    // Uniswap V2
    pub sync: B256,
    pub swap: B256,
    pub mint: B256,
    pub burn: B256,
    pub pair_created: B256,
    
    // Uniswap V3
    pub pool_created: B256,
    pub initialize: B256,
    pub mint_v3: B256,
    pub burn_v3: B256,
    pub swap_v3: B256,
    pub increase_liquidity: B256,
    pub decrease_liquidity: B256,
    pub collect: B256,
    
    // Uniswap V4
    pub initialize_v4: B256,
    pub modify_liquidity: B256,
    pub swap_v4: B256,
    pub donate: B256,
    pub protocol_fee_updated: B256,
    pub dynamic_lp_fee_updated: B256,
    pub protocol_fee_controller_updated: B256,
    pub balance_delta: B256,
    pub permit2: B256,
    
    // General actions
    pub deposit: B256,
    pub withdraw: B256,
    
    // Contract events
    pub ownership_transferred: B256,
    pub trading_enabled: B256,
    pub trading_disabled: B256,
}

impl EventSignatures {
    pub fn new() -> Self {
        use alloy_primitives::keccak256;
        
        Self {
            // ERC20
            transfer: keccak256(b"Transfer(address,address,uint256)"),
            approval: keccak256(b"Approval(address,address,uint256)"),
            
            // ERC721
            transfer_erc721: keccak256(b"Transfer(address,address,uint256)"),
            approval_erc721: keccak256(b"Approval(address,address,uint256)"),
            approval_for_all: keccak256(b"ApprovalForAll(address,address,bool)"),
            
            // ERC1155
            transfer_single: keccak256(b"TransferSingle(address,address,address,uint256,uint256)"),
            transfer_batch: keccak256(b"TransferBatch(address,address,address,uint256[],uint256[])"),
            
            // Uniswap V2
            sync: keccak256(b"Sync(uint112,uint112)"),
            swap: keccak256(b"Swap(address,uint256,uint256,uint256,uint256,address)"),
            mint: keccak256(b"Mint(address,uint256,uint256)"),
            burn: keccak256(b"Burn(address,uint256,uint256,address)"),
            pair_created: keccak256(b"PairCreated(address,address,address,uint256)"),
            
            // Uniswap V3
            pool_created: keccak256(b"PoolCreated(address,address,uint24,int24,address)"),
            initialize: keccak256(b"Initialize(uint160,int24)"),
            mint_v3: keccak256(b"Mint(address,address,int24,int24,uint128,uint256,uint256)"),
            burn_v3: keccak256(b"Burn(address,int24,int24,uint128,uint256,uint256)"),
            swap_v3: keccak256(b"Swap(address,address,int256,int256,uint160,uint128,int24)"),
            increase_liquidity: keccak256(b"IncreaseLiquidity(uint256,uint128,uint256,uint256)"),
            decrease_liquidity: keccak256(b"DecreaseLiquidity(uint256,uint128,uint256,uint256)"),
            collect: keccak256(b"Collect(uint256,address,uint256,uint256)"),
            
            // Uniswap V4
            initialize_v4: keccak256(b"Initialize(bytes32,address,address,uint24,int24,address,uint160,int24)"),
            modify_liquidity: keccak256(b"ModifyLiquidity(bytes32,address,int24,int24,int256,bytes32)"),
            swap_v4: keccak256(b"Swap(bytes32,address,int128,int128,uint160,uint128,int24,uint24)"),
            donate: keccak256(b"Donate(bytes32,address,int256,int256)"),
            protocol_fee_updated: keccak256(b"ProtocolFeeUpdated(bytes32,uint24)"),
            dynamic_lp_fee_updated: keccak256(b"DynamicLPFeeUpdated(bytes32,uint24)"),
            protocol_fee_controller_updated: keccak256(b"ProtocolFeeControllerUpdated(address)"),
            balance_delta: B256::from([0x40, 0xe9, 0xce, 0xcb, 0x9f, 0x5f, 0x1f, 0x1c, 0x5b, 0x9c, 0x97, 0xde, 0xc2, 0x91, 0x7b, 0x7e, 0xe9, 0x2e, 0x57, 0xba, 0x55, 0x63, 0x70, 0x8d, 0xac, 0xa9, 0x4d, 0xd8, 0x4a, 0xd7, 0x11, 0x2f]),
            permit2: keccak256(b"Permit(address,address,address,uint160,uint48,uint48)"),
            
            // General actions
            deposit: keccak256(b"Deposit(address,uint256)"),
            withdraw: keccak256(b"Withdraw(address,uint256)"),
            
            // Contract events
            ownership_transferred: keccak256(b"OwnershipTransferred(address,address)"),
            trading_enabled: keccak256(b"TradingEnabled()"),
            trading_disabled: keccak256(b"TradingDisabled()"),
        }
    }
}

pub struct LogDecoder {
    signatures: EventSignatures,
}

impl LogDecoder {
    pub fn new() -> Self {
        Self {
            signatures: EventSignatures::new(),
        }
    }
    
    /// Decode a log into known event types
    pub fn decode_log(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().is_empty() {
            return Ok(None);
        }
        
        let event_signature = log.topics()[0];
        
        // ERC20/ERC721 Transfer (same signature, differentiated by topic count)
        if event_signature == self.signatures.transfer && log.topics().len() == 3 {
            return self.decode_erc20_transfer(log, log_index);
        }
        if event_signature == self.signatures.transfer_erc721 && log.topics().len() == 4 {
            return self.decode_erc721_transfer(log, log_index);
        }
        
        // ERC20/ERC721 Approval (same signature, differentiated by topic count)
        if event_signature == self.signatures.approval && log.topics().len() == 3 {
            return self.decode_erc20_approval(log, log_index);
        }
        if event_signature == self.signatures.approval_erc721 && log.topics().len() == 4 {
            return self.decode_erc721_approval(log, log_index);
        }
        
        // ERC1155 TransferSingle
        if event_signature == self.signatures.transfer_single && log.topics().len() == 4 {
            return self.decode_erc1155_transfer(log, log_index);
        }
        
        // Uniswap V2 Swap
        if event_signature == self.signatures.swap && log.topics().len() == 3 {
            return self.decode_uniswap_v2_swap(log, log_index);
        }
        
        // Uniswap V2 Sync
        if event_signature == self.signatures.sync && log.topics().len() == 1 {
            return self.decode_uniswap_v2_sync(log, log_index);
        }
        
        // Uniswap V2 Mint
        if event_signature == self.signatures.mint && log.topics().len() == 2 {
            return self.decode_uniswap_v2_mint(log, log_index);
        }
        
        // Uniswap V2 Burn
        if event_signature == self.signatures.burn && log.topics().len() == 3 {
            return self.decode_uniswap_v2_burn(log, log_index);
        }
        
        // Uniswap V2 PairCreated
        if event_signature == self.signatures.pair_created && log.topics().len() == 4 {
            return self.decode_uniswap_v2_pair_created(log, log_index);
        }
        
        // Uniswap V3 Events
        if event_signature == self.signatures.swap_v3 && log.topics().len() == 3 {
            return self.decode_uniswap_v3_swap(log, log_index);
        }
        if event_signature == self.signatures.mint_v3 && log.topics().len() == 4 {
            return self.decode_uniswap_v3_mint(log, log_index);
        }
        if event_signature == self.signatures.burn_v3 && log.topics().len() == 4 {
            return self.decode_uniswap_v3_burn(log, log_index);
        }
        if event_signature == self.signatures.pool_created && log.topics().len() == 4 {
            return self.decode_uniswap_v3_pool_created(log, log_index);
        }
        if event_signature == self.signatures.initialize && log.topics().len() == 1 {
            return self.decode_uniswap_v3_initialize(log, log_index);
        }
        
        // Uniswap V4 Events
        if event_signature == self.signatures.swap_v4 && log.topics().len() == 3 {
            return self.decode_uniswap_v4_swap(log, log_index);
        }
        if event_signature == self.signatures.initialize_v4 && log.topics().len() == 4 {
            return self.decode_uniswap_v4_initialize(log, log_index);
        }
        if event_signature == self.signatures.modify_liquidity && log.topics().len() == 3 {
            return self.decode_uniswap_v4_modify_liquidity(log, log_index);
        }
        if event_signature == self.signatures.donate && log.topics().len() == 3 {
            return self.decode_uniswap_v4_donate(log, log_index);
        }
        if event_signature == self.signatures.protocol_fee_updated && log.topics().len() == 2 {
            return self.decode_uniswap_v4_protocol_fee_updated(log, log_index);
        }
        if event_signature == self.signatures.dynamic_lp_fee_updated && log.topics().len() == 2 {
            return self.decode_uniswap_v4_dynamic_lp_fee_updated(log, log_index);
        }
        if event_signature == self.signatures.protocol_fee_controller_updated && log.topics().len() == 1 {
            return self.decode_uniswap_v4_protocol_fee_controller_updated(log, log_index);
        }
        if event_signature == self.signatures.balance_delta && log.topics().len() == 3 {
            return self.decode_uniswap_v4_balance_delta(log, log_index);
        }
        
        // Uniswap V3 missing events
        if event_signature == self.signatures.increase_liquidity && log.topics().len() == 2 {
            return self.decode_uniswap_v3_increase_liquidity(log, log_index);
        }
        if event_signature == self.signatures.decrease_liquidity && log.topics().len() == 2 {
            return self.decode_uniswap_v3_decrease_liquidity(log, log_index);
        }
        if event_signature == self.signatures.collect && log.topics().len() == 3 {
            return self.decode_uniswap_v3_collect(log, log_index);
        }
        // Check if this could be a position event (with more topics for extended data)
        if event_signature == self.signatures.increase_liquidity && log.topics().len() > 2 {
            return self.decode_uniswap_v3_position(log, log_index);
        }
        
        // General events
        if event_signature == self.signatures.deposit && log.topics().len() == 2 {
            return self.decode_deposit(log, log_index);
        }
        if event_signature == self.signatures.withdraw && log.topics().len() == 2 {
            return self.decode_withdraw(log, log_index);
        }
        if event_signature == self.signatures.ownership_transferred && log.topics().len() == 3 {
            return self.decode_ownership_transferred(log, log_index);
        }
        if event_signature == self.signatures.trading_enabled && log.topics().len() == 1 {
            return self.decode_trading_enabled(log, log_index);
        }
        if event_signature == self.signatures.trading_disabled && log.topics().len() == 1 {
            return self.decode_trading_disabled(log, log_index);
        }
        if event_signature == self.signatures.permit2 && log.topics().len() == 4 {
            return self.decode_permit2(log, log_index);
        }
        
        Ok(None)
    }
    
    fn decode_erc20_transfer(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 32 {
            return Ok(None);
        }
        
        let from_bytes: &[u8] = log.topics()[1].as_ref();
        let to_bytes: &[u8] = log.topics()[2].as_ref();
        
        // Ensure we have enough bytes for address extraction (32 bytes in topic, address uses last 20)
        if from_bytes.len() < 32 || to_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for ERC20 transfer"));
        }
        
        let from_address = Address::from_slice(&from_bytes[12..32]);
        let to_address = Address::from_slice(&to_bytes[12..32]);
        let amount = U256::from_be_slice(&log.data.data);
        
        Ok(Some(DecodedEvent::ERC20Transfer(ERC20Transfer {
            token_address: log.address,
            from_address,
            to_address,
            amount,
            log_index,
        })))
    }
    
    fn decode_erc20_approval(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 32 {
            return Ok(None);
        }
        
        let owner_bytes: &[u8] = log.topics()[1].as_ref();
        let spender_bytes: &[u8] = log.topics()[2].as_ref();
        
        // Ensure we have enough bytes for address extraction
        if owner_bytes.len() < 32 || spender_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for ERC20 approval"));
        }
        
        let owner = Address::from_slice(&owner_bytes[12..32]);
        let spender = Address::from_slice(&spender_bytes[12..32]);
        let amount = U256::from_be_slice(&log.data.data);
        
        Ok(Some(DecodedEvent::ERC20Approval(ERC20Approval {
            token_address: log.address,
            owner,
            spender,
            amount,
            log_index,
        })))
    }
    
    fn decode_uniswap_v2_swap(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 128 {
            return Ok(None);
        }
        
        let sender_bytes: &[u8] = log.topics()[1].as_ref();
        let to_bytes: &[u8] = log.topics()[2].as_ref();
        
        // Ensure we have enough bytes for address extraction
        if sender_bytes.len() < 32 || to_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for Uniswap V2 swap"));
        }
        
        let sender = Address::from_slice(&sender_bytes[12..32]);
        let to = Address::from_slice(&to_bytes[12..32]);
        
        // Use get() for safe slicing with proper error handling
        let amount0_in = U256::from_be_slice(log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0_in"))?);
        let amount1_in = U256::from_be_slice(log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1_in"))?);
        let amount0_out = U256::from_be_slice(log.data.data.get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0_out"))?);
        let amount1_out = U256::from_be_slice(log.data.data.get(96..128)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1_out"))?);
        
        Ok(Some(DecodedEvent::UniswapV2Swap(UniswapV2Swap {
            pair_address: log.address,
            sender,
            to,
            amount0_in,
            amount1_in,
            amount0_out,
            amount1_out,
            log_index,
        })))
    }
    
    fn decode_uniswap_v2_sync(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 1 || log.data.data.len() != 64 {
            return Ok(None);
        }
        
        // Use get() for safe slicing
        let reserve0 = U256::from_be_slice(log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for reserve0"))?);
        let reserve1 = U256::from_be_slice(log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for reserve1"))?);
        
        Ok(Some(DecodedEvent::UniswapV2Sync(UniswapV2Sync {
            pair_address: log.address,
            reserve0,
            reserve1,
            log_index,
        })))
    }
    
    fn decode_uniswap_v2_mint(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 2 || log.data.data.len() != 64 {
            return Ok(None);
        }
        
        let sender_bytes: &[u8] = log.topics()[1].as_ref();
        
        // Ensure we have enough bytes for address extraction
        if sender_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for Uniswap V2 mint"));
        }
        
        let sender = Address::from_slice(&sender_bytes[12..32]);
        
        // Use get() for safe slicing
        let amount0 = U256::from_be_slice(log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?);
        let amount1 = U256::from_be_slice(log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?);
        
        Ok(Some(DecodedEvent::MintAction(MintAction {
            pair_address: log.address,
            sender,
            amount0,
            amount1,
            log_index,
        })))
    }
    
    fn decode_uniswap_v2_burn(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        // Uniswap V2 Burn event: Burn(address indexed sender, uint amount0, uint amount1, address indexed to)
        // Topics: [signature, sender, to]
        // Data: [amount0, amount1]
        if log.topics().len() != 3 || log.data.data.len() != 64 {
            return Ok(None);
        }
        
        let sender_bytes: &[u8] = log.topics()[1].as_ref();
        let to_bytes: &[u8] = log.topics()[2].as_ref();
        
        // Ensure we have enough bytes for address extraction
        if sender_bytes.len() < 32 || to_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for Uniswap V2 burn"));
        }
        
        let sender = Address::from_slice(&sender_bytes[12..32]);
        // to address is indexed but not used in BurnAction
        
        // Extract amounts from data
        let amount0 = U256::from_be_slice(log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?);
        let amount1 = U256::from_be_slice(log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?);
        
        // For now, use amount0 as the main amount (or we could sum them)
        // This matches behavior where we track LP token burn amount
        let amount = amount0;
        
        Ok(Some(DecodedEvent::BurnAction(BurnAction {
            pair_address: log.address,
            sender,
            amount,
            log_index,
        })))
    }
    
    fn decode_uniswap_v2_pair_created(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 64 {
            return Ok(None);
        }
        
        let token0_bytes: &[u8] = log.topics()[1].as_ref();
        let token1_bytes: &[u8] = log.topics()[2].as_ref();
        
        // Ensure we have enough bytes for address extraction
        if token0_bytes.len() < 32 || token1_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for PairCreated"));
        }
        
        let token0 = Address::from_slice(&token0_bytes[12..32]);
        let token1 = Address::from_slice(&token1_bytes[12..32]);
        
        // Extract pair address from data
        let pair_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for pair address"))?;
        let pair_address = Address::from_slice(&pair_bytes[12..32]);
        
        Ok(Some(DecodedEvent::PairAction(PairAction {
            pair_address,
            token0,
            token1,
            log_index,
        })))
    }
    
    fn decode_uniswap_v3_swap(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 160 {
            return Ok(None);
        }
        
        let sender_bytes: &[u8] = log.topics()[1].as_ref();
        let recipient_bytes: &[u8] = log.topics()[2].as_ref();
        
        // Ensure we have enough bytes for address extraction
        if sender_bytes.len() < 32 || recipient_bytes.len() < 32 {
            return Err(eyre::eyre!("Invalid topic length for Uniswap V3 swap"));
        }
        
        let sender = Address::from_slice(&sender_bytes[12..32]);
        let recipient = Address::from_slice(&recipient_bytes[12..32]);
        
        // Decode signed integers from data with safe slicing
        let amount0_bytes = log.data.data.get(16..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?
            .try_into()
            .map_err(|_| eyre::eyre!("Failed to convert amount0 bytes"))?;
        let amount0 = i128::from_be_bytes(amount0_bytes);
        
        let amount1_bytes = log.data.data.get(48..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?
            .try_into()
            .map_err(|_| eyre::eyre!("Failed to convert amount1 bytes"))?;
        let amount1 = i128::from_be_bytes(amount1_bytes);
        
        let sqrt_price_x96 = U256::from_be_slice(log.data.data.get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for sqrt_price_x96"))?);
        
        let liquidity_bytes = log.data.data.get(112..128)
            .ok_or_else(|| eyre::eyre!("Invalid data length for liquidity"))?
            .try_into()
            .map_err(|_| eyre::eyre!("Failed to convert liquidity bytes"))?;
        let liquidity = u128::from_be_bytes(liquidity_bytes);
        
        let tick_bytes = log.data.data.get(156..160)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick"))?
            .try_into()
            .map_err(|_| eyre::eyre!("Failed to convert tick bytes"))?;
        let tick = i32::from_be_bytes(tick_bytes);
        
        Ok(Some(DecodedEvent::UniswapV3Swap(UniswapV3Swap {
            pool_address: log.address,
            sender,
            recipient,
            amount0,
            amount1,
            sqrt_price_x96,
            liquidity,
            tick,
            log_index,
        })))
    }

    // ERC721 Transfer decoder
    fn decode_erc721_transfer(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 {
            return Ok(None);
        }

        let from_bytes: &[u8] = log.topics()[1].as_ref();
        let to_bytes: &[u8] = log.topics()[2].as_ref();
        let token_id_bytes: &[u8] = log.topics()[3].as_ref();

        if from_bytes.len() < 32 || to_bytes.len() < 32 || token_id_bytes.len() < 32 {
            return Ok(None);
        }

        let from_address = Address::from_slice(&from_bytes[12..32]);
        let to_address = Address::from_slice(&to_bytes[12..32]);
        let token_id = U256::from_be_slice(token_id_bytes);

        Ok(Some(DecodedEvent::ERC721Transfer(ERC721Transfer {
            token_address: log.address,
            from_address,
            to_address,
            token_id,
            log_index,
        })))
    }

    // ERC721 Approval decoder
    fn decode_erc721_approval(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 {
            return Ok(None);
        }

        let owner_bytes: &[u8] = log.topics()[1].as_ref();
        let approved_bytes: &[u8] = log.topics()[2].as_ref();
        let token_id_bytes: &[u8] = log.topics()[3].as_ref();

        if owner_bytes.len() < 32 || approved_bytes.len() < 32 || token_id_bytes.len() < 32 {
            return Ok(None);
        }

        let owner = Address::from_slice(&owner_bytes[12..32]);
        let approved = Address::from_slice(&approved_bytes[12..32]);
        let token_id = U256::from_be_slice(token_id_bytes);

        Ok(Some(DecodedEvent::ERC721Approval(ERC721Approval {
            token_address: log.address,
            owner,
            approved_address: approved,
            token_id,
            log_index,
        })))
    }

    // ERC1155 TransferSingle decoder
    fn decode_erc1155_transfer(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let operator_bytes: &[u8] = log.topics()[1].as_ref();
        let from_bytes: &[u8] = log.topics()[2].as_ref();
        let to_bytes: &[u8] = log.topics()[3].as_ref();

        if operator_bytes.len() < 32 || from_bytes.len() < 32 || to_bytes.len() < 32 {
            return Ok(None);
        }

        let operator = Address::from_slice(&operator_bytes[12..32]);
        let from_address = Address::from_slice(&from_bytes[12..32]);
        let to_address = Address::from_slice(&to_bytes[12..32]);

        let token_id = U256::from_be_slice(log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for token_id"))?);
        let amount = U256::from_be_slice(log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount"))?);

        Ok(Some(DecodedEvent::ERC1155Transfer(ERC1155Transfer {
            token_address: log.address,
            operator,
            from_address,
            to_address,
            token_ids: vec![token_id],
            amounts: vec![amount],
            log_index,
        })))
    }

    // Uniswap V3 Mint decoder
    fn decode_uniswap_v3_mint(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 128 {
            return Ok(None);
        }

        let owner_bytes: &[u8] = log.topics()[1].as_ref();
        let tick_lower_bytes: &[u8] = log.topics()[2].as_ref();
        let tick_upper_bytes: &[u8] = log.topics()[3].as_ref();

        if owner_bytes.len() < 32 || tick_lower_bytes.len() < 32 || tick_upper_bytes.len() < 32 {
            return Ok(None);
        }

        let owner = Address::from_slice(&owner_bytes[12..32]);
        let tick_lower = i32::from_be_bytes(
            tick_lower_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_lower bytes"))?
        );
        let tick_upper = i32::from_be_bytes(
            tick_upper_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_upper bytes"))?
        );

        let sender_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for sender"))?;
        let sender = Address::from_slice(&sender_bytes[12..32]);

        let amount_bytes = log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount"))?;
        let amount = U256::from_be_slice(amount_bytes);

        let amount0 = U256::from_be_slice(log.data.data.get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?);
        let amount1 = U256::from_be_slice(log.data.data.get(96..128)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?);

        Ok(Some(DecodedEvent::UniswapV3Mint(UniswapV3Mint {
            pool_address: log.address,
            sender,
            owner,
            tick_lower,
            tick_upper,
            amount,
            amount0,
            amount1,
            log_index,
        })))
    }

    // Uniswap V3 Burn decoder
    fn decode_uniswap_v3_burn(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 96 {
            return Ok(None);
        }

        let owner_bytes: &[u8] = log.topics()[1].as_ref();
        let tick_lower_bytes: &[u8] = log.topics()[2].as_ref();
        let tick_upper_bytes: &[u8] = log.topics()[3].as_ref();

        if owner_bytes.len() < 32 || tick_lower_bytes.len() < 32 || tick_upper_bytes.len() < 32 {
            return Ok(None);
        }

        let owner = Address::from_slice(&owner_bytes[12..32]);
        let tick_lower = i32::from_be_bytes(
            tick_lower_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_lower bytes"))?
        );
        let tick_upper = i32::from_be_bytes(
            tick_upper_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_upper bytes"))?
        );

        let amount_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount"))?;
        let amount = U256::from_be_slice(amount_bytes);

        let amount0 = U256::from_be_slice(log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?);
        let amount1 = U256::from_be_slice(log.data.data.get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?);

        Ok(Some(DecodedEvent::UniswapV3Burn(UniswapV3Burn {
            pool_address: log.address,
            owner,
            tick_lower,
            tick_upper,
            amount,
            amount0,
            amount1,
            log_index,
        })))
    }

    // Uniswap V3 PoolCreated decoder
    fn decode_uniswap_v3_pool_created(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let token0_bytes: &[u8] = log.topics()[1].as_ref();
        let token1_bytes: &[u8] = log.topics()[2].as_ref();
        let fee_bytes: &[u8] = log.topics()[3].as_ref();

        if token0_bytes.len() < 32 || token1_bytes.len() < 32 || fee_bytes.len() < 32 {
            return Ok(None);
        }

        let token0 = Address::from_slice(&token0_bytes[12..32]);
        let token1 = Address::from_slice(&token1_bytes[12..32]);
        let fee = u32::from_be_bytes(
            fee_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert fee bytes"))?
        );

        let tick_spacing_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick_spacing"))?;
        let tick_spacing = i32::from_be_bytes(
            tick_spacing_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_spacing bytes"))?
        );

        let pool_bytes = log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for pool"))?;
        let pool = Address::from_slice(&pool_bytes[12..32]);

        Ok(Some(DecodedEvent::UniswapV3PoolCreated(UniswapV3PoolCreated {
            token0,
            token1,
            fee,
            tick_spacing,
            pool,
            log_index,
        })))
    }

    // Uniswap V3 Initialize decoder
    fn decode_uniswap_v3_initialize(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 1 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let sqrt_price_x96_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for sqrt_price_x96"))?;
        let sqrt_price_x96 = U256::from_be_slice(sqrt_price_x96_bytes);

        let tick_bytes = log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick"))?;
        let tick = i32::from_be_bytes(
            tick_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick bytes"))?
        );

        Ok(Some(DecodedEvent::UniswapV3Initialize(UniswapV3Initialize {
            pool_address: log.address,
            sqrt_price_x96,
            tick,
            log_index,
        })))
    }

    // Uniswap V3 Position decoder (similar to IncreaseLiquidity but with more fields)
    fn decode_uniswap_v3_position(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        // This handles position events that have extended topics for owner/tick information
        // Based on Python's parse_uniswap_v3_position logic
        if log.topics().len() < 2 || log.data.data.len() < 96 {
            return Ok(None);
        }

        let token_id_bytes: &[u8] = log.topics()[1].as_ref();
        let token_id = U256::from_be_slice(token_id_bytes);

        // Extract owner from topic 2 if present
        let owner = if log.topics().len() > 2 {
            let owner_bytes: &[u8] = log.topics()[2].as_ref();
            if owner_bytes.len() >= 32 {
                Some(Address::from_slice(&owner_bytes[12..32]))
            } else {
                None
            }
        } else {
            None
        };

        // Extract tick bounds from topics 3 and 4 if present
        let tick_lower = if log.topics().len() > 3 {
            let tick_bytes: &[u8] = log.topics()[3].as_ref();
            if tick_bytes.len() >= 32 {
                i32::from_be_bytes(
                    tick_bytes[28..32].try_into()
                        .unwrap_or([0, 0, 0, 0])
                )
            } else {
                0
            }
        } else {
            0
        };

        let tick_upper = if log.topics().len() > 4 {
            let tick_bytes: &[u8] = log.topics()[4].as_ref();
            if tick_bytes.len() >= 32 {
                i32::from_be_bytes(
                    tick_bytes[28..32].try_into()
                        .unwrap_or([0, 0, 0, 0])
                )
            } else {
                0
            }
        } else {
            0
        };

        // Parse data fields
        let liquidity = U256::from_be_slice(log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for liquidity"))?);
        let amount0 = U256::from_be_slice(log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?);
        let amount1 = U256::from_be_slice(log.data.data.get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?);

        Ok(Some(DecodedEvent::UniswapV3Position(UniswapV3Position {
            token_id,
            liquidity,
            amount0,
            amount1,
            pool_address: log.address,
            owner: owner.unwrap_or(Address::ZERO),
            tick_lower,
            tick_upper,
            log_index,
        })))
    }

    // Uniswap V4 Swap decoder
    fn decode_uniswap_v4_swap(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 192 {
            return Ok(None);
        }

        let id = log.topics()[1];
        let sender_bytes: &[u8] = log.topics()[2].as_ref();

        if sender_bytes.len() < 32 {
            return Ok(None);
        }

        let sender = Address::from_slice(&sender_bytes[12..32]);

        let amount0_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?;
        let amount0 = i128::from_be_bytes(
            amount0_bytes[16..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert amount0 bytes"))?
        );

        let amount1_bytes = log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?;
        let amount1 = i128::from_be_bytes(
            amount1_bytes[16..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert amount1 bytes"))?
        );

        let sqrt_price_x96 = U256::from_be_slice(log.data.data.get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for sqrt_price_x96"))?);

        let liquidity_bytes = log.data.data.get(96..128)
            .ok_or_else(|| eyre::eyre!("Invalid data length for liquidity"))?;
        let liquidity = u128::from_be_bytes(
            liquidity_bytes[16..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert liquidity bytes"))?
        );

        let tick_bytes = log.data.data.get(128..160)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick"))?;
        let tick = i32::from_be_bytes(
            tick_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick bytes"))?
        );

        let fee_bytes = log.data.data.get(160..192)
            .ok_or_else(|| eyre::eyre!("Invalid data length for fee"))?;
        let fee = u32::from_be_bytes(
            fee_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert fee bytes"))?
        );

        Ok(Some(DecodedEvent::UniswapV4Swap(UniswapV4Swap {
            pool_manager_address: log.address,
            event_id: id,
            sender,
            amount0,
            amount1,
            sqrt_price_x96,
            liquidity,
            tick,
            fee,
            log_index,
        })))
    }

    // Uniswap V4 Initialize decoder
    fn decode_uniswap_v4_initialize(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 160 {
            return Ok(None);
        }

        let id = log.topics()[1];
        let currency0_bytes: &[u8] = log.topics()[2].as_ref();
        let currency1_bytes: &[u8] = log.topics()[3].as_ref();

        if currency0_bytes.len() < 32 || currency1_bytes.len() < 32 {
            return Ok(None);
        }

        let currency0 = Address::from_slice(&currency0_bytes[12..32]);
        let currency1 = Address::from_slice(&currency1_bytes[12..32]);

        let fee_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for fee"))?;
        let fee = u32::from_be_bytes(
            fee_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert fee bytes"))?
        );

        let tick_spacing_bytes = log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick_spacing"))?;
        let tick_spacing = i32::from_be_bytes(
            tick_spacing_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_spacing bytes"))?
        );

        let hooks_bytes = log.data.data.get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for hooks"))?;
        let hooks = Address::from_slice(&hooks_bytes[12..32]);

        let sqrt_price_x96 = U256::from_be_slice(log.data.data.get(96..128)
            .ok_or_else(|| eyre::eyre!("Invalid data length for sqrt_price_x96"))?);

        let tick_bytes = log.data.data.get(128..160)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick"))?;
        let tick = i32::from_be_bytes(
            tick_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick bytes"))?
        );

        Ok(Some(DecodedEvent::UniswapV4Initialize(UniswapV4Initialize {
            pool_manager_address: log.address,
            event_id: id,
            currency0,
            currency1,
            fee,
            tick_spacing,
            hooks,
            sqrt_price_x96,
            tick,
            log_index,
        })))
    }

    // Uniswap V4 ModifyLiquidity decoder
    fn decode_uniswap_v4_modify_liquidity(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 128 {
            return Ok(None);
        }

        let id = log.topics()[1];
        let sender_bytes: &[u8] = log.topics()[2].as_ref();

        if sender_bytes.len() < 32 {
            return Ok(None);
        }

        let sender = Address::from_slice(&sender_bytes[12..32]);

        let tick_lower_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick_lower"))?;
        let tick_lower = i32::from_be_bytes(
            tick_lower_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_lower bytes"))?
        );

        let tick_upper_bytes = log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for tick_upper"))?;
        let tick_upper = i32::from_be_bytes(
            tick_upper_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert tick_upper bytes"))?
        );

        let liquidity_delta_bytes = log.data.data.get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for liquidity_delta"))?;
        let liquidity_delta = i128::from_be_bytes(
            liquidity_delta_bytes[16..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert liquidity_delta bytes"))?
        );

        let salt = B256::from_slice(log.data.data.get(96..128)
            .ok_or_else(|| eyre::eyre!("Invalid data length for salt"))?);

        Ok(Some(DecodedEvent::UniswapV4ModifyLiquidity(UniswapV4ModifyLiquidity {
            pool_manager_address: log.address,
            event_id: id,
            sender,
            tick_lower,
            tick_upper,
            liquidity_delta,
            salt,
            log_index,
        })))
    }

    // Uniswap V4 Donate decoder
    fn decode_uniswap_v4_donate(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let id = log.topics()[1];
        let sender_bytes: &[u8] = log.topics()[2].as_ref();

        if sender_bytes.len() < 32 {
            return Ok(None);
        }

        let sender = Address::from_slice(&sender_bytes[12..32]);

        let amount0_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?;
        let amount0 = i128::from_be_bytes(
            amount0_bytes[16..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert amount0 bytes"))?
        );

        let amount1_bytes = log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?;
        let amount1 = i128::from_be_bytes(
            amount1_bytes[16..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert amount1 bytes"))?
        );

        Ok(Some(DecodedEvent::UniswapV4Donate(UniswapV4Donate {
            pool_manager_address: log.address,
            event_id: id,
            sender,
            amount0: U256::from(amount0 as u128),
            amount1: U256::from(amount1 as u128),
            log_index,
        })))
    }

    // Uniswap V4 ProtocolFeeUpdated decoder
    fn decode_uniswap_v4_protocol_fee_updated(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 2 || log.data.data.len() != 32 {
            return Ok(None);
        }

        let id = log.topics()[1];

        let protocol_fee_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for protocol_fee"))?;
        let protocol_fee = u32::from_be_bytes(
            protocol_fee_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert protocol_fee bytes"))?
        );

        Ok(Some(DecodedEvent::UniswapV4ProtocolFeeUpdated(UniswapV4ProtocolFeeUpdated {
            pool_manager_address: log.address,
            event_id: id,
            protocol_fee,
            log_index,
        })))
    }

    // Uniswap V4 DynamicLPFeeUpdated decoder
    fn decode_uniswap_v4_dynamic_lp_fee_updated(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 2 || log.data.data.len() != 32 {
            return Ok(None);
        }

        let id = log.topics()[1];

        let dynamic_lp_fee_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for dynamic_lp_fee"))?;
        let dynamic_lp_fee = u32::from_be_bytes(
            dynamic_lp_fee_bytes[28..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert dynamic_lp_fee bytes"))?
        );

        Ok(Some(DecodedEvent::UniswapV4DynamicLPFeeUpdated(UniswapV4DynamicLPFeeUpdated {
            pool_manager_address: log.address,
            event_id: id,
            dynamic_lp_fee,
            log_index,
        })))
    }

    // Uniswap V4 ProtocolFeeControllerUpdated decoder
    fn decode_uniswap_v4_protocol_fee_controller_updated(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 1 || log.data.data.len() != 32 {
            return Ok(None);
        }

        let protocol_fee_controller_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for protocol_fee_controller"))?;
        let protocol_fee_controller = Address::from_slice(&protocol_fee_controller_bytes[12..32]);

        Ok(Some(DecodedEvent::UniswapV4ProtocolFeeControllerUpdated(UniswapV4ProtocolFeeControllerUpdated {
            pool_manager_address: log.address,
            protocol_fee_controller,
            log_index,
        })))
    }

    // Uniswap V4 BalanceDelta decoder
    fn decode_uniswap_v4_balance_delta(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let pool_id = log.topics()[1];
        let settler_bytes: &[u8] = log.topics()[2].as_ref();

        if settler_bytes.len() < 32 {
            return Ok(None);
        }

        let settler = Address::from_slice(&settler_bytes[12..32]);

        let delta0_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for delta0"))?;
        let delta0 = i128::from_be_bytes(
            delta0_bytes[16..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert delta0 bytes"))?
        );

        let delta1_bytes = log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for delta1"))?;
        let delta1 = i128::from_be_bytes(
            delta1_bytes[16..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert delta1 bytes"))?
        );

        Ok(Some(DecodedEvent::UniswapV4BalanceDelta(UniswapV4BalanceDelta {
            pool_manager_address: log.address,
            pool_id,
            settler,
            delta0,
            delta1,
            log_index,
        })))
    }

    // Uniswap V3 IncreaseLiquidity decoder
    fn decode_uniswap_v3_increase_liquidity(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 2 || log.data.data.len() != 96 {
            return Ok(None);
        }

        let token_id = U256::from_be_slice(log.topics()[1].as_ref());

        let liquidity_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for liquidity"))?;
        let liquidity = U256::from_be_slice(liquidity_bytes);

        let amount0 = U256::from_be_slice(log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?);
        let amount1 = U256::from_be_slice(log.data.data.get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?);

        Ok(Some(DecodedEvent::UniswapV3IncreaseLiquidity(UniswapV3IncreaseLiquidity {
            token_id,
            liquidity,
            amount0,
            amount1,
            pool_address: log.address,
            log_index,
        })))
    }

    // Uniswap V3 DecreaseLiquidity decoder
    fn decode_uniswap_v3_decrease_liquidity(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 2 || log.data.data.len() != 96 {
            return Ok(None);
        }

        let token_id = U256::from_be_slice(log.topics()[1].as_ref());

        let liquidity_bytes = log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for liquidity"))?;
        let liquidity = U256::from_be_slice(liquidity_bytes);

        let amount0 = U256::from_be_slice(log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?);
        let amount1 = U256::from_be_slice(log.data.data.get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?);

        Ok(Some(DecodedEvent::UniswapV3DecreaseLiquidity(UniswapV3DecreaseLiquidity {
            token_id,
            liquidity,
            amount0,
            amount1,
            pool_address: log.address,
            log_index,
        })))
    }

    // Uniswap V3 Collect decoder
    fn decode_uniswap_v3_collect(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 64 {
            return Ok(None);
        }

        let token_id = U256::from_be_slice(log.topics()[1].as_ref());
        let recipient_bytes: &[u8] = log.topics()[2].as_ref();

        if recipient_bytes.len() < 32 {
            return Ok(None);
        }

        let recipient = Address::from_slice(&recipient_bytes[12..32]);

        let amount0 = U256::from_be_slice(log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount0"))?);
        let amount1 = U256::from_be_slice(log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount1"))?);

        Ok(Some(DecodedEvent::UniswapV3Collect(UniswapV3Collect {
            token_id,
            recipient,
            amount0,
            amount1,
            pool_address: log.address,
            log_index,
        })))
    }

    // Deposit decoder - handles both complex and simple deposit formats
    fn decode_deposit(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        // Handle two types of deposits:
        // 1. Complex deposit: Deposit(uint256 id, address indexed tokenAddress, address indexed withdrawalAddress, uint256 amount, uint256 unlockTime)
        // 2. WETH deposit: Deposit(address indexed dst, uint256 wad)
        
        // Check if this is a complex deposit (with id, token, withdrawal - has more topics)
        if log.topics().len() > 2 {
            // Complex deposit has 3 topics (signature + 2 indexed addresses)
            // Data contains: id, amount, unlockTime
            if log.data.data.len() < 96 {
                return Ok(None);
            }
            
            let token_bytes: &[u8] = log.topics()[1].as_ref();
            let withdrawal_bytes: &[u8] = log.topics()[2].as_ref();
            
            if token_bytes.len() < 32 || withdrawal_bytes.len() < 32 {
                return Ok(None);
            }
            
            let token_address = Address::from_slice(&token_bytes[12..32]);
            let withdrawal_address = Address::from_slice(&withdrawal_bytes[12..32]);
            
            let id = u64::try_from(U256::from_be_slice(log.data.data.get(0..32)
                .ok_or_else(|| eyre::eyre!("Invalid data length for id"))?))
                .unwrap_or(0);
            let amount = U256::from_be_slice(log.data.data.get(32..64)
                .ok_or_else(|| eyre::eyre!("Invalid data length for amount"))?);
            let unlock_time = u64::try_from(U256::from_be_slice(log.data.data.get(64..96)
                .ok_or_else(|| eyre::eyre!("Invalid data length for unlock_time"))?))
                .unwrap_or(0);
            
            Ok(Some(DecodedEvent::DepositAction(DepositAction {
                id: Some(id),
                token_address: Some(token_address),
                withdrawal_address: Some(withdrawal_address),
                amount: Some(amount),
                unlock_time: Some(unlock_time),
                pair_address: None,
                sender: None,
                log_index: Some(log_index),
            })))
        } else if log.topics().len() == 2 && log.data.data.len() == 32 {
            // Simple deposit (like WETH) - data contains only amount
            let sender_bytes: &[u8] = log.topics()[1].as_ref();
            if sender_bytes.len() < 32 {
                return Ok(None);
            }
            
            let sender = Address::from_slice(&sender_bytes[12..32]);
            let amount = U256::from_be_slice(&log.data.data);
            
            Ok(Some(DecodedEvent::DepositAction(DepositAction {
                id: None,
                token_address: None,
                withdrawal_address: None,
                amount: Some(amount),
                unlock_time: None,
                pair_address: Some(log.address),
                sender: Some(sender),
                log_index: Some(log_index),
            })))
        } else {
            Ok(None)
        }
    }

    // Withdraw decoder
    fn decode_withdraw(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 2 || log.data.data.len() != 32 {
            return Ok(None);
        }

        let sender_bytes: &[u8] = log.topics()[1].as_ref();

        if sender_bytes.len() < 32 {
            return Ok(None);
        }

        let sender = Address::from_slice(&sender_bytes[12..32]);
        let amount = U256::from_be_slice(&log.data.data);

        Ok(Some(DecodedEvent::WithdrawAction(WithdrawAction {
            pair_address: log.address,
            sender,
            amount,
            log_index,
        })))
    }

    // OwnershipTransferred decoder
    fn decode_ownership_transferred(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 {
            return Ok(None);
        }

        let previous_owner_bytes: &[u8] = log.topics()[1].as_ref();
        let new_owner_bytes: &[u8] = log.topics()[2].as_ref();

        if previous_owner_bytes.len() < 32 || new_owner_bytes.len() < 32 {
            return Ok(None);
        }

        let previous_owner = Address::from_slice(&previous_owner_bytes[12..32]);
        let new_owner = Address::from_slice(&new_owner_bytes[12..32]);

        Ok(Some(DecodedEvent::OwnerEvent(OwnerEvent {
            contract_address: log.address,
            previous_owner,
            new_owner,
            log_index,
        })))
    }

    // TradingEnabled decoder
    fn decode_trading_enabled(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 1 {
            return Ok(None);
        }

        Ok(Some(DecodedEvent::TradingEnabledEvent(TradingEnabledEvent {
            token_address: log.address,
            block_number: 0, // Will be set by the processor
            log_index,
        })))
    }

    // TradingDisabled decoder
    fn decode_trading_disabled(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 1 {
            return Ok(None);
        }

        Ok(Some(DecodedEvent::TradingDisabledEvent(TradingDisabledEvent {
            token_address: log.address,
            block_number: 0, // Will be set by the processor
            log_index,
        })))
    }

    // Permit2 decoder
    fn decode_permit2(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 4 || log.data.data.len() != 96 {
            return Ok(None);
        }

        let owner_bytes: &[u8] = log.topics()[1].as_ref();
        let token_bytes: &[u8] = log.topics()[2].as_ref();
        let spender_bytes: &[u8] = log.topics()[3].as_ref();

        if owner_bytes.len() < 32 || token_bytes.len() < 32 || spender_bytes.len() < 32 {
            return Ok(None);
        }

        let owner = Address::from_slice(&owner_bytes[12..32]);
        let token = Address::from_slice(&token_bytes[12..32]);
        let spender = Address::from_slice(&spender_bytes[12..32]);

        let amount = U256::from_be_slice(log.data.data.get(0..32)
            .ok_or_else(|| eyre::eyre!("Invalid data length for amount"))?);
        
        let expiration_bytes = log.data.data.get(32..64)
            .ok_or_else(|| eyre::eyre!("Invalid data length for expiration"))?;
        let expiration = u64::from_be_bytes(
            expiration_bytes[24..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert expiration bytes"))?
        );

        let nonce_bytes = log.data.data.get(64..96)
            .ok_or_else(|| eyre::eyre!("Invalid data length for nonce"))?;
        let nonce = u64::from_be_bytes(
            nonce_bytes[24..32].try_into()
                .map_err(|_| eyre::eyre!("Failed to convert nonce bytes"))?
        );

        Ok(Some(DecodedEvent::Permit2(Permit2 {
            pool_manager_address: log.address,
            owner,
            token,
            spender,
            amount,
            expiration,
            nonce,
            log_index,
        })))
    }
    
    /// Extract logs from CallFrame recursively
    /// 
    /// This method replaces the functionality from tx_simulator's trace_extraction.rs
    /// Moving log extraction to tx_processor where it belongs - tx_simulator should only simulate,
    /// tx_processor should handle result processing including log extraction.
    pub fn extract_logs_from_call_frame(&self, frame: &tx_simulator::CallFrame) -> Vec<AlloyLog> {
        let mut logs = Vec::new();
        self.extract_logs_recursive(frame, &mut logs);
        logs
    }
    
    /// Recursively extract logs from call frame and its children
    fn extract_logs_recursive(&self, frame: &tx_simulator::CallFrame, logs: &mut Vec<AlloyLog>) {
        // Add logs from this frame
        for log in &frame.logs {
            if let (Some(address), Some(topics)) = (&log.address, &log.topics) {
                logs.push(AlloyLog::new_unchecked(
                    *address,
                    topics.clone(),
                    log.data.clone().unwrap_or_default(),
                ));
            }
        }
        
        // Process child calls
        for child in &frame.calls {
            self.extract_logs_recursive(child, logs);
        }
    }
}

/// Enum for decoded events
#[derive(Debug, Clone)]
pub enum DecodedEvent {
    ERC20Transfer(ERC20Transfer),
    ERC721Transfer(ERC721Transfer),
    ERC1155Transfer(ERC1155Transfer),
    ERC20Approval(ERC20Approval),
    ERC721Approval(ERC721Approval),
    UniswapV2Sync(UniswapV2Sync),
    UniswapV2Swap(UniswapV2Swap),
    MintAction(MintAction),
    BurnAction(BurnAction),
    PairAction(PairAction),
    UniswapV3Swap(UniswapV3Swap),
    UniswapV3Mint(UniswapV3Mint),
    UniswapV3Burn(UniswapV3Burn),
    UniswapV3PoolCreated(UniswapV3PoolCreated),
    UniswapV3Initialize(UniswapV3Initialize),
    UniswapV3Position(UniswapV3Position),
    UniswapV3IncreaseLiquidity(UniswapV3IncreaseLiquidity),
    UniswapV3DecreaseLiquidity(UniswapV3DecreaseLiquidity),
    UniswapV3Collect(UniswapV3Collect),
    UniswapV4Swap(UniswapV4Swap),
    UniswapV4Initialize(UniswapV4Initialize),
    UniswapV4ModifyLiquidity(UniswapV4ModifyLiquidity),
    UniswapV4Donate(UniswapV4Donate),
    UniswapV4ProtocolFeeUpdated(UniswapV4ProtocolFeeUpdated),
    UniswapV4DynamicLPFeeUpdated(UniswapV4DynamicLPFeeUpdated),
    UniswapV4ProtocolFeeControllerUpdated(UniswapV4ProtocolFeeControllerUpdated),
    UniswapV4BalanceDelta(UniswapV4BalanceDelta),
    DepositAction(DepositAction),
    WithdrawAction(WithdrawAction),
    OwnerEvent(OwnerEvent),
    TradingEnabledEvent(TradingEnabledEvent),
    TradingDisabledEvent(TradingDisabledEvent),
    Permit2(Permit2),
}