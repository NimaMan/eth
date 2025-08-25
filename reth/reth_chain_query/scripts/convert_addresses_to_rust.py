#!/usr/bin/env python3
"""
Convert Python address dictionaries to Rust const arrays.

This script parses the Python address files from eth_data and generates
Rust source files with proper const arrays and lookup structures.
"""

import sys
import os
import re
from pathlib import Path

# Add eth_data to path
ETH_DATA_PATH = "/home/nima/code/crypto/py/eth_data"
sys.path.append(ETH_DATA_PATH)

from eth_data.chain_utils.common_addresses import (
    STABLECOINS_ADDRESS_BY_NAME,
    STABLECOIN_UNIT_BY_NAME,
    ERC20_TOKEN_DECIMALS,
)
from eth_data.chain_utils.common_addresses.all_cex_addresses import CEX_ADDRESSES_BY_NAME
from eth_data.chain_utils.common_addresses.all_etf_addresses import ETF_ADDRESSES_BY_NAME


def format_address(addr: str) -> str:
    """Format address for Rust, removing 0x prefix."""
    return addr[2:] if addr.startswith("0x") else addr


def generate_stablecoins_rust():
    """Generate Rust code for stablecoin addresses with decimals."""
    rust_code = '''//! Stablecoin addresses and metadata
//! 
//! This file is auto-generated from Python address files.
//! Do not edit manually - regenerate using scripts/convert_addresses_to_rust.py

use alloy_primitives::{address, Address};
use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Information about a stablecoin
#[derive(Debug, Clone)]
pub struct StablecoinInfo {
    pub address: Address,
    pub symbol: &'static str,
    pub decimals: u8,
    pub unit: &'static str,
}

/// All stablecoin information
pub const STABLECOINS: &[StablecoinInfo] = &[
'''
    
    # Generate stablecoin entries
    entries = []
    for name, addr in STABLECOINS_ADDRESS_BY_NAME.items():
        decimals = ERC20_TOKEN_DECIMALS.get(name, 18)
        unit = STABLECOIN_UNIT_BY_NAME.get(name, "Unknown")
        
        entry = f'''    StablecoinInfo {{
        address: address!("{format_address(addr)}"),
        symbol: "{name}",
        decimals: {decimals},
        unit: "{unit}",
    }}'''
        entries.append(entry)
    
    rust_code += ',\n'.join(entries)
    rust_code += '''
];

/// Lazy static HashMap for quick lookups by address
pub static STABLECOIN_BY_ADDRESS: Lazy<HashMap<Address, &'static StablecoinInfo>> = Lazy::new(|| {
    STABLECOINS
        .iter()
        .map(|info| (info.address, info))
        .collect()
});

/// Lazy static HashMap for quick lookups by symbol
pub static STABLECOIN_BY_SYMBOL: Lazy<HashMap<&'static str, &'static StablecoinInfo>> = Lazy::new(|| {
    STABLECOINS
        .iter()
        .map(|info| (info.symbol, info))
        .collect()
});

/// Check if an address is a stablecoin
pub fn is_stablecoin(address: Address) -> bool {
    STABLECOIN_BY_ADDRESS.contains_key(&address)
}

/// Get stablecoin info by address
pub fn get_stablecoin_by_address(address: Address) -> Option<&'static StablecoinInfo> {
    STABLECOIN_BY_ADDRESS.get(&address).copied()
}

/// Get stablecoin info by symbol
pub fn get_stablecoin_by_symbol(symbol: &str) -> Option<&'static StablecoinInfo> {
    STABLECOIN_BY_SYMBOL.get(symbol).copied()
}
'''
    
    return rust_code


def generate_cex_rust():
    """Generate Rust code for CEX addresses."""
    rust_code = '''//! CEX (Centralized Exchange) addresses
//! 
//! This file is auto-generated from Python address files.
//! Do not edit manually - regenerate using scripts/convert_addresses_to_rust.py

use alloy_primitives::{address, Address};
use std::collections::{HashMap, HashSet};
use once_cell::sync::Lazy;

/// CEX address entry
#[derive(Debug, Clone)]
pub struct CexAddress {
    pub address: Address,
    pub name: &'static str,
    pub exchange: &'static str,
}

/// All CEX addresses
pub const CEX_ADDRESSES: &[CexAddress] = &[
'''
    
    # Group addresses by exchange
    exchanges = {}
    for name, addr in CEX_ADDRESSES_BY_NAME.items():
        # Extract base exchange name (e.g., "Binance" from "Binance_1")
        if '_' in name:
            exchange = name.split('_')[0]
        else:
            exchange = name.split()[0] if ' ' in name else name
        
        if exchange not in exchanges:
            exchanges[exchange] = []
        exchanges[exchange].append((name, addr))
    
    # Generate entries
    entries = []
    for exchange, addrs in sorted(exchanges.items()):
        for name, addr in addrs:
            entry = f'''    CexAddress {{
        address: address!("{format_address(addr)}"),
        name: "{name}",
        exchange: "{exchange}",
    }}'''
            entries.append(entry)
    
    rust_code += ',\n'.join(entries)
    rust_code += f'''
];

/// Total number of CEX addresses
pub const CEX_ADDRESS_COUNT: usize = {len(CEX_ADDRESSES_BY_NAME)};

/// Lazy static HashSet for quick lookups
pub static CEX_ADDRESS_SET: Lazy<HashSet<Address>> = Lazy::new(|| {{
    CEX_ADDRESSES.iter().map(|entry| entry.address).collect()
}});

/// Lazy static HashMap for address to exchange mapping
pub static CEX_BY_ADDRESS: Lazy<HashMap<Address, &'static CexAddress>> = Lazy::new(|| {{
    CEX_ADDRESSES
        .iter()
        .map(|entry| (entry.address, entry))
        .collect()
}});

/// Group addresses by exchange
pub static ADDRESSES_BY_EXCHANGE: Lazy<HashMap<&'static str, Vec<Address>>> = Lazy::new(|| {{
    let mut map: HashMap<&'static str, Vec<Address>> = HashMap::new();
    for entry in CEX_ADDRESSES {{
        map.entry(entry.exchange)
            .or_insert_with(Vec::new)
            .push(entry.address);
    }}
    map
}});

/// Check if an address belongs to a CEX
pub fn is_cex_address(address: Address) -> bool {{
    CEX_ADDRESS_SET.contains(&address)
}}

/// Get CEX info by address
pub fn get_cex_by_address(address: Address) -> Option<&'static CexAddress> {{
    CEX_BY_ADDRESS.get(&address).copied()
}}

/// Get all addresses for a specific exchange
pub fn get_exchange_addresses(exchange: &str) -> Option<&'static Vec<Address>> {{
    ADDRESSES_BY_EXCHANGE.get(exchange)
}}

/// Exchange names with counts
pub fn exchange_stats() -> Vec<(&'static str, usize)> {{
    let mut stats: Vec<_> = ADDRESSES_BY_EXCHANGE
        .iter()
        .map(|(name, addrs)| (*name, addrs.len()))
        .collect();
    stats.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    stats
}}
'''
    
    # Add summary comment
    exchange_counts = {}
    for name in CEX_ADDRESSES_BY_NAME.keys():
        exchange = name.split('_')[0] if '_' in name else (name.split()[0] if ' ' in name else name)
        exchange_counts[exchange] = exchange_counts.get(exchange, 0) + 1
    
    summary = "\n/// Exchange counts:\n"
    for exchange, count in sorted(exchange_counts.items(), key=lambda x: -x[1])[:10]:
        summary += f"/// - {exchange}: {count} addresses\n"
    
    rust_code = rust_code.replace("/// Exchange names with counts", summary + "/// Exchange names with counts")
    
    return rust_code


def generate_etf_rust():
    """Generate Rust code for ETF addresses."""
    rust_code = '''//! ETF (Exchange-Traded Fund) addresses
//! 
//! This file is auto-generated from Python address files.
//! Do not edit manually - regenerate using scripts/convert_addresses_to_rust.py

use alloy_primitives::{address, Address};
use std::collections::{HashMap, HashSet};
use once_cell::sync::Lazy;

/// ETF address entry
#[derive(Debug, Clone)]
pub struct EtfAddress {
    pub address: Address,
    pub name: &'static str,
    pub provider: &'static str,
}

/// All ETF addresses
pub const ETF_ADDRESSES: &[EtfAddress] = &[
'''
    
    # Group addresses by provider
    providers = {}
    for name, addr in ETF_ADDRESSES_BY_NAME.items():
        # Extract provider (e.g., "BlackRock" from "BlackRock ETHA_1")
        parts = name.split()
        if len(parts) >= 2:
            provider = parts[0]
            if parts[0] == "Grayscale" and len(parts) > 1:
                provider = "Grayscale"
        else:
            provider = name
        
        if provider not in providers:
            providers[provider] = []
        providers[provider].append((name, addr))
    
    # Generate entries
    entries = []
    for provider, addrs in sorted(providers.items()):
        for name, addr in addrs:
            entry = f'''    EtfAddress {{
        address: address!("{format_address(addr)}"),
        name: "{name}",
        provider: "{provider}",
    }}'''
            entries.append(entry)
    
    rust_code += ',\n'.join(entries)
    rust_code += f'''
];

/// Total number of ETF addresses
pub const ETF_ADDRESS_COUNT: usize = {len(ETF_ADDRESSES_BY_NAME)};

/// Lazy static HashSet for quick lookups
pub static ETF_ADDRESS_SET: Lazy<HashSet<Address>> = Lazy::new(|| {{
    ETF_ADDRESSES.iter().map(|entry| entry.address).collect()
}});

/// Lazy static HashMap for address to ETF mapping
pub static ETF_BY_ADDRESS: Lazy<HashMap<Address, &'static EtfAddress>> = Lazy::new(|| {{
    ETF_ADDRESSES
        .iter()
        .map(|entry| (entry.address, entry))
        .collect()
}});

/// Group addresses by provider
pub static ADDRESSES_BY_PROVIDER: Lazy<HashMap<&'static str, Vec<Address>>> = Lazy::new(|| {{
    let mut map: HashMap<&'static str, Vec<Address>> = HashMap::new();
    for entry in ETF_ADDRESSES {{
        map.entry(entry.provider)
            .or_insert_with(Vec::new)
            .push(entry.address);
    }}
    map
}});

/// Check if an address belongs to an ETF
pub fn is_etf_address(address: Address) -> bool {{
    ETF_ADDRESS_SET.contains(&address)
}}

/// Get ETF info by address
pub fn get_etf_by_address(address: Address) -> Option<&'static EtfAddress> {{
    ETF_BY_ADDRESS.get(&address).copied()
}}

/// Get all addresses for a specific provider
pub fn get_provider_addresses(provider: &str) -> Option<&'static Vec<Address>> {{
    ADDRESSES_BY_PROVIDER.get(provider)
}}

/// Provider names with counts
pub fn provider_stats() -> Vec<(&'static str, usize)> {{
    let mut stats: Vec<_> = ADDRESSES_BY_PROVIDER
        .iter()
        .map(|(name, addrs)| (*name, addrs.len()))
        .collect();
    stats.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    stats
}}
'''
    
    # Add summary comment
    provider_counts = {}
    for name in ETF_ADDRESSES_BY_NAME.keys():
        parts = name.split()
        provider = parts[0] if len(parts) >= 2 else name
        provider_counts[provider] = provider_counts.get(provider, 0) + 1
    
    summary = "\n/// Provider counts:\n"
    for provider, count in sorted(provider_counts.items(), key=lambda x: -x[1])[:10]:
        summary += f"/// - {provider}: {count} addresses\n"
    
    rust_code = rust_code.replace("/// Provider names with counts", summary + "/// Provider names with counts")
    
    return rust_code


def main():
    """Generate all Rust files."""
    output_dir = Path("/home/nima/code/crypto/rust/reth_chain_query/src/entities")
    
    # Generate stablecoins
    stablecoins_code = generate_stablecoins_rust()
    stablecoins_file = output_dir / "stablecoins" / "addresses.rs"
    stablecoins_file.write_text(stablecoins_code)
    print(f"Generated {stablecoins_file}")
    print(f"  - {len(STABLECOINS_ADDRESS_BY_NAME)} stablecoins with decimals")
    
    # Generate CEX addresses
    cex_code = generate_cex_rust()
    cex_file = output_dir / "cex" / "addresses.rs"
    cex_file.write_text(cex_code)
    print(f"Generated {cex_file}")
    print(f"  - {len(CEX_ADDRESSES_BY_NAME)} CEX addresses")
    
    # Generate ETF addresses
    etf_code = generate_etf_rust()
    etf_file = output_dir / "etfs" / "addresses.rs"
    etf_file.write_text(etf_code)
    print(f"Generated {etf_file}")
    print(f"  - {len(ETF_ADDRESSES_BY_NAME)} ETF addresses")
    
    total = len(STABLECOINS_ADDRESS_BY_NAME) + len(CEX_ADDRESSES_BY_NAME) + len(ETF_ADDRESSES_BY_NAME)
    print(f"\nTotal addresses generated: {total}")


if __name__ == "__main__":
    main()