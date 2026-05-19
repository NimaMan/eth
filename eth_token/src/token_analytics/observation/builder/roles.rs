use std::collections::HashSet;

use crate::erc20::ERC20Token;
use crate::pools::uniswap::v2::LPHolderSnapshot;
use crate::pools::BasePool;

use super::utils::{
    is_evm_address, is_lp_burn_holder, normalize_address, ROUTER_ADDRESSES, ZERO_ADDRESS,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ObservationAddressRole {
    Unknown,
    Zero,
    SelectedPool,
    Pool,
    TokenContract,
    Control,
    Router,
    External,
}

impl ObservationAddressRole {
    pub(super) fn is_pool(self) -> bool {
        matches!(self, Self::SelectedPool | Self::Pool)
    }

    pub(super) fn is_seller_candidate(self) -> bool {
        !matches!(
            self,
            Self::Unknown | Self::Zero | Self::SelectedPool | Self::Pool | Self::TokenContract
        )
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Zero => "zero",
            Self::SelectedPool => "selected_pool",
            Self::Pool => "pool",
            Self::TokenContract => "token_contract",
            Self::Control => "control",
            Self::Router => "router",
            Self::External => "external",
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct ObservationRoleContext {
    selected_pool_address: String,
    token_address: String,
    pool_addresses: HashSet<String>,
    control_addresses: HashSet<String>,
}

pub(super) fn observation_role_context(
    token: &ERC20Token,
    pool: &BasePool,
) -> ObservationRoleContext {
    let selected_pool_address = normalize_address(&pool.identity.pool_address);
    let token_address = normalize_address(&token.contract_address);
    let mut pool_addresses: HashSet<String> = token
        .all_pool_bases()
        .into_iter()
        .map(|pool| normalize_address(&pool.identity.pool_address))
        .filter(|address| is_evm_address(address))
        .collect();
    pool_addresses.insert(selected_pool_address.clone());

    let mut control_addresses: HashSet<String> = token
        .token_control_addresses
        .iter()
        .map(|address| normalize_address(address))
        .filter(|address| is_evm_address(address))
        .collect();
    control_addresses.extend(
        pool.token_control_addresses
            .iter()
            .map(|address| normalize_address(address))
            .filter(|address| is_evm_address(address)),
    );
    if let Some(address) = token.creator_address.as_deref() {
        let address = normalize_address(address);
        if is_evm_address(&address) {
            control_addresses.insert(address);
        }
    }
    if let Some(address) = token.current_owner() {
        let address = normalize_address(&address);
        if is_evm_address(&address) {
            control_addresses.insert(address);
        }
    }
    add_high_share_lp_holders(token, &selected_pool_address, &mut control_addresses);

    ObservationRoleContext {
        selected_pool_address,
        token_address,
        pool_addresses,
        control_addresses,
    }
}

fn add_high_share_lp_holders(
    token: &ERC20Token,
    pool_address: &str,
    control_addresses: &mut HashSet<String>,
) {
    if let Some(pool) = token.v2_pools.get(pool_address) {
        add_high_share_holders(pool.lp_holders(), control_addresses);
    }
    if let Some(pool) = token.v3_pools.get(pool_address) {
        add_high_share_holders(pool.lp_holders(), control_addresses);
    }
    if let Some(pool) = token.v4_pools.get(pool_address) {
        add_high_share_holders(pool.lp_holders(), control_addresses);
    }
}

fn add_high_share_holders(holders: Vec<LPHolderSnapshot>, control_addresses: &mut HashSet<String>) {
    for holder in holders
        .into_iter()
        .filter(|holder| holder.share >= 95.0 && !is_lp_burn_holder(&holder.address))
    {
        let address = normalize_address(&holder.address);
        if is_evm_address(&address) {
            control_addresses.insert(address);
        }
    }
}

pub(super) fn address_role(
    address: &str,
    context: &ObservationRoleContext,
) -> ObservationAddressRole {
    let address = normalize_address(address);
    if address.is_empty() {
        return ObservationAddressRole::Unknown;
    }
    if address == ZERO_ADDRESS {
        return ObservationAddressRole::Zero;
    }
    if address == context.selected_pool_address {
        return ObservationAddressRole::SelectedPool;
    }
    if context.pool_addresses.contains(&address) {
        return ObservationAddressRole::Pool;
    }
    if address == context.token_address {
        return ObservationAddressRole::TokenContract;
    }
    if context.control_addresses.contains(&address) {
        return ObservationAddressRole::Control;
    }
    if ROUTER_ADDRESSES.contains(&address.as_str()) {
        return ObservationAddressRole::Router;
    }
    ObservationAddressRole::External
}
