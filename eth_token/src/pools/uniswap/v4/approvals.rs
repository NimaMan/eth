use super::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4PositionApproval {
    pub position_id: String,
    pub owner: String,
    pub spender: String,
    pub block_number: u64,
    pub tx_hash: String,
    pub block_timestamp: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UniswapV4OperatorApproval {
    pub owner: String,
    pub operator: String,
    pub approved: bool,
    pub block_number: u64,
    pub tx_hash: String,
    pub block_timestamp: Option<u64>,
}

impl UniswapV4Pool {
    pub fn lp_holders(&self) -> Vec<LPHolderSnapshot> {
        let balances = self.lp_balances_by_holder();
        let approvals_by_holder = self.lp_approvals_by_holder(&balances);
        let total: f64 = balances.values().sum();
        let mut holders = balances
            .into_iter()
            .filter(|(_, balance)| *balance > 0.0)
            .map(|(address, balance)| LPHolderSnapshot {
                approvals: approvals_by_holder
                    .get(&address)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
                address,
                balance,
                share: if total > 0.0 {
                    (balance / total) * 100.0
                } else {
                    0.0
                },
            })
            .collect::<Vec<_>>();
        holders.sort_by(|left, right| {
            right
                .balance
                .partial_cmp(&left.balance)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.address.cmp(&right.address))
        });
        holders
    }

    pub fn total_approved_to_routers(&self) -> f64 {
        self.lp_holders()
            .into_iter()
            .map(|holder| {
                holder
                    .approvals
                    .values()
                    .filter(|approval| approval.is_router)
                    .map(|approval| approval.amount.min(holder.balance))
                    .sum::<f64>()
            })
            .sum()
    }

    pub fn lp_approved_percentage(&self) -> f64 {
        let total = self.lp_total_supply();
        if total == 0.0 {
            0.0
        } else {
            (self.total_approved_to_routers() / total) * 100.0
        }
    }

    pub fn last_lp_approval_block(&self) -> Option<u64> {
        self.lp_approval_events
            .last()
            .and_then(|event| event.get("block_number"))
            .and_then(Value::as_u64)
    }

    pub fn last_lp_approval_event(&self) -> Option<Value> {
        self.lp_approval_events.last().cloned()
    }

    pub fn holders_with_approvals(&self) -> Vec<String> {
        self.lp_holders()
            .into_iter()
            .filter(|holder| !holder.approvals.is_empty())
            .map(|holder| holder.address)
            .collect()
    }

    pub fn touches_position_approval(&self, transaction: &ProcessedTransaction) -> bool {
        transaction
            .erc721_approval_events
            .iter()
            .any(|approval| self.position_approval_matches_known_position(approval))
            || transaction
                .approval_for_all_events
                .iter()
                .any(|approval| self.operator_approval_matches_known_owner(approval))
    }

    pub(super) fn process_position_approvals(
        &mut self,
        transaction: &ProcessedTransaction,
        tx: &UniswapV2TxContext,
    ) {
        let mut events = Vec::new();
        for event in &transaction.erc721_approval_events {
            if self.position_approval_matches_known_position(event) {
                events.push(V4ApprovalAction::Position(event));
            }
        }
        for event in &transaction.approval_for_all_events {
            if self.operator_approval_matches_known_owner(event) {
                events.push(V4ApprovalAction::Operator(event));
            }
        }

        events.sort_by_key(V4ApprovalAction::log_index);
        for event in events {
            match event {
                V4ApprovalAction::Position(event) => self.process_position_approval(event, tx),
                V4ApprovalAction::Operator(event) => self.process_operator_approval(event, tx),
            }
        }
    }

    fn process_position_approval(&mut self, event: &ERC721ApprovalEvent, tx: &UniswapV2TxContext) {
        let Some(position_id) = self.position_id_for_token_id(event.token_id) else {
            return;
        };
        let owner = address_string(&event.owner);
        let spender = address_string(&event.approved_address);
        let amount = self
            .liquidity_positions
            .get(&position_id)
            .map(|position| position.liquidity as f64)
            .unwrap_or(0.0);

        if event.approved_address.is_zero() {
            self.position_approvals.remove(&position_id);
        } else {
            self.position_approvals.insert(
                position_id.clone(),
                UniswapV4PositionApproval {
                    position_id: position_id.clone(),
                    owner: owner.clone(),
                    spender: spender.clone(),
                    block_number: tx.block_number,
                    tx_hash: tx.tx_hash.clone(),
                    block_timestamp: Some(tx.block_timestamp),
                },
            );
        }

        append_with_history_limit(
            &mut self.lp_approval_events,
            json!({
                "approval_type": "erc721_position",
                "block_number": tx.block_number,
                "block_timestamp": tx.block_timestamp,
                "tx_hash": tx.tx_hash,
                "position_id": position_id,
                "owner": owner,
                "spender": spender,
                "amount": amount,
                "approved": !event.approved_address.is_zero(),
                "is_router": self.known_routers.contains(&address_string(&event.approved_address)),
                "log_index": event.log_index,
            }),
            self.base.config.history_limit,
        );
    }

    fn process_operator_approval(&mut self, event: &ApprovalForAllEvent, tx: &UniswapV2TxContext) {
        let owner = address_string(&event.owner);
        let operator = address_string(&event.operator);
        let amount = self
            .lp_balances_by_holder()
            .get(&owner)
            .copied()
            .unwrap_or(0.0);

        if event.approved {
            self.operator_approvals
                .entry(owner.clone())
                .or_default()
                .insert(
                    operator.clone(),
                    UniswapV4OperatorApproval {
                        owner: owner.clone(),
                        operator: operator.clone(),
                        approved: true,
                        block_number: tx.block_number,
                        tx_hash: tx.tx_hash.clone(),
                        block_timestamp: Some(tx.block_timestamp),
                    },
                );
        } else if let Some(approvals) = self.operator_approvals.get_mut(&owner) {
            approvals.remove(&operator);
            if approvals.is_empty() {
                self.operator_approvals.remove(&owner);
            }
        }

        append_with_history_limit(
            &mut self.lp_approval_events,
            json!({
                "approval_type": "erc721_approval_for_all",
                "block_number": tx.block_number,
                "block_timestamp": tx.block_timestamp,
                "tx_hash": tx.tx_hash,
                "owner": owner,
                "spender": operator,
                "operator": address_string(&event.operator),
                "amount": amount,
                "approved": event.approved,
                "is_router": self.known_routers.contains(&address_string(&event.operator)),
                "log_index": event.log_index,
            }),
            self.base.config.history_limit,
        );
    }

    fn position_approval_matches_known_position(&self, approval: &ERC721ApprovalEvent) -> bool {
        let Some(position_manager) = self.position_manager_address.as_ref() else {
            return false;
        };
        same_address(&approval.token_address, position_manager)
            && self.position_id_for_token_id(approval.token_id).is_some()
    }

    fn operator_approval_matches_known_owner(&self, approval: &ApprovalForAllEvent) -> bool {
        let Some(position_manager) = self.position_manager_address.as_ref() else {
            return false;
        };
        let owner = address_string(&approval.owner);
        same_address(&approval.token_address, position_manager)
            && (self.operator_approvals.contains_key(&owner)
                || self
                    .liquidity_positions
                    .values()
                    .any(|position| position.owner == owner))
    }

    fn lp_approvals_by_holder(
        &self,
        balances: &BTreeMap<String, f64>,
    ) -> BTreeMap<String, BTreeMap<String, LPApprovalSnapshot>> {
        let mut approvals = BTreeMap::<String, BTreeMap<String, LPApprovalSnapshot>>::new();
        for position in self.liquidity_positions.values() {
            if position.liquidity == 0 {
                continue;
            }
            let Some(approval) = self.position_approvals.get(&position.position_id) else {
                continue;
            };
            if approval.owner != position.owner || approval.spender == V4_NATIVE_ETH_ADDRESS {
                continue;
            }
            add_lp_approval_amount(
                &mut approvals,
                &self.known_routers,
                &position.owner,
                &approval.spender,
                position.liquidity as f64,
                approval.block_number,
                &approval.tx_hash,
            );
        }

        for (owner, operators) in &self.operator_approvals {
            let Some(balance) = balances
                .get(owner)
                .copied()
                .filter(|balance| *balance > 0.0)
            else {
                continue;
            };
            for approval in operators.values().filter(|approval| approval.approved) {
                set_lp_approval_amount_at_least(
                    &mut approvals,
                    &self.known_routers,
                    owner,
                    &approval.operator,
                    balance,
                    approval.block_number,
                    &approval.tx_hash,
                );
            }
        }
        approvals
    }
}

enum V4ApprovalAction<'a> {
    Position(&'a ERC721ApprovalEvent),
    Operator(&'a ApprovalForAllEvent),
}

impl V4ApprovalAction<'_> {
    pub(super) fn log_index(&self) -> u64 {
        match self {
            Self::Position(event) => event.log_index,
            Self::Operator(event) => event.log_index,
        }
    }
}
