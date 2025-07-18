//! Routing rules for graph exploration
//! Decides which addresses to expand and which to stop at

use crate::graph_discovery::db_queries::AddressInfoTyped;

pub struct RoutingRules;

impl RoutingRules {
    /// Should we expand exploration from this address?
    pub fn should_expand(info: &AddressInfoTyped) -> bool {
        match info.entity_type.as_deref() {
            // Dead ends - don't expand
            Some("CEX") => false,              // Centralized exchanges are endpoints
            Some("CEX_DEPOSIT") => false,      // CEX deposit addresses
            Some("BURN") => false,             // Burn addresses
            Some("NULL") => false,             // Null address
            Some("BLACKHOLE") => false,        // Black hole addresses
            
            // Limited expansion - these are important but complex
            Some("DEX_ROUTER") => true,        // Need to trace through routers
            Some("DEX_AGGREGATOR") => true,    // Aggregators hide real traders
            Some("BRIDGE") => true,            // Bridges connect to other chains
            
            // High priority expansion
            Some("WHALE") => true,             // Whales are important to track
            Some("PROTOCOL_TREASURY") => true, // Protocol treasuries
            Some("LENDING_POOL") => true,      // Lending protocols
            
            // MEV related - very important
            Some("MEV_BOT") => true,           // MEV bots
            Some("FLASHLOAN_PROVIDER") => true,// Flash loan providers
            
            // Default behavior
            _ => {
                if !info.is_contract {
                    // EOA (Externally Owned Account) - generally worth exploring
                    true
                } else if info.entity_type.is_none() {
                    // Unknown contract - worth investigating
                    true
                } else {
                    // Known contract type not in above list
                    true
                }
            }
        }
    }
    
    /// Should we mark this transaction for deep analysis in Layer 2?
    /// This is more selective than expansion
    pub fn should_analyze_deeply(
        from_info: &AddressInfoTyped,
        to_info: &AddressInfoTyped,
        value: &alloy_primitives::U256,
    ) -> bool {
        use alloy_primitives::U256;
        
        // Always analyze high-value transactions (> 10 ETH)
        let ten_eth = U256::from(10) * U256::from(10).pow(U256::from(18));
        if value > &ten_eth {
            return true;
        }
        
        // Always analyze if either party is unknown
        if from_info.entity_type.is_none() || to_info.entity_type.is_none() {
            return true;
        }
        
        // Router transactions need deep analysis to find actual fund flows
        if Self::is_router(from_info) || Self::is_router(to_info) {
            return true;
        }
        
        // MEV transactions are always interesting
        if Self::is_mev_related(from_info) || Self::is_mev_related(to_info) {
            return true;
        }
        
        // Transactions between CEX and unknown addresses
        if (Self::is_cex(from_info) && to_info.entity_type.is_none()) ||
           (from_info.entity_type.is_none() && Self::is_cex(to_info)) {
            return true;
        }
        
        // Medium value (> 1 ETH) involving protocols
        let one_eth = U256::from(10).pow(U256::from(18));
        if value > &one_eth && (Self::is_protocol(from_info) || Self::is_protocol(to_info)) {
            return true;
        }
        
        false
    }
    
    /// Check if address is a router/aggregator that needs deep analysis
    pub fn is_router(info: &AddressInfoTyped) -> bool {
        matches!(
            info.entity_type.as_deref(),
            Some("DEX_ROUTER") | Some("DEX_AGGREGATOR") | Some("1INCH_ROUTER")
        )
    }
    
    /// Check if address is MEV related
    fn is_mev_related(info: &AddressInfoTyped) -> bool {
        matches!(
            info.entity_type.as_deref(),
            Some("MEV_BOT") | Some("FLASHLOAN_PROVIDER") | Some("SANDWICH_BOT")
        )
    }
    
    /// Check if address is a CEX
    fn is_cex(info: &AddressInfoTyped) -> bool {
        matches!(
            info.entity_type.as_deref(),
            Some("CEX") | Some("CEX_DEPOSIT") | Some("CEX_HOT_WALLET")
        )
    }
    
    /// Check if address is a protocol
    fn is_protocol(info: &AddressInfoTyped) -> bool {
        info.entity_type
            .as_ref()
            .map(|t| t.contains("PROTOCOL") || t.contains("POOL") || t.contains("VAULT"))
            .unwrap_or(false)
    }
}