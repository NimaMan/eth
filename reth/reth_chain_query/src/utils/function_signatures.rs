//! Shared function and event signature metadata used across the stack.
//!
//! These tables replace the historical Python-only definitions so both Rust
//! and Python read from a single canonical source.

use alloy_primitives::{keccak256, B256};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

fn event_topic(signature: &str) -> B256 {
    keccak256(signature.as_bytes())
}

fn insert_literal_selector<'a>(
    map: &mut HashMap<String, &'a str>,
    selector_hex: &str,
    label: &'a str,
) {
    map.insert(selector_hex.to_lowercase(), label);
}

pub static FUNCTION_SIGNATURES: Lazy<HashMap<String, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();

    insert_literal_selector(&mut map, "06fdde03", "Name");
    insert_literal_selector(&mut map, "95d89b41", "Symbol");
    insert_literal_selector(&mut map, "313ce567", "Decimals");
    insert_literal_selector(&mut map, "18160ddd", "Total Supply");
    insert_literal_selector(&mut map, "70a08231", "Balance Of");
    insert_literal_selector(&mut map, "dd62ed3e", "Allowance");
    insert_literal_selector(&mut map, "39509351", "Increase Allowance");
    insert_literal_selector(&mut map, "a457c2d7", "Decrease Allowance");
    insert_literal_selector(&mut map, "022c0d9f", "Uniswap Swap");
    insert_literal_selector(&mut map, "791ac947", "Swap");
    insert_literal_selector(&mut map, "04e45aaf", "Uniswap V3: Router 2");
    insert_literal_selector(&mut map, "38ed1739", "Swap");
    insert_literal_selector(&mut map, "8803dbee", "Swap");
    insert_literal_selector(&mut map, "7ff36ab5", "Swap");
    insert_literal_selector(&mut map, "4a25d94a", "Swap");
    insert_literal_selector(&mut map, "18cbafe5", "Swap");
    insert_literal_selector(&mut map, "fb3bdb41", "Swap");
    insert_literal_selector(&mut map, "5c11d795", "Swap");
    insert_literal_selector(&mut map, "b6f9de95", "Swap");
    insert_literal_selector(&mut map, "e8e33700", "Add Liquidity");
    insert_literal_selector(&mut map, "f305d719", "Add Liquidity");
    insert_literal_selector(&mut map, "baa2abde", "Remove Liquidity");
    insert_literal_selector(&mut map, "02751cec", "Remove Liquidity ETH");
    insert_literal_selector(&mut map, "2195995c", "Remove Liquidity");
    insert_literal_selector(&mut map, "ded9382a", "Remove Liquidity");
    insert_literal_selector(&mut map, "af2979eb", "Remove Liquidity ETH Supporting Fee");
    insert_literal_selector(&mut map, "5b0d5984", "Remove Liquidity");
    insert_literal_selector(&mut map, "1c4d3861", "Lock Liquidity Tokens");
    insert_literal_selector(&mut map, "0b78f9c0", "Set Fees");
    insert_literal_selector(&mut map, "6db79437", "Update Fees");
    insert_literal_selector(&mut map, "c647b20e", "Set Taxes");
    insert_literal_selector(&mut map, "dc1052e2", "Set Buy Tax");
    insert_literal_selector(&mut map, "8cd09d50", "Set Sell Tax");
    insert_literal_selector(&mut map, "21ecff5b", "Change Fees");
    insert_literal_selector(&mut map, "95913d17", "Modify Taxes");
    insert_literal_selector(&mut map, "db2e21bc", "Emergency Withdraw");
    insert_literal_selector(&mut map, "9e252f00", "Rescue ETH");
    insert_literal_selector(&mut map, "33f3d628", "Rescue Token");
    insert_literal_selector(&mut map, "853828b6", "Withdraw All");
    insert_literal_selector(&mut map, "a7b4961f", "Drain Pool");
    insert_literal_selector(&mut map, "8456cb59", "Pause");
    insert_literal_selector(&mut map, "3f4ba83a", "Unpause");
    insert_literal_selector(&mut map, "17700f01", "Disable Trading");
    insert_literal_selector(&mut map, "f9f92be4", "Blacklist");
    insert_literal_selector(&mut map, "44337ea1", "Add To Blacklist");
    insert_literal_selector(&mut map, "b515566a", "Set Bots");
    insert_literal_selector(&mut map, "d0e30db0", "Deposit");
    insert_literal_selector(&mut map, "2e1a7d4d", "Withdraw");
    insert_literal_selector(&mut map, "f6326fb3", "Deposit");
    insert_literal_selector(&mut map, "f14210a6", "Withdraw");
    insert_literal_selector(&mut map, "a694fc3a", "Stake");
    insert_literal_selector(&mut map, "2e17de78", "Unstake");
    insert_literal_selector(&mut map, "4e71d92d", "Claim Rewards");
    insert_literal_selector(&mut map, "e9fad8ee", "Exit Staking");
    insert_literal_selector(&mut map, "c5ebeaec", "Borrow");
    insert_literal_selector(&mut map, "371fd8e6", "Repay");
    insert_literal_selector(&mut map, "ab9c4b5d", "Flash Loan");
    insert_literal_selector(&mut map, "4914c008", "Liquidate");
    insert_literal_selector(&mut map, "da95691a", "Propose");
    insert_literal_selector(&mut map, "56781388", "Cast Vote");
    insert_literal_selector(&mut map, "5c19a95c", "Delegate");
    insert_literal_selector(&mut map, "ddf0b009", "Queue Proposal");
    insert_literal_selector(&mut map, "fe0d94c1", "Execute Proposal");
    insert_literal_selector(&mut map, "40c10f19", "Mint");
    insert_literal_selector(&mut map, "42966c68", "Burn Position V3");
    insert_literal_selector(&mut map, "b88d4fde", "Safe Transfer NFT");
    insert_literal_selector(&mut map, "095ea7b3", "Approve");
    insert_literal_selector(&mut map, "a9059cbb", "Transfer");
    insert_literal_selector(&mut map, "23b872dd", "Transfer From");
    insert_literal_selector(&mut map, "e58306f9", "Admin Mint");
    insert_literal_selector(&mut map, "484b973c", "Owner Mint");
    insert_literal_selector(&mut map, "627804af", "Dev Mint");
    insert_literal_selector(&mut map, "68573107", "Batch Mint");
    insert_literal_selector(&mut map, "8ba4cc3c", "Airdrop");
    insert_literal_selector(&mut map, "ec28438a", "Set Max Tx Amount");
    insert_literal_selector(&mut map, "27a14fc2", "Set Max Wallet");
    insert_literal_selector(&mut map, "31baf7d5", "Update Max tx");
    insert_literal_selector(&mut map, "0b006d60", "Change Max Wallet");
    insert_literal_selector(&mut map, "751039fc", "Remove Limits");
    insert_literal_selector(&mut map, "ac9650d8", "Multicall");
    insert_literal_selector(&mut map, "a22cb465", "Set Approval For All");
    insert_literal_selector(&mut map, "3659cfe6", "Upgrade Contract");
    insert_literal_selector(&mut map, "88316456", "Mint Position V3");
    insert_literal_selector(&mut map, "219f5d17", "Increase Liquidity V3");
    insert_literal_selector(&mut map, "0c49ccbe", "Decrease Liquidity V3");
    insert_literal_selector(&mut map, "fc6f7865", "Collect V3");
    insert_literal_selector(&mut map, "f637731d", "Initialize Pool V3");
    insert_literal_selector(&mut map, "3c8a7d8d", "Add Liquidity V3");
    insert_literal_selector(&mut map, "128acb08", "Swap V3");
    insert_literal_selector(&mut map, "490e6cbc", "Flash V3");
    insert_literal_selector(&mut map, "a1671295", "Create Pool V3");
    insert_literal_selector(&mut map, "13af4035", "Set Factory Owner V3");
    insert_literal_selector(&mut map, "8a7c195f", "Enable Fee Amount V3");
    insert_literal_selector(&mut map, "0162e2d0", "BananaGun");
    insert_literal_selector(&mut map, "088890dc", "Maestro");
    insert_literal_selector(&mut map, "2f100e4a", "Maestro");
    insert_literal_selector(&mut map, "09c182c3", "SigmaBuy");
    insert_literal_selector(&mut map, "3a571299", "SigmaSell");
    insert_literal_selector(&mut map, "51b001", "LayerSwap 1");
    insert_literal_selector(&mut map, "c9567bf9", "Trading Enabled");
    insert_literal_selector(&mut map, "fb201b1d", "Trading Enabled");
    insert_literal_selector(&mut map, "8a8c523c", "Set Fees/Enable Trading");
    insert_literal_selector(&mut map, "ed995307", "Add Liquidity");
    insert_literal_selector(&mut map, "667f6526", "Set Tax");
    insert_literal_selector(&mut map, "9012c4a8", "Update Fees");
    insert_literal_selector(&mut map, "74010ece", "Set Max tx Amount");
    insert_literal_selector(&mut map, "ea1644d5", "Set Max Wallet Size");
    insert_literal_selector(&mut map, "715018a6", "Renounce Ownership");
    insert_literal_selector(&mut map, "8af416f6", "Lock Liquidity Tokens");
    insert_literal_selector(&mut map, "dd466e67", "InitializeV4");
    insert_literal_selector(&mut map, "f208f491", "ModifyLiquidityV4");
    insert_literal_selector(&mut map, "40e9cecb", "SwapV4");
    insert_literal_selector(&mut map, "c6a377bf", "Permit2");

    debug_assert_eq!(map.len(), 113);
    map
});

pub static EVENT_TOPICS: Lazy<HashMap<&'static str, B256>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // ERC standards
    map.insert("Transfer", event_topic("Transfer(address,address,uint256)"));
    map.insert(
        "TransferSingle",
        event_topic("TransferSingle(address,address,address,uint256,uint256)"),
    );
    map.insert(
        "TransferBatch",
        event_topic("TransferBatch(address,address,address,uint256[],uint256[])"),
    );
    map.insert("Approval", event_topic("Approval(address,address,uint256)"));
    map.insert(
        "ApprovalForAll",
        event_topic("ApprovalForAll(address,address,bool)"),
    );

    // Basic token operations
    map.insert("Mint", event_topic("Mint(address,uint256,uint256)"));
    map.insert("Burn", event_topic("Burn(address,uint256,uint256,address)"));
    map.insert("Deposit", event_topic("Deposit(address,uint256)"));
    map.insert("Withdraw", event_topic("Withdraw(address,uint256)"));
    map.insert("WETHWithdrawal", event_topic("Withdrawal(address,uint256)"));

    // Uniswap V2
    map.insert("Sync", event_topic("Sync(uint112,uint112)"));
    map.insert(
        "Swap",
        event_topic("Swap(address,uint256,uint256,uint256,uint256,address)"),
    );
    map.insert(
        "PairCreated",
        event_topic("PairCreated(address,address,address,uint256)"),
    );
    map.insert(
        "Collect",
        event_topic("Collect(address,address,uint256,uint256)"),
    );
    map.insert(
        "Flash",
        event_topic("Flash(address,uint256,uint256,uint256,uint256)"),
    );
    map.insert(
        "IncreaseObservationCardinalityNext",
        event_topic("IncreaseObservationCardinalityNext(uint16,uint16)"),
    );
    map.insert("SetFeeProtocol", event_topic("SetFeeProtocol(uint8,uint8)"));
    map.insert(
        "CollectProtocol",
        event_topic("CollectProtocol(address,address,uint128,uint128)"),
    );

    // Token management / ownership
    map.insert(
        "OwnershipTransferred",
        event_topic("OwnershipTransferred(address,address)"),
    );
    map.insert(
        "OwnershipTransferStarted",
        event_topic("OwnershipTransferStarted(address,address)"),
    );
    map.insert(
        "RoleGranted",
        event_topic("RoleGranted(bytes32,address,address)"),
    );
    map.insert(
        "RoleRevoked",
        event_topic("RoleRevoked(bytes32,address,address)"),
    );
    map.insert("AdminChanged", event_topic("AdminChanged(address,address)"));
    map.insert("TradingEnabled", event_topic("TradingEnabled(uint256)"));
    map.insert("TradingDisabled", event_topic("TradingDisabled(uint256)"));
    map.insert(
        "ExcludeFromFees",
        event_topic("ExcludeFromFees(address,bool)"),
    );
    map.insert(
        "ExcludeFromLimits",
        event_topic("ExcludeFromLimits(address,bool)"),
    );
    map.insert("SetMaxTxAmount", event_topic("SetMaxTxAmount(uint256)"));
    map.insert(
        "SetMaxWalletToken",
        event_topic("SetMaxWalletToken(uint256)"),
    );
    map.insert("SetMaxWallet", event_topic("SetMaxWallet(uint256)"));

    // Uniswap V3 pool
    map.insert("InitializeV3", event_topic("Initialize(uint160,int24)"));
    map.insert(
        "MintV3",
        event_topic("Mint(address,address,int24,int24,uint128,uint256,uint256)"),
    );
    map.insert(
        "BurnV3",
        event_topic("Burn(address,int24,int24,uint128,uint256,uint256)"),
    );
    map.insert(
        "SwapV3",
        event_topic("Swap(address,address,int256,int256,uint160,uint128,int24)"),
    );
    map.insert(
        "SetFeeProtocolV3",
        event_topic("SetFeeProtocol(uint8,uint8,uint8,uint8)"),
    );
    map.insert(
        "CollectProtocolV3",
        event_topic("CollectProtocol(address,address,uint128,uint128)"),
    );

    // Uniswap V3 factory
    map.insert(
        "PoolCreatedV3",
        event_topic("PoolCreated(address,address,uint24,int24,address)"),
    );
    map.insert(
        "FeeAmountEnabled",
        event_topic("FeeAmountEnabled(uint24,int24)"),
    );
    map.insert("OwnerChanged", event_topic("OwnerChanged(address,address)"));

    // NonfungiblePositionManager
    map.insert(
        "IncreaseLiquidityV3",
        event_topic("IncreaseLiquidity(uint256,uint128,uint256,uint256)"),
    );
    map.insert(
        "DecreaseLiquidityV3",
        event_topic("DecreaseLiquidity(uint256,uint128,uint256,uint256)"),
    );

    // Uniswap V4
    map.insert(
        "InitializeV4",
        event_topic("Initialize(bytes32,address,address,uint24,int24,address,uint160,int24)"),
    );
    map.insert(
        "ModifyLiquidityV4",
        event_topic("ModifyLiquidity(bytes32,address,int24,int24,int256,bytes32)"),
    );
    map.insert(
        "SwapV4",
        event_topic("Swap(bytes32,address,int128,int128,uint160,uint128,int24,uint24)"),
    );
    map.insert(
        "DonateV4",
        event_topic("Donate(bytes32,address,int256,int256)"),
    );
    map.insert(
        "ProtocolFeeUpdatedV4",
        event_topic("ProtocolFeeUpdated(bytes32,uint24)"),
    );
    map.insert(
        "DynamicLPFeeUpdatedV4",
        event_topic("DynamicLPFeeUpdated(bytes32,uint24)"),
    );
    map.insert(
        "ProtocolFeeControllerUpdatedV4",
        event_topic("ProtocolFeeControllerUpdated(address)"),
    );
    map.insert(
        "BalanceDeltaV4",
        B256::from_slice(&[
            0x40, 0xe9, 0xce, 0xcb, 0x9f, 0x5f, 0x1f, 0x1c, 0x5b, 0x9c, 0x97, 0xde, 0xc2, 0x91,
            0x7b, 0x7e, 0xe9, 0x2e, 0x57, 0xba, 0x55, 0x63, 0x70, 0x8d, 0xac, 0xa9, 0x4d, 0xd8,
            0x4a, 0xd7, 0x11, 0x2f,
        ]),
    );

    // Permit2
    map.insert(
        "Permit2",
        event_topic("Permit(address,address,address,uint160,uint48,uint48)"),
    );

    debug_assert_eq!(map.len(), 50);
    map
});

pub static EVENT_TOPICS_REVERSE: Lazy<HashMap<String, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for (name, topic) in EVENT_TOPICS.iter() {
        map.insert(hex::encode(topic.as_slice()), *name);
    }

    // Include Mint(address,uint256) used by certain contracts.
    let alt_mint = event_topic("Mint(address,uint256)");
    map.insert(hex::encode(alt_mint.as_slice()), "Mint");
    map
});

pub static CRITICAL_SCAM_FUNCTIONS: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut set = HashSet::new();
    for selector_hex in [
        "02751cec", "baa2abde", "af2979eb", "db2e21bc", "8a8c523c", "9012c4a8", "8456cb59",
        "f9f92be4", "40c10f19", "715018a6",
    ] {
        set.insert(selector_hex.to_string());
    }
    set
});

pub static UNISWAP_CONTRACTS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    HashMap::from([
        (
            "0x7a250d5630B4cF539739dF2C5dAcb4c659f2488D",
            "Uniswap V2: Router 2",
        ),
        (
            "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45",
            "Uniswap V3: Router 2",
        ),
        (
            "0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
            "POSITION_MANAGER",
        ),
        ("0x1f98431c8ad98523631ae4a59f267346ea31f984", "FACTORY"),
        ("0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45", "ROUTER"),
        ("0xb27308f9F90D607463bb33eA1BeBb41C27CE5AB6", "QUOTER"),
    ])
});
