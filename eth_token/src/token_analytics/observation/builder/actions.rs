use serde_json::Value;

use crate::erc20::ERC20Token;
use crate::pools::BasePool;
use crate::state::{TokenTransferFromCallRecord, TokenTransferRecord};
use crate::token_analytics::{
    ObservationBlockAction, ObservationBlockActivity, ObservationBlockEventFlags,
    ObservationSellFlow,
};

use super::events::{event_block_number, event_transfers_to_burn_address, event_tx_hash};
use super::roles::{
    address_role, observation_role_context, ObservationAddressRole, ObservationRoleContext,
};
use super::transfers::{denom_transfer_records_at_block, token_transfer_records_at_block};
use super::utils::{
    finite_non_negative, is_usd_denom, is_weth_denom, normalize_address, ZERO_ADDRESS,
};

pub(super) fn block_actions(
    token: &ERC20Token,
    pool: &BasePool,
    block_number: u64,
    activity: &ObservationBlockActivity,
    flags: &ObservationBlockEventFlags,
    sell_flow: &ObservationSellFlow,
) -> Vec<ObservationBlockAction> {
    let mut actions = Vec::new();
    let denom = pool.identity.denom_address.as_str();
    if activity.buy_volume_for_denom(denom) > 0.0 {
        add_action(
            &mut actions,
            ObservationBlockAction::new(
                "buy_swap",
                "Buy swaps",
                "Decoded buy flow through this pool.",
            ),
        );
    }
    if activity.sell_volume_for_denom(denom) > 0.0 {
        add_action(
            &mut actions,
            ObservationBlockAction::new(
                "sell_swap",
                "Sell swaps",
                "Decoded sell flow through this pool.",
            ),
        );
    }
    if sell_flow.has_taxed_sell_pattern {
        add_action(
            &mut actions,
            ObservationBlockAction::new(
                "taxed_sell_flow",
                "Taxed sell flow",
                "Observed sell token flow was routed away from the pool or through the token contract.",
            ),
        );
    }
    if flags.liquidity_removal_in_block {
        let mut action = ObservationBlockAction::new(
            "liquidity_removed",
            "Liquidity event",
            "The pool is marked as having a liquidity or reserve-removal event.",
        );
        if let Some(tx_hash) = pool.scam_tx_hash.as_deref() {
            action = action.with_tx_hash(tx_hash);
        }
        add_action(&mut actions, action);
    }
    if activity.total_bribe_eth > 0.0 {
        add_action(
            &mut actions,
            ObservationBlockAction::new("bribe", "Bribe", "Transactions paid builder bribe value.")
                .with_amount(activity.total_bribe_eth),
        );
    }

    add_event_actions(
        &mut actions,
        &pool.sync_events,
        block_number,
        "pool_sync",
        "Pool sync",
        "Pair reserves were synchronized to current token and denom balances.",
    );
    add_event_actions(
        &mut actions,
        &pool.mint_events,
        block_number,
        "pool_mint",
        "Pool mint",
        "Pool liquidity mint event.",
    );
    add_event_actions(
        &mut actions,
        &pool.burn_events,
        block_number,
        "pool_burn",
        "Pool burn",
        "Pool liquidity burn event.",
    );
    add_ownership_actions(token, block_number, &mut actions);
    add_token_approval_actions(token, block_number, &mut actions);
    add_lp_actions(
        token,
        &pool.identity.pool_address,
        block_number,
        &mut actions,
    );

    let context = observation_role_context(token, pool);
    add_token_control_transfer_from_actions(token, block_number, &context, &mut actions);
    add_token_control_transaction_actions(token, block_number, &context, &mut actions);
    for record in token_transfer_records_at_block(token, block_number) {
        if let Some(action) =
            transfer_action_for_record(record, TransferAssetClass::Token, &context)
        {
            add_action(&mut actions, action);
        }
    }
    for record in denom_transfer_records_at_block(token, block_number) {
        let asset = denom_asset_class(record);
        if let Some(action) = transfer_action_for_record(record, asset, &context) {
            add_action(&mut actions, action);
        }
    }

    actions
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransferAssetClass {
    Token,
    Weth,
    Usd,
    OtherDenom,
}

impl TransferAssetClass {
    fn prefix(self) -> &'static str {
        match self {
            Self::Token => "token",
            Self::Weth => "weth",
            Self::Usd => "usd",
            Self::OtherDenom => "other_denom",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Token => "Token",
            Self::Weth => "WETH",
            Self::Usd => "USD",
            Self::OtherDenom => "Other denom",
        }
    }
}

impl ObservationBlockAction {
    fn new(key: impl Into<String>, label: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            title: title.into(),
            count: 1,
            amount: 0.0,
            tx_hashes: Vec::new(),
        }
    }

    fn with_amount(mut self, amount: f64) -> Self {
        self.amount = finite_non_negative(amount);
        self
    }

    fn with_tx_hash(mut self, tx_hash: impl Into<String>) -> Self {
        let tx_hash = tx_hash.into().trim().to_ascii_lowercase();
        if !tx_hash.is_empty() {
            self.tx_hashes.push(tx_hash);
        }
        self
    }
}

fn add_action(actions: &mut Vec<ObservationBlockAction>, action: ObservationBlockAction) {
    if action.key.is_empty() {
        return;
    }
    let Some(existing) = actions
        .iter_mut()
        .find(|existing| existing.key == action.key)
    else {
        actions.push(action);
        return;
    };
    existing.count = existing.count.saturating_add(action.count);
    existing.amount += action.amount;
    for tx_hash in action.tx_hashes {
        if !existing.tx_hashes.contains(&tx_hash) {
            existing.tx_hashes.push(tx_hash);
        }
    }
}

fn add_event_actions(
    actions: &mut Vec<ObservationBlockAction>,
    events: &[Value],
    block_number: u64,
    key: &str,
    label: &str,
    title: &str,
) {
    for event in events
        .iter()
        .filter(|event| event_block_number(event) == Some(block_number))
    {
        let action = event_tx_hash(event)
            .map(|tx_hash| ObservationBlockAction::new(key, label, title).with_tx_hash(tx_hash))
            .unwrap_or_else(|| ObservationBlockAction::new(key, label, title));
        add_action(actions, action);
    }
}

fn add_lp_transfer_actions(
    actions: &mut Vec<ObservationBlockAction>,
    events: &[Value],
    block_number: u64,
    title: &str,
) {
    for event in events
        .iter()
        .filter(|event| event_block_number(event) == Some(block_number))
    {
        let (key, label, title) = if event_transfers_to_burn_address(event) {
            (
                "lp_burn",
                "LP burned",
                "Liquidity-provider token or position was transferred to a burn/dead address.",
            )
        } else {
            ("lp_transfer", "LP transfers", title)
        };
        let action = event_tx_hash(event)
            .map(|tx_hash| ObservationBlockAction::new(key, label, title).with_tx_hash(tx_hash))
            .unwrap_or_else(|| ObservationBlockAction::new(key, label, title));
        add_action(actions, action);
    }
}

fn add_ownership_actions(
    token: &ERC20Token,
    block_number: u64,
    actions: &mut Vec<ObservationBlockAction>,
) {
    for event in token
        .authority_tracker
        .owner_events
        .iter()
        .filter(|event| event.block_number == block_number)
    {
        let previous_owner = normalize_address(&event.previous_owner);
        let new_owner = normalize_address(&event.new_owner);
        let (key, label) = if new_owner == ZERO_ADDRESS {
            ("ownership_renounced", "Ownership renounced")
        } else if previous_owner == ZERO_ADDRESS {
            ("ownership_initialized", "Ownership initialized")
        } else {
            ("ownership_transferred", "Ownership transferred")
        };
        add_action(
            actions,
            ObservationBlockAction::new(
                key,
                label,
                format!("Token ownership changed from {previous_owner} to {new_owner}."),
            )
            .with_tx_hash(&event.tx_hash),
        );
    }
}

fn add_token_approval_actions(
    token: &ERC20Token,
    block_number: u64,
    actions: &mut Vec<ObservationBlockAction>,
) {
    for approval in token.transfer_tracker.approvals.iter().filter(|approval| {
        approval.block_number == block_number
            && approval
                .token_address
                .eq_ignore_ascii_case(&token.contract_address)
    }) {
        add_action(
            actions,
            ObservationBlockAction::new(
                "token_approval",
                "Token approvals",
                "ERC-20 approval on the token contract.",
            )
            .with_tx_hash(&approval.tx_hash),
        );
    }
}

fn add_lp_actions(
    token: &ERC20Token,
    pool_address: &str,
    block_number: u64,
    actions: &mut Vec<ObservationBlockAction>,
) {
    let pool_address = normalize_address(pool_address);
    if let Some(pool) = token.v2_pools.get(&pool_address) {
        add_event_actions(
            actions,
            &pool.lp_tracker.approval_events,
            block_number,
            "lp_approval",
            "LP approvals",
            "Liquidity-provider token approvals observed for this pool.",
        );
        add_lp_transfer_actions(
            actions,
            &pool.lp_tracker.transfers,
            block_number,
            "Liquidity-provider token transfer observed for this pool.",
        );
    }
    if let Some(pool) = token.v3_pools.get(&pool_address) {
        add_lp_transfer_actions(
            actions,
            &pool.liquidity_position_events,
            block_number,
            "Liquidity position event observed for this pool.",
        );
    }
    if let Some(pool) = token.v4_pools.get(&pool_address) {
        add_event_actions(
            actions,
            &pool.lp_approval_events,
            block_number,
            "lp_approval",
            "LP approvals",
            "Liquidity-provider token approvals observed for this pool.",
        );
        add_lp_transfer_actions(
            actions,
            &pool.liquidity_position_events,
            block_number,
            "Liquidity position event observed for this pool.",
        );
    }
}

fn add_token_control_transfer_from_actions(
    token: &ERC20Token,
    block_number: u64,
    context: &ObservationRoleContext,
    actions: &mut Vec<ObservationBlockAction>,
) {
    for call in token
        .transfer_tracker
        .transfer_from_calls
        .iter()
        .filter(|call| call.block_number == block_number)
    {
        if address_role(&call.caller, context) != ObservationAddressRole::Control {
            continue;
        }
        let action = token_control_transfer_from_action(token, call, context);
        add_action(actions, action);
    }
}

fn add_token_control_transaction_actions(
    token: &ERC20Token,
    block_number: u64,
    context: &ObservationRoleContext,
    actions: &mut Vec<ObservationBlockAction>,
) {
    for transaction in token
        .activity
        .transactions_by_hash
        .values()
        .filter(|transaction| transaction.block_number == block_number)
    {
        let Some(maker) = transaction.maker.as_deref() else {
            continue;
        };
        if address_role(maker, context) != ObservationAddressRole::Control {
            continue;
        }
        add_action(
            actions,
            ObservationBlockAction::new(
                "token_control_call",
                "Token control call",
                "Known owner/control address touched token state in this block.",
            )
            .with_tx_hash(&transaction.tx_hash),
        );
    }
}

fn token_control_transfer_from_action(
    token: &ERC20Token,
    call: &TokenTransferFromCallRecord,
    context: &ObservationRoleContext,
) -> ObservationBlockAction {
    let from_role = address_role(&call.from_address, context);
    let to_role = address_role(&call.to_address, context);
    let from_pair_to_control = from_role.is_pool() && to_role == ObservationAddressRole::Control;
    let holder_to_burn = reth_chain_query::common_addresses::is_burn_address_str(&call.to_address);
    let without_transfer_log = call.emitted_transfer_count == 0;
    let after_renounce = token
        .authority_tracker
        .renouncement_block
        .is_some_and(|block| call.block_number >= block);
    let (key, label, title_prefix) = if from_pair_to_control {
        (
            "token_control_transfer_from_pair",
            "Control transferFrom pair",
            "Control caller used transferFrom to pull tokens from a pool.",
        )
    } else if holder_to_burn {
        (
            "token_control_transfer_from_holder_to_burn",
            "Control transferFrom holder -> burn",
            "Control caller used transferFrom to move holder tokens to a burn address.",
        )
    } else if without_transfer_log {
        (
            "token_control_transfer_from_without_transfer",
            "Control transferFrom without transfer",
            "Control caller used transferFrom without a matching token Transfer log.",
        )
    } else if after_renounce {
        (
            "token_control_transfer_from_after_renounce",
            "Control transferFrom after renounce",
            "Control caller used transferFrom after token ownership was renounced.",
        )
    } else {
        (
            "token_control_transfer_from",
            "Control transferFrom",
            "Control caller used transferFrom on the token contract.",
        )
    };
    let amount = finite_non_negative(call.amount);
    let title = format!(
        "{title_prefix} {} from {} to {}",
        transfer_amount_label(amount, TransferAssetClass::Token),
        from_role.as_str(),
        to_role.as_str(),
    );
    ObservationBlockAction::new(key, label, title)
        .with_amount(amount)
        .with_tx_hash(&call.tx_hash)
}

fn transfer_action_for_record(
    record: &TokenTransferRecord,
    asset: TransferAssetClass,
    context: &ObservationRoleContext,
) -> Option<ObservationBlockAction> {
    let from_role = address_role(&record.from_address, context);
    let to_role = address_role(&record.to_address, context);
    if !transfer_touches_relevant_role(from_role, to_role) {
        return None;
    }

    let amount = finite_non_negative(record.amount);
    let title = format!(
        "{} from {} to {}",
        transfer_amount_label(amount, asset),
        from_role.as_str(),
        to_role.as_str()
    );

    if asset == TransferAssetClass::Token {
        if from_role.is_pool() && to_role == ObservationAddressRole::Control {
            return Some(action_from_transfer(
                "token_pool_to_control",
                "Token pool -> control",
                title,
                amount,
                &record.tx_hash,
            ));
        }
        if from_role.is_pool() && to_role.is_pool() {
            return Some(action_from_transfer(
                "token_pool_to_pool",
                "Token pool -> pool",
                title,
                amount,
                &record.tx_hash,
            ));
        }
        if from_role.is_pool() {
            return Some(action_from_transfer(
                "token_pool_out",
                "Token pool out",
                title,
                amount,
                &record.tx_hash,
            ));
        }
        if to_role.is_pool() && from_role == ObservationAddressRole::TokenContract {
            return Some(action_from_transfer(
                "token_contract_to_pool",
                "Token contract -> pool",
                title,
                amount,
                &record.tx_hash,
            ));
        }
        if to_role.is_pool() {
            return Some(action_from_transfer(
                "token_pool_in",
                "Token pool in",
                title,
                amount,
                &record.tx_hash,
            ));
        }
        if to_role == ObservationAddressRole::TokenContract {
            return Some(action_from_transfer(
                "token_contract_intake",
                "Token contract intake",
                title,
                amount,
                &record.tx_hash,
            ));
        }
        if to_role == ObservationAddressRole::Control {
            return Some(action_from_transfer(
                "token_to_control",
                "Token to control",
                title,
                amount,
                &record.tx_hash,
            ));
        }
        if from_role == ObservationAddressRole::Control {
            return Some(action_from_transfer(
                "token_from_control",
                "Token from control",
                title,
                amount,
                &record.tx_hash,
            ));
        }
        return Some(action_from_transfer(
            "token_transfer",
            "Token transfers",
            title,
            amount,
            &record.tx_hash,
        ));
    }

    let prefix = asset.prefix();
    let label = asset.label();
    if from_role.is_pool() && to_role.is_pool() {
        return Some(action_from_transfer(
            format!("{prefix}_pool_to_pool"),
            format!("{label} pool -> pool"),
            title,
            amount,
            &record.tx_hash,
        ));
    }
    if from_role.is_pool() {
        return Some(action_from_transfer(
            format!("{prefix}_pool_out"),
            format!("{label} pool out"),
            title,
            amount,
            &record.tx_hash,
        ));
    }
    if to_role.is_pool() {
        return Some(action_from_transfer(
            format!("{prefix}_pool_in"),
            format!("{label} pool in"),
            title,
            amount,
            &record.tx_hash,
        ));
    }
    if to_role == ObservationAddressRole::TokenContract {
        return Some(action_from_transfer(
            format!("{prefix}_to_token_contract"),
            format!("{label} to token contract"),
            title,
            amount,
            &record.tx_hash,
        ));
    }
    if to_role == ObservationAddressRole::Control {
        return Some(action_from_transfer(
            format!("{prefix}_to_control"),
            format!("{label} to control"),
            title,
            amount,
            &record.tx_hash,
        ));
    }
    if from_role == ObservationAddressRole::Control {
        return Some(action_from_transfer(
            format!("{prefix}_from_control"),
            format!("{label} from control"),
            title,
            amount,
            &record.tx_hash,
        ));
    }
    Some(action_from_transfer(
        format!("{prefix}_transfer"),
        format!("{label} transfers"),
        title,
        amount,
        &record.tx_hash,
    ))
}

fn action_from_transfer(
    key: impl Into<String>,
    label: impl Into<String>,
    title: impl Into<String>,
    amount: f64,
    tx_hash: &str,
) -> ObservationBlockAction {
    ObservationBlockAction::new(key, label, title)
        .with_amount(amount)
        .with_tx_hash(tx_hash)
}

fn transfer_touches_relevant_role(
    from_role: ObservationAddressRole,
    to_role: ObservationAddressRole,
) -> bool {
    [from_role, to_role].into_iter().any(|role| {
        matches!(
            role,
            ObservationAddressRole::SelectedPool
                | ObservationAddressRole::Pool
                | ObservationAddressRole::TokenContract
                | ObservationAddressRole::Control
                | ObservationAddressRole::Router
        )
    })
}

fn transfer_amount_label(amount: f64, asset: TransferAssetClass) -> String {
    if amount <= 0.0 {
        return "Transfer".to_string();
    }
    format!("{amount:.6} {}", asset.label())
}

fn denom_asset_class(record: &TokenTransferRecord) -> TransferAssetClass {
    if is_weth_denom(&record.token_address) {
        TransferAssetClass::Weth
    } else if is_usd_denom(&record.token_address) {
        TransferAssetClass::Usd
    } else {
        TransferAssetClass::OtherDenom
    }
}
