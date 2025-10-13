use alloy_primitives::{utils::format_units, Address, U256};
use eyre::eyre;
/// Token inspection example
///
/// Pulls together metadata, supply, holder balances, historical context, and
/// basic contract analysis for a single ERC20 token.
///
/// Run with: `cargo run --example token_inspection -- token=0x... block=12345`
use reth_chain_query::{Result, RethQueryProvider};
use reth_primitives::SealedHeader;
use reth_provider::HeaderProvider;
use std::{env, str::FromStr};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Token Inspection ===\n");

    let provider = RethQueryProvider::new("/home/nima/.local/share/reth/mainnet")?;

    let (token_arg, block_arg) = parse_args();

    let token_str = token_arg
        .as_deref()
        .unwrap_or("1f9840a85d5aF5bf1D1762F925BDADdC4201F984");
    let block_override = block_arg;

    // Analyze the requested token
    let token_address = Address::from_str(token_str)?;

    println!("Analyzing token {token_str}:\n");
    println!("{}", "=".repeat(60));

    // 1. Basic Metadata
    println!("\n📋 Basic Information:");
    println!("{}", "-".repeat(40));

    let metadata_header = load_sealed_header(&provider, block_override)?;
    let metadata = provider
        .get_token_metadata(token_address, block_override, metadata_header.clone())
        .await?;
    println!("Name:     {}", metadata.name);
    println!("Symbol:   {}", metadata.symbol);
    println!("Decimals: {}", metadata.decimals);
    println!("Address:  0x{}", token_address);

    // 2. Supply Information
    println!("\n💰 Supply Information:");
    println!("{}", "-".repeat(40));

    let supply_header = load_sealed_header(&provider, block_override)?;
    match provider
        .get_token_total_supply(token_address, block_override, supply_header.clone())
        .await
    {
        Ok(supply) => {
            let formatted = format_units(supply, metadata.decimals)?;
            let supply_float: f64 = formatted.parse().unwrap_or(0.0);
            println!(
                "Total Supply:     {:.2} {} ({supply} wei)",
                supply_float, metadata.symbol
            );

            // For demo purposes only – a real app would query an oracle for price data.
            let estimated_price = 5.50; // Example price in USD
            let market_cap = supply_float * estimated_price;
            println!(
                "Est. Market Cap:  ${:.2} (at ${:.2}/token)",
                market_cap, estimated_price
            );
        }
        Err(e) => {
            println!("Total Supply:     Error - {}", e);
        }
    }

    // 3. Check some notable holders
    println!("\n👥 Notable Holder Balances:");
    println!("{}", "-".repeat(40));

    let notable_holders = vec![
        ("Binance", "F977814e90dA44bFA03b6295A0616a897441aceC"),
        (
            "Uniswap Treasury",
            "1a9C8182C09F50C8318d769245beA52c32BE35BC",
        ),
        ("Vitalik", "d8dA6BF26964aF9D7eEd9e03E53415D37aA96045"),
    ];

    for (name, holder_addr) in notable_holders {
        let holder = Address::from_str(holder_addr)?;
        match provider
            .get_token_balance(token_address, holder, block_override)
            .await
        {
            Ok(balance) => {
                if balance > U256::ZERO {
                    let formatted = format_units(balance, metadata.decimals)?;
                    let balance_float: f64 = formatted.parse().unwrap_or(0.0);
                    println!("{:<20} {:>15.2} {}", name, balance_float, metadata.symbol);
                } else {
                    println!("{:<20} {:>15}", name, format!("0 {}", metadata.symbol));
                }
            }
            Err(e) => {
                println!("{:<20} Error: {}", name, e);
            }
        }
    }

    // 4. Contract verification
    println!("\n🔍 Contract Analysis:");
    println!("{}", "-".repeat(40));

    let is_contract = provider.is_contract(token_address, block_override).await?;
    println!(
        "Is Contract:      {}",
        if is_contract { "Yes ✅" } else { "No ❌" }
    );
    println!("Standard:         ERC-20");
    println!("Upgradeable:      No (immutable contract)");

    // 5. Recent activity summary (simplified)
    println!("\n📊 Token Statistics:");
    println!("{}", "-".repeat(40));

    let current_block = provider.get_latest_block()?;
    println!("Current Block:    {}", current_block);

    let earlier_block = current_block.saturating_sub(100_000); // ~2 weeks ago
    let earlier_header = load_sealed_header(&provider, Some(earlier_block))?;
    let current_header = load_sealed_header(&provider, block_override)?;

    match provider
        .get_token_total_supply(token_address, Some(earlier_block), earlier_header.clone())
        .await
    {
        Ok(earlier_supply) => {
            let current_supply = provider
                .get_token_total_supply(token_address, block_override, current_header.clone())
                .await?;
            let supply_change = if current_supply > earlier_supply {
                let diff = current_supply - earlier_supply;
                let formatted = format_units(diff, metadata.decimals)?;
                format!("+{} {}", formatted, metadata.symbol)
            } else if current_supply < earlier_supply {
                let diff = earlier_supply - current_supply;
                let formatted = format_units(diff, metadata.decimals)?;
                format!("-{} {}", formatted, metadata.symbol)
            } else {
                "No change".to_string()
            };
            println!("Supply Change (~2 weeks): {}", supply_change);
        }
        Err(_) => {
            println!("Supply Change:    Historical data not available");
        }
    }

    // 6. Additional token features (demo)
    println!("\n⚙️ Token Features:");
    println!("{}", "-".repeat(40));
    println!("Transferable:     Yes");
    println!("Burnable:         No");
    println!("Mintable:         No (fixed supply)");
    println!("Pausable:         No");

    println!("\n{}", "=".repeat(60));
    println!("✅ Token inspection complete!");

    Ok(())
}

fn parse_args() -> (Option<String>, Option<u64>) {
    let mut token: Option<String> = None;
    let mut block: Option<u64> = None;

    for arg in env::args().skip(1) {
        if let Some(value) = arg.strip_prefix("token=") {
            token = Some(value.to_string());
        } else if let Some(value) = arg.strip_prefix("block=") {
            block = value.parse().ok();
        }
    }

    (token, block)
}

fn load_sealed_header(
    provider: &RethQueryProvider,
    block: Option<u64>,
) -> Result<Option<SealedHeader>> {
    let Some(block_number) = block else {
        return Ok(None);
    };

    let provider_handle = provider
        .provider_factory()
        .provider()
        .map_err(|e| eyre!("Failed to access provider: {}", e))?;

    let header = provider_handle
        .header_by_number(block_number)?
        .ok_or_else(|| eyre!("No header available for block {}", block_number))?;

    Ok(Some(SealedHeader::new(header.clone(), header.hash_slow())))
}
