use alloy_primitives::{keccak256, Address, B256, U256};
/// Find Whitelisted Addresses
///
/// This example demonstrates how to find whitelisted/excluded addresses for any token
/// by analyzing storage slots and successful transactions.
///
/// METHODOLOGY:
/// 1. Analyze recent successful sell transactions to identify potential whitelisted addresses
/// 2. Check storage slots for these addresses to find the exclusion mapping
/// 3. Verify by testing buy/sell simulations with identified addresses
/// 4. Scan for other addresses with the same storage pattern
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::{BlockId, Filter, TransactionRequest};
use alloy_sol_types::{SolCall, SolValue};
use eyre::Result;
use reth_tx_simulator::{CallRequest, RethTxSimulator};
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

// ERC20 interface for checking transfers
alloy_sol_types::sol! {
    event Transfer(address indexed from, address indexed to, uint256 value);

    function balanceOf(address account) external view returns (uint256);
    function decimals() external view returns (uint8);
    function symbol() external view returns (string);
    function name() external view returns (string);
}

/// Calculate storage slot for mapping(address => bool) or mapping(address => uint256)
fn calculate_mapping_slot(address: Address, base_slot: u64) -> U256 {
    let mut data = [0u8; 64];
    // Address is left-padded to 32 bytes
    data[12..32].copy_from_slice(address.as_slice());
    // Slot number is right-aligned in the second 32 bytes
    data[56..64].copy_from_slice(&base_slot.to_be_bytes());
    let hash = keccak256(&data);
    U256::from_be_bytes(hash.0)
}

/// Find successful sellers from recent transactions
async fn find_successful_sellers(
    provider: &impl Provider,
    token: Address,
    pool: Address,
    from_block: u64,
    to_block: u64,
) -> Result<HashSet<Address>> {
    println!(
        "📊 Finding successful sellers from blocks {} to {}",
        from_block, to_block
    );

    let mut sellers = HashSet::new();

    // Get Transfer events from token to pool (sells)
    let filter = Filter::new()
        .address(token)
        .from_block(from_block)
        .to_block(to_block)
        .event("Transfer(address,address,uint256)");

    let logs = provider.get_logs(&filter).await?;

    for log in logs {
        if log.topics().len() >= 3 {
            // Transfer event: topics[0] = event sig, topics[1] = from, topics[2] = to
            let to_address = Address::from_slice(&log.topics()[2].as_slice()[12..32]);

            // If transfer is TO the pool, it's a sell
            if to_address == pool {
                let from_address = Address::from_slice(&log.topics()[1].as_slice()[12..32]);
                sellers.insert(from_address);
                println!("   Found seller: {:?}", from_address);
            }
        }
    }

    println!("   Total unique sellers found: {}", sellers.len());
    Ok(sellers)
}

/// Check which storage slot contains the whitelist mapping
async fn find_whitelist_slot(
    provider: &impl Provider,
    token: Address,
    test_addresses: &[Address],
    block: u64,
) -> Result<Option<u64>> {
    println!("\n🔍 Searching for whitelist storage slot...");

    // Common slots for whitelist/exclusion mappings
    let possible_slots: Vec<u64> = (0..30).collect();
    let mut slot_hits: HashMap<u64, Vec<(Address, U256)>> = HashMap::new();

    for addr in test_addresses {
        for slot_num in &possible_slots {
            let storage_slot = calculate_mapping_slot(*addr, *slot_num);

            let value = provider
                .get_storage_at(token, storage_slot)
                .block_id(BlockId::Number(block.into()))
                .await?;

            if !value.is_zero() {
                slot_hits
                    .entry(*slot_num)
                    .or_insert_with(Vec::new)
                    .push((*addr, value));
            }
        }
    }

    // Find the slot that has the most consistent pattern
    let mut best_slot = None;
    let mut best_score = 0;

    for (slot_num, addresses) in &slot_hits {
        // Check if values are booleans (1) or small numbers
        let boolean_count = addresses
            .iter()
            .filter(|(_, v)| *v == U256::from(1))
            .count();

        println!(
            "   Slot {}: {} addresses with data ({} boolean TRUE)",
            slot_num,
            addresses.len(),
            boolean_count
        );

        // Prefer slots with boolean values
        if boolean_count > best_score {
            best_score = boolean_count;
            best_slot = Some(*slot_num);
        }
    }

    if let Some(slot) = best_slot {
        println!("   ✅ Likely whitelist slot: {}", slot);
    } else {
        println!("   ⚠️  No clear whitelist slot found");
    }

    Ok(best_slot)
}

/// Verify if an address can sell by simulation
async fn verify_can_sell(
    simulator: &RethTxSimulator,
    token: Address,
    pool: Address,
    seller: Address,
    block: u64,
) -> Result<bool> {
    // Use the sequential simulator approach from our previous examples
    use alloy_sol_types::SolCall;

    alloy_sol_types::sol! {
        function approve(address spender, uint256 amount) external returns (bool);
        function swapExactTokensForETHSupportingFeeOnTransferTokens(
            uint256 amountIn,
            uint256 amountOutMin,
            address[] calldata path,
            address to,
            uint256 deadline
        ) external;
    }

    let router = Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D")?; // Uniswap V2
    let weth = Address::from_str("0xC02aAA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?;

    // Test amount - small amount to test
    let test_amount = U256::from(1000000000u64); // Small amount

    // Try to approve and sell
    let approve_calldata = approveCall {
        spender: router,
        amount: test_amount,
    }
    .abi_encode();

    let approve_request = CallRequest {
        from: Some(seller),
        to: Some(token),
        value: None,
        data: Some(approve_calldata.into()),
        gas: Some(100000),
        gas_price: Some(30_000_000_000),
        max_fee_per_gas: None,
        max_priority_fee_per_gas: None,
        nonce: None,
    };

    // First check if approve works
    match simulator
        .simulate_unsigned_transaction_at_block(approve_request, block)
        .await
    {
        Ok(result) if result.success => {
            // Now try sell
            let path = vec![token, weth];
            let sell_calldata = swapExactTokensForETHSupportingFeeOnTransferTokensCall {
                amountIn: test_amount,
                amountOutMin: U256::ZERO,
                path,
                to: seller,
                deadline: U256::from(9999999999u64),
            }
            .abi_encode();

            let sell_request = CallRequest {
                from: Some(seller),
                to: Some(router),
                value: None,
                data: Some(sell_calldata.into()),
                gas: Some(300000),
                gas_price: Some(30_000_000_000),
                max_fee_per_gas: None,
                max_priority_fee_per_gas: None,
                nonce: None,
            };

            match simulator
                .simulate_unsigned_transaction_at_block(sell_request, block)
                .await
            {
                Ok(sell_result) => Ok(sell_result.success),
                Err(_) => Ok(false),
            }
        }
        _ => Ok(false),
    }
}

/// Scan for all whitelisted addresses given a storage slot
async fn scan_all_whitelisted(
    provider: &impl Provider,
    token: Address,
    whitelist_slot: u64,
    known_whitelisted: &[Address],
    block: u64,
) -> Result<Vec<Address>> {
    println!("\n🔍 Scanning for additional whitelisted addresses...");

    // This is more complex - would need to either:
    // 1. Scan all transaction history for unique addresses that interacted
    // 2. Use event logs to find all addresses that received tokens
    // 3. Check common known addresses (exchanges, routers, etc.)

    let mut all_whitelisted = Vec::new();

    // Check some common addresses that might be whitelisted
    let common_addresses = vec![
        "0x000000000000000000000000000000000000dEaD", // Burn address
        "0x0000000000000000000000000000000000000000", // Zero address
    ];

    for addr_str in common_addresses {
        if let Ok(addr) = Address::from_str(addr_str) {
            let storage_slot = calculate_mapping_slot(addr, whitelist_slot);
            let value = provider
                .get_storage_at(token, storage_slot)
                .block_id(BlockId::Number(block.into()))
                .await?;

            if !value.is_zero() {
                all_whitelisted.push(addr);
                println!("   Found whitelisted: {:?}", addr);
            }
        }
    }

    // Add known whitelisted
    for addr in known_whitelisted {
        if !all_whitelisted.contains(addr) {
            all_whitelisted.push(*addr);
        }
    }

    Ok(all_whitelisted)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!(
            "Usage: {} <token_address> <pool_address> [block_number]",
            args[0]
        );
        println!("\nExample for DOGEINA:");
        println!("  {} 0xF2e24D564a08A3Acc31985eBD3C6Ee7FA9943a84 0x5423621C6D3E22465876deb92aD4f40Dc25b62A3", args[0]);
        return Ok(());
    }

    let token = Address::from_str(&args[1])?;
    let pool = Address::from_str(&args[2])?;
    let block = if args.len() > 3 {
        args[3].parse()?
    } else {
        // Use a recent block
        22939570u64
    };

    println!("🔍 Finding Whitelisted Addresses for Token");
    println!("{}", "=".repeat(60));
    println!("Token: {:?}", token);
    println!("Pool: {:?}", pool);
    println!("Block: {}", block);

    let provider = ProviderBuilder::new().connect_http("http://localhost:8545".parse()?);

    // Step 1: Find recent successful sellers
    let from_block = block.saturating_sub(1000); // Look back 1000 blocks
    let sellers = find_successful_sellers(&provider, token, pool, from_block, block).await?;

    if sellers.is_empty() {
        println!("\n⚠️  No sellers found in recent blocks");
        return Ok(());
    }

    // Step 2: Find the whitelist storage slot
    let test_addresses: Vec<Address> = sellers.iter().cloned().collect();
    let whitelist_slot = find_whitelist_slot(&provider, token, &test_addresses, block).await?;

    if let Some(slot) = whitelist_slot {
        // Step 3: Check each seller's storage
        println!("\n📊 Checking storage for each seller at slot {}:", slot);

        let mut whitelisted = Vec::new();
        let mut non_whitelisted = Vec::new();

        for seller in &sellers {
            let storage_slot = calculate_mapping_slot(*seller, slot);
            let value = provider
                .get_storage_at(token, storage_slot)
                .block_id(BlockId::Number(block.into()))
                .await?;

            if !value.is_zero() {
                whitelisted.push(*seller);
                println!("   ✅ {:?} = {} (WHITELISTED)", seller, value);
            } else {
                non_whitelisted.push(*seller);
                println!("   ❌ {:?} = 0 (not whitelisted?)", seller);
            }
        }

        // Step 4: Verify with simulation (optional, requires reth)
        if std::env::var("VERIFY_WITH_SIMULATION").is_ok() {
            println!("\n🔍 Verifying with transaction simulation...");

            let reth_datadir = std::env::var("RETH_DATADIR")
                .unwrap_or_else(|_| "/home/nima/.local/share/reth/mainnet".to_string());
            let simulator = RethTxSimulator::new(&reth_datadir)?;

            for addr in &whitelisted[..whitelisted.len().min(3)] {
                let can_sell = verify_can_sell(&simulator, token, pool, *addr, block).await?;
                println!("   {:?} can sell: {}", addr, can_sell);
            }
        }

        // Step 5: Scan for additional whitelisted addresses
        let all_whitelisted =
            scan_all_whitelisted(&provider, token, slot, &whitelisted, block).await?;

        // Final summary
        println!("\n{}", "=".repeat(60));
        println!("📊 SUMMARY");
        println!(
            "\n🔓 Whitelisted Addresses Found: {}",
            all_whitelisted.len()
        );
        for addr in &all_whitelisted {
            println!("   {:?}", addr);
        }

        println!("\n💡 HOW TO USE THIS INFORMATION:");
        println!("   1. Storage slot {} contains the whitelist mapping", slot);
        println!(
            "   2. Check any address: keccak256(address . {}) -> storage value",
            slot
        );
        println!("   3. Non-zero value = whitelisted, can trade freely");
        println!("   4. Zero value = restricted, subject to taxes/limits");

        println!("\n⚠️  DETECTION METHODS:");
        println!("   - Monitor storage changes when addresses are added/removed");
        println!("   - Watch for events (though many tokens don't emit them)");
        println!("   - Test buy/sell simulations from different addresses");
        println!("   - Analyze successful transactions to find patterns");
    } else {
        println!("\n⚠️  Could not determine whitelist storage slot");
        println!("   The token might use a different mechanism:");
        println!("   - Complex access control (not simple mapping)");
        println!("   - Time-based restrictions");
        println!("   - Dynamic conditions");
    }

    Ok(())
}
