use crate::mempool_fetcher::MempoolTransaction;
use crate::position_approval_call::PositionApprovalCall;
use crate::token_tracking::TokenTrackingCache;
use alloy_primitives::{Address, U256};
use reth_chain_query::to_checksum_address;

use super::{pool_type_label, LpApprovalSignal, Signal};

pub async fn build_position_approval_signals(
    token_cache: &TokenTrackingCache,
    tx: &MempoolTransaction,
    approval: PositionApprovalCall,
) -> Vec<Signal> {
    let approver = to_checksum_address(&Address::from_slice(&tx.from));
    let (spender, contexts) = match approval {
        PositionApprovalCall::SinglePosition {
            position_manager,
            spender,
            token_id,
        } => {
            let manager = to_checksum_address(&position_manager);
            let contexts = token_cache
                .position_context_by_token_id(&manager, token_id)
                .await
                .into_iter()
                .collect::<Vec<_>>();
            (spender, contexts)
        }
        PositionApprovalCall::OperatorForAll {
            position_manager,
            operator,
            approved,
        } => {
            if !approved {
                return Vec::new();
            }
            let manager = to_checksum_address(&position_manager);
            let contexts = token_cache
                .position_contexts_by_owner(&manager, &approver)
                .await;
            (operator, contexts)
        }
    };

    let spender_address = to_checksum_address(&spender);
    contexts
        .into_iter()
        .filter_map(|context| {
            let share_pct = context.position_share_pct?;
            Some(Signal::LpApproval(LpApprovalSignal {
                tx_hash: tx.hash.clone(),
                creator: approver.clone(),
                lp_token_address: context.position_manager_address.clone(),
                router_address: spender_address.clone(),
                amount: parse_u256_amount(&context.position_liquidity).unwrap_or(U256::ZERO),
                timestamp: chrono::Utc::now().timestamp(),
                token_address: context.token_address,
                pool_address: context.pool_address,
                pool_type: pool_type_label(&context.pool_type),
                denom_address: Some(context.denom_address),
                denom_currency: Some(context.denom_currency),
                denom_decimals: None,
                spender_address: spender_address.clone(),
                approval_percentage: Some(share_pct),
                approved_share_pct: Some(share_pct),
                approval_model: Some("position_liquidity_share".to_string()),
                lp_total_supply: context.pool_liquidity.clone(),
                position_manager: Some(context.position_manager_address),
                position_id: Some(context.position_id),
                position_liquidity: Some(context.position_liquidity),
                pool_liquidity: context.pool_liquidity,
                position_share_pct: Some(share_pct),
                previous_allowance: None,
                approver_address: approver.clone(),
                creator_address: context.owner,
            }))
        })
        .collect()
}

fn parse_u256_amount(value: &str) -> Option<U256> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(hex) = value.strip_prefix("0x") {
        return U256::from_str_radix(hex, 16).ok();
    }
    value.parse::<U256>().ok()
}
