#!/usr/bin/env python3
"""
DEX Swap Simulation

Demonstrates simulating token swaps on decentralized exchanges like Uniswap,
including complex multi-hop swaps and slippage analysis.

Algorithm:
1. Initialize PyReth simulator with singleton pattern
2. Encode swap function calls with proper ABI encoding
3. Simulate swaps with different parameters (amounts, slippage)
4. Analyze swap outcomes and gas consumption
5. Test multi-hop swap scenarios and edge cases
"""

import pyreth
from typing import Dict, Any, List, Optional, Tuple

# DEX and token addresses
UNISWAP_V2_ROUTER = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"
UNISWAP_V3_ROUTER = "0xE592427A0AEce92De3Edee1F18E0157C05861564"
WETH_ADDRESS = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
USDC_ADDRESS = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
UNI_ADDRESS = "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984"
DAI_ADDRESS = "0x6B175474E89094C44Da98b954EedeAC495271d0F"

# Test addresses
VITALIK_ADDRESS = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"
WHALE_ADDRESS = "0x742d35cc6568966c0f99d1b7bd0a99b4c2b4d6b8"

def encode_address(address: str) -> str:
    """Encode address to 32-byte hex"""
    return address.replace("0x", "").lower().zfill(64)

def encode_uint256(value: int) -> str:
    """Encode uint256 to 32-byte hex"""
    return format(value, '064x')

def encode_uniswap_v2_swap_exact_eth_for_tokens(
    amount_out_min: int,
    path: List[str],
    to: str,
    deadline: int
) -> str:
    """
    Encode swapExactETHForTokens function call
    swapExactETHForTokens(uint amountOutMin, address[] calldata path, address to, uint deadline)
    Function selector: 0x7ff36ab5
    """
    selector = "7ff36ab5"
    
    # Encode parameters
    amount_out_min_hex = encode_uint256(amount_out_min)
    to_hex = encode_address(to)
    deadline_hex = encode_uint256(deadline)
    
    # Encode path array (simplified for WETH -> token swap)
    # Array offset, length, then addresses
    path_offset = encode_uint256(0x80)  # Offset to path array
    path_length = encode_uint256(len(path))
    path_data = ''.join(encode_address(addr) for addr in path)
    
    return f"0x{selector}{amount_out_min_hex}{path_offset}{to_hex}{deadline_hex}{path_length}{path_data}"

def encode_uniswap_v2_swap_exact_tokens_for_eth(
    amount_in: int,
    amount_out_min: int,
    path: List[str],
    to: str,
    deadline: int
) -> str:
    """
    Encode swapExactTokensForETH function call
    Function selector: 0x18cbafe5
    """
    selector = "18cbafe5"
    
    amount_in_hex = encode_uint256(amount_in)
    amount_out_min_hex = encode_uint256(amount_out_min)
    to_hex = encode_address(to)
    deadline_hex = encode_uint256(deadline)
    
    path_offset = encode_uint256(0xa0)  # Adjusted offset
    path_length = encode_uint256(len(path))
    path_data = ''.join(encode_address(addr) for addr in path)
    
    return f"0x{selector}{amount_in_hex}{amount_out_min_hex}{path_offset}{to_hex}{deadline_hex}{path_length}{path_data}"

def encode_erc20_approve(spender: str, amount: int) -> str:
    """Encode ERC20 approve function"""
    selector = "095ea7b3"
    spender_hex = encode_address(spender)
    amount_hex = encode_uint256(amount)
    return f"0x{selector}{spender_hex}{amount_hex}"

def get_future_timestamp() -> int:
    """Get a future timestamp for swap deadline"""
    import time
    return int(time.time()) + 1800  # 30 minutes from now

def simulate_eth_to_token_swap():
    """Simulate ETH to token swap on Uniswap V2"""
    
    print("=" * 80)
    print("ETH TO TOKEN SWAP SIMULATION")
    print("=" * 80)
    print("Simulating ETH -> USDC swap on Uniswap V2")
    
    reth = pyreth.PyReth()
    simulator = reth.simulator()
    
    # Swap parameters
    eth_amount = 1_000_000_000_000_000_000  # 1 ETH
    min_usdc_out = 2000 * 10**6  # Expect at least 2000 USDC (conservative)
    swap_path = [WETH_ADDRESS, USDC_ADDRESS]
    deadline = get_future_timestamp()
    
    # Encode the swap transaction
    swap_data = encode_uniswap_v2_swap_exact_eth_for_tokens(
        amount_out_min=min_usdc_out,
        path=swap_path,
        to=VITALIK_ADDRESS,
        deadline=deadline
    )
    
    swap_tx = {
        'from': VITALIK_ADDRESS,
        'to': UNISWAP_V2_ROUTER,
        'value': eth_amount,
        'data': swap_data,
        'gas': 200000,  # Higher gas for DEX operations
        'gas_price': 25_000_000_000,  # 25 gwei
        'nonce': 1500
    }
    
    print(f"Swap details:")
    print(f"  Amount in: {eth_amount / 1e18:.2f} ETH")
    print(f"  Min amount out: {min_usdc_out / 1e6:.0f} USDC")
    print(f"  Path: ETH -> WETH -> USDC")
    print(f"  Router: {UNISWAP_V2_ROUTER}")
    print(f"  Gas limit: {swap_tx['gas']:,}")
    
    try:
        result = simulator.simulate_transaction(swap_tx, None)
        
        if result.success:
            print(f"\n✅ Swap simulation successful!")
            print(f"   Gas used: {result.gas_used:,}")
            print(f"   Gas efficiency: {(result.gas_used / swap_tx['gas']) * 100:.1f}%")
            
            # Calculate transaction cost
            gas_cost = result.gas_used * swap_tx['gas_price']
            total_cost = eth_amount + gas_cost
            print(f"   Gas cost: {gas_cost / 1e18:.6f} ETH")
            print(f"   Total cost: {total_cost / 1e18:.6f} ETH")
            
            print(f"\n💡 This swap would succeed with current liquidity")
            
        else:
            print(f"\n❌ Swap simulation failed!")
            print(f"   Revert reason: {result.revert_reason}")
            print(f"   Gas used before revert: {result.gas_used:,}")
            
            # Analyze common failure reasons
            if "slippage" in result.revert_reason.lower():
                print(f"   💡 Try reducing min_amount_out or increasing slippage tolerance")
            elif "liquidity" in result.revert_reason.lower():
                print(f"   💡 Insufficient liquidity for this trade size")
            elif "deadline" in result.revert_reason.lower():
                print(f"   💡 Transaction deadline has passed")
            
    except Exception as e:
        error_msg = str(e)
        print(f"\n⚠️  Simulation error: {error_msg[:100]}...")
        
        # Common errors
        if "nonce" in error_msg.lower():
            print(f"   💡 Nonce issue - this is expected in simulation")

def simulate_token_to_eth_swap():
    """Simulate token to ETH swap (requires approval first)"""
    
    print("\n" + "=" * 80)
    print("TOKEN TO ETH SWAP SIMULATION")
    print("=" * 80)
    print("Simulating USDC -> ETH swap (with approval)")
    
    reth = pyreth.PyReth()
    simulator = reth.simulator()
    
    # Swap parameters
    usdc_amount = 3000 * 10**6  # 3000 USDC
    min_eth_out = 800_000_000_000_000_000  # Expect at least 0.8 ETH
    swap_path = [USDC_ADDRESS, WETH_ADDRESS]
    deadline = get_future_timestamp()
    
    print(f"Swap setup:")
    print(f"  Amount in: {usdc_amount / 1e6:.0f} USDC")
    print(f"  Min amount out: {min_eth_out / 1e18:.2f} ETH")
    print(f"  Path: USDC -> WETH -> ETH")
    
    # Step 1: Approval transaction
    print(f"\n1. Testing USDC approval for Uniswap Router")
    print("-" * 45)
    
    approve_data = encode_erc20_approve(UNISWAP_V2_ROUTER, usdc_amount)
    approve_tx = {
        'from': VITALIK_ADDRESS,
        'to': USDC_ADDRESS,
        'value': 0,
        'data': approve_data,
        'gas': 60000,
        'gas_price': 25_000_000_000,
        'nonce': 1500
    }
    
    try:
        approve_result = simulator.simulate_transaction(approve_tx, None)
        
        if approve_result.success:
            print(f"✅ Approval would succeed - Gas: {approve_result.gas_used:,}")
        else:
            print(f"❌ Approval would fail: {approve_result.revert_reason}")
            print(f"   Cannot proceed with swap without approval")
            return
            
    except Exception as e:
        print(f"⚠️  Approval error: {str(e)[:60]}...")
        return
    
    # Step 2: Swap transaction
    print(f"\n2. Testing USDC -> ETH swap")
    print("-" * 30)
    
    swap_data = encode_uniswap_v2_swap_exact_tokens_for_eth(
        amount_in=usdc_amount,
        amount_out_min=min_eth_out,
        path=swap_path,
        to=VITALIK_ADDRESS,
        deadline=deadline
    )
    
    swap_tx = {
        'from': VITALIK_ADDRESS,
        'to': UNISWAP_V2_ROUTER,
        'value': 0,
        'data': swap_data,
        'gas': 180000,
        'gas_price': 25_000_000_000,
        'nonce': 1501
    }
    
    try:
        swap_result = simulator.simulate_transaction(swap_tx, None)
        
        if swap_result.success:
            print(f"✅ Swap would succeed - Gas: {swap_result.gas_used:,}")
            
            total_gas = approve_result.gas_used + swap_result.gas_used
            total_gas_cost = total_gas * swap_tx['gas_price']
            
            print(f"\n📊 Complete operation summary:")
            print(f"   Approval gas: {approve_result.gas_used:,}")
            print(f"   Swap gas: {swap_result.gas_used:,}")
            print(f"   Total gas: {total_gas:,}")
            print(f"   Total gas cost: {total_gas_cost / 1e18:.6f} ETH")
            
        else:
            print(f"❌ Swap would fail: {swap_result.revert_reason}")
            
    except Exception as e:
        print(f"⚠️  Swap error: {str(e)[:60]}...")

def simulate_multi_hop_swap():
    """Simulate a multi-hop swap through multiple tokens"""
    
    print("\n" + "=" * 80)
    print("MULTI-HOP SWAP SIMULATION")
    print("=" * 80)
    print("Simulating ETH -> UNI -> DAI swap (two hops)")
    
    reth = pyreth.PyReth()
    simulator = reth.simulator()
    
    # Multi-hop path: ETH -> UNI -> DAI
    eth_amount = 2_000_000_000_000_000_000  # 2 ETH
    min_dai_out = 3000 * 10**18  # Expect at least 3000 DAI
    swap_path = [WETH_ADDRESS, UNI_ADDRESS, DAI_ADDRESS]
    deadline = get_future_timestamp()
    
    print(f"Multi-hop swap details:")
    print(f"  Amount in: {eth_amount / 1e18:.1f} ETH")
    print(f"  Min amount out: {min_dai_out / 1e18:.0f} DAI")
    print(f"  Path: ETH -> WETH -> UNI -> DAI")
    print(f"  Hops: 2 (ETH->UNI, UNI->DAI)")
    
    # Note: This is a simplified encoding - real multi-hop might need different function
    swap_data = encode_uniswap_v2_swap_exact_eth_for_tokens(
        amount_out_min=min_dai_out,
        path=swap_path,
        to=VITALIK_ADDRESS,
        deadline=deadline
    )
    
    multihop_tx = {
        'from': VITALIK_ADDRESS,
        'to': UNISWAP_V2_ROUTER,
        'value': eth_amount,
        'data': swap_data,
        'gas': 300000,  # Higher gas for multi-hop
        'gas_price': 30_000_000_000,  # 30 gwei
        'nonce': 1500
    }
    
    try:
        result = simulator.simulate_transaction(multihop_tx, None)
        
        if result.success:
            print(f"\n✅ Multi-hop swap would succeed!")
            print(f"   Gas used: {result.gas_used:,}")
            
            # Calculate efficiency vs single hops
            gas_per_hop = result.gas_used / 2
            print(f"   Gas per hop: {gas_per_hop:,.0f}")
            
            gas_cost = result.gas_used * multihop_tx['gas_price']
            total_cost = eth_amount + gas_cost
            print(f"   Total cost: {total_cost / 1e18:.6f} ETH")
            
            print(f"\n💡 Multi-hop provides access to more trading pairs")
            print(f"   but costs more gas than direct swaps")
            
        else:
            print(f"\n❌ Multi-hop swap would fail!")
            print(f"   Revert reason: {result.revert_reason}")
            print(f"   Gas wasted: {result.gas_used:,}")
            
            # Analyze failure
            if "return amount" in result.revert_reason.lower():
                print(f"   💡 Slippage too high - try reducing min_amount_out")
            elif "liquidity" in result.revert_reason.lower():
                print(f"   💡 Insufficient liquidity in one of the pairs")
                
    except Exception as e:
        print(f"⚠️  Multi-hop error: {str(e)[:80]}...")

def analyze_slippage_tolerance():
    """Analyze how different slippage tolerances affect swap success"""
    
    print("\n" + "=" * 80)
    print("SLIPPAGE TOLERANCE ANALYSIS")
    print("=" * 80)
    print("Testing swap success with different slippage tolerances")
    
    reth = pyreth.PyReth()
    simulator = reth.simulator()
    
    eth_amount = 5_000_000_000_000_000_000  # 5 ETH (large amount for slippage testing)
    
    # Test different slippage tolerances
    slippage_tests = [
        (0.1, "0.1% (very tight)"),
        (0.5, "0.5% (tight)"),
        (1.0, "1.0% (normal)"),
        (2.0, "2.0% (loose)"),
        (5.0, "5.0% (very loose)")
    ]
    
    print(f"Testing {eth_amount / 1e18:.0f} ETH -> USDC swap with different slippage:")
    print("-" * 65)
    
    # Estimated output (rough calculation for demo)
    estimated_usdc_out = 12000 * 10**6  # ~12,000 USDC for 5 ETH
    
    results = []
    
    for slippage_pct, description in slippage_tests:
        # Calculate min amount out based on slippage
        slippage_multiplier = 1 - (slippage_pct / 100)
        min_amount_out = int(estimated_usdc_out * slippage_multiplier)
        
        swap_data = encode_uniswap_v2_swap_exact_eth_for_tokens(
            amount_out_min=min_amount_out,
            path=[WETH_ADDRESS, USDC_ADDRESS],
            to=VITALIK_ADDRESS,
            deadline=get_future_timestamp()
        )
        
        tx = {
            'from': VITALIK_ADDRESS,
            'to': UNISWAP_V2_ROUTER,
            'value': eth_amount,
            'data': swap_data,
            'gas': 200000,
            'gas_price': 25_000_000_000,
            'nonce': 1500
        }
        
        print(f"\n{description}:")
        print(f"  Min USDC out: {min_amount_out / 1e6:,.0f}")
        
        try:
            result = simulator.simulate_transaction(tx, None)
            
            if result.success:
                print(f"  ✅ SUCCESS - Gas: {result.gas_used:,}")
                results.append((slippage_pct, True, result.gas_used))
            else:
                print(f"  ❌ FAILED - {result.revert_reason}")
                results.append((slippage_pct, False, result.gas_used))
                
        except Exception as e:
            print(f"  ⚠️  ERROR - {str(e)[:40]}...")
            results.append((slippage_pct, False, 0))
    
    # Analysis
    print(f"\n📊 Slippage Analysis Summary:")
    print("-" * 35)
    
    successful = [r for r in results if r[1]]
    failed = [r for r in results if not r[1]]
    
    if successful:
        min_working_slippage = min(r[0] for r in successful)
        print(f"✅ Minimum working slippage: {min_working_slippage}%")
        print(f"✅ {len(successful)}/{len(results)} slippage levels work")
    else:
        print(f"❌ No slippage tolerance worked for this trade size")
        print(f"💡 Try smaller trade size or check liquidity")
    
    if failed:
        print(f"❌ Failed slippages: {[r[0] for r in failed]}")

def simulate_sandwich_attack_scenario():
    """Simulate how a sandwich attack might affect a swap"""
    
    print("\n" + "=" * 80)
    print("SANDWICH ATTACK SCENARIO SIMULATION")
    print("=" * 80)
    print("Simulating swap behavior under MEV pressure")
    
    reth = pyreth.PyReth()
    simulator = reth.simulator()
    
    # Scenario: Large swap that could be sandwiched
    large_eth_amount = 50_000_000_000_000_000_000  # 50 ETH (very large)
    normal_slippage = 1.0  # 1% slippage tolerance
    
    print(f"Large swap simulation:")
    print(f"  Amount: {large_eth_amount / 1e18:.0f} ETH -> USDC")
    print(f"  Slippage tolerance: {normal_slippage}%")
    print(f"  Risk: High MEV/sandwich attack potential")
    
    # Calculate expected output with normal conditions
    estimated_usdc = 120000 * 10**6  # ~120k USDC
    min_usdc_normal = int(estimated_usdc * (1 - normal_slippage/100))
    
    # Test under different market conditions
    scenarios = [
        ("Normal conditions", min_usdc_normal, "Standard market conditions"),
        ("MEV competition", int(min_usdc_normal * 0.98), "2% additional slippage from MEV"),
        ("Heavy sandwich", int(min_usdc_normal * 0.95), "5% additional slippage from sandwich"),
        ("Extreme MEV", int(min_usdc_normal * 0.90), "10% total slippage from heavy MEV")
    ]
    
    for scenario_name, min_amount, description in scenarios:
        print(f"\n🎯 {scenario_name}:")
        print(f"   {description}")
        print(f"   Min USDC out: {min_amount / 1e6:,.0f}")
        
        swap_data = encode_uniswap_v2_swap_exact_eth_for_tokens(
            amount_out_min=min_amount,
            path=[WETH_ADDRESS, USDC_ADDRESS],
            to=VITALIK_ADDRESS,
            deadline=get_future_timestamp()
        )
        
        tx = {
            'from': VITALIK_ADDRESS,
            'to': UNISWAP_V2_ROUTER,
            'value': large_eth_amount,
            'data': swap_data,
            'gas': 200000,
            'gas_price': 100_000_000_000,  # 100 gwei (high to avoid being sandwiched)
            'nonce': 1500
        }
        
        try:
            result = simulator.simulate_transaction(tx, None)
            
            if result.success:
                actual_slippage = (estimated_usdc - min_amount) / estimated_usdc * 100
                print(f"   ✅ Would succeed with {actual_slippage:.1f}% effective slippage")
                print(f"   Gas used: {result.gas_used:,}")
            else:
                print(f"   ❌ Would fail: {result.revert_reason}")
                
        except Exception as e:
            print(f"   ⚠️  Error: {str(e)[:50]}...")
    
    print(f"\n💡 MEV Protection Strategies:")
    print("   • Use higher gas prices to get priority")
    print("   • Split large trades into smaller chunks")
    print("   • Use MEV-protected pools or protocols")
    print("   • Consider private mempools for large trades")

def main():
    """Run all swap simulation demonstrations"""
    
    print("🧪 PyReth DEX Swap Simulation Examples")
    print("🔄 Comprehensive DEX trading simulation and analysis")
    print("🔗 Using singleton pattern for efficient database access")
    
    try:
        # Basic ETH to token swap
        simulate_eth_to_token_swap()
        
        # Token to ETH swap (with approval)
        simulate_token_to_eth_swap()
        
        # Multi-hop swap
        simulate_multi_hop_swap()
        
        # Slippage tolerance analysis
        analyze_slippage_tolerance()
        
        # Sandwich attack scenario
        simulate_sandwich_attack_scenario()
        
        print("\n" + "=" * 80)
        print("✅ SWAP SIMULATION EXAMPLES COMPLETED")
        print("=" * 80)
        print("\n🎯 Key Takeaways:")
        print("  • DEX swaps can be simulated before execution")
        print("  • Multi-step operations (approve + swap) need sequential planning")
        print("  • Slippage tolerance directly affects swap success rates")
        print("  • Large swaps are vulnerable to MEV and sandwich attacks")
        print("  • Gas costs vary significantly between simple and complex swaps")
        print("  • Multi-hop swaps provide more trading pairs but cost more gas")
        print("  • Simulation helps optimize trade parameters before execution")
        
    except Exception as e:
        print(f"\n❌ Swap simulation examples failed: {e}")
        raise

if __name__ == "__main__":
    main()