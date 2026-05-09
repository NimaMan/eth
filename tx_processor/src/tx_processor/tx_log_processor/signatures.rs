use alloy_primitives::B256;
use reth_chain_query::function_signatures::EVENT_TOPICS;

pub struct EventSignatures {
    // ERC20
    pub transfer: B256,
    pub approval: B256,

    // ERC721
    pub transfer_erc721: B256,
    pub approval_erc721: B256,
    pub approval_for_all: B256,

    // ERC1155
    pub transfer_single: B256,
    pub transfer_batch: B256,

    // Uniswap V2
    pub sync: B256,
    pub swap: B256,
    pub mint: B256,
    pub burn: B256,
    pub pair_created: B256,

    // Uniswap V3
    pub pool_created: B256,
    pub initialize: B256,
    pub mint_v3: B256,
    pub burn_v3: B256,
    pub swap_v3: B256,
    pub increase_liquidity: B256,
    pub decrease_liquidity: B256,
    pub collect: B256,

    // Uniswap V4
    pub initialize_v4: B256,
    pub modify_liquidity: B256,
    pub swap_v4: B256,
    pub donate: B256,
    pub protocol_fee_updated: B256,
    pub dynamic_lp_fee_updated: B256,
    pub protocol_fee_controller_updated: B256,
    pub balance_delta: B256,
    pub permit2: B256,

    // General actions
    pub deposit: B256,
    pub withdraw: B256,

    // Contract events
    pub ownership_transferred: B256,
    pub ownership_transfer_started: B256,
    pub role_granted: B256,
    pub role_revoked: B256,
    pub admin_changed: B256,
    pub trading_enabled: B256,
    pub trading_disabled: B256,
}

fn topic(name: &str) -> B256 {
    *EVENT_TOPICS
        .get(name)
        .unwrap_or_else(|| panic!("missing event topic: {name}"))
}

impl EventSignatures {
    pub fn new() -> Self {
        Self {
            // ERC20
            transfer: topic("Transfer"),
            approval: topic("Approval"),

            // ERC721
            transfer_erc721: topic("Transfer"),
            approval_erc721: topic("Approval"),
            approval_for_all: topic("ApprovalForAll"),

            // ERC1155
            transfer_single: topic("TransferSingle"),
            transfer_batch: topic("TransferBatch"),

            // Uniswap V2
            sync: topic("Sync"),
            swap: topic("Swap"),
            mint: topic("Mint"),
            burn: topic("Burn"),
            pair_created: topic("PairCreated"),

            // Uniswap V3
            pool_created: topic("PoolCreatedV3"),
            initialize: topic("InitializeV3"),
            mint_v3: topic("MintV3"),
            burn_v3: topic("BurnV3"),
            swap_v3: topic("SwapV3"),
            increase_liquidity: topic("IncreaseLiquidityV3"),
            decrease_liquidity: topic("DecreaseLiquidityV3"),
            collect: topic("Collect"),

            // Uniswap V4
            initialize_v4: topic("InitializeV4"),
            modify_liquidity: topic("ModifyLiquidityV4"),
            swap_v4: topic("SwapV4"),
            donate: topic("DonateV4"),
            protocol_fee_updated: topic("ProtocolFeeUpdatedV4"),
            dynamic_lp_fee_updated: topic("DynamicLPFeeUpdatedV4"),
            protocol_fee_controller_updated: topic("ProtocolFeeControllerUpdatedV4"),
            balance_delta: topic("BalanceDeltaV4"),
            permit2: topic("Permit2"),

            // General actions
            deposit: topic("Deposit"),
            withdraw: topic("Withdraw"),

            // Contract events
            ownership_transferred: topic("OwnershipTransferred"),
            ownership_transfer_started: topic("OwnershipTransferStarted"),
            role_granted: topic("RoleGranted"),
            role_revoked: topic("RoleRevoked"),
            admin_changed: topic("AdminChanged"),
            trading_enabled: topic("TradingEnabled"),
            trading_disabled: topic("TradingDisabled"),
        }
    }
}
