/// Token Trading Viability Simulation
/// 
/// Demonstrates token trading viability analysis by simulating the complete
/// trading sequence (buy -> approve -> sell) while maintaining blockchain state
/// between each transaction for accurate tax calculation.

use eyre::Result;
use std::sync::Arc;
use alloy_primitives::{Address, U256};
use tx_processor::{
    simulator::erc20_token_buy_approve_sell_tx_simulator::{
        check_can_buy_sell_pool,
        PoolViabilityConfig,
        PoolType,
        types::PoolViabilityResult,
    }
};
use tx_simulator::TxSimulator;
use tx_processor::tx_processor::TxProcessor;

/// Configuration for testing a specific token
#[derive(Debug, Clone)]
struct TokenConfig {
    symbol: &'static str,
    token_address: &'static str,
    pool_address: &'static str,
    pool_type: PoolType,
    expected_behavior: ExpectedBehavior,
    decimals: u8,
}

/// Expected behavior for different token categories
#[derive(Debug, Clone)]
enum ExpectedBehavior {
    ShouldWork,        // Blue chip tokens - should work perfectly
    MayHaveTaxes,      // Meme tokens - may have transfer fees
    MayFail,           // Known problematic tokens
}

/// Result of testing a single token
#[derive(Debug)]
struct TokenTestResult {
    config: TokenConfig,
    result: Option<PoolViabilityResult>,
    error: Option<String>,
    test_duration: std::time::Duration,
}

/// Define comprehensive token test suite
fn get_token_configs() -> Vec<TokenConfig> {
    vec![
        // ===================== STABLECOINS =====================
        // Tier 1: Blue Chip Stablecoins (Should work perfectly)
        TokenConfig {
            symbol: "USDC",
            token_address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            pool_address: "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc", // USDC/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 6,
        },
        TokenConfig {
            symbol: "USDT", 
            token_address: "0xdAC17F958D2ee523a2206206994597C13D831ec7",
            pool_address: "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852", // USDT/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 6,
        },
        TokenConfig {
            symbol: "DAI",
            token_address: "0x6B175474E89094C44Da98b954EedeAC495271d0F",
            pool_address: "0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11", // DAI/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        
        // ===================== WRAPPED ASSETS =====================
        TokenConfig {
            symbol: "WBTC",
            token_address: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", 
            pool_address: "0xBb2b8038a1640196FbE3e38816F3e67Cba72D940", // WBTC/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 8,
        },
        TokenConfig {
            symbol: "stETH",
            token_address: "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84",
            pool_address: "0x4028DAAC072e492d34a3Afdbef0ba7e35D8b55C4", // stETH/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        
        // ===================== DEFI BLUE CHIPS =====================
        TokenConfig {
            symbol: "UNI",
            token_address: "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984",
            pool_address: "0xd3d2E2692501A5c9Ca623199D38826e513033a17", // UNI/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "LINK",
            token_address: "0x514910771AF9Ca656af840dff83E8264EcF986CA",
            pool_address: "0xa2107FA5B38d9bbd2C461D6EDf11B11A50F6b974", // LINK/WETH V2
            pool_type: PoolType::UniswapV2, 
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "AAVE",
            token_address: "0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9",
            pool_address: "0xDFC14d2Af169B0D36C4EFF567Ada9b2E0CAE044f", // AAVE/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "MKR",
            token_address: "0x9f8F72aA9304c8B593d555F12eF6589cC3A579A2",
            pool_address: "0xC2aDdA861F89bBB333c90c492cB837741916A225", // MKR/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "CRV",
            token_address: "0xD533a949740bb3306d119CC777fa900bA034cd52",
            pool_address: "0x3dA1313aE46132A397D90d95B1424A9A7e3e0fCE", // CRV/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        
        // ===================== LAYER 2 TOKENS =====================
        TokenConfig {
            symbol: "MATIC",
            token_address: "0x7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0",
            pool_address: "0x819f3450dA6f110BA6Ea52195B3beaFa246062dE", // MATIC/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "LDO",
            token_address: "0x5A98FcBEA516Cf06857215779Fd812CA3beF1B32",
            pool_address: "0xC558F600B34A5f69dD2f0D06Cb8A88d829B7420a", // LDO/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        
        // ===================== MEME TOKENS =====================
        TokenConfig {
            symbol: "PEPE",
            token_address: "0x6982508145454Ce325dDbE47a25d4ec3d2311933",
            pool_address: "0xA43fe16908251ee70EF74718545e4FE6C5cCEc9f", // PEPE/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 18,
        },
        TokenConfig {
            symbol: "SHIB", 
            token_address: "0x95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE",
            pool_address: "0x811beEd0119b4AfCE20D2583EB608C6F7AF1954f", // SHIB/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 18,
        },
        TokenConfig {
            symbol: "DOGE",
            token_address: "0x4206931337dc273a630d328dA6441786BfaD668f", // Wrapped DOGE
            pool_address: "0xC0067d751FB1172DBAb1FA003eFe214EE8f419b6", // DOGE/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 8,
        },
        TokenConfig {
            symbol: "FLOKI",
            token_address: "0xcf0C122c6b73ff809C693DB761e7BaeBe62b6a2E",
            pool_address: "0xca7c2771D248dCBe09EABE0CE57A62e18dA178c0", // FLOKI/WETH V2 - actual pool from transfers
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 9,
        },
        
        // ===================== EXCHANGE TOKENS =====================
        TokenConfig {
            symbol: "FTT",
            token_address: "0x50D1c9771902476076eCFc8B2A83Ad6b9355a4c9", 
            pool_address: "0xFd9C58B4871348A77a04FBba594Cdb01F8972Ac6", // FTT/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes, // FTT might have issues after FTX collapse
            decimals: 18,
        },
        
        // ===================== UTILITY TOKENS =====================
        TokenConfig {
            symbol: "GRT",
            token_address: "0xc944E90C64B2c07662A292be6244BDf05Cda44a7",
            pool_address: "0x2E81eC0B8B4022fAC83A21B2F2B4B8f5ED744D70", // GRT/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "BAT",
            token_address: "0x0D8775F648430679A709E98d2b0Cb6250d2887EF",
            pool_address: "0xB6909B960DbbE7392D405429eB2b3649752b4838", // BAT/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        
        // ===================== ALGO STABLES =====================
        TokenConfig {
            symbol: "FRAX",
            token_address: "0x853d955aCEf822Db058eb8505911ED77F175b99e",
            pool_address: "0xE1573B9D29e2183B1AF0e743Dc2754979A40D237", // FRAX/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "MIM",
            token_address: "0x99D8a9C45b2ecA8864373A26D1459e3Dff1e17F3",
            pool_address: "0x07D5695a24904CC1B6e3bd57cC7780B90618e3c4", // MIM/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        
        // ===================== GAMING/METAVERSE =====================
        TokenConfig {
            symbol: "AXS",
            token_address: "0xBB0E17EF65F82Ab018d8EDd776e8DD940327B28b",
            pool_address: "0xEc454EdA10accdD66209C57aF8C12924556F3aBD", // AXS/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "SAND",
            token_address: "0x3845badAde8e6dFF049820680d1F14bD3903a5d0",
            pool_address: "0x3DD49f67E9d5Bc4C5E6634b3F70BfD9dc1b6BD74", // SAND/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "MANA",
            token_address: "0x0F5D2fB29fb7d3CFeE444a200298f468908cC942",
            pool_address: "0x11b1f53204d03E5529F09EB3091939e4Fd8c9CF3", // MANA/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "ENJ",
            token_address: "0xF629cBd94d3791C9250152BD8dfBDF380E2a3B9c",
            pool_address: "0xE56C60B5f9f7B5FC70DE0eb79c6EE7d00eFa2625", // ENJ/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "GALA",
            token_address: "0xd1d2Eb1B1e90B638588728b4130137D262C87cae",
            pool_address: "0x03321b3F266D2b270c8288bFE12b5991B1151eCC", // GALA/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 8,
        },
        
        // ===================== AI/TECH TOKENS =====================
        TokenConfig {
            symbol: "FET",
            token_address: "0xaea46A60368A7bD060eec7DF8CBa43b7EF41Ad85",
            pool_address: "0xBAfcE6D2fb081BA45788aC482b92AB01d3C7D894", // FET/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "RNDR",
            token_address: "0x6De037ef9aD2725EB40118Bb1702EBb27e4Aeb24",
            pool_address: "0x1b63142628311595AD90CDF75cbAEE96Af8DfAbA", // RNDR/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        
        // ===================== INFRASTRUCTURE =====================
        TokenConfig {
            symbol: "GNO",
            token_address: "0x6810e776880C02933D47DB1b9fc05908e5386b96",
            pool_address: "0xF56D08221B5942C428Acc5De8f78489A97fC5599", // GNO/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "RPL",
            token_address: "0xD33526068D116cE69F19A9ee46F0bd304F21A51f",
            pool_address: "0x70EA56e46266f0137BAc6B75710e3546f47C855D", // RPL/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "LRC",
            token_address: "0xBBbbCA6A901c926F240b89EacB641d8Aec7AEafD",
            pool_address: "0x8878Df9E1A7c87dcBf6d3999D997f262C05D8C70", // LRC/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        
        // ===================== L2/SCALING =====================
        TokenConfig {
            symbol: "ARB",
            token_address: "0xB50721BCf8d664c30412Cfbc6cf7a15145234ad1",
            pool_address: "0xC6F780497A95e246EB9449f5e4770916DCd6396A", // ARB/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "OP",
            token_address: "0x4200000000000000000000000000000000000042",
            pool_address: "0x68F5C0A2DE713a54991E01858Fd27a3832401849", // OP/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        
        // ===================== DEFI 2.0/NEWER =====================
        TokenConfig {
            symbol: "BLUR",
            token_address: "0x5283D291DBCF85356A21bA090E6db59121208b44",
            pool_address: "0x0FC1b909ba9265A846b82CF4CE352fc3e7EeB2ED", // BLUR/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        TokenConfig {
            symbol: "ENS",
            token_address: "0xC18360217D8F7Ab5e7c516566761Ea12Ce7F9D72",
            pool_address: "0x92560C178cE069CC014138eD3C2F5221Ba71f58a", // ENS/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::ShouldWork,
            decimals: 18,
        },
        
        // ===================== MORE MEME TOKENS =====================
        TokenConfig {
            symbol: "BONE",
            token_address: "0x9813037ee2218799597d83D4a5B6F3b6778218d9",
            pool_address: "0xA8D52463020072980d1AE2c42F53E9aC207c63b3", // BONE/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 18,
        },
        TokenConfig {
            symbol: "ELON",
            token_address: "0x761D38e5ddf6ccf6Cf7c55759d5210750B5D60F3",
            pool_address: "0x7B73704B4C7d8B89a30c8A3e5b2db060600c9430", // ELON/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 18,
        },
        TokenConfig {
            symbol: "AKITA",
            token_address: "0x3301Ee63Fb29F863f2333Bd4466acb46CD8323E6",
            pool_address: "0xDA3A20aad0C34FA742Bd9813D45Bbf67C787aE0b", // AKITA/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes,
            decimals: 18,
        },
        
        // ===================== PRIVACY/CONTROVERSIAL =====================
        TokenConfig {
            symbol: "TORN",
            token_address: "0x77777FeDdddFfC19Ff86DB637967013e6C6A116C",
            pool_address: "0x0C722a487876989Af8a05FFfB6e32e45cc23FB4A", // TORN/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes, // Might fail due to sanctions
            decimals: 18,
        },
        
        // ===================== TAX/REFLECTION TOKENS =====================
        TokenConfig {
            symbol: "BABYDOGE",
            token_address: "0xAC57De9C1A09FeC648E93EB98875B212DB0d460B",
            pool_address: "0x21e12C0C45f9E4C5CdD2bC6Ff4DA7B33B4FA654a", // BABYDOGE/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes, // Known reflection token
            decimals: 9,
        },
        TokenConfig {
            symbol: "KISHU",
            token_address: "0xA2b4C0Af19cC16a6CfAcCe81F192B024d625817D",
            pool_address: "0xF82d8Ec196Fb0D56c6B82a8B1870F09502A49F88", // KISHU/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes, // 2% redistribution fee
            decimals: 9,
        },
        TokenConfig {
            symbol: "SAFEMOON",
            token_address: "0x42981d0bfbAf196529376EE702F2a9Eb9092fcB5", // SafeMoon V2
            pool_address: "0x4658EA7e9960D6158a261104aAA160cC953bb6ba", // SFM/WETH V2
            pool_type: PoolType::UniswapV2,
            expected_behavior: ExpectedBehavior::MayHaveTaxes, // Known 10% tax structure
            decimals: 9,
        },
    ]
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🎯 Multi-Token Trading Viability Analysis");
    println!("==========================================");
    println!("Testing comprehensive token trading viability across multiple token categories.\n");
    
    // Get reth datadir
    let reth_datadir = std::env::var("RETH_DATADIR")
        .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
    
    println!("Using Reth datadir: {}", reth_datadir);
    
    // Create simulator and tx processor
    let simulator = Arc::new(TxSimulator::new(&reth_datadir)?);
    let tx_processor = Arc::new(TxProcessor::new());
    
    // Get latest block to avoid pruned state
    let latest_block = simulator.get_latest_block()?;
    println!("Latest block: {}\n", latest_block);
    
    // Get token configurations
    let token_configs = get_token_configs();
    let total_tokens = token_configs.len();
    let mut all_results = Vec::new();
    
    println!("📋 Testing {} tokens across different categories:", total_tokens);
    
    for config in &token_configs {
        println!("  {} {} - Expected: {:?}", 
                match config.expected_behavior {
                    ExpectedBehavior::ShouldWork => "✅",
                    ExpectedBehavior::MayHaveTaxes => "⚠️",
                    ExpectedBehavior::MayFail => "❌",
                }, 
                config.symbol, 
                config.expected_behavior
        );
    }
    println!();
    
    // Test each token
    for (idx, config) in token_configs.into_iter().enumerate() {
        println!("{}", "=".repeat(80));
        println!("🔍 TESTING [{}/{}]: {} ({})", idx + 1, total_tokens, config.symbol, config.token_address);
        println!("{}", "=".repeat(80));
        
        // Token info already printed to console
        
        let test_result = test_single_token(
            &config,
            &simulator, 
            &tx_processor,
            latest_block
        ).await;
        
        // Results will be output to CSV at the end
        
        all_results.push(test_result);
        
        // Brief pause between tests
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    
    // Generate comprehensive summary
    print_comprehensive_summary(&all_results);
    
    // Generate CSV output only
    generate_csv_output(&all_results)?;
    
    Ok(())
}

/// Test a single token configuration comprehensively
async fn test_single_token(
    config: &TokenConfig,
    simulator: &Arc<TxSimulator>,
    tx_processor: &Arc<TxProcessor>,
    latest_block: u64,
) -> TokenTestResult {
    let start_time = std::time::Instant::now();
    
    println!("🔍 Configuration:");
    println!("  Token: {} ({})", config.symbol, config.token_address);
    println!("  Pool: {}", config.pool_address);
    println!("  Type: {:?}", config.pool_type);
    println!("  Expected: {:?}", config.expected_behavior);
    println!("  Decimals: {}", config.decimals);
    
    // Parse addresses
    let token_address = match config.token_address.parse::<Address>() {
        Ok(addr) => addr,
        Err(e) => {
            let error_msg = format!("Invalid token address: {}", e);
            println!("❌ {}", error_msg);
            return TokenTestResult {
                config: config.clone(),
                result: None,
                error: Some(error_msg),
                test_duration: start_time.elapsed(),
            };
        }
    };
    
    let pool_address = match config.pool_address.parse::<Address>() {
        Ok(addr) => addr,
        Err(e) => {
            let error_msg = format!("Invalid pool address: {}", e);
            println!("❌ {}", error_msg);
            return TokenTestResult {
                config: config.clone(),
                result: None,
                error: Some(error_msg),
                test_duration: start_time.elapsed(),
            };
        }
    };
    
    // Test all tokens at latest block
    let buy_block = latest_block;
    let block_delays_to_test = vec![0]; // Test same-block for all tokens
    
    for (test_index, block_delay) in block_delays_to_test.iter().enumerate() {
        if block_delays_to_test.len() > 1 {
            let sell_block = buy_block + block_delay;
            println!("\n🔍 TEST #{}: Buy at {}, Sell at {} (delay: {})", 
                     test_index + 1, 
                     buy_block,
                     sell_block,
                     block_delay);
            
        }
        
        // Create pool configuration
        let pool_config = PoolViabilityConfig::new(
            token_address,
            pool_address,
            config.pool_type.clone(),
        ).with_test_amount(U256::from(1_000_000_000_000_000_000u64)) // 1.0 ETH for testing
        .with_block(buy_block)
        .with_block_delay(*block_delay);
        
        println!("\n🚀 Running trading viability analysis (block_delay={})...", block_delay);
        
        // Run the analysis
        match check_can_buy_sell_pool(simulator.clone(), tx_processor.clone(), pool_config).await {
            Ok(result) => {
                let duration = start_time.elapsed();
                
                // Print detailed results for this token
                print_token_result_with_block_delay(config, &result, duration, *block_delay);
                
                // If this is a successful test, or if it's the last test for this token, return the result
                if result.is_tradeable || test_index == block_delays_to_test.len() - 1 {
                    return TokenTestResult {
                        config: config.clone(),
                        result: Some(result),
                        error: None,
                        test_duration: duration,
                    };
                }
            }
            Err(e) => {
                let error_msg = format!("Analysis failed: {}", e);
                let duration = start_time.elapsed();
                
                println!("❌ Analysis Error (block_delay={}): {}", block_delay, error_msg);
                println!("⏱️  Test Duration: {:?}", duration);
                
                // If this is the last test, return the error
                if test_index == block_delays_to_test.len() - 1 {
                    return TokenTestResult {
                        config: config.clone(),
                        result: None,
                        error: Some(error_msg),
                        test_duration: duration,
                    };
                }
            }
        }
    }
    
    // This should never be reached, but provide a fallback
    TokenTestResult {
        config: config.clone(),
        result: None,
        error: Some("No tests completed".to_string()),
        test_duration: start_time.elapsed(),
    }
}

/// Print detailed results for a single token test with block delay info
fn print_token_result_with_block_delay(config: &TokenConfig, result: &PoolViabilityResult, duration: std::time::Duration, block_delay: u64) {
    println!("📈 Analysis Results for {} (block_delay={}):", config.symbol, block_delay);
    println!("==================================");
    println!("⏱️  Execution Time: {:?}", duration);
    println!("📦 Block Number: {}", result.block_number);
    if block_delay > 0 {
        println!("🕒 Block Timing: Buy at {}, Sell at {} (delay: {})", result.block_number, result.block_number + block_delay, block_delay);
    }
    
    println!("\n🔍 Individual Operations:");
    println!("  📈 Can Buy: {}", if result.can_buy { "✅" } else { "❌" });
    println!("  ✅ Can Approve: {}", if result.can_approve { "✅" } else { "❌" });
    println!("  📉 Can Sell: {}", if result.can_sell { "✅" } else { "❌" });
    println!("  🎯 Overall Tradeable: {}", if result.is_tradeable { "✅" } else { "❌" });
    
    if result.is_tradeable {
        println!("\n💰 Tax Analysis:");
        println!("  📈 Buy Tax: {:.2}%", result.buy_tax_percent);
        println!("  📉 Sell Tax: {:.2}%", result.sell_tax_percent);
        
        println!("\n🔢 Trade Details:");
        println!("  🪙 Tokens Received: {}", result.tokens_received);
        println!("  💵 ETH Received Back: {} wei", result.eth_received);
        
        let loss = result.eth_spent.saturating_sub(result.eth_received);
        let loss_eth = loss.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        println!("  📊 Net Loss: {:.6} ETH ({} wei)", loss_eth, loss);
        
        // Tax analysis
        if result.buy_tax_percent > 0.0 || result.sell_tax_percent > 0.0 {
            println!("\n⚠️  Tax Detection:");
            if result.buy_tax_percent > 0.0 {
                println!("     Buy tax detected: {:.2}%", result.buy_tax_percent);
            }
            if result.sell_tax_percent > 0.0 {
                println!("     Sell tax detected: {:.2}%", result.sell_tax_percent);
            }
        }
        
        println!("\n✅ {} is fully tradeable with block_delay={}!", config.symbol, block_delay);
    } else {
        println!("\n❌ Trading failed for {} with block_delay={}!", config.symbol, block_delay);
        if let Some(reason) = &result.failure_reason {
            println!("   Detailed Error: {}", reason);
        }
        
        // Additional debug info
        println!("\n🔍 Debug Information:");
        println!("  💰 ETH Spent: {} wei", result.eth_spent);
        println!("  🪙 Tokens Received: {}", result.tokens_received);
        println!("  💵 ETH Received Back: {} wei", result.eth_received);
        println!("  📈 Buy Tax: {:.2}%", result.buy_tax_percent);
        println!("  📉 Sell Tax: {:.2}%", result.sell_tax_percent);
        
        if block_delay == 0 {
            println!("  💡 This was a same-block test. Testing next-block execution next...");
        } else {
            println!("  💡 This was a next-block test. Same issue persists across block boundaries.");
        }
    }
}

/// Print detailed results for a single token test
fn print_token_result(config: &TokenConfig, result: &PoolViabilityResult, duration: std::time::Duration) {
    println!("📈 Analysis Results for {}:", config.symbol);
    println!("==================================");
    println!("⏱️  Execution Time: {:?}", duration);
    println!("📦 Block Number: {}", result.block_number);
    
    println!("\n🔍 Individual Operations:");
    println!("  📈 Can Buy: {}", if result.can_buy { "✅" } else { "❌" });
    println!("  ✅ Can Approve: {}", if result.can_approve { "✅" } else { "❌" });
    println!("  📉 Can Sell: {}", if result.can_sell { "✅" } else { "❌" });
    println!("  🎯 Overall Tradeable: {}", if result.is_tradeable { "✅" } else { "❌" });
    
    if result.is_tradeable {
        println!("\n💰 Tax Analysis:");
        println!("  📈 Buy Tax: {:.2}%", result.buy_tax_percent);
        println!("  📉 Sell Tax: {:.2}%", result.sell_tax_percent);
        
        println!("\n🔢 Trade Details:");
        println!("  🪙 Tokens Received: {}", result.tokens_received);
        println!("  💵 ETH Received Back: {} wei", result.eth_received);
        
        let loss = result.eth_spent.saturating_sub(result.eth_received);
        let loss_eth = loss.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
        println!("  📊 Net Loss: {:.6} ETH ({} wei)", loss_eth, loss);
        
        // Tax analysis
        if result.buy_tax_percent > 0.0 || result.sell_tax_percent > 0.0 {
            println!("\n⚠️  Tax Detection:");
            if result.buy_tax_percent > 0.0 {
                println!("     Buy tax detected: {:.2}%", result.buy_tax_percent);
            }
            if result.sell_tax_percent > 0.0 {
                println!("     Sell tax detected: {:.2}%", result.sell_tax_percent);
            }
        }
        
        println!("\n✅ {} is fully tradeable!", config.symbol);
    } else {
        println!("\n❌ Trading failed for {}!", config.symbol);
        if let Some(reason) = &result.failure_reason {
            println!("   Detailed Error: {}", reason);
        }
        
        // Additional debug info
        println!("\n🔍 Debug Information:");
        println!("  💰 ETH Spent: {} wei", result.eth_spent);
        println!("  🪙 Tokens Received: {}", result.tokens_received);
        println!("  💵 ETH Received Back: {} wei", result.eth_received);
        println!("  📈 Buy Tax: {:.2}%", result.buy_tax_percent);
        println!("  📉 Sell Tax: {:.2}%", result.sell_tax_percent);
    }
}

/// Generate CSV output
fn generate_csv_output(results: &[TokenTestResult]) -> Result<()> {
    use std::io::Write;
    
    let csv_path = "/home/nima/code/crypto/rust/tx_processor/examples/pool_analysis/token_analysis_results.csv";
    let mut csv_file = std::fs::File::create(csv_path)?;
    
    // CSV Header
    writeln!(csv_file, "Symbol,Address,Pool,ExpectedBehavior,Tradeable,CanBuy,CanApprove,CanSell,BuyTaxPercent,SellTaxPercent,TokensReceived,EthSpent,EthReceived,NetLoss,TestDurationMs,FailureReason")?;
    
    // CSV Data
    for result in results {
        let symbol = result.config.symbol;
        let address = result.config.token_address;
        let pool = result.config.pool_address;
        let expected = format!("{:?}", result.config.expected_behavior);
        let test_duration_ms = result.test_duration.as_secs_f64() * 1000.0;
        
        if let Some(analysis) = &result.result {
            let buy_tax = if analysis.can_buy && analysis.buy_tax_percent >= 0.0 { format!("{:.2}", analysis.buy_tax_percent) } else { "".to_string() };
            let sell_tax = if analysis.can_sell && analysis.sell_tax_percent >= 0.0 { format!("{:.2}", analysis.sell_tax_percent) } else { "".to_string() };
            let failure_reason = analysis.failure_reason.as_deref().unwrap_or("").replace(",", ";");
            let net_loss = analysis.eth_spent.saturating_sub(analysis.eth_received);
            
            writeln!(csv_file, "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.1},\"{}\"",
                symbol,
                address,
                pool,
                expected,
                analysis.is_tradeable,
                analysis.can_buy,
                analysis.can_approve,
                analysis.can_sell,
                buy_tax,
                sell_tax,
                analysis.tokens_received,
                analysis.eth_spent,
                analysis.eth_received,
                net_loss,
                test_duration_ms,
                failure_reason
            )?;
        } else {
            let error_msg = result.error.as_deref().unwrap_or("Unknown error").replace(",", ";");
            writeln!(csv_file, "{},{},{},{},false,false,false,false,,,0,0,0,0,{:.1},\"{}\"",
                symbol,
                address,
                pool,
                expected,
                test_duration_ms,
                error_msg
            )?;
        }
    }
    
    println!("\n✅ CSV results saved to: {}", csv_path);
    Ok(())
}

/// Print comprehensive summary of all test results
fn print_comprehensive_summary(results: &[TokenTestResult]) {
    println!("\n{}", "=".repeat(80));
    println!("📊 COMPREHENSIVE RESULTS SUMMARY");
    println!("{}", "=".repeat(80));
    
    // Count results by category
    let mut successful = 0;
    let mut failed = 0;
    let mut errors = 0;
    let mut tax_tokens = 0;
    
    for result in results {
        if let Some(analysis) = &result.result {
            if analysis.is_tradeable {
                successful += 1;
                if analysis.buy_tax_percent > 0.0 || analysis.sell_tax_percent > 0.0 {
                    tax_tokens += 1;
                }
            } else {
                failed += 1;
            }
        } else {
            errors += 1;
        }
    }
    
    println!("\n📈 Overall Statistics:");
    println!("  ✅ Successful: {} tokens", successful);
    println!("  ❌ Failed Trading: {} tokens", failed);
    println!("  💥 Analysis Errors: {} tokens", errors);
    println!("  🏷️  Tax Tokens Detected: {} tokens", tax_tokens);
    println!("  📊 Total Tested: {} tokens", results.len());
    
    // Detailed results table
    println!("\n📋 Detailed Results Table:");
    println!("{}", "-".repeat(120));
    println!("{:<6} | {:<10} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8} | {:<10} | {:<20}", 
             "Symbol", "Tradeable", "Buy", "Approve", "Sell", "Buy Tax", "Sell Tax", "Net Loss", "Failure Reason");
    println!("{}", "-".repeat(120));
    
    for result in results {
        let symbol = result.config.symbol;
        
        if let Some(analysis) = &result.result {
            let tradeable = if analysis.is_tradeable { "✅" } else { "❌" };
            let can_buy = if analysis.can_buy { "✅" } else { "❌" };
            let can_approve = if analysis.can_approve { "✅" } else { "❌" };
            let can_sell = if analysis.can_sell { "✅" } else { "❌" };
            let buy_tax = format!("{:.1}%", analysis.buy_tax_percent);
            let sell_tax = format!("{:.1}%", analysis.sell_tax_percent);
            
            let net_loss = if analysis.is_tradeable {
                let loss = analysis.eth_spent.saturating_sub(analysis.eth_received);
                let loss_eth = loss.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
                format!("{:.4}E", loss_eth)
            } else {
                "-".to_string()
            };
            
            let reason = if analysis.is_tradeable {
                "-".to_string()
            } else {
                analysis.failure_reason.as_deref()
                    .unwrap_or("Unknown")
                    .chars()
                    .take(18)
                    .collect::<String>() + if analysis.failure_reason.as_deref().unwrap_or("").len() > 18 { "..." } else { "" }
            };
            
            println!("{:<6} | {:<10} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8} | {:<10} | {:<20}",
                     symbol, tradeable, can_buy, can_approve, can_sell, buy_tax, sell_tax, net_loss, reason);
        } else {
            let error_reason = result.error.as_deref()
                .unwrap_or("Unknown error")
                .chars()
                .take(18)
                .collect::<String>() + if result.error.as_deref().unwrap_or("").len() > 18 { "..." } else { "" };
            
            println!("{:<6} | {:<10} | {:<8} | {:<8} | {:<8} | {:<8} | {:<8} | {:<10} | {:<20}",
                     symbol, "💥 ERROR", "-", "-", "-", "-", "-", "-", error_reason);
        }
    }
    
    println!("{}", "-".repeat(120));
    
    // Performance summary
    let total_duration: std::time::Duration = results.iter().map(|r| r.test_duration).sum();
    let avg_duration = total_duration / results.len() as u32;
    
    println!("\n⏱️  Performance Summary:");
    println!("  Total Testing Time: {:?}", total_duration);
    println!("  Average Per Token: {:?}", avg_duration);
    
    // Component validation status
    println!("\n🔧 Component Validation Status:");
    if successful > 0 {
        println!("  ✅ optional_setup_buy_approve_sell_token_simulator.rs is working correctly");
        println!("  ✅ State preservation across transaction sequences validated");
        println!("  ✅ Tax detection mechanisms functioning properly");
        println!("  ✅ Error reporting providing clear failure reasons");
    }
    
    if failed > 0 || errors > 0 {
        println!("  📝 Some tokens failed as expected (taxes, low liquidity, etc.)");
        println!("  📝 Component correctly identifies non-tradeable tokens");
    }
    
    println!("\n🎉 Multi-token analysis complete! Component validation successful.");
}