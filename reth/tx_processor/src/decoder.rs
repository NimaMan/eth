use alloy_primitives::{Address, B256, U256, Log as AlloyLog};
use crate::models::events::*;
use eyre::Result;

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
    pub permit2: B256,
    
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
            permit2: keccak256(b"Permit(address,address,address,uint160,uint48,uint48)"),
            
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
    pub fn decode_log(&self, log: &AlloyLog) -> Result<Option<DecodedEvent>> {
        if log.topics().is_empty() {
            return Ok(None);
        }
        
        let event_signature = log.topics()[0];
        let log_index = 0u64; // This should come from transaction receipt
        
        // ERC20 Transfer
        if event_signature == self.signatures.transfer && log.topics().len() == 3 {
            return self.decode_erc20_transfer(log, log_index);
        }
        
        // ERC20 Approval
        if event_signature == self.signatures.approval && log.topics().len() == 3 {
            return self.decode_erc20_approval(log, log_index);
        }
        
        // Uniswap V2 Swap
        if event_signature == self.signatures.swap && log.topics().len() == 3 {
            return self.decode_uniswap_v2_swap(log, log_index);
        }
        
        // Uniswap V2 Sync
        if event_signature == self.signatures.sync && log.topics().len() == 1 {
            return self.decode_uniswap_v2_sync(log, log_index);
        }
        
        // Uniswap V3 Swap
        if event_signature == self.signatures.swap_v3 && log.topics().len() == 3 {
            return self.decode_uniswap_v3_swap(log, log_index);
        }
        
        // Add more decoders as needed...
        
        Ok(None)
    }
    
    fn decode_erc20_transfer(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 32 {
            return Ok(None);
        }
        
        let from_bytes: &[u8] = log.topics()[1].as_ref();
        let to_bytes: &[u8] = log.topics()[2].as_ref();
        let from_address = Address::from_slice(&from_bytes[12..]);
        let to_address = Address::from_slice(&to_bytes[12..]);
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
        let owner = Address::from_slice(&owner_bytes[12..]);
        let spender = Address::from_slice(&spender_bytes[12..]);
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
        let sender = Address::from_slice(&sender_bytes[12..]);
        let to = Address::from_slice(&to_bytes[12..]);
        
        let amount0_in = U256::from_be_slice(&log.data.data[0..32]);
        let amount1_in = U256::from_be_slice(&log.data.data[32..64]);
        let amount0_out = U256::from_be_slice(&log.data.data[64..96]);
        let amount1_out = U256::from_be_slice(&log.data.data[96..128]);
        
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
        
        let reserve0 = U256::from_be_slice(&log.data.data[0..32]);
        let reserve1 = U256::from_be_slice(&log.data.data[32..64]);
        
        Ok(Some(DecodedEvent::UniswapV2Sync(UniswapV2Sync {
            pair_address: log.address,
            reserve0,
            reserve1,
            log_index,
        })))
    }
    
    fn decode_uniswap_v3_swap(&self, log: &AlloyLog, log_index: u64) -> Result<Option<DecodedEvent>> {
        if log.topics().len() != 3 || log.data.data.len() != 160 {
            return Ok(None);
        }
        
        let sender_bytes: &[u8] = log.topics()[1].as_ref();
        let recipient_bytes: &[u8] = log.topics()[2].as_ref();
        let sender = Address::from_slice(&sender_bytes[12..]);
        let recipient = Address::from_slice(&recipient_bytes[12..]);
        
        // Decode signed integers from data
        let amount0 = i128::from_be_bytes(log.data.data[16..32].try_into()?);
        let amount1 = i128::from_be_bytes(log.data.data[48..64].try_into()?);
        let sqrt_price_x96 = U256::from_be_slice(&log.data.data[64..96]);
        let liquidity = u128::from_be_bytes(log.data.data[112..128].try_into()?);
        let tick = i32::from_be_bytes(log.data.data[156..160].try_into()?);
        
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
    UniswapV3Swap(UniswapV3Swap),
    UniswapV3Mint(UniswapV3Mint),
    UniswapV3Burn(UniswapV3Burn),
    UniswapV3PoolCreated(UniswapV3PoolCreated),
    UniswapV3Initialize(UniswapV3Initialize),
    UniswapV4Swap(UniswapV4Swap),
    UniswapV4Initialize(UniswapV4Initialize),
    // Add more variants as needed
}