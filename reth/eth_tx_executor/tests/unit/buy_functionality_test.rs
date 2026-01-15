//! Unit tests for ETH->Token buy functionality

use eth_kartal::pools::{Pool, SwapParams};
use eth_kartal::pools::uniswap_v2::{UniswapV2Pool, addresses};
use ethers::prelude::*;
use ethers::utils::{parse_ether, parse_units};

#[tokio::test]
async fn test_eth_to_token_swap_tx_building() {
    // Create mock provider
    let provider = Provider::<Http>::try_from("http://127.0.0.1:8545")
        .expect("Failed to create provider");
    let provider = std::sync::Arc::new(provider);
    
    // USDC address
    let usdc = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse::<Address>().unwrap();
    
    // Create pool instance
    let pool = UniswapV2Pool::from_tokens(*addresses::WETH, usdc, provider.clone())
        .await
        .expect("Failed to create pool");
    
    // Build ETH -> USDC swap transaction
    let swap_params = SwapParams {
        token_in: *addresses::WETH,
        token_out: usdc,
        amount_in: parse_ether("1.0").unwrap(), // 1 ETH
        amount_out_min: parse_units("3000", 6).unwrap(), // Min 3000 USDC
        recipient: "0xb340ad45e7729b9C54c79e744fB3708FB6fb245C".parse().unwrap(),
        deadline: U256::from(1234567890),
    };
    
    let tx = pool.build_swap_tx(swap_params).await
        .expect("Failed to build swap transaction");
    
    // Verify transaction structure
    assert_eq!(tx.to(), Some(&addresses::ROUTER.into()));
    assert_eq!(tx.value(), Some(&parse_ether("1.0").unwrap()));
    
    // Verify it's calling swapExactETHForTokens
    let data = tx.data().unwrap();
    let expected_selector = &ethers::utils::keccak256("swapExactETHForTokens(uint256,address[],address,uint256)")[0..4];
    assert_eq!(&data[0..4], expected_selector);
}

#[tokio::test]
async fn test_token_to_eth_swap_tx_building() {
    let provider = Provider::<Http>::try_from("http://127.0.0.1:8545")
        .expect("Failed to create provider");
    let provider = std::sync::Arc::new(provider);
    
    let usdc = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".parse::<Address>().unwrap();
    
    let pool = UniswapV2Pool::from_tokens(usdc, *addresses::WETH, provider.clone())
        .await
        .expect("Failed to create pool");
    
    // Build USDC -> ETH swap transaction (selling tokens)
    let swap_params = SwapParams {
        token_in: usdc,
        token_out: *addresses::WETH,
        amount_in: parse_units("1000", 6).unwrap(), // 1000 USDC
        amount_out_min: parse_ether("0.3").unwrap(), // Min 0.3 ETH
        recipient: "0xb340ad45e7729b9C54c79e744fB3708FB6fb245C".parse().unwrap(),
        deadline: U256::from(1234567890),
    };
    
    let tx = pool.build_swap_tx(swap_params).await
        .expect("Failed to build swap transaction");
    
    // Verify transaction structure
    assert_eq!(tx.to(), Some(&addresses::ROUTER.into()));
    assert_eq!(tx.value(), Some(&U256::zero())); // No ETH sent for token->ETH swaps
    
    // Verify it's calling swapExactTokensForETH
    let data = tx.data().unwrap();
    let expected_selector = &ethers::utils::keccak256("swapExactTokensForETH(uint256,uint256,address[],address,uint256)")[0..4];
    assert_eq!(&data[0..4], expected_selector);
}

#[test]
fn test_pre_computed_selectors() {
    // Test that our pre-computed selectors match runtime calculations
    use eth_kartal::pools::uniswap_v2::selectors;
    
    let eth_for_tokens = ethers::utils::keccak256("swapExactETHForTokens(uint256,address[],address,uint256)");
    assert_eq!(&selectors::SWAP_ETH_FOR_TOKENS[..], &eth_for_tokens[0..4]);
    
    let tokens_for_eth = ethers::utils::keccak256("swapExactTokensForETH(uint256,uint256,address[],address,uint256)");
    assert_eq!(&selectors::SWAP_TOKENS_FOR_ETH[..], &tokens_for_eth[0..4]);
    
    let tokens_for_tokens = ethers::utils::keccak256("swapExactTokensForTokens(uint256,uint256,address[],address,uint256)");
    assert_eq!(&selectors::SWAP_TOKENS_FOR_TOKENS[..], &tokens_for_tokens[0..4]);
}