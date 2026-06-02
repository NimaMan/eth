use super::*;

use crate::custody::{
    CustodyCapability, CustodyDrainVictim, CustodyFinding, CustodyState, HolderBalanceLedger,
};

/// A buyer is "wiped" when at least this fraction of the balance they held
/// (reconstructed from Transfer events) was confiscated via event-less custody
/// drains. The event-less drain is invisible to the transfer ledger, so the
/// ledger's expected balance is the holder's pre-drain holdings.
const CUSTODY_BUYER_WIPE_FRACTION: f64 = 0.95;
/// Minimum number of independently-wiped buyers to classify the token as a
/// systematic buyer-confiscation rug (rather than a single holder-drain
/// finding). Two distinct victims is the smallest "they do this to buyers"
/// signal; tune up to reduce false positives, down to fire earlier.
const CUSTODY_MIN_WIPED_BUYERS: usize = 2;

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
        self.mark_holder_balance_backdoor_drains_from_processed_transaction(transaction);
        self.refresh_lifecycle_status();
        Ok(())
    }

    /// Detect a holder-balance backdoor drain in this transaction and mark the
    /// token's pools as scam so the live feed surfaces it as a liquidity-removal
    /// class signal (`Backdoored Holder-Balance Drain`).
    ///
    /// Pattern: a control address calls `transferFrom(holder, dead, amount)` to
    /// take a holder's balance with no normal Transfer log (zero-allowance
    /// backdoor), or moves a holder's balance to a burn address. This is
    /// distinct from `pair_balance_backdoor_drain`, where the drained `from` is
    /// the pool/pair itself; here `from` is an ordinary holder (e.g. our trading
    /// vault) and pool reserves do not move, so reserve-drain inference never
    /// fires.
    pub(super) fn mark_holder_balance_backdoor_drains_from_processed_transaction(
        &mut self,
        transaction: &ProcessedTransaction,
    ) {
        let tx_hash = hash_string(&transaction.hash);
        let block_number = transaction.block_number;

        // Token-level control addresses — kept only as corroborating evidence,
        // NOT as a gate. The backdoor helper that performs the drain is usually
        // not a known owner/creator, so gating on it would miss the drain.
        let mut control: HashSet<String> = self
            .token_control_addresses
            .iter()
            .map(|address| normalize_address(address))
            .collect();
        if let Some(creator) = self.creator_address.as_deref() {
            control.insert(normalize_address(creator));
        }
        if let Some(owner) = self.current_owner() {
            control.insert(normalize_address(&owner));
        }

        // Venue addresses (pools + the token contract) are NOT "holders" — a
        // drain of those is the pair-balance variant handled elsewhere.
        let mut venue_addresses: HashSet<String> = self
            .all_pool_bases()
            .iter()
            .map(|pool| normalize_address(&pool.identity.pool_address))
            .collect();
        venue_addresses.insert(normalize_address(&self.contract_address));

        // The drain is an *internal* standard-ERC-20 call: some contract calls
        // `transferFrom(holder, recipient, amount)` on this token from inside
        // another contract, so the top-level `tx.to` is the helper, not the
        // token, and it never appears as a top-level `transfer_from_call`. It is
        // captured on the ProcessedTransaction as an `internal_erc20_calls`
        // entry (decoded from the trace).
        //
        // The unambiguous backdoor signature: a SUCCESSFUL `transferFrom` that
        // moved a nonzero balance but emitted NO matching `Transfer` event. A
        // standard-compliant ERC-20 always emits `Transfer` on a balance move,
        // so an event-less successful `transferFrom` means the token has a
        // backdoor that mutates balances behind the event interface. This is the
        // exact move that fools event-derived balance/valuation, so it is what we
        // must label — independent of whether we have identified the caller as a
        // control address.
        let token_address = normalize_address(&self.contract_address);
        let decimals = self.decimals;
        let drains: Vec<&tx_processor::InternalErc20Call> = transaction
            .internal_erc20_calls
            .iter()
            .filter(|call| {
                call.succeeded
                    && matches!(call.kind, tx_processor::Erc20CallKind::TransferFrom)
                    && !call.amount.is_zero()
            })
            .filter(|call| {
                if normalize_address(address_string(&call.token_address)) != token_address {
                    return false;
                }
                let from = address_string(&call.from_address);
                if venue_addresses.contains(&normalize_address(&from)) || is_burn_address_str(&from)
                {
                    return false;
                }
                // Event-less move: a successful transferFrom with no matching
                // Transfer log is a balance mutation hidden from the event
                // interface — the backdoor-drain signature.
                let has_matching_transfer = transaction.erc20_transfers.iter().any(|t| {
                    t.token_address == call.token_address
                        && t.from_address == call.from_address
                        && t.to_address == call.to_address
                        && t.amount == call.amount
                });
                !has_matching_transfer
            })
            .collect();

        if drains.is_empty() {
            return;
        }

        // Custody axis: record WHO lost tokens (the victim holder = `from`) so a
        // position held at that address can be marked drained. This is distinct
        // from the pool scam label below, which only answers sellability.
        let mut representative_evidence = None;
        for drain in &drains {
            let caller = address_string(&drain.caller);
            let from = address_string(&drain.from_address);
            let to = address_string(&drain.to_address);
            let amount_scaled = drain.amount.to_string().parse::<f64>().unwrap_or(0.0)
                / 10f64.powi(decimals as i32);
            // BurnDrain when the balance went to a dead/burn address; otherwise a
            // live recipient took it (Seize).
            let capability = if is_burn_address_str(&to) {
                CustodyCapability::BurnDrain
            } else {
                CustodyCapability::Seize
            };
            let evidence = json!({
                "risk_kind": crate::pools::SCAM_HOLDER_BALANCE_BACKDOOR_DRAIN,
                "detail": "internal_transfer_from_holder_without_matching_transfer_event",
                "caller_is_known_control":
                    control.contains(&normalize_address(&caller)),
                "caller": caller,
                "victim": from,
                "to_address": to,
                "amount": drain.amount.to_string(),
                "amount_scaled": amount_scaled,
                "call_depth": drain.depth,
                "tx_hash": tx_hash,
                "block_number": block_number,
            });

            // Idempotent across replay/reprocessing: dedupe by (tx, victim, to,
            // amount).
            let already_recorded = self.custody_findings.iter().any(|f| {
                f.evidence.get("tx_hash") == evidence.get("tx_hash")
                    && f.evidence.get("victim") == evidence.get("victim")
                    && f.evidence.get("to_address") == evidence.get("to_address")
                    && f.evidence.get("amount") == evidence.get("amount")
            });
            if !already_recorded {
                self.custody_findings.push(CustodyFinding {
                    capability,
                    state: CustodyState::Realized,
                    block_number: Some(block_number),
                    evidence: evidence.clone(),
                });
            }
            if representative_evidence.is_none() {
                representative_evidence = Some(evidence);
            }
        }

        // Sellability axis: flag the pools as scam so the live feed surfaces it
        // as a liquidity-removal-class signal.
        let evidence = representative_evidence.expect("non-empty drains => some evidence");
        mark_pool_holder_balance_drain(
            self.v2_pools.values_mut().map(|pool| &mut pool.base),
            block_number,
            Some(&tx_hash),
            &evidence,
        );
        mark_pool_holder_balance_drain(
            self.v3_pools.values_mut().map(|pool| &mut pool.base),
            block_number,
            Some(&tx_hash),
            &evidence,
        );
        mark_pool_holder_balance_drain(
            self.v4_pools.values_mut().map(|pool| &mut pool.base),
            block_number,
            Some(&tx_hash),
            &evidence,
        );

        // Aggregate axis: if the per-victim findings now show a material number
        // of buyers each wiped of >=95% of their holdings, upgrade the pool
        // mechanism to the systematic buyer-confiscation rug — the clearer,
        // more specific classification.
        self.classify_custody_buyer_confiscation();
    }

    /// Persist reconciliation-discovered holder confiscations as custody
    /// findings. This catches victims whose balance disappeared from state even
    /// when the exact backdoor call frame was not traced/cached for that block.
    ///
    /// The method records only the newly-unexplained delta above already-known
    /// custody findings for the same holder, so it is safe to call repeatedly or
    /// after a trace-backed finding for the same victim.
    pub fn record_reconciled_holder_confiscations(
        &mut self,
        victims: &[CustodyDrainVictim],
        evidence_by_holder: Option<&HashMap<String, Value>>,
    ) -> usize {
        let mut added = 0usize;
        let mut representative_evidence = None;
        let mut earliest_block = None::<u64>;

        for victim in victims {
            if victim.missing_balance <= 0.0 {
                continue;
            }
            let already_recorded = self.custody_drained_amount(&victim.holder);
            let amount_to_record = (victim.missing_balance - already_recorded).max(0.0);
            if amount_to_record <= reconciliation_amount_epsilon(victim.missing_balance) {
                continue;
            }

            let holder_key = normalize_address(&victim.holder);
            let context = evidence_by_holder
                .and_then(|contexts| contexts.get(&holder_key))
                .cloned()
                .unwrap_or_else(|| json!({}));
            let evidence = json!({
                "risk_kind": crate::pools::SCAM_HOLDER_BALANCE_BACKDOOR_DRAIN,
                "source": "balance_reconciliation",
                "detail": "holder_balance_missing_from_state_without_transfer_event",
                "victim": holder_key,
                "block_number": victim.block_number,
                "expected_balance": victim.expected_balance,
                "actual_balance": victim.actual_balance,
                "missing_balance": victim.missing_balance,
                "already_recorded_scaled": already_recorded,
                "amount_scaled": amount_to_record,
                "drained_fraction": victim.drained_fraction,
                "expected_supply_share": victim.expected_supply_share,
                "context": context,
            });

            self.custody_findings.push(CustodyFinding {
                // Reconciliation proves the holder lost balance, but not where
                // it went. Use Seize unless trace evidence proves BurnDrain.
                capability: CustodyCapability::Seize,
                state: CustodyState::Realized,
                block_number: Some(victim.block_number),
                evidence: evidence.clone(),
            });
            added += 1;
            representative_evidence.get_or_insert(evidence);
            earliest_block = Some(
                earliest_block.map_or(victim.block_number, |block| block.min(victim.block_number)),
            );
        }

        if let (Some(block_number), Some(evidence)) = (earliest_block, representative_evidence) {
            mark_pool_holder_balance_drain(
                self.v2_pools.values_mut().map(|pool| &mut pool.base),
                block_number,
                None,
                &evidence,
            );
            mark_pool_holder_balance_drain(
                self.v3_pools.values_mut().map(|pool| &mut pool.base),
                block_number,
                None,
                &evidence,
            );
            mark_pool_holder_balance_drain(
                self.v4_pools.values_mut().map(|pool| &mut pool.base),
                block_number,
                None,
                &evidence,
            );
            self.classify_custody_buyer_confiscation();
        }

        added
    }

    /// Evaluate whether the recorded custody findings constitute a systematic
    /// buyer-token-confiscation rug: a material number of distinct buyers each
    /// confiscated of at least [`CUSTODY_BUYER_WIPE_FRACTION`] of the balance
    /// they held. Returns the summary evidence and earliest wipe block when the
    /// pattern holds. Pure read over the token's own data (Transfer-event ledger
    /// + custody findings); no external/state reads.
    fn evaluate_custody_buyer_confiscation(&self) -> Option<(u64, Value)> {
        if self.custody_findings.is_empty() {
            return None;
        }

        // Sum confiscated amount per victim, and track each victim's earliest
        // drain block.
        let mut drained_per_victim: HashMap<String, f64> = HashMap::new();
        let mut first_block_per_victim: HashMap<String, u64> = HashMap::new();
        for finding in &self.custody_findings {
            let Some(victim) = finding.evidence.get("victim").and_then(|v| v.as_str()) else {
                continue;
            };
            let amount = finding
                .evidence
                .get("amount_scaled")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let victim = normalize_address(victim);
            *drained_per_victim.entry(victim.clone()).or_default() += amount;
            if let Some(block) = finding.block_number {
                let entry = first_block_per_victim.entry(victim).or_insert(block);
                *entry = (*entry).min(block);
            }
        }

        // The event-less drain is invisible to the Transfer ledger, so the
        // ledger's expected balance is each victim's pre-drain holdings.
        let ledger = HolderBalanceLedger::from_token(self);

        let mut wiped_buyers: Vec<Value> = Vec::new();
        let mut earliest_block = u64::MAX;
        let mut total_drained = 0.0;
        for (victim, drained) in &drained_per_victim {
            if *drained <= 0.0 {
                continue;
            }
            // Pre-drain holdings = ledger expected balance, but never less than
            // what was demonstrably taken from the address.
            let held_before = ledger.expected_balance(victim).max(*drained);
            if held_before <= 0.0 {
                continue;
            }
            let fraction = (*drained / held_before).clamp(0.0, 1.0);
            if fraction >= CUSTODY_BUYER_WIPE_FRACTION {
                total_drained += *drained;
                if let Some(block) = first_block_per_victim.get(victim) {
                    earliest_block = earliest_block.min(*block);
                }
                wiped_buyers.push(json!({
                    "victim": victim,
                    "drained": drained,
                    "held_before": held_before,
                    "wiped_fraction": fraction,
                    "first_drain_block": first_block_per_victim.get(victim),
                }));
            }
        }

        if wiped_buyers.len() < CUSTODY_MIN_WIPED_BUYERS {
            return None;
        }

        let block = if earliest_block == u64::MAX {
            self.latest_block_number.unwrap_or_default()
        } else {
            earliest_block
        };
        let evidence = json!({
            "risk_kind": crate::pools::SCAM_CUSTODY_BUYER_TOKEN_CONFISCATION,
            "detail": "multiple_buyers_each_wiped_by_holder_balance_custody_drain",
            "wipe_fraction_threshold": CUSTODY_BUYER_WIPE_FRACTION,
            "wiped_buyer_count": wiped_buyers.len(),
            "total_drained_scaled": total_drained,
            "earliest_wipe_block": block,
            "wiped_buyers": wiped_buyers,
        });
        Some((block, evidence))
    }

    /// Upgrade every pool's scam mechanism to the buyer-confiscation rug when the
    /// aggregate pattern holds, overriding the generic holder-balance-drain
    /// label with the clearer, more specific classification.
    fn classify_custody_buyer_confiscation(&mut self) {
        let Some((block, evidence)) = self.evaluate_custody_buyer_confiscation() else {
            return;
        };
        for pool in self
            .v2_pools
            .values_mut()
            .map(|pool| &mut pool.base)
            .chain(self.v3_pools.values_mut().map(|pool| &mut pool.base))
            .chain(self.v4_pools.values_mut().map(|pool| &mut pool.base))
        {
            // Override only the holder-balance-drain primitive (or unflagged
            // pools); do not clobber an independent reserve-drain mechanism.
            let upgradeable = pool.scam_mechanism.is_none()
                || pool.scam_mechanism.as_deref()
                    == Some(crate::pools::SCAM_HOLDER_BALANCE_BACKDOOR_DRAIN);
            if !upgradeable {
                continue;
            }
            pool.mark_scam_mechanism(
                crate::pools::SCAM_CUSTODY_BUYER_TOKEN_CONFISCATION,
                Some(block),
                None,
                evidence.clone(),
            );
            pool.reserve_tracker.is_scam = true;
            pool.latest_block_number = Some(
                pool.latest_block_number
                    .map_or(block, |latest| latest.max(block)),
            );
        }
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

/// Mark each pool base as a holder-balance backdoor drain (only if it is not
/// already flagged scam, so a more specific reserve-drain mechanism is not
/// clobbered). Sets `reserve_tracker.is_scam` so `has_liquidity_removal()` is
/// true, which is what the live feed snapshot reads to surface the signal.
fn mark_pool_holder_balance_drain<'a>(
    pools: impl Iterator<Item = &'a mut BasePool>,
    block_number: u64,
    tx_hash: Option<&str>,
    evidence: &Value,
) {
    for pool in pools {
        if pool.reserve_tracker.is_scam || pool.scam_mechanism.is_some() {
            continue;
        }
        pool.mark_scam_mechanism(
            crate::pools::SCAM_HOLDER_BALANCE_BACKDOOR_DRAIN,
            Some(block_number),
            tx_hash.map(str::to_string),
            evidence.clone(),
        );
        pool.reserve_tracker.is_scam = true;
        // Advance the pool's latest block to the drain block so the live block
        // frame includes this pool at the drain block (`pool_is_in_frame`).
        // Without this the holder-drain tx never touches the pool, the pool is
        // omitted from the frame, and neither the liquidity-removal risk event
        // nor the position revaluation fires at the drain block.
        pool.latest_block_number = Some(
            pool.latest_block_number
                .map_or(block_number, |latest| latest.max(block_number)),
        );
    }
}

fn reconciliation_amount_epsilon(amount: f64) -> f64 {
    (amount.abs() * 1e-9).max(1e-9)
}

fn is_burn_address_str(address: &str) -> bool {
    reth_chain_query::common_addresses::is_burn_address_str(address)
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
