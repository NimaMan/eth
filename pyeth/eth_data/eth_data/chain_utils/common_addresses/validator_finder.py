

# ============================================================================
# Validator Discovery Functions
# ============================================================================

import asyncio
import os
import sys
import json
from collections import defaultdict, Counter
from typing import Dict, Set, Tuple, List
from datetime import datetime
from pathlib import Path

try:
    from web3 import AsyncWeb3, AsyncHTTPProvider
    from web3.exceptions import BlockNotFound
except ImportError:
    print("Error: web3 not installed. Run: pip install web3")
    AsyncWeb3 = None


async def discover_validators_from_recent_blocks(num_blocks: int = 10000) -> Tuple[Dict[str, int], int, int]:
    """
    Discover all fee recipients (validators/builders) from recent blocks.
    
    Args:
        num_blocks: Number of recent blocks to analyze (default: 10000)
    
    Returns:
        Tuple of (validator_counts, start_block, end_block)
    """
    if AsyncWeb3 is None:
        print("Error: web3 not available")
        return {}, 0, 0
    
    # Get RPC URL from environment or use default
    node_url = os.getenv('ETH_RPC_URL', 'http://localhost:8545')
    print(f"Connecting to Ethereum node: {node_url}")
    
    # Initialize async web3
    w3 = AsyncWeb3(AsyncHTTPProvider(node_url))
    
    # Check connection
    try:
        is_connected = await w3.is_connected()
        if not is_connected:
            print("Error: Cannot connect to Ethereum node")
            return {}, 0, 0
    except Exception as e:
        print(f"Error connecting to node: {e}")
        return {}, 0, 0
    
    # Get latest block number
    try:
        latest_block = await w3.eth.block_number
        print(f"Latest block: {latest_block:,}")
    except Exception as e:
        print(f"Error getting latest block: {e}")
        return {}, 0, 0
    
    # Calculate block range
    start_block = max(0, latest_block - num_blocks + 1)
    end_block = latest_block
    
    print(f"Analyzing blocks {start_block:,} to {end_block:,} ({num_blocks:,} blocks)")
    print("Fetching block data...")
    
    # Track fee recipients
    fee_recipient_counts = Counter()
    blocks_processed = 0
    blocks_with_errors = 0
    
    # Process blocks in batches for better performance
    batch_size = 100
    
    for batch_start in range(start_block, end_block + 1, batch_size):
        batch_end = min(batch_start + batch_size - 1, end_block)
        batch_blocks = batch_end - batch_start + 1
        
        # Progress indicator
        progress = (blocks_processed / num_blocks) * 100
        print(f"Progress: {progress:.1f}% ({blocks_processed}/{num_blocks} blocks)", end='\r')
        
        # Fetch blocks in batch
        tasks = []
        for block_num in range(batch_start, batch_end + 1):
            tasks.append(w3.eth.get_block(block_num))
        
        try:
            blocks = await asyncio.gather(*tasks, return_exceptions=True)
            
            for block in blocks:
                if isinstance(block, Exception):
                    blocks_with_errors += 1
                    continue
                
                if block and 'miner' in block:
                    # Get fee recipient (miner/validator)
                    miner = block['miner']
                    # Convert to checksum address
                    checksum_addr = w3.to_checksum_address(miner)
                    fee_recipient_counts[checksum_addr] += 1
                
                blocks_processed += 1
        
        except Exception as e:
            print(f"\nError processing batch {batch_start}-{batch_end}: {e}")
            blocks_with_errors += batch_blocks
    
    print(f"\nProcessing complete!")
    print(f"Blocks processed: {blocks_processed:,}")
    if blocks_with_errors > 0:
        print(f"Blocks with errors: {blocks_with_errors:,}")
    
    return dict(fee_recipient_counts), start_block, end_block


def compare_with_existing(discovered: Dict[str, int]) -> Tuple[Set[str], Set[str], Set[str]]:
    """
    Compare discovered validators with existing fee_recipients.
    
    Returns:
        Tuple of (known_validators, new_validators, all_validators)
    """
    discovered_set = set(discovered.keys())
    existing_set = set(fee_recipients.keys())
    
    known = discovered_set & existing_set
    new = discovered_set - existing_set
    
    return known, new, discovered_set


def generate_update_code(new_validators: Dict[str, int]) -> str:
    """
    Generate Python code to add new validators to fee_recipients.
    
    Args:
        new_validators: Dictionary of new validator addresses and their counts
    
    Returns:
        Python code string to add to fee_recipients
    """
    if not new_validators:
        return "# No new validators found"
    
    code_lines = ["# New validators discovered (add to fee_recipients):"]
    
    # Sort by frequency (most active first)
    sorted_validators = sorted(new_validators.items(), key=lambda x: x[1], reverse=True)
    
    for addr, count in sorted_validators:
        # Generate default name for unknown builders
        name = f"MEV Builder: {addr[:5]}...{addr[-3:]}"
        
        # Check against known validators (using checksum addresses)
        if addr == '0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5':
            name = 'beaverbuild'
        elif addr == '0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5':
            name = 'Flashbots: Builder'
        elif addr == '0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97':
            name = 'Titan Builder'
        # Add more pattern matching as needed
        
        code_lines.append(f"    '{addr}': '{name}',  # {count} blocks")
    
    return '\n'.join(code_lines)


def save_new_validators_to_file(
    new_validators: Dict[str, int], 
    discovered: Dict[str, int],
    start_block: int,
    end_block: int,
    num_blocks: int
) -> str:
    """
    Save newly discovered validators to a timestamped file.
    
    Args:
        new_validators: Dictionary of new validator addresses and their counts
        discovered: All discovered validators with counts
        start_block: Starting block number analyzed
        end_block: Ending block number analyzed
        num_blocks: Total number of blocks analyzed
    
    Returns:
        Path to the saved file
    """
    if not new_validators:
        return None
    
    # Generate timestamp for filename
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    
    # Get directory of this validators.py file
    current_dir = Path(__file__).parent
    filename = current_dir / f"new_validators_{timestamp}.py"
    
    # Prepare file content
    lines = [
        f"# New validators discovered on {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        f"# Analyzed {num_blocks:,} blocks ({start_block:,} to {end_block:,})",
        f"# Found {len(new_validators)} new validators/builders",
        "",
        "# Add these entries to the fee_recipients dictionary in validators.py:",
        "",
        "new_validators = {",
    ]
    
    # Sort by frequency (most active first)
    sorted_validators = sorted(new_validators.items(), key=lambda x: x[1], reverse=True)
    
    for addr, count in sorted_validators:
        # Generate default name for unknown builders
        name = f"MEV Builder: {addr[:5]}...{addr[-3:]}"
        
        # Check against known validators (using checksum addresses)
        if addr == '0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5':
            name = 'beaverbuild'
        elif addr == '0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5':
            name = 'Flashbots: Builder'
        elif addr == '0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97':
            name = 'Titan Builder'
        
        lines.append(f"    '{addr}': '{name}',")
    
    lines.append("}")
    lines.append("")
    lines.append("# To apply these updates:")
    lines.append("# 1. Review the addresses above")
    lines.append("# 2. Copy the entries you want to add")
    lines.append("# 3. Paste them into the fee_recipients dictionary in validators.py")
    lines.append("")
    lines.append("# Or apply all at once by adding this to validators.py:")
    lines.append("# fee_recipients.update(new_validators)")
    
    # Write to file
    content = "\n".join(lines)
    filename.write_text(content)
    
    return str(filename)


def print_statistics(discovered: Dict[str, int]):
    """Print detailed statistics about discovered validators."""
    known, new, all_validators = compare_with_existing(discovered)
    
    print("\n" + "="*70)
    print("VALIDATOR DISCOVERY STATISTICS")
    print("="*70)
    
    print(f"\n📊 Summary:")
    print(f"  Total unique validators found: {len(all_validators)}")
    print(f"  Already in our list: {len(known)}")
    print(f"  NEW validators discovered: {len(new)}")
    
    if discovered:
        # Top 10 most active validators
        print(f"\n🏆 Top 10 Most Active Validators:")
        sorted_validators = sorted(discovered.items(), key=lambda x: x[1], reverse=True)[:10]
        
        for i, (addr, count) in enumerate(sorted_validators, 1):
            name = fee_recipients.get(addr, "UNKNOWN - NEW VALIDATOR")
            percentage = (count / sum(discovered.values())) * 100
            print(f"  {i:2}. {addr}: {count:5} blocks ({percentage:5.2f}%)")
            print(f"      Name: {name}")
    
    if new:
        print(f"\n🆕 NEW Validators Not In Our List ({len(new)}):")
        new_dict = {addr: discovered[addr] for addr in new}
        sorted_new = sorted(new_dict.items(), key=lambda x: x[1], reverse=True)
        
        for addr, count in sorted_new[:20]:  # Show top 20 new ones
            percentage = (count / sum(discovered.values())) * 100
            print(f"  {addr}: {count:5} blocks ({percentage:5.2f}%)")
        
        if len(new) > 20:
            print(f"  ... and {len(new) - 20} more")
        
        # Generate update code
        print("\n" + "="*70)
        print("CODE TO ADD NEW VALIDATORS:")
        print("="*70)
        print(generate_update_code(new_dict))
    else:
        print("\n✅ All discovered validators are already in our list!")
    
    # Check for inactive validators in our list
    if known:
        inactive = set(fee_recipients.keys()) - set(discovered.keys())
        if inactive:
            print(f"\n⚠️  Validators in our list but NOT seen in recent blocks: {len(inactive)}")
            for addr in list(inactive)[:10]:
                print(f"  {addr}: {fee_recipients[addr]}")
            if len(inactive) > 10:
                print(f"  ... and {len(inactive) - 10} more")


async def main(num_blocks: int = 10000):
    """Main function to run validator discovery."""
    print("🔍 Ethereum Validator/Builder Discovery Tool")
    print("="*70)
    
    # Discover validators
    discovered, start_block, end_block = await discover_validators_from_recent_blocks(num_blocks)
    
    if discovered:
        # Print statistics and generate update code
        print_statistics(discovered)
        
        # Check for new validators and save to file if found
        known, new, all_validators = compare_with_existing(discovered)
        if new:
            new_dict = {addr: discovered[addr] for addr in new}
            saved_file = save_new_validators_to_file(
                new_dict, 
                discovered, 
                start_block, 
                end_block, 
                num_blocks
            )
            if saved_file:
                print("\n" + "="*70)
                print("📁 AUTO-SAVE COMPLETE")
                print("="*70)
                print(f"✅ New validators saved to: {saved_file}")
                print("You can review this file and apply the updates to validators.py")
        else:
            print("\n✅ No new validators to save - all discovered validators are already in our list!")
    else:
        print("\n❌ No validators discovered. Check your Ethereum node connection.")


if __name__ == "__main__":
    # Parse command line arguments
    import argparse
    
    parser = argparse.ArgumentParser(description='Discover Ethereum validators/builders from recent blocks')
    parser.add_argument('--blocks', type=int, default=10000,
                        help='Number of recent blocks to analyze (default: 10000)')
    
    args = parser.parse_args()
    
    # Run discovery
    asyncio.run(main(args.blocks))
