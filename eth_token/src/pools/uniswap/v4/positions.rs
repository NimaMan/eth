use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4LiquidityPosition {
    pub position_id: String,
    pub owner: String,
    pub position_manager_address: String,
    pub liquidity: u128,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub last_update_block: u64,
    pub last_update_tx: String,
}

impl UniswapV4Pool {
    pub fn lp_total_supply(&self) -> f64 {
        self.liquidity_positions
            .values()
            .map(|position| position.liquidity as f64)
            .sum()
    }

    pub fn touches_position_transfer(&self, transaction: &ProcessedTransaction) -> bool {
        transaction
            .erc721_transfers
            .iter()
            .any(|transfer| self.position_transfer_matches_known_position(transfer))
    }

    pub(super) fn record_liquidity_position(
        &mut self,
        event: &ProcessedV4ModifyLiquidityEvent,
        tx: &UniswapV2TxContext,
        transaction: &ProcessedTransaction,
    ) -> UniswapV4LiquidityPosition {
        let position_id = normalize_hash_string(hash_string(&event.salt));
        let transfer = position_transfer_for_modify_event(event, transaction);
        let previous = self.liquidity_positions.get(&position_id);
        let previous_owner = previous.map(|position| position.owner.clone());
        let position_manager_address = transfer
            .map(|transfer| address_string(&transfer.token_address))
            .unwrap_or_else(|| address_string(&event.sender));
        self.position_manager_address = Some(position_manager_address.clone());
        let owner = transfer
            .and_then(owner_from_position_transfer)
            .or_else(|| previous.map(|position| position.owner.clone()))
            .or_else(|| tx.from_address.clone())
            .unwrap_or_else(|| address_string(&event.sender));
        let previous_liquidity = previous.map(|position| position.liquidity).unwrap_or(0);
        let liquidity = apply_liquidity_delta(previous_liquidity, event.liquidity_delta);
        let position = UniswapV4LiquidityPosition {
            position_id: position_id.clone(),
            owner,
            position_manager_address,
            liquidity,
            tick_lower: event.tick_lower,
            tick_upper: event.tick_upper,
            last_update_block: tx.block_number,
            last_update_tx: tx.tx_hash.clone(),
        };
        self.liquidity_positions
            .insert(position_id.clone(), position.clone());
        if previous_owner
            .as_deref()
            .is_some_and(|previous_owner| previous_owner != position.owner)
        {
            self.position_approvals.remove(&position_id);
        }
        position
    }

    pub(super) fn process_position_transfers(
        &mut self,
        transaction: &ProcessedTransaction,
        tx: &UniswapV2TxContext,
        modified_position_ids: &BTreeSet<String>,
    ) {
        let transfers = transaction
            .erc721_transfers
            .iter()
            .filter_map(|transfer| {
                let position_id = self.position_transfer_id(transfer)?;
                if modified_position_ids.contains(&position_id) {
                    return None;
                }
                Some((position_id, transfer))
            })
            .collect::<Vec<_>>();

        for (position_id, transfer) in transfers {
            let Some(owner) = owner_from_position_transfer(transfer) else {
                continue;
            };
            let Some(position) = self.liquidity_positions.get_mut(&position_id) else {
                continue;
            };
            position.owner = owner.clone();
            position.last_update_block = tx.block_number;
            position.last_update_tx = tx.tx_hash.clone();
            self.position_approvals.remove(&position_id);

            let transfer_event = json!({
                "event": "position_transfer",
                "block_number": tx.block_number,
                "block_timestamp": tx.block_timestamp,
                "tx_hash": tx.tx_hash,
                "position_id": position_id,
                "position_manager_address": address_string(&transfer.token_address),
                "from": address_string(&transfer.from_address),
                "to": owner,
                "from_address": address_string(&transfer.from_address),
                "to_address": address_string(&transfer.to_address),
                "position_liquidity": position.liquidity.to_string(),
            });
            append_with_history_limit(
                &mut self.liquidity_position_events,
                transfer_event,
                self.base.config.history_limit,
            );
        }
    }

    fn position_transfer_matches_known_position(&self, transfer: &ERC721TransferEvent) -> bool {
        self.position_transfer_id(transfer).is_some()
    }

    fn position_transfer_id(&self, transfer: &ERC721TransferEvent) -> Option<String> {
        let position_manager = self.position_manager_address.as_ref()?;
        if !same_address(&transfer.token_address, position_manager) {
            return None;
        }
        self.position_id_for_token_id(transfer.token_id)
    }

    pub(super) fn position_id_for_token_id(&self, token_id: U256) -> Option<String> {
        self.liquidity_positions
            .keys()
            .find(|position_id| {
                parse_hash(position_id)
                    .map(position_token_id)
                    .is_ok_and(|position_token_id| position_token_id == token_id)
            })
            .cloned()
    }

    pub(super) fn lp_balances_by_holder(&self) -> BTreeMap<String, f64> {
        let mut balances = BTreeMap::<String, f64>::new();
        for position in self.liquidity_positions.values() {
            if position.liquidity == 0 {
                continue;
            }
            *balances.entry(position.owner.clone()).or_insert(0.0) += position.liquidity as f64;
        }
        balances
    }
}
