/// High-level blockchain analysis methods
/// 
/// This module provides sophisticated analysis functions that combine
/// multiple query types to deliver insights that would typically require
/// hundreds of RPC calls.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use alloy_primitives::{Address, U256};
use std::str::FromStr;
use std::time::Instant;

use super::main::PyChainQuery;

// Well-known stablecoin addresses for market analysis
const MAJOR_STABLECOINS: &[(&str, &str)] = &[
    ("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", "USDC"),
    ("0xdAC17F958D2ee523a2206206994597C13D831ec7", "USDT"),
    ("0x6B175474E89094C44Da98b954EedeAC495271d0F", "DAI"),
    ("0x4Fabb145d64652a948d72533023f6E7A623C7C53", "BUSD"),
    ("0x853d955aCEf822Db058eb8505911ED77F175b99e", "FRAX"),
];

// Major whale addresses for concentration analysis
const WHALE_ADDRESSES: &[(&str, &str)] = &[
    ("0x28C6c06298d514Db089934071355E5743bf21d60", "Binance Hot Wallet"),
    ("0x21a31Ee1afC51d94C2eFcCAa2092aD1028285549", "Binance Cold Wallet"),
    ("0xF977814e90dA44bFA03b6295A0616a897441aceC", "Binance US"),
];

/// Format token amount with proper decimals for display
fn format_token_amount(amount: U256, decimals: u8) -> f64 {
    let divisor = U256::from(10).pow(U256::from(decimals));
    let whole = amount / divisor;
    let fraction = amount % divisor;
    
    let whole_f64 = whole.to_string().parse::<f64>().unwrap_or(0.0);
    let fraction_f64 = fraction.to_string().parse::<f64>().unwrap_or(0.0) / 10_f64.powi(decimals as i32);
    
    whole_f64 + fraction_f64
}

/// Analyze stablecoin market share
/// 
/// Performs comprehensive stablecoin market analysis similar to the Rust example
/// that achieves 908x speedup over RPC. Returns market data, whale concentration,
/// and performance metrics.
/// 
/// Returns:
///     dict: Market analysis with stablecoins, metrics, and performance data
pub fn analyze_stablecoin_market(chain_query: &PyChainQuery, py: Python) -> PyResult<Py<PyDict>> {
    let start_time = Instant::now();
    let query = chain_query.chain_query().clone();
    let latest_block = query.get_latest_block().map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Failed to get latest block: {}", e))
    })?;
    
    let mut stablecoin_results = Vec::new();
    let mut total_market_supply = 0.0;
    let mut query_count = 0;
    
    // Analyze each stablecoin
    for (address_str, symbol) in MAJOR_STABLECOINS {
        let address = Address::from_str(address_str).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid address {}: {}", address_str, e))
        })?;
        
        let (name, decimals, total_supply, whale_data) = chain_query.runtime().block_on(async {
            // Get token metadata and supply
            let name = query.token.get_erc20_name(address, Some(latest_block)).await.unwrap_or_else(|_| symbol.to_string());
            let decimals = query.token.get_erc20_decimals(address, Some(latest_block)).await.unwrap_or(18);
            let total_supply = query.token.get_erc20_total_supply(address, Some(latest_block)).await.unwrap_or(U256::ZERO);
            
            // Whale analysis for top stablecoins
            let mut whale_balances = Vec::new();
            let mut total_whale_balance = U256::ZERO;
            
            if ["USDC", "USDT", "DAI"].contains(&symbol) {
                for (whale_addr_str, whale_name) in WHALE_ADDRESSES {
                    let whale_addr = Address::from_str(whale_addr_str).unwrap();
                    let balance = query.token.get_erc20_balance(address, whale_addr, Some(latest_block)).await.unwrap_or(U256::ZERO);
                    let balance_formatted = format_token_amount(balance, decimals);
                    
                    if balance_formatted > 1.0 {
                        whale_balances.push((whale_name.to_string(), balance_formatted));
                        total_whale_balance += balance;
                    }
                }
            }
            
            let whale_balance_formatted = format_token_amount(total_whale_balance, decimals);
            (name, decimals, total_supply, (whale_balances, whale_balance_formatted))
        });
        
        query_count += 3 + WHALE_ADDRESSES.len(); // metadata + whale balances
        
        let total_supply_formatted = format_token_amount(total_supply, decimals);
        total_market_supply += total_supply_formatted;
        
        let (whale_balances, whale_balance_formatted) = whale_data;
        let concentration_percent = if total_supply_formatted > 0.0 {
            (whale_balance_formatted / total_supply_formatted) * 100.0
        } else {
            0.0
        };
        
        // Create stablecoin data
        let coin_dict = PyDict::new(py);
        coin_dict.set_item("address", address_str)?;
        coin_dict.set_item("symbol", symbol)?;
        coin_dict.set_item("name", name)?;
        coin_dict.set_item("decimals", decimals)?;
        coin_dict.set_item("total_supply", total_supply_formatted)?;
        coin_dict.set_item("whale_balance", whale_balance_formatted)?;
        coin_dict.set_item("concentration_percent", concentration_percent)?;
        
        // Add whale balances
        let whale_list = PyList::empty(py);
        for (whale_name, balance) in whale_balances {
            let whale_dict = PyDict::new(py);
            whale_dict.set_item("name", whale_name)?;
            whale_dict.set_item("balance", balance)?;
            whale_list.append(whale_dict)?;
        }
        coin_dict.set_item("top_whales", whale_list)?;
        
        stablecoin_results.push(coin_dict);
    }
    
    // Calculate market shares and sort
    for coin_dict in &stablecoin_results {
        let total_supply = coin_dict.get_item("total_supply")?
            .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, _>("total_supply key not found"))?
            .extract::<f64>()?;
        let market_share = (total_supply / total_market_supply) * 100.0;
        coin_dict.set_item("market_share_percent", market_share)?;
    }
    
    // Sort by market share (descending)
    stablecoin_results.sort_by(|a, b| {
        let a_share = a.get_item("market_share_percent")
            .ok()
            .flatten()
            .and_then(|item| item.extract::<f64>().ok())
            .unwrap_or(0.0);
        let b_share = b.get_item("market_share_percent")
            .ok()
            .flatten()
            .and_then(|item| item.extract::<f64>().ok())
            .unwrap_or(0.0);
        b_share.partial_cmp(&a_share).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    let total_time = start_time.elapsed();
    
    // Calculate concentration metrics
    let top_3_share: f64 = stablecoin_results.iter().take(3)
        .map(|coin| coin.get_item("market_share_percent")
            .ok()
            .flatten()
            .and_then(|item| item.extract::<f64>().ok())
            .unwrap_or(0.0))
        .sum();
    
    let market_health = if top_3_share > 90.0 {
        "highly_concentrated"
    } else if top_3_share > 75.0 {
        "moderately_concentrated"  
    } else {
        "well_distributed"
    };
    
    // Create result dict
    let result = PyDict::new(py);
    
    // Add stablecoin data
    let stablecoins_list = PyList::empty(py);
    for coin in stablecoin_results {
        stablecoins_list.append(coin)?;
    }
    result.set_item("stablecoins", stablecoins_list)?;
    
    // Add market metrics
    let metrics = PyDict::new(py);
    metrics.set_item("total_market_supply", total_market_supply)?;
    metrics.set_item("total_stablecoins", MAJOR_STABLECOINS.len())?;
    metrics.set_item("top_3_market_share", top_3_share)?;
    metrics.set_item("market_health", market_health)?;
    metrics.set_item("latest_block", latest_block)?;
    result.set_item("market_metrics", metrics)?;
    
    // Add performance metrics
    let performance = PyDict::new(py);
    performance.set_item("query_count", query_count)?;
    performance.set_item("total_time_ms", total_time.as_millis())?;
    performance.set_item("avg_query_time_ms", total_time.as_millis() as f64 / query_count as f64)?;
    
    // RPC comparison
    let estimated_rpc_time = query_count * 150;
    let speedup = estimated_rpc_time as f64 / total_time.as_millis() as f64;
    performance.set_item("estimated_rpc_time_ms", estimated_rpc_time)?;
    performance.set_item("speedup_factor", speedup)?;
    
    result.set_item("performance", performance)?;
    
    Ok(result.into())
}

/// Simplified token holdings analysis - returns basic portfolio data
/// 
/// Args:
///     token_addresses (list): List of token contract addresses
///     holder_addresses (list): List of addresses to check
///     block_number (int, optional): Block number to query at
/// 
/// Returns:
///     dict: Holdings data with performance metrics
pub fn analyze_token_holdings(
    chain_query: &PyChainQuery,
    py: Python,
    token_addresses: Vec<&str>,
    holder_addresses: Vec<&str>,
    block_number: Option<u64>,
) -> PyResult<Py<PyDict>> {
    let start_time = Instant::now();
    let query = chain_query.chain_query().clone();
    
    let mut query_count = 0;
    let holdings_list = PyList::empty(py);
    
    // Simple implementation - get holdings for each address
    for holder_addr_str in &holder_addresses {
        let holder_addr = Address::from_str(holder_addr_str.trim_start_matches("0x"))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Invalid holder address: {}", e)
            ))?;
        
        let holder_dict = PyDict::new(py);
        holder_dict.set_item("address", holder_addr_str)?;
        
        let balances_list = PyList::empty(py);
        let mut total_positions = 0;
        
        for token_addr_str in &token_addresses {
            let token_addr = Address::from_str(token_addr_str.trim_start_matches("0x"))
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    format!("Invalid token address: {}", e)
                ))?;
            
            let (balance, symbol, decimals) = chain_query.runtime().block_on(async {
                let balance = query.token.get_erc20_balance(token_addr, holder_addr, block_number).await.unwrap_or(U256::ZERO);
                let symbol = query.token.get_erc20_symbol(token_addr, block_number).await.unwrap_or_else(|_| "UNKNOWN".to_string());
                let decimals = query.token.get_erc20_decimals(token_addr, block_number).await.unwrap_or(18);
                (balance, symbol, decimals)
            });
            
            query_count += 3;
            
            let balance_formatted = format_token_amount(balance, decimals);
            if balance_formatted > 0.001 {
                total_positions += 1;
                
                let balance_dict = PyDict::new(py);
                balance_dict.set_item("token_address", token_addr_str)?;
                balance_dict.set_item("symbol", symbol)?;
                balance_dict.set_item("balance", balance_formatted)?;
                balance_dict.set_item("decimals", decimals)?;
                
                balances_list.append(balance_dict)?;
            }
        }
        
        holder_dict.set_item("token_balances", balances_list)?;
        holder_dict.set_item("total_positions", total_positions)?;
        
        holdings_list.append(holder_dict)?;
    }
    
    let total_time = start_time.elapsed();
    
    // Create result
    let result = PyDict::new(py);
    result.set_item("holdings", holdings_list)?;
    
    // Performance metrics
    let performance = PyDict::new(py);
    performance.set_item("tokens_analyzed", token_addresses.len())?;
    performance.set_item("holders_analyzed", holder_addresses.len())?;
    performance.set_item("query_count", query_count)?;
    performance.set_item("total_time_ms", total_time.as_millis())?;
    
    if query_count > 0 {
        performance.set_item("avg_query_time_ms", total_time.as_millis() as f64 / query_count as f64)?;
        let estimated_rpc_time = query_count * 120;
        let speedup = estimated_rpc_time as f64 / total_time.as_millis() as f64;
        performance.set_item("estimated_rpc_time_ms", estimated_rpc_time)?;
        performance.set_item("speedup_factor", speedup)?;
    }
    
    result.set_item("performance", performance)?;
    
    Ok(result.into())
}

/// Get whale portfolios across multiple tokens - simplified version
pub fn get_whale_portfolios(
    chain_query: &PyChainQuery,
    py: Python,
    token_addresses: Vec<&str>,
    whale_addresses: Vec<&str>,
    block_number: Option<u64>,
) -> PyResult<Py<PyDict>> {
    // For now, just return the token holdings analysis
    // This can be enhanced with whale-specific metrics later
    analyze_token_holdings(chain_query, py, token_addresses, whale_addresses, block_number)
}