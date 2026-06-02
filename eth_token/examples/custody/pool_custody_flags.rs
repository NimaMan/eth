//! Demonstrate pool-state flags for custody-aware risk labeling.
//!
//! This is an in-memory example. It does not require Reth and it separates two
//! states that must not be conflated:
//! - observed holder-balance drain: realized custody risk, terminal position
//!   risk, but not necessarily reserve liquidity removal;
//! - USDT-style centralized custody authority: latent custody risk, not a scam
//!   label and not terminal while trading/liquidity remain healthy.
//!
//! Run:
//!   cargo run -p eth_token --example pool_custody_flags

use alloy_primitives::{address, Address, B256, U256};
use eth_token::{
    custody::{CustodyCapability, CustodyFinding, CustodyState},
    erc20::{ERC20Token, ERC20TokenMetadata},
    pools::{BasePoolConfig, PoolStateFlags},
};
use eyre::Result;
use serde_json::json;
use tx_processor::{Erc20CallKind, InternalErc20Call, InternalErc20Transfer, ProcessedTransaction};

const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const ROUTER: &str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";

const SESSION_TOKEN: &str = "0xc3A640bD249381F8097f44C1b61C46172068cDff";
const SESSION_POOL: &str = "0xc3A640bD249381F8097f44C1b61C4617201adc1a";
const TRADING_VAULT: Address = address!("0x28474cbCd780AeEb3ED1501B68254bEd87cF5597");
const DEAD: Address = address!("0x000000000000000000000000000000000000dEaD");
const DRAIN_HELPER: Address = address!("0xD8e11826e82619bf49C58c05F26C8e00B0B64eA4");

const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
const USDT_POOL: &str = "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852";

fn main() -> Result<()> {
    let session_flags = session_holder_balance_drain_flags()?;
    let usdt_flags = usdt_style_latent_custody_flags()?;

    println!("=== observed holder-balance drain ===");
    println!("{}", serde_json::to_string_pretty(&session_flags)?);
    println!();
    println!("=== usdt-style latent custody authority ===");
    println!("{}", serde_json::to_string_pretty(&usdt_flags)?);

    Ok(())
}

fn session_holder_balance_drain_flags() -> Result<PoolStateFlags> {
    let mut token = ERC20Token::new(ERC20TokenMetadata::new(
        SESSION_TOKEN,
        "Session Case",
        "SESSION",
        18,
        "1000000000000000000000000000",
    ));
    add_weth_v2_pool(
        &mut token,
        SESSION_POOL,
        3_000_000_000.0,
        1.1,
        25_202_410,
        1_779_802_150,
    );

    let mut tx = ProcessedTransaction::new(
        B256::from([0x42; 32]),
        25_202_411,
        1_779_802_165,
        0,
        address!("0x02467dd05200e5B3FBcdddbF43735aE4d0681a59"),
        Some(DRAIN_HELPER),
        U256::ZERO,
        true,
        0,
        2,
        Vec::new(),
    );
    tx.internal_erc20_calls.push(InternalErc20Call {
        token_address: address!("0xc3A640bD249381F8097f44C1b61C46172068cDff"),
        caller: DRAIN_HELPER,
        kind: Erc20CallKind::TransferFrom,
        from_address: TRADING_VAULT,
        to_address: DEAD,
        amount: U256::from(9_871_487_343_970_612_u128),
        depth: 1,
        call_type: Some("CALL".to_string()),
        succeeded: true,
    });
    tx.internal_erc20_transfers =
        InternalErc20Transfer::event_less_complement(&tx.internal_erc20_calls, &tx.erc20_transfers);
    tx.refresh_trace_derived_indexes();

    token.update_token_state_from_processed_transaction(&tx)?;

    Ok(token
        .current_pool_state_flags()
        .remove(&SESSION_POOL.to_ascii_lowercase())
        .expect("example pool inserted"))
}

fn usdt_style_latent_custody_flags() -> Result<PoolStateFlags> {
    let mut token = ERC20Token::new(ERC20TokenMetadata::new(USDT, "Tether USD", "USDT", 6, "0"));
    add_weth_v2_pool(
        &mut token,
        USDT_POOL,
        100_000_000.0,
        30_000.0,
        25_202_410,
        1_779_802_150,
    );
    token.custody_findings.push(CustodyFinding {
        capability: CustodyCapability::Freeze,
        state: CustodyState::Latent,
        block_number: None,
        evidence: json!({
            "source": "example",
            "detail": "USDT-style blacklist/freezer authority exists, but no holder drain was observed",
        }),
    });
    token.custody_findings.push(CustodyFinding {
        capability: CustodyCapability::BurnDrain,
        state: CustodyState::Latent,
        block_number: None,
        evidence: json!({
            "source": "example",
            "detail": "centralized destroy/blacklist-style custody power is latent, not realized",
        }),
    });

    Ok(token
        .current_pool_state_flags()
        .remove(&USDT_POOL.to_ascii_lowercase())
        .expect("example pool inserted"))
}

fn add_weth_v2_pool(
    token: &mut ERC20Token,
    pool_address: &str,
    token_reserve: f64,
    weth_reserve: f64,
    block_number: u64,
    timestamp: u64,
) {
    let pool = token.create_uniswap_v2_pool(
        pool_address,
        WETH,
        BasePoolConfig {
            denom_decimals: Some(18),
            token1_is_denom: Some(true),
            history_limit: 64,
            ..BasePoolConfig::new(token.decimals)
        },
        [ROUTER],
    );
    pool.base.update_reserves(
        token_reserve,
        weth_reserve,
        block_number,
        timestamp,
        "0xsync",
    );
    pool.base
        .set_simulated_buy_status(true, block_number, "0xbuy", timestamp);
    pool.base
        .set_simulated_sell_status(true, Some(0.0), Some(0.0), block_number, "0xsell");
    pool.base.latest_block_number = Some(block_number);
}
