use crate::core::PriceError;
/// Price Ticker Mapping System
///
/// **Algorithm**: Central mapping system that translates generic stablecoin pair names
/// to protocol-specific pair names that actually work with each DEX implementation.
/// Focus on high-volume protocols (Uniswap V2/V3, SushiSwap, Curve) that provide
/// consistent ETH pricing across major USD stablecoins.
///
/// **High-Volume Protocol Focus**:
/// - Uniswap V3: ~$3.0B daily volume (concentrated liquidity, multiple fee tiers)
/// - Uniswap V2: ~$2.1B daily volume (classic AMM)
/// - SushiSwap: ~$170M daily volume (community-driven)
/// - Curve Finance: ~$2.0B daily volume (stablecoin specialist)
///
/// **Stablecoin Mapping Strategy**:
/// - ETH/USDC → Maps to actual "ETH/USD" pools (USDC is primary USD proxy)
/// - ETH/USDT → Maps to "ETH/USDT" or "ETH/USDT_CURVE" pools
/// - ETH/DAI → Maps to "ETH/DAI" pools where available
/// - FRAX support removed due to limited protocol coverage
///
/// **Usage**:
/// ```rust,ignore
/// let mapper = PriceTickerMapper::new();
/// let uniswap_v2_pair = mapper.get_protocol_pair("ETH/USDC", Protocol::UniswapV2)?; // Returns "ETH/USD"
/// let uniswap_v3_pairs = mapper.get_protocol_pairs("ETH/USDC", Protocol::UniswapV3)?; // Returns ["ETH/USD_V3_500", "ETH/USD_V3_3000"]
/// ```
use std::collections::HashMap;

/// Supported high-volume protocols for price mapping
/// Focus on protocols with >$100M daily volume and consistent mainnet support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// Uniswap V2: ~$2.1B daily volume, classic constant product AMM
    UniswapV2,
    /// Uniswap V3: ~$3.0B daily volume, concentrated liquidity with fee tiers
    UniswapV3,
    /// SushiSwap: ~$170M daily volume, Uniswap V2 fork
    SushiSwap,
    /// Curve Finance: ~$2.0B daily volume, optimized for correlated assets
    Curve,
    /// Balancer V2: Multi-token weighted pools
    Balancer,
    /// Chainlink: Decentralized oracle network
    Chainlink,
}

/// Price ticker mapping system
pub struct PriceTickerMapper {
    /// Maps generic pairs to protocol-specific pairs
    /// Structure: generic_pair -> protocol -> Vec<protocol_specific_pairs>
    mappings: HashMap<String, HashMap<Protocol, Vec<String>>>,
}

impl PriceTickerMapper {
    /// Create new price ticker mapper with default mappings
    pub fn new() -> Self {
        let mut mapper = Self {
            mappings: HashMap::new(),
        };

        // Initialize default mappings
        mapper.init_default_mappings();
        mapper
    }

    /// Initialize default mappings for major stablecoin pairs
    /// Maps generic stablecoin pairs to actual protocol-specific pair names that work
    fn init_default_mappings(&mut self) {
        // ETH/USDC mappings - Primary stablecoin pair (USDC = USD proxy)
        // Maps to actual "ETH/USD" pools since protocols use USDC as USD reference
        let mut eth_usdc_map = HashMap::new();
        eth_usdc_map.insert(Protocol::UniswapV2, vec!["ETH/USD".to_string()]);
        eth_usdc_map.insert(
            Protocol::UniswapV3,
            vec![
                "ETH/USD_V3_500".to_string(),  // 0.05% fee tier - highest liquidity
                "ETH/USD_V3_3000".to_string(), // 0.3% fee tier - standard trading
            ],
        );
        eth_usdc_map.insert(Protocol::SushiSwap, vec!["ETH/USD".to_string()]);
        eth_usdc_map.insert(Protocol::Chainlink, vec!["ETH/USD".to_string()]);
        self.mappings.insert("ETH/USDC".to_string(), eth_usdc_map);

        // ETH/USDT mappings - Second largest stablecoin
        let mut eth_usdt_map = HashMap::new();
        eth_usdt_map.insert(Protocol::UniswapV3, vec!["ETH/USDT_V3".to_string()]);
        eth_usdt_map.insert(Protocol::SushiSwap, vec!["ETH/USDT".to_string()]);
        eth_usdt_map.insert(Protocol::Curve, vec!["ETH/USDT_CURVE".to_string()]);
        self.mappings.insert("ETH/USDT".to_string(), eth_usdt_map);

        // ETH/DAI mappings - Decentralized stablecoin
        let mut eth_dai_map = HashMap::new();
        eth_dai_map.insert(Protocol::SushiSwap, vec!["ETH/DAI".to_string()]);
        // Note: DAI/USDC available in Uniswap V3 as stablecoin pair
        self.mappings.insert("ETH/DAI".to_string(), eth_dai_map);

        // DAI/USDC stablecoin pair - Available in Curve 3Pool and Uniswap V3
        let mut dai_usdc_map = HashMap::new();
        dai_usdc_map.insert(Protocol::UniswapV3, vec!["DAI/USDC_V3".to_string()]);
        dai_usdc_map.insert(Protocol::Curve, vec!["USDC/DAI".to_string()]); // Curve 3Pool
        self.mappings.insert("DAI/USDC".to_string(), dai_usdc_map);

        // ETH/USD mapping.
        // Maps to USDC-based pools (same as ETH/USDC).
        let mut eth_usd_compat_map = HashMap::new();
        eth_usd_compat_map.insert(Protocol::UniswapV2, vec!["ETH/USD".to_string()]);
        eth_usd_compat_map.insert(
            Protocol::UniswapV3,
            vec!["ETH/USD_V3_500".to_string(), "ETH/USD_V3_3000".to_string()],
        );
        eth_usd_compat_map.insert(Protocol::SushiSwap, vec!["ETH/USD".to_string()]);
        eth_usd_compat_map.insert(Protocol::Chainlink, vec!["ETH/USD".to_string()]);
        self.mappings
            .insert("ETH/USD".to_string(), eth_usd_compat_map);
    }

    /// Get single protocol pair (for protocols that support only one pair per generic)
    pub fn get_protocol_pair(
        &self,
        generic_pair: &str,
        protocol: Protocol,
    ) -> Result<String, PriceError> {
        let pairs = self.get_protocol_pairs(generic_pair, protocol)?;
        if pairs.is_empty() {
            return Err(PriceError::SourceNotSupported(format!(
                "No pairs found for {} on {:?}",
                generic_pair, protocol
            )));
        }
        Ok(pairs[0].clone())
    }

    /// Get all protocol pairs (for protocols that support multiple pairs per generic)
    pub fn get_protocol_pairs(
        &self,
        generic_pair: &str,
        protocol: Protocol,
    ) -> Result<Vec<String>, PriceError> {
        let protocol_map = self.mappings.get(generic_pair).ok_or_else(|| {
            PriceError::SourceNotSupported(format!("Generic pair '{}' not supported", generic_pair))
        })?;

        let pairs = protocol_map.get(&protocol).ok_or_else(|| {
            PriceError::SourceNotSupported(format!(
                "Protocol {:?} doesn't support pair '{}'",
                protocol, generic_pair
            ))
        })?;

        Ok(pairs.clone())
    }

    /// Check if a generic pair is supported
    pub fn is_generic_pair_supported(&self, generic_pair: &str) -> bool {
        self.mappings.contains_key(generic_pair)
    }

    /// Check if a protocol supports a generic pair
    pub fn is_protocol_pair_supported(&self, generic_pair: &str, protocol: Protocol) -> bool {
        if let Some(protocol_map) = self.mappings.get(generic_pair) {
            protocol_map.contains_key(&protocol)
        } else {
            false
        }
    }

    /// Get all supported generic pairs
    pub fn get_supported_generic_pairs(&self) -> Vec<String> {
        self.mappings.keys().cloned().collect()
    }

    /// Get all protocols that support a generic pair
    pub fn get_supporting_protocols(
        &self,
        generic_pair: &str,
    ) -> Result<Vec<Protocol>, PriceError> {
        let protocol_map = self.mappings.get(generic_pair).ok_or_else(|| {
            PriceError::SourceNotSupported(format!("Generic pair '{}' not supported", generic_pair))
        })?;

        Ok(protocol_map.keys().cloned().collect())
    }

    /// Add custom mapping for a generic pair and protocol
    pub fn add_mapping(
        &mut self,
        generic_pair: &str,
        protocol: Protocol,
        protocol_pairs: Vec<String>,
    ) {
        let protocol_map = self.mappings.entry(generic_pair.to_string()).or_default();
        protocol_map.insert(protocol, protocol_pairs);
    }

    /// Remove mapping for a generic pair and protocol
    pub fn remove_mapping(
        &mut self,
        generic_pair: &str,
        protocol: Protocol,
    ) -> Result<(), PriceError> {
        let protocol_map = self.mappings.get_mut(generic_pair).ok_or_else(|| {
            PriceError::SourceNotSupported(format!("Generic pair '{}' not found", generic_pair))
        })?;

        protocol_map.remove(&protocol);

        // If no protocols left for this generic pair, remove the entire entry
        if protocol_map.is_empty() {
            self.mappings.remove(generic_pair);
        }

        Ok(())
    }
}

impl Default for PriceTickerMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_stablecoin_mapping() {
        let mapper = PriceTickerMapper::new();

        // Test ETH/USDC mapping (primary stablecoin)
        let uniswap_v2_pair = mapper
            .get_protocol_pair("ETH/USDC", Protocol::UniswapV2)
            .unwrap();
        assert_eq!(uniswap_v2_pair, "ETH/USD"); // Maps to actual pool name

        let chainlink_pair = mapper
            .get_protocol_pair("ETH/USDC", Protocol::Chainlink)
            .unwrap();
        assert_eq!(chainlink_pair, "ETH/USD");

        // Test ETH/USDT mapping
        let curve_pair = mapper
            .get_protocol_pair("ETH/USDT", Protocol::Curve)
            .unwrap();
        assert_eq!(curve_pair, "ETH/USDT_CURVE");

        let sushi_usdt_pair = mapper
            .get_protocol_pair("ETH/USDT", Protocol::SushiSwap)
            .unwrap();
        assert_eq!(sushi_usdt_pair, "ETH/USDT");
    }

    #[test]
    fn test_multiple_fee_tiers() {
        let mapper = PriceTickerMapper::new();

        // Test UniswapV3 multiple fee tiers for ETH/USDC
        let v3_pairs = mapper
            .get_protocol_pairs("ETH/USDC", Protocol::UniswapV3)
            .unwrap();
        assert_eq!(v3_pairs.len(), 2);
        assert!(v3_pairs.contains(&"ETH/USD_V3_500".to_string()));
        assert!(v3_pairs.contains(&"ETH/USD_V3_3000".to_string()));

        // Test backward compatibility with ETH/USD
        let v3_eth_usd = mapper
            .get_protocol_pairs("ETH/USD", Protocol::UniswapV3)
            .unwrap();
        assert_eq!(v3_eth_usd.len(), 2);
        assert!(v3_eth_usd.contains(&"ETH/USD_V3_500".to_string()));
    }

    #[test]
    fn test_unsupported_pair() {
        let mapper = PriceTickerMapper::new();

        // Test non-existent pair
        let result = mapper.get_protocol_pair("NONEXISTENT/PAIR", Protocol::UniswapV2);
        assert!(result.is_err());

        // Test removed FRAX support
        let frax_result = mapper.get_protocol_pair("ETH/FRAX", Protocol::UniswapV2);
        assert!(frax_result.is_err());
    }

    #[test]
    fn test_unsupported_protocol() {
        let mapper = PriceTickerMapper::new();

        // Try to get Curve for ETH/DAI (not supported - only SushiSwap has this pair)
        let result = mapper.get_protocol_pair("ETH/DAI", Protocol::Curve);
        assert!(result.is_err());

        // Try to get UniswapV2 for ETH/USDT (not supported - V2 only has USDC)
        let result2 = mapper.get_protocol_pair("ETH/USDT", Protocol::UniswapV2);
        assert!(result2.is_err());
    }

    #[test]
    fn test_supported_stablecoin_pairs() {
        let mapper = PriceTickerMapper::new();

        let supported = mapper.get_supported_generic_pairs();
        // Test major stablecoin pairs
        assert!(supported.contains(&"ETH/USDC".to_string()));
        assert!(supported.contains(&"ETH/USDT".to_string()));
        assert!(supported.contains(&"ETH/DAI".to_string()));
        assert!(supported.contains(&"DAI/USDC".to_string()));
        assert!(supported.contains(&"ETH/USD".to_string())); // Compatibility support

        // Test removed pairs
        assert!(!supported.contains(&"ETH/FRAX".to_string()));
        assert!(!supported.contains(&"BTC/USD".to_string()));
    }

    #[test]
    fn test_custom_mapping() {
        let mut mapper = PriceTickerMapper::new();

        // Add custom mapping for new stablecoin
        mapper.add_mapping(
            "ETH/LUSD",
            Protocol::UniswapV3,
            vec!["ETH/LUSD_V3_3000".to_string()],
        );
        mapper.add_mapping(
            "ETH/LUSD",
            Protocol::SushiSwap,
            vec!["ETH/LUSD".to_string()],
        );

        let v3_lusd = mapper
            .get_protocol_pair("ETH/LUSD", Protocol::UniswapV3)
            .unwrap();
        assert_eq!(v3_lusd, "ETH/LUSD_V3_3000");

        let supporting_protocols = mapper.get_supporting_protocols("ETH/LUSD").unwrap();
        assert!(supporting_protocols.contains(&Protocol::UniswapV3));
        assert!(supporting_protocols.contains(&Protocol::SushiSwap));

        // Test protocol coverage for ETH/USDC
        let usdc_protocols = mapper.get_supporting_protocols("ETH/USDC").unwrap();
        assert!(usdc_protocols.contains(&Protocol::UniswapV2));
        assert!(usdc_protocols.contains(&Protocol::UniswapV3));
        assert!(usdc_protocols.contains(&Protocol::SushiSwap));
        assert!(usdc_protocols.contains(&Protocol::Chainlink));
    }
}
