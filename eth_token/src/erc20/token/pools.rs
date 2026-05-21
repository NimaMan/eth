use super::*;

impl ERC20Token {
    pub fn create_uniswap_v2_pool(
        &mut self,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        mut config: BasePoolConfig,
        known_routers: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> &mut UniswapV2Pool {
        config.token_decimals = self.decimals;
        let pool = UniswapV2Pool::new(
            pool_address,
            self.contract_address.clone(),
            denom_address,
            config,
            known_routers,
        );
        let pool_address = pool.base.identity.pool_address.clone();
        self.add_uniswap_v2_pool(pool);
        self.v2_pools
            .get_mut(&pool_address)
            .expect("pool was inserted")
    }

    pub fn create_known_v2_pool(
        &mut self,
        protocol: KnownV2Protocol,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        mut config: BasePoolConfig,
        known_routers: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> &mut UniswapV2Pool {
        config.token_decimals = self.decimals;
        let pool = UniswapV2Pool::new_with_protocol(
            pool_address,
            self.contract_address.clone(),
            denom_address,
            protocol.label(),
            config,
            known_routers,
        );
        let pool_address = pool.base.identity.pool_address.clone();
        self.add_uniswap_v2_pool(pool);
        self.v2_pools
            .get_mut(&pool_address)
            .expect("pool was inserted")
    }

    pub fn add_uniswap_v2_pool(&mut self, pool: UniswapV2Pool) -> Option<UniswapV2Pool> {
        let mut pool = pool;
        pool.base
            .register_token_control_addresses(&self.token_control_addresses);
        let pool_address = pool.base.identity.pool_address.clone();
        let token_address = pool.base.identity.token_address.clone();
        let denom_address = pool.base.identity.denom_address.clone();
        let token_decimals = pool.base.config.token_decimals;
        let denom_decimals = pool.base.config.denom_decimals.unwrap_or(token_decimals);
        let history_limit = pool.base.config.history_limit;
        self.pnl.register_pool(
            &pool_address,
            &token_address,
            &denom_address,
            token_decimals,
            denom_decimals,
            history_limit,
        );
        let previous = self.v2_pools.insert(pool_address, pool);
        self.refresh_lifecycle_status();
        previous
    }

    pub fn uniswap_v2_pool(&self, pool_address: impl AsRef<str>) -> Option<&UniswapV2Pool> {
        self.v2_pools.get(&normalize_address(pool_address))
    }

    pub fn uniswap_v2_pool_mut(
        &mut self,
        pool_address: impl AsRef<str>,
    ) -> Option<&mut UniswapV2Pool> {
        self.v2_pools.get_mut(&normalize_address(pool_address))
    }

    pub fn update_uniswap_v2_pool_events(
        &mut self,
        pool_address: impl AsRef<str>,
        events: &UniswapV2TransactionEvents,
        tx: &UniswapV2TxContext,
    ) -> Result<()> {
        let pool_address = normalize_address(pool_address);
        self.record_transaction_metadata(
            &tx.tx_hash,
            tx.from_address.as_deref(),
            tx.block_number,
            tx.block_timestamp,
        );
        self.record_v2_swap_activity(&pool_address, events, tx)?;
        let pool = self
            .uniswap_v2_pool_mut(&pool_address)
            .ok_or_else(|| eyre!("unknown Uniswap V2 pool {pool_address}"))?;
        pool.update_from_events(events, tx)?;
        self.refresh_lifecycle_status();
        Ok(())
    }

    pub fn update_uniswap_v2_pool_from_processed_transaction(
        &mut self,
        pool_address: impl AsRef<str>,
        transaction: &ProcessedTransaction,
    ) -> Result<()> {
        let pool_address = normalize_address(pool_address);
        let tx_context = UniswapV2TxContext {
            block_number: transaction.block_number,
            block_timestamp: transaction.block_timestamp,
            tx_hash: hash_string(&transaction.hash),
            from_address: Some(address_string(&transaction.from_address)),
        };
        let events = v2_events_from_processed_transaction(transaction, &pool_address);

        self.record_transaction_metadata(
            &tx_context.tx_hash,
            tx_context.from_address.as_deref(),
            tx_context.block_number,
            tx_context.block_timestamp,
        );
        self.record_bribe_activity_from_processed_transaction(transaction)?;
        self.record_v2_swap_activity(&pool_address, &events, &tx_context)?;
        let (
            pnl_pool_address,
            pnl_token_address,
            pnl_denom_address,
            pnl_token_decimals,
            pnl_denom_decimals,
            pnl_history_limit,
        ) = {
            let pool = self
                .uniswap_v2_pool_mut(&pool_address)
                .ok_or_else(|| eyre!("unknown Uniswap V2 pool {pool_address}"))?;
            pool.update_from_events(&events, &tx_context)?;
            for transfer in lp_transfers_from_processed_transaction(transaction, &pool_address) {
                pool.process_lp_transfer(&transfer)?;
            }
            for approval in lp_approvals_from_processed_transaction(transaction, &pool_address) {
                pool.process_lp_approval(&approval)?;
            }
            (
                pool.base.identity.pool_address.clone(),
                pool.base.identity.token_address.clone(),
                pool.base.identity.denom_address.clone(),
                pool.base.config.token_decimals,
                pool.base
                    .config
                    .denom_decimals
                    .unwrap_or(pool.base.config.token_decimals),
                pool.base.config.history_limit,
            )
        };
        self.pnl.record_v2_pool_transaction(
            pnl_pool_address,
            pnl_token_address,
            pnl_denom_address,
            pnl_token_decimals,
            pnl_denom_decimals,
            pnl_history_limit,
            transaction,
        );
        self.refresh_lifecycle_status();
        Ok(())
    }

    pub fn create_sushiswap_v2_pool(
        &mut self,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        mut config: BasePoolConfig,
        known_routers: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> &mut UniswapV2Pool {
        config.token_decimals = self.decimals;
        let pool = new_sushiswap_v2_pool(
            pool_address,
            self.contract_address.clone(),
            denom_address,
            config,
            known_routers,
        );
        let pool_address = pool.base.identity.pool_address.clone();
        self.add_uniswap_v2_pool(pool);
        self.v2_pools
            .get_mut(&pool_address)
            .expect("pool was inserted")
    }

    pub fn create_uniswap_v3_pool(
        &mut self,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        token0: impl Into<String>,
        token1: impl Into<String>,
        fee_tier: u32,
        tick_spacing: i32,
        mut config: BasePoolConfig,
    ) -> &mut UniswapV3Pool {
        config.token_decimals = self.decimals;
        let pool = UniswapV3Pool::new(
            pool_address,
            self.contract_address.clone(),
            denom_address,
            token0,
            token1,
            fee_tier,
            tick_spacing,
            config,
        );
        let pool_address = pool.base.identity.pool_address.clone();
        self.add_uniswap_v3_pool(pool);
        self.v3_pools
            .get_mut(&pool_address)
            .expect("pool was inserted")
    }

    pub fn create_known_v3_pool(
        &mut self,
        protocol: KnownV3Protocol,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        token0: impl Into<String>,
        token1: impl Into<String>,
        fee_tier: u32,
        tick_spacing: i32,
        mut config: BasePoolConfig,
    ) -> &mut UniswapV3Pool {
        config.token_decimals = self.decimals;
        let pool = match protocol {
            KnownV3Protocol::UniswapV3 => UniswapV3Pool::new(
                pool_address,
                self.contract_address.clone(),
                denom_address,
                token0,
                token1,
                fee_tier,
                tick_spacing,
                config,
            ),
            KnownV3Protocol::SushiSwapV3 => new_sushiswap_v3_pool(
                pool_address,
                self.contract_address.clone(),
                denom_address,
                token0,
                token1,
                fee_tier,
                tick_spacing,
                config,
            ),
            KnownV3Protocol::PancakeSwapV3 => new_pancakeswap_v3_pool(
                pool_address,
                self.contract_address.clone(),
                denom_address,
                token0,
                token1,
                fee_tier,
                tick_spacing,
                config,
            ),
        };
        let pool_address = pool.base.identity.pool_address.clone();
        self.add_uniswap_v3_pool(pool);
        self.v3_pools
            .get_mut(&pool_address)
            .expect("pool was inserted")
    }

    pub fn add_uniswap_v3_pool(&mut self, pool: UniswapV3Pool) -> Option<UniswapV3Pool> {
        let mut pool = pool;
        pool.base
            .register_token_control_addresses(&self.token_control_addresses);
        let pool_address = pool.base.identity.pool_address.clone();
        let previous = self.v3_pools.insert(pool_address, pool);
        self.refresh_lifecycle_status();
        previous
    }

    pub fn uniswap_v3_pool(&self, pool_address: impl AsRef<str>) -> Option<&UniswapV3Pool> {
        self.v3_pools.get(&normalize_address(pool_address))
    }

    pub fn uniswap_v3_pool_mut(
        &mut self,
        pool_address: impl AsRef<str>,
    ) -> Option<&mut UniswapV3Pool> {
        self.v3_pools.get_mut(&normalize_address(pool_address))
    }

    pub fn update_uniswap_v3_pool_from_processed_transaction(
        &mut self,
        pool_address: impl AsRef<str>,
        transaction: &ProcessedTransaction,
    ) -> Result<()> {
        let pool_address = normalize_address(pool_address);
        let tx_context = UniswapV2TxContext {
            block_number: transaction.block_number,
            block_timestamp: transaction.block_timestamp,
            tx_hash: hash_string(&transaction.hash),
            from_address: Some(address_string(&transaction.from_address)),
        };
        self.record_transaction_metadata(
            &tx_context.tx_hash,
            tx_context.from_address.as_deref(),
            tx_context.block_number,
            tx_context.block_timestamp,
        );
        self.record_bribe_activity_from_processed_transaction(transaction)?;
        self.record_v3_swap_activity(&pool_address, transaction, &tx_context)?;
        let pool = self
            .uniswap_v3_pool_mut(&pool_address)
            .ok_or_else(|| eyre!("unknown Uniswap V3 pool {pool_address}"))?;
        pool.update_from_processed_transaction(transaction, &tx_context)?;
        self.refresh_lifecycle_status();
        Ok(())
    }

    pub fn add_uniswap_v4_pool(&mut self, pool: UniswapV4Pool) -> Option<UniswapV4Pool> {
        let mut pool = pool;
        pool.base
            .register_token_control_addresses(&self.token_control_addresses);
        let pool_key = pool.base.identity.pool_address.clone();
        let previous = self.v4_pools.insert(pool_key, pool);
        self.refresh_lifecycle_status();
        previous
    }

    pub fn uniswap_v4_pool(&self, pool_key: impl AsRef<str>) -> Option<&UniswapV4Pool> {
        self.v4_pools.get(&normalize_address(pool_key))
    }

    pub fn uniswap_v4_pool_mut(&mut self, pool_key: impl AsRef<str>) -> Option<&mut UniswapV4Pool> {
        self.v4_pools.get_mut(&normalize_address(pool_key))
    }

    pub fn update_uniswap_v4_pool_from_processed_transaction(
        &mut self,
        pool_key: impl AsRef<str>,
        transaction: &ProcessedTransaction,
    ) -> Result<()> {
        let pool_key = normalize_address(pool_key);
        let tx_context = UniswapV2TxContext {
            block_number: transaction.block_number,
            block_timestamp: transaction.block_timestamp,
            tx_hash: hash_string(&transaction.hash),
            from_address: Some(address_string(&transaction.from_address)),
        };
        self.record_transaction_metadata(
            &tx_context.tx_hash,
            tx_context.from_address.as_deref(),
            tx_context.block_number,
            tx_context.block_timestamp,
        );
        self.record_bribe_activity_from_processed_transaction(transaction)?;
        self.record_v4_swap_activity(&pool_key, transaction, &tx_context)?;
        let pool = self
            .uniswap_v4_pool_mut(&pool_key)
            .ok_or_else(|| eyre!("unknown Uniswap V4 pool {pool_key}"))?;
        pool.update_from_processed_transaction(transaction, &tx_context)?;
        self.refresh_lifecycle_status();
        Ok(())
    }

    pub fn create_curve_pool(
        &mut self,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        mut config: BasePoolConfig,
        name: Option<String>,
        lp_token_address: Option<String>,
        base_token_index: usize,
        quote_token_index: usize,
        tokens: Vec<CurvePoolToken>,
    ) -> &mut CurvePool {
        config.token_decimals = self.decimals;
        let pool = CurvePool::new(
            pool_address,
            self.contract_address.clone(),
            denom_address,
            config,
            name,
            lp_token_address,
            base_token_index,
            quote_token_index,
            tokens,
        );
        let pool_address = pool.base.identity.pool_address.clone();
        self.add_curve_pool(pool);
        self.curve_pools
            .get_mut(&pool_address)
            .expect("pool was inserted")
    }

    pub fn add_curve_pool(&mut self, pool: CurvePool) -> Option<CurvePool> {
        let mut pool = pool;
        pool.base
            .register_token_control_addresses(&self.token_control_addresses);
        let pool_address = pool.base.identity.pool_address.clone();
        let previous = self.curve_pools.insert(pool_address, pool);
        self.refresh_lifecycle_status();
        previous
    }

    pub fn create_balancer_pool(
        &mut self,
        pool_address: impl Into<String>,
        denom_address: impl Into<String>,
        mut config: BasePoolConfig,
        pool_id: impl Into<String>,
        vault_address: impl Into<String>,
        swap_fee_bps: Option<u32>,
        tokens: Vec<BalancerPoolToken>,
    ) -> &mut BalancerPool {
        config.token_decimals = self.decimals;
        let pool = BalancerPool::new(
            pool_address,
            self.contract_address.clone(),
            denom_address,
            config,
            pool_id,
            vault_address,
            swap_fee_bps,
            tokens,
        );
        let pool_address = pool.base.identity.pool_address.clone();
        self.add_balancer_pool(pool);
        self.balancer_pools
            .get_mut(&pool_address)
            .expect("pool was inserted")
    }

    pub fn add_balancer_pool(&mut self, pool: BalancerPool) -> Option<BalancerPool> {
        let mut pool = pool;
        pool.base
            .register_token_control_addresses(&self.token_control_addresses);
        let pool_address = pool.base.identity.pool_address.clone();
        let previous = self.balancer_pools.insert(pool_address, pool);
        self.refresh_lifecycle_status();
        previous
    }

    pub fn pool_addresses(&self) -> Vec<String> {
        let mut addresses = self.uniswap_v2_pool_addresses();
        addresses.extend(self.uniswap_v3_pool_addresses());
        addresses.extend(self.uniswap_v4_pool_keys());
        addresses.extend(self.curve_pools.keys().cloned());
        addresses.extend(self.balancer_pools.keys().cloned());
        addresses.sort();
        addresses.dedup();
        addresses
    }

    pub fn uniswap_v2_pool_addresses(&self) -> Vec<String> {
        let mut addresses: Vec<_> = self.v2_pools.keys().cloned().collect();
        addresses.sort();
        addresses
    }

    pub fn uniswap_v3_pool_addresses(&self) -> Vec<String> {
        let mut addresses: Vec<_> = self.v3_pools.keys().cloned().collect();
        addresses.sort();
        addresses
    }

    pub fn uniswap_v4_pool_keys(&self) -> Vec<String> {
        let mut addresses: Vec<_> = self.v4_pools.keys().cloned().collect();
        addresses.sort();
        addresses
    }

    pub fn pool_count(&self) -> usize {
        self.v2_pools.len()
            + self.v3_pools.len()
            + self.v4_pools.len()
            + self.curve_pools.len()
            + self.balancer_pools.len()
    }

    pub fn has_pool(&self) -> bool {
        self.pool_count() > 0
    }

    pub fn current_prices(&self) -> HashMap<String, PoolStateSnapshot> {
        let mut prices = HashMap::new();
        for (address, pool) in &self.v2_pools {
            prices.insert(address.clone(), PoolStateSnapshot::from(pool));
        }
        for (address, pool) in &self.v3_pools {
            prices.insert(address.clone(), PoolStateSnapshot::from(pool));
        }
        for (address, pool) in &self.v4_pools {
            prices.insert(address.clone(), PoolStateSnapshot::from(pool));
        }
        for (address, pool) in &self.curve_pools {
            prices.insert(address.clone(), PoolStateSnapshot::from_base(&pool.base));
        }
        for (address, pool) in &self.balancer_pools {
            prices.insert(address.clone(), PoolStateSnapshot::from_base(&pool.base));
        }
        prices
    }

    pub fn get_pool_info(&self) -> HashMap<String, PoolStateSnapshot> {
        self.current_prices()
    }

    pub fn total_liquidity_by_denom(&self) -> HashMap<String, f64> {
        let mut liquidity = HashMap::new();
        for pool in self.all_pool_bases() {
            *liquidity
                .entry(pool.identity.denom_address.clone())
                .or_insert(0.0) += pool.state.total_liquidity;
        }
        liquidity
    }

    pub(super) fn register_control_addresses_with_pools(
        &mut self,
        addresses: impl IntoIterator<Item = String>,
    ) {
        let addresses: Vec<_> = addresses.into_iter().collect();
        if addresses.is_empty() {
            return;
        }
        for pool in self.v2_pools.values_mut() {
            pool.base.register_token_control_addresses(&addresses);
        }
        for pool in self.v3_pools.values_mut() {
            pool.base.register_token_control_addresses(&addresses);
        }
        for pool in self.v4_pools.values_mut() {
            pool.base.register_token_control_addresses(&addresses);
        }
        for pool in self.curve_pools.values_mut() {
            pool.base.register_token_control_addresses(&addresses);
        }
        for pool in self.balancer_pools.values_mut() {
            pool.base.register_token_control_addresses(&addresses);
        }
    }

    pub fn pool_base(&self, pool_key: impl AsRef<str>) -> Option<&BasePool> {
        let pool_key = normalize_address(pool_key);
        self.v2_pools
            .get(&pool_key)
            .map(|pool| &pool.base)
            .or_else(|| self.v3_pools.get(&pool_key).map(|pool| &pool.base))
            .or_else(|| self.v4_pools.get(&pool_key).map(|pool| &pool.base))
            .or_else(|| self.curve_pools.get(&pool_key).map(|pool| &pool.base))
            .or_else(|| self.balancer_pools.get(&pool_key).map(|pool| &pool.base))
    }

    pub fn all_pool_bases(&self) -> Vec<&BasePool> {
        let mut pools = Vec::with_capacity(self.pool_count());
        pools.extend(self.v2_pools.values().map(|pool| &pool.base));
        pools.extend(self.v3_pools.values().map(|pool| &pool.base));
        pools.extend(self.v4_pools.values().map(|pool| &pool.base));
        pools.extend(self.curve_pools.values().map(|pool| &pool.base));
        pools.extend(self.balancer_pools.values().map(|pool| &pool.base));
        pools
    }
}
