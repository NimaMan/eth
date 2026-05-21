use super::*;

impl ERC20Token {
    pub fn update_token_state_from_processed_transaction(
        &mut self,
        transaction: &ProcessedTransaction,
    ) -> Result<()> {
        self.record_transaction_metadata(
            hash_string(&transaction.hash),
            Some(&address_string(&transaction.from_address)),
            transaction.block_number,
            transaction.block_timestamp,
        );
        self.record_bribe_activity_from_processed_transaction(transaction)?;
        self.record_transfer_activity_from_processed_transaction(transaction)?;
        self.transfer_tracker
            .update_from_processed_transaction(transaction)?;
        self.record_pair_token_transfers_from_processed_transaction(transaction)?;
        self.status_manager.update_from_processed_transaction(
            transaction,
            self.transfer_tracker.total_supply_from_transfers,
            self.decimals,
        )?;
        let newly_added = self
            .authority_tracker
            .update_from_processed_transaction(transaction);
        if !newly_added.is_empty() {
            self.token_control_addresses.extend(newly_added.clone());
            self.register_control_addresses_with_pools(newly_added);
        }
        self.refresh_lifecycle_status();
        Ok(())
    }

    pub fn record_transaction_metadata(
        &mut self,
        tx_hash: impl AsRef<str>,
        from_address: Option<&str>,
        block_number: u64,
        block_timestamp: u64,
    ) {
        self.activity.record_transaction(
            tx_hash.as_ref(),
            from_address,
            block_number,
            Some(block_timestamp),
        );
        if let Some(from_address) = from_address {
            self.tx_hashes_to_makers.insert(
                tx_hash.as_ref().to_string(),
                normalize_address(from_address),
            );
        }
        self.latest_block_number = Some(block_number);
        self.latest_block_timestamp = Some(block_timestamp);
    }

    pub(super) fn record_bribe_activity_from_processed_transaction(
        &mut self,
        transaction: &ProcessedTransaction,
    ) -> Result<()> {
        if transaction.bribe_amount.is_zero() {
            return Ok(());
        }

        let amount_eth = scale_raw_units(transaction.bribe_amount.to_string(), 18)?;
        self.activity.record_bribe_eth(
            hash_string(&transaction.hash),
            transaction.block_number,
            Some(transaction.block_timestamp),
            amount_eth,
        );
        Ok(())
    }

    pub(super) fn record_transfer_activity_from_processed_transaction(
        &mut self,
        transaction: &ProcessedTransaction,
    ) -> Result<()> {
        let mut token_transfer_count = 0u32;
        let mut token_transfer_volume = 0.0;
        let mut denom_transfer_count = 0u32;
        let token_address = normalize_address_string(&self.contract_address);

        for transfer in &transaction.erc20_transfers {
            if same_address(&transfer.token_address, &token_address) {
                token_transfer_count = token_transfer_count.saturating_add(1);
                token_transfer_volume +=
                    scale_raw_units(transfer.amount.to_string(), self.decimals)?;
            } else if DENOM_ADDRESSES.contains_key(&transfer.token_address) {
                denom_transfer_count = denom_transfer_count.saturating_add(1);
            }
        }

        if token_transfer_count > 0 {
            self.activity.record_token_transfer(
                hash_string(&transaction.hash),
                transaction.block_number,
                Some(transaction.block_timestamp),
                token_transfer_count,
                token_transfer_volume,
            );
        }
        if denom_transfer_count > 0 {
            self.activity.record_denom_transfer(
                hash_string(&transaction.hash),
                transaction.block_number,
                Some(transaction.block_timestamp),
                denom_transfer_count,
            );
        }
        Ok(())
    }

    pub(super) fn record_pair_token_transfers_from_processed_transaction(
        &mut self,
        transaction: &ProcessedTransaction,
    ) -> Result<()> {
        if self.v2_pools.is_empty() {
            return Ok(());
        }

        let token_address = normalize_address_string(&self.contract_address);
        let pool_addresses: Vec<String> = self.v2_pools.keys().cloned().collect();
        let tx_hash = hash_string(&transaction.hash);
        let mut records = Vec::new();

        for transfer in &transaction.erc20_transfers {
            if !same_address(&transfer.token_address, &token_address) {
                continue;
            }

            let from_address = address_string(&transfer.from_address);
            let to_address = address_string(&transfer.to_address);
            let from_normalized = normalize_address(&from_address);
            let to_normalized = normalize_address(&to_address);
            let touched_pool = pool_addresses.iter().find(|pool_address| {
                from_normalized == **pool_address || to_normalized == **pool_address
            });
            let Some(pool_address) = touched_pool else {
                continue;
            };

            let amount = scale_raw_units(transfer.amount.to_string(), self.decimals)?;
            records.push((
                pool_address.clone(),
                from_address,
                to_address,
                amount,
                transfer.log_index,
                transaction_has_v2_pool_event(transaction, pool_address),
            ));
        }

        for (pool_address, from_address, to_address, amount, log_index, has_pool_event) in records {
            if let Some(pool) = self.v2_pools.get_mut(&pool_address) {
                pool.base.record_pair_token_transfer(
                    from_address,
                    to_address,
                    amount,
                    transaction.block_number,
                    transaction.block_timestamp,
                    tx_hash.clone(),
                    Some(log_index),
                    has_pool_event,
                );
            }
        }

        Ok(())
    }

    pub(super) fn record_v2_swap_activity(
        &mut self,
        pool_address: &str,
        events: &UniswapV2TransactionEvents,
        tx: &UniswapV2TxContext,
    ) -> Result<()> {
        let pool = self
            .uniswap_v2_pool(pool_address)
            .ok_or_else(|| eyre!("unknown Uniswap V2 pool {pool_address}"))?;
        let denom_address = pool.base.identity.denom_address.clone();
        let denom_decimals = pool.base.config.denom_decimals.unwrap_or(18);
        let token1_is_denom = pool.base.config.token1_is_denom.unwrap_or(false);

        for swap in &events.swaps {
            if !same_address(&parse_address_lossy(pool_address), &swap.pair_address) {
                continue;
            }

            let denom_in = if token1_is_denom {
                scale_raw_units(&swap.amount1_in, denom_decimals)?
            } else {
                scale_raw_units(&swap.amount0_in, denom_decimals)?
            };
            let denom_out = if token1_is_denom {
                scale_raw_units(&swap.amount1_out, denom_decimals)?
            } else {
                scale_raw_units(&swap.amount0_out, denom_decimals)?
            };
            self.record_swap_activity(
                &tx.tx_hash,
                tx.block_number,
                tx.block_timestamp,
                &denom_address,
                denom_in,
                denom_out,
            );
        }

        Ok(())
    }

    pub(super) fn record_v3_swap_activity(
        &mut self,
        pool_address: &str,
        transaction: &ProcessedTransaction,
        tx: &UniswapV2TxContext,
    ) -> Result<()> {
        let pool = self
            .uniswap_v3_pool(pool_address)
            .ok_or_else(|| eyre!("unknown Uniswap V3 pool {pool_address}"))?;
        let denom_address = pool.base.identity.denom_address.clone();
        let denom_decimals = pool.base.config.denom_decimals.unwrap_or(18);
        let token1_is_denom = pool.base.config.token1_is_denom.unwrap_or(false);

        for event in &transaction.uniswap_v3_swaps {
            if !same_address(&event.pool_address, pool_address) {
                continue;
            }
            let (denom_in, denom_out) = signed_denom_swap_amounts(
                event.amount0,
                event.amount1,
                self.decimals,
                denom_decimals,
                token1_is_denom,
            );
            self.record_swap_activity(
                &tx.tx_hash,
                tx.block_number,
                tx.block_timestamp,
                &denom_address,
                denom_in,
                denom_out,
            );
        }

        Ok(())
    }

    pub(super) fn record_v4_swap_activity(
        &mut self,
        pool_key: &str,
        transaction: &ProcessedTransaction,
        tx: &UniswapV2TxContext,
    ) -> Result<()> {
        let pool = self
            .uniswap_v4_pool(pool_key)
            .ok_or_else(|| eyre!("unknown Uniswap V4 pool {pool_key}"))?;
        let denom_address = pool.base.identity.denom_address.clone();
        let denom_decimals = pool.base.config.denom_decimals.unwrap_or(18);
        let token1_is_denom = pool.base.config.token1_is_denom.unwrap_or(false);

        for event in &transaction.uniswap_v4_swaps {
            if normalize_address(v4_event_display_key(
                event.pool_manager_address,
                event.event_id,
            )) != normalize_address(pool_key)
            {
                continue;
            }
            let (denom_in, denom_out) = signed_denom_swap_amounts(
                event.amount0,
                event.amount1,
                self.decimals,
                denom_decimals,
                token1_is_denom,
            );
            self.record_swap_activity(
                &tx.tx_hash,
                tx.block_number,
                tx.block_timestamp,
                &denom_address,
                denom_out,
                denom_in,
            );
        }

        Ok(())
    }

    pub(super) fn record_swap_activity(
        &mut self,
        tx_hash: &str,
        block_number: u64,
        block_timestamp: u64,
        denom_address: &str,
        denom_in: f64,
        denom_out: f64,
    ) {
        if denom_in > 0.0 {
            self.activity.record_buy(
                tx_hash,
                block_number,
                Some(block_timestamp),
                denom_address,
                denom_in,
            );
        }
        if denom_out > 0.0 {
            self.activity.record_sell(
                tx_hash,
                block_number,
                Some(block_timestamp),
                denom_address,
                denom_out,
            );
        }
    }
}

fn transaction_has_v2_pool_event(transaction: &ProcessedTransaction, pool_address: &str) -> bool {
    transaction
        .uniswap_v2_syncs
        .iter()
        .any(|event| same_address(&event.pair_address, pool_address))
        || transaction
            .uniswap_v2_swaps
            .iter()
            .any(|event| same_address(&event.pair_address, pool_address))
        || transaction
            .uniswap_v2_mints
            .iter()
            .any(|event| same_address(&event.pair_address, pool_address))
        || transaction
            .uniswap_v2_burns
            .iter()
            .any(|event| same_address(&event.pair_address, pool_address))
}
