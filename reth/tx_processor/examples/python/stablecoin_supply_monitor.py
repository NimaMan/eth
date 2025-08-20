#!/usr/bin/env python3
"""
Stablecoin Supply Monitor

Gets the latest total supply of all major stablecoins using ChainQuery.
Uses the stablecoin addresses from eth_data common_addresses.
"""

import ethtx
import time
from datetime import datetime

# Stablecoin addresses from eth_data/chain_utils/common_addresses/stablecoin_addresses.py
STABLECOINS = {
    # Major USD stablecoins
    "USDC": ("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6),
    "USDT": ("0xdAC17F958D2ee523a2206206994597C13D831ec7", 6),
    "DAI": ("0x6B175474E89094C44Da98b954EedeAC495271d0F", 18),
    "BUSD": ("0x4fabb145d64652a948d72533023f6e7a623c7c53", 18),
    "TUSD": ("0x0000000000085d4780B73119b644AE5ecd22b376", 18),
    "USDP": ("0x8E870D67F660D95D5be530380D0eC0bd388289E1", 18),
    "GUSD": ("0x056FD409E1d7A124BD7017459dFEa2F387B6d5Cd", 2),
    "FRAX": ("0x853d955aCEf822Db058eb8505911ED77F175b99e", 18),
    "LUSD": ("0x5f98805A4E8be255a32880FDeC7F6728C6568bA0", 18),
    "FDUSD": ("0xc5f0F7B66764F6EC8c8dFF7Ba683102295E16409", 18),
    "PYUSD": ("0x6c3EA9036406852006290770BEdFcAbA0e23A0E8", 6),
    "USDE": ("0x4C9EDD5852cD905F086c759e8383E09BFF1E68B3", 18),
    "USDD": ("0x0C10BF8FCB7BF5412187A595aB97A3609160B5C6", 18),
    "USDS": ("0xA4BDB11dC0a2beC88d24A3AA1e6bb17201112EBE", 18),
    "USD0": ("0x73a15fed60bf67631dc6cd7bc5b6e8da8190acf5", 18),
    
    # Euro stablecoins
    "EUROC": ("0x1aBaEA1f7C830bd89Acc67EC4af516284b1bC33c", 6),
    "EURCV": ("0x5F7827FDeb7c20b443265Fc2F40845B715385Ff2", 18),
    "EURS": ("0xdb25f211ab05b1c97d595516f45794528a807ad8", 2),
    "EURT": ("0xC581b735A1688071A1746c968e0798D642EDE491", 6),
    
    # Other currency stablecoins
    "GYEN": ("0xC08512927D12348F6620a698105e1BAac6EcD911", 6),  # Japanese Yen
    "XSGD": ("0x70e8dE73cE538DA2bEEd35d14187F6959a8ecA96", 6),  # Singapore Dollar
    "CADC": ("0xcaDC0aCD4B445166f12D2C07EaC6E2544FbE2Eef", 18), # Canadian Dollar
    "BRZ": ("0x01D33Fd36ec67C6adA32Cf36B31E88Ee190b1839", 4),   # Brazilian Real
    
    # Gold-backed stablecoins
    "PAXG": ("0x45804880De22913dAFE09f4980848ECE6EcbAf78", 18),  # Pax Gold
    "XAUt": ("0x68749665FF8D2d112Fa859AA293F07A622782F38", 6),   # Tether Gold
    
    # Algorithmic/Crypto-collateralized
    "GHO": ("0x40D16FC0246aD3160Ccc09B8D0D3A2cD28aE6C2f", 18),   # Aave GHO
    "MKUSD": ("0x4591DBfF62656E7859Afe5e45f6f47D3669fBB28", 18), # Prisma mkUSD
    "sUSD": ("0x57ab1eC28D129707052DF4DF418D58A2D46d5f51", 18),  # Synthetix sUSD
    "DOLA": ("0x865377367054516e17014ccded1e7d814edc9ce4", 18),  # Inverse DOLA
    "RAI": ("0x03ab458634910Aad20Ef5f1C8eE96f1d6Ac54919", 18),   # RAI (not USD pegged)
}

def format_supply(supply_str, decimals):
    """Format supply with proper decimals and thousand separators"""
    supply = int(supply_str)
    if supply == 0:
        return "0"
    
    # Convert to float with decimals
    value = supply / (10 ** decimals)
    
    # Format with thousand separators
    if value >= 1_000_000_000:
        return f"${value/1_000_000_000:,.2f}B"
    elif value >= 1_000_000:
        return f"${value/1_000_000:,.2f}M"
    elif value >= 1_000:
        return f"${value:,.0f}"
    else:
        return f"${value:.2f}"

def get_category(name):
    """Categorize stablecoin by type"""
    if name in ["USDC", "USDT", "BUSD", "TUSD", "USDP", "GUSD", "FDUSD", "PYUSD"]:
        return "Fiat-Backed USD"
    elif name in ["DAI", "FRAX", "LUSD", "GHO", "MKUSD", "sUSD", "DOLA"]:
        return "Crypto-Collateralized"
    elif name in ["USDE", "USDD", "USDS", "USD0"]:
        return "Algorithmic/Synthetic"
    elif name in ["EUROC", "EURCV", "EURS", "EURT"]:
        return "Euro Stablecoins"
    elif name in ["PAXG", "XAUt"]:
        return "Gold-Backed"
    elif name in ["GYEN", "XSGD", "CADC", "BRZ"]:
        return "Other Currencies"
    elif name == "RAI":
        return "Non-Pegged"
    else:
        return "Other"

def main():
    print("="*80)
    print("STABLECOIN SUPPLY MONITOR")
    print("="*80)
    print(f"Timestamp: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    
    # Initialize ChainQuery
    print("\nInitializing ChainQuery...")
    query = ethtx.ChainQuery()
    
    # Get latest block
    latest_block = query.get_latest_block()
    print(f"Latest block: {latest_block:,}")
    print("\nFetching stablecoin supplies...\n")
    
    # Collect supplies by category
    supplies_by_category = {}
    total_usd_supply = 0
    query_times = []
    
    for name, (address, decimals) in STABLECOINS.items():
        start = time.time()
        
        try:
            # Get total supply
            supply = query.get_token_total_supply(address)
            query_time = (time.time() - start) * 1000
            query_times.append(query_time)
            
            # Format supply
            formatted = format_supply(supply, decimals)
            
            # Calculate USD value (assuming 1:1 peg for USD stables)
            if "USD" in name or name in ["DAI", "FRAX", "LUSD", "GHO", "MKUSD", "sUSD", "DOLA"]:
                usd_value = int(supply) / (10 ** decimals)
                total_usd_supply += usd_value
            
            # Categorize
            category = get_category(name)
            if category not in supplies_by_category:
                supplies_by_category[category] = []
            
            supplies_by_category[category].append((name, formatted, query_time))
            
        except Exception as e:
            print(f"Error fetching {name}: {e}")
    
    # Display results by category
    print("="*80)
    print("STABLECOIN SUPPLIES BY CATEGORY")
    print("="*80)
    
    category_order = [
        "Fiat-Backed USD",
        "Crypto-Collateralized", 
        "Algorithmic/Synthetic",
        "Euro Stablecoins",
        "Gold-Backed",
        "Other Currencies",
        "Non-Pegged",
        "Other"
    ]
    
    for category in category_order:
        if category in supplies_by_category:
            print(f"\n{category}:")
            print("-" * 40)
            
            # Sort by name within category
            supplies = sorted(supplies_by_category[category], key=lambda x: x[0])
            
            for name, supply, query_time in supplies:
                # Right-align supply value
                print(f"  {name:8} {supply:>15}  ({query_time:5.1f}ms)")
    
    # Summary statistics
    print("\n" + "="*80)
    print("SUMMARY STATISTICS")
    print("="*80)
    
    print(f"\nTotal USD-denominated supply: ${total_usd_supply:,.0f}")
    print(f"Number of stablecoins tracked: {len(STABLECOINS)}")
    
    if query_times:
        avg_time = sum(query_times) / len(query_times)
        print(f"\nQuery Performance:")
        print(f"  Average query time: {avg_time:.1f}ms")
        print(f"  Min query time: {min(query_times):.1f}ms")
        print(f"  Max query time: {max(query_times):.1f}ms")
        print(f"  Total time: {sum(query_times):.1f}ms")
    
    # Top 10 by supply (USD stables only)
    print("\n" + "="*80)
    print("TOP 10 USD STABLECOINS BY SUPPLY")
    print("="*80)
    
    usd_supplies = []
    for name, (address, decimals) in STABLECOINS.items():
        if "USD" in name or name in ["DAI", "FRAX", "LUSD", "GHO", "MKUSD", "sUSD", "DOLA"]:
            try:
                supply = query.get_token_total_supply(address)
                usd_value = int(supply) / (10 ** decimals)
                if usd_value > 0:
                    usd_supplies.append((name, usd_value))
            except:
                pass
    
    # Sort by supply
    usd_supplies.sort(key=lambda x: x[1], reverse=True)
    
    print("\nRank  Token      Supply")
    print("-" * 40)
    for i, (name, supply) in enumerate(usd_supplies[:10], 1):
        formatted = format_supply(str(int(supply * 1e18)), 18)  # Convert back for formatting
        print(f" {i:2}.  {name:8}  {formatted:>15}")
    
    # Market share
    print("\n" + "="*80)
    print("MARKET SHARE (Top 5)")
    print("="*80)
    
    for name, supply in usd_supplies[:5]:
        share = (supply / total_usd_supply) * 100 if total_usd_supply > 0 else 0
        bar_length = int(share / 2)  # Scale to 50 chars max
        bar = "█" * bar_length
        print(f"{name:8} {share:5.1f}% {bar}")
    
    print("\n✅ Stablecoin supply monitoring completed!")

if __name__ == "__main__":
    main()