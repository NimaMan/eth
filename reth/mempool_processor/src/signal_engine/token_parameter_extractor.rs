/// Token Parameter Extractor
/// 
/// Extracts token parameters from creation, storage, and function calls

use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;

#[derive(Debug, Clone)]
pub struct TokenParameters {
    pub buy_tax: Option<u8>,
    pub sell_tax: Option<u8>,
    pub max_wallet: Option<U256>,
    pub max_transaction: Option<U256>,
    pub has_trading_enabled: bool,
    pub owner: Option<Address>,
    pub is_renounced: bool,
    pub detected_honeypot_flags: Vec<String>,
}

pub struct TokenParameterExtractor {
    provider: Provider<Http>,
}

impl TokenParameterExtractor {
    pub fn new(provider: Provider<Http>) -> Self {
        Self { provider }
    }
    
    /// Extract parameters from token creation
    pub async fn extract_from_creation(
        &self,
        creation_tx: &Transaction,
        contract_address: Address,
    ) -> Result<TokenParameters, Box<dyn std::error::Error>> {
        let mut params = TokenParameters::default();
        
        // 1. Try to decode constructor parameters
        // TODO: Implement constructor parameter decoding
        // if creation_tx.input.len() > 0 {
        //     params = self.decode_constructor_params(&creation_tx.input).unwrap_or(params);
        // }
        
        // 2. Read storage slots for common patterns
        let storage_params = self.read_storage_parameters(contract_address).await?;
        params.merge(storage_params);
        
        // 3. Call view functions if available
        let view_params = self.call_view_functions(contract_address).await?;
        params.merge(view_params);
        
        // 4. Detect honeypot patterns
        self.detect_honeypot_patterns(&mut params, contract_address).await?;
        
        Ok(params)
    }
    
    /// Read common storage slots
    async fn read_storage_parameters(
        &self,
        contract_address: Address,
    ) -> Result<TokenParameters, Box<dyn std::error::Error>> {
        let mut params = TokenParameters::default();
        
        // Common storage slot patterns:
        // Slot 0-10: Often contain tax values
        // Slot 0: owner (or first tax)
        // Slot 1-2: buy/sell taxes
        // Slot 3-4: max wallet/transaction
        
        for slot in 0..10 {
            let value = self.provider
                .get_storage_at(contract_address, H256::from_low_u64_be(slot), None)
                .await?;
            
            // Try to interpret as percentage (0-100)
            let as_u256 = U256::from_big_endian(&value.as_bytes());
            if as_u256 <= U256::from(100) {
                match slot {
                    1 => params.buy_tax = Some(as_u256.as_u32() as u8),
                    2 => params.sell_tax = Some(as_u256.as_u32() as u8),
                    _ => {}
                }
            }
        }
        
        Ok(params)
    }
    
    /// Call common view functions
    async fn call_view_functions(
        &self,
        contract_address: Address,
    ) -> Result<TokenParameters, Box<dyn std::error::Error>> {
        let mut params = TokenParameters::default();
        
        // Common function signatures
        let tax_getters = vec![
            ("0x2c32e512", "buyTax"),      // buyTax()
            ("0x6b67c4df", "sellTax"),     // sellTax()
            ("0x8da5cb5b", "owner"),       // owner()
            ("0xf8b45b05", "maxWallet"),   // maxWalletSize()
            ("0x8a8c523c", "tradingActive"), // tradingActive()
        ];
        
        for (selector, name) in tax_getters {
            let tx_request = TransactionRequest::new()
                .to(contract_address)
                .data(hex::decode(selector.trim_start_matches("0x"))?);
            
            let result = self.provider
                .call(
                    &TypedTransaction::Legacy(tx_request),
                    None,
                )
                .await;
                
            if let Ok(data) = result {
                match name {
                    "buyTax" => {
                        if data.len() >= 32 {
                            let tax = U256::from_big_endian(&data[0..32]);
                            if tax <= U256::from(100) {
                                params.buy_tax = Some(tax.as_u32() as u8);
                            }
                        }
                    }
                    "sellTax" => {
                        if data.len() >= 32 {
                            let tax = U256::from_big_endian(&data[0..32]);
                            if tax <= U256::from(100) {
                                params.sell_tax = Some(tax.as_u32() as u8);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        
        Ok(params)
    }
    
    /// Detect honeypot patterns
    async fn detect_honeypot_patterns(
        &self,
        params: &mut TokenParameters,
        contract_address: Address,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Pattern 1: High sell tax
        if let Some(sell_tax) = params.sell_tax {
            if sell_tax > 50 {
                params.detected_honeypot_flags.push("high_sell_tax".to_string());
            }
        }
        
        // Pattern 2: Asymmetric taxes
        if let (Some(buy), Some(sell)) = (params.buy_tax, params.sell_tax) {
            if sell > buy + 20 {
                params.detected_honeypot_flags.push("asymmetric_taxes".to_string());
            }
        }
        
        // Pattern 3: Check if taxes can actually be changed
        // Simulate calling setTaxes with different values
        
        Ok(())
    }
    
    /// Extract from live tax change transaction
    pub fn extract_from_tax_change(
        &self,
        input_data: &[u8],
    ) -> Option<(u8, u8)> {
        // This would use the tax_decoder logic
        if input_data.len() < 68 {
            return None;
        }
        
        let buy_tax = U256::from_big_endian(&input_data[4..36]);
        let sell_tax = U256::from_big_endian(&input_data[36..68]);
        
        if buy_tax <= U256::from(100) && sell_tax <= U256::from(100) {
            Some((buy_tax.as_u32() as u8, sell_tax.as_u32() as u8))
        } else {
            None
        }
    }
}

impl Default for TokenParameters {
    fn default() -> Self {
        Self {
            buy_tax: None,
            sell_tax: None,
            max_wallet: None,
            max_transaction: None,
            has_trading_enabled: false,
            owner: None,
            is_renounced: false,
            detected_honeypot_flags: Vec::new(),
        }
    }
}

impl TokenParameters {
    fn merge(&mut self, other: TokenParameters) {
        if other.buy_tax.is_some() { self.buy_tax = other.buy_tax; }
        if other.sell_tax.is_some() { self.sell_tax = other.sell_tax; }
        if other.max_wallet.is_some() { self.max_wallet = other.max_wallet; }
        if other.max_transaction.is_some() { self.max_transaction = other.max_transaction; }
        if other.has_trading_enabled { self.has_trading_enabled = true; }
        if other.owner.is_some() { self.owner = other.owner; }
        if other.is_renounced { self.is_renounced = true; }
        self.detected_honeypot_flags.extend(other.detected_honeypot_flags);
    }
}