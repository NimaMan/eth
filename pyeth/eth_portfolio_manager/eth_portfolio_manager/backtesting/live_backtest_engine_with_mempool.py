"""
LiveBacktestEngineWithMempool: Real-time Strategy Execution with Mempool Monitoring

**Objective:**
----------
Create a comprehensive trading system that performs two main functions concurrently:
1.  **Real-time Strategy Execution:** Processes confirmed blockchain blocks, updates token states, executes defined trading strategies based on these updates, manages portfolio positions, and persists results.
2.  **Proactive Scam Detection:** Monitors the Ethereum mempool for pending transactions, simulates their potential impact on the liquidity (ETH levels) of known token pools, and issues alerts if a transaction is predicted to drain a pool below a critical threshold, potentially indicating a scam (e.g., rug pull).

**Core Architecture & Components:**
--------------------------------
This class orchestrates several asynchronous tasks and components:

1.  **`LiveBlockTokenProcessor` (`self.live_token_processor`):**
    *   **Source of Truth for Confirmed Blocks:** Manages the connection to the blockchain (via `BlockSubscriber`), processes historical blocks for warm-up, and processes live blocks as they arrive.
    *   **Token State Tracking:** Identifies token creations and updates within blocks using underlying `TokenDataFetcher` and `LiveTokensCache`.
    *   **Output:** Populates `self.unprocessed_token_updates` (an `asyncio.PriorityQueue`) with tuples `(block_number, updated_tokens)` and signals availability by setting `self.new_updates_event` (an `asyncio.Event`).

2.  **`MempoolProcessor` (`self.mempool_processor`):**
    *   **Independent Mempool Monitor:** Runs separately, continuously polling the node's mempool (e.g., via `txpool_content`).
    *   **State Diff Simulation:** For new pending/queued transactions, it attempts to simulate their execution (e.g., using `trace_call`) to determine potential state changes (ETH balances, storage, etc.).
    *   **Output:** Stores simulated state diffs keyed by affected address, accessible via `get_address_state_diffs(address)`.

3.  **Strategy Engines & Position Managers (`self.strategy_engines`, `self.strategy_position_managers`):**
    *   Standard components responsible for applying trading logic based on `updated_tokens` data and managing the resulting positions for each configured strategy.

4.  **Results Writer (`self.results_writer`):**
    *   (Optional) Handles persistence of strategy parameters, run details, and token positions to a database.

5.  **Internal Pool Tracking (`self._pool_eth_levels`, `_token_pools`, `_pool_token_map`):**
    *   Dictionaries maintained by this class to store the *current known ETH level* for monitored pools, map tokens to their pools, and map pools back to their tokens.

6.  **Internal Queues & Events:**
    *   `self._pool_updates_queue` (asyncio.Queue): Used to pass `(block, tokens)` from the main processing loop to the pool level update task.
    *   `self._pool_update_event` (asyncio.Event): Signal used by the main processing loop to notify the pool level update task that new data is in the `_pool_updates_queue`.

**Detailed Event & Data Flow:**
-----------------------------

**A) Confirmed Block Processing & Strategy Execution:**

1.  **Block Arrival:** `LiveBlockTokenProcessor` receives a new confirmed block.
2.  **Token Processing:** It processes the block, identifies `updated_tokens`, puts `(block, tokens)` onto its `unprocessed_token_updates` queue, and sets `live_token_processor.new_updates_event`.
3.  **Main Task Wake-up (`_process_token_updates`):** This task, running within `LiveBacktestEngineWithMempool`, wakes up upon seeing `live_token_processor.new_updates_event`.
4.  **Get Updates:** It gets `(block, tokens)` from the `live_token_processor.unprocessed_token_updates` queue.
5.  **Execute Strategies:** It calls `_update_strategy_positions` for each configured strategy, passing the `updated_tokens`.
6.  **Save Results:** It calls `_save_strategy_results_to_database` (if enabled) to persist strategy run info and updated positions.
7.  **Trigger Pool Update:** It puts the same `(block, tokens)` onto the internal `self._pool_updates_queue` and sets `self._pool_update_event`.
8.  **Mark Done:** It calls `task_done()` on the `live_token_processor.unprocessed_token_updates` queue.
9.  **Loop:** Repeats from step 3 if the queue is not empty, otherwise waits for the next `new_updates_event`.

**B) Pool Level Updating:**

1.  **Pool Task Wake-up (`_update_pool_levels_from_tokens`):** This task wakes up upon seeing `self._pool_update_event`.
2.  **Get Pool Updates:** It gets `(block, tokens)` from `self._pool_updates_queue`.
3.  **Update Internal State:** It calls `_update_pool_levels`, passing the `updated_tokens`.
4.  **`_update_pool_levels` Logic:** Iterates through `updated_tokens`, extracts `pool_addresses` (from `token.token_data`), calls `token.token_data.get_pool_reserve()` for each pool, and updates the internal `self._pool_eth_levels`, `self._token_pools`, and `self._pool_token_map` dictionaries.
5.  **Mark Done:** It calls `task_done()` on `self._pool_updates_queue`.
6.  **Loop:** Repeats from step 2 if the queue is not empty, otherwise waits for the next `_pool_update_event`.

**C) Mempool Scam Detection:**

1.  **Periodic Check (`_monitor_for_scams`):** This task runs every `self.poll_interval` seconds.
2.  **Call Scam Check Logic:** It calls `_check_for_scam_transactions`.
3.  **`_check_for_scam_transactions` Logic:**
    a.  Gets a copy of the current known pool levels from `self._pool_eth_levels`.
    b.  If no pools are monitored, it returns.
    c.  Iterates through a *sample* of monitored `pool_address` keys.
    d.  For each sampled pool, it calls `self.mempool_processor.get_address_state_diffs(pool_address)` to see if any *pending* transaction affects this pool's ETH balance according to the mempool processor's simulation.
    e.  Collects all relevant state diffs (`pending_diffs`).
    f.  If `pending_diffs` is not empty, it calls `_simulate_pool_eth_levels`.
4.  **`_simulate_pool_eth_levels` Logic:** Takes the current levels and applies the `change` or `after` values from the `pending_diffs` to calculate the predicted levels *if* the pending transactions were mined.
5.  **`_detect_suspicious_pools` Logic:** Compares the `simulated_levels` with `current_levels`. If a pool's simulated level drops below `self.eth_threshold` while its current level is above it, it's flagged as suspicious.
6.  **Alerting:** `_check_for_scam_transactions` iterates through `suspicious_pools` and logs a `WARNING` message for each.

**Key Methods & Tasks:**
----------------------
*   `__init__`: Initializes all components, queues, events, and state dictionaries.
*   `start`: Creates and starts all the concurrent `asyncio` tasks (`token_processor_task`, `_main_token_processing_task`, `_pool_update_task`, `_scam_detection_task`) and the `mempool_processor`.
*   `stop`: Sets the shutdown flag, stops the processors, and cancels all created tasks gracefully.
*   `_process_token_updates`: The core loop handling confirmed block updates, strategy execution, results saving, and triggering pool updates.
*   `_update_pool_levels_from_tokens`: Task dedicated to updating internal pool ETH levels based on triggers from the main loop.
*   `_monitor_for_scams`: Periodic task triggering the mempool check.
*   `_check_for_scam_transactions`: Contains the logic to query mempool state diffs, simulate effects, and detect/alert potential scams.
*   `_update_strategy_positions`, `_save_strategy_results_to_database`: Logic primarily inherited/adapted from the original `LiveBacktestEngine` for strategy execution and persistence.
*   `_update_pool_levels`, `_simulate_pool_eth_levels`, `_detect_suspicious_pools`: Helper methods for pool level management and scam detection logic.
"""

import asyncio
from typing import Dict, List, Set, Tuple, Optional, Any
from web3 import Web3

from eth_block_processor.blockchain.mempool_processor import MempoolProcessor
from eth_token.token_manager.live_block_token_processor import LiveBlockTokenProcessor
from eth_portfolio_manager.core.strategy_position_manager import StrategyPositionManager
from eth_portfolio_manager.backtesting.backtest_strategy_engine import BacktestStrategyEngine
from eth_portfolio_manager.db.live_token_position_results_writer import LiveResultsWriter
from eth_portfolio_manager.utils.logger import get_logger
from datetime import datetime


class LiveBacktestEngineWithMempool:
    """
    Combined live execution engine with mempool monitoring.
    Handles strategy execution, results writing, AND mempool scam detection.
    """
    
    def __init__(self,
                 w3: Web3,
                 config,
                 logger=None,
                 warmup_blocks=1000,
                 poll_interval=0.5,
                 eth_threshold=0.05,
                 save_strategy_results: bool = True,
                 max_mempool_transactions: int = 10000,
                 add_pnl_to_db: bool = True):
        """
        Initialize the combined live execution engine.
        """
        self.logger = logger or get_logger("portfolio_live_mempool")
        self.w3 = w3
        self.config = config
        self.warmup_blocks = warmup_blocks
        self.save_strategy_results = save_strategy_results

        # Initialize token processor directly
        self.live_token_processor = LiveBlockTokenProcessor(
            logger=self.logger,
            warmup_blocks=self.warmup_blocks,
            add_pnl_to_db=add_pnl_to_db
        )

        # Initialize strategy engines and position managers directly
        self.strategy_engines = {}
        for strategy_name, strategy in self.config.strategies.items():
            self.strategy_engines[strategy_name] = BacktestStrategyEngine(investment_strategy=strategy)
        self.strategy_position_managers = {
            strategy_name: StrategyPositionManager(strategy_engine=engine, logger=self.logger)
            for strategy_name, engine in self.strategy_engines.items()
        }

        # Initialize results writer directly if saving is enabled
        self.results_writer = None
        if self.save_strategy_results:
            self.results_writer = LiveResultsWriter(logger=self.logger)
            self.strategy_run_ids = {}

        # Mempool-specific initializations
        self.mempool_processor = MempoolProcessor(
            w3=self.w3,
            poll_interval=poll_interval,
            max_transactions=max_mempool_transactions,
            logger=self.logger
        )
        self.poll_interval = poll_interval
        self.eth_threshold = eth_threshold
        self._pool_eth_levels = {}
        self._pool_token_map = {}
        self._pool_updates_queue = asyncio.Queue()
        self._pool_update_event = asyncio.Event()
        self._is_shutting_down = False

        # Task tracking attributes
        self.token_processor_task = None
        self._main_token_processing_task = None
        self._pool_update_task = None
        self._scam_detection_task = None

        self.updated_positions_by_strategy = {}

        self.live_tasks = []

    async def start(self):
        """Start the combined live engine with mempool monitoring."""
        self.logger.info("Starting combined live engine with mempool monitoring")
        self._is_shutting_down = False

        # Start token processor directly
        self.logger.info("Starting token processor...")
        self.token_processor_task = asyncio.create_task(self.live_token_processor.start())
        self.logger.info("Token processor started")

        # Start the main token processing task (handles strategies and triggers pool updates)
        self.logger.info("Starting main token processing task...")
        self._main_token_processing_task = asyncio.create_task(self._process_token_updates())
        self.logger.info("Main token processing task started")

        # Start mempool processor
        self.logger.info("Starting mempool processor...")
        await self.mempool_processor.start()
        self.logger.info("Mempool processor started")

        # Start pool update task (waits for _pool_update_event set by _process_token_updates)
        self.logger.info("Starting pool update task...")
        self._pool_update_task = asyncio.create_task(self._update_pool_levels_from_tokens())
        self.logger.info("Pool update task started")

        # Start scam detection task
        self.logger.info("Starting scam detection task...")
        self._scam_detection_task = asyncio.create_task(self._monitor_for_scams())
        self.logger.info("Scam detection task started")

    async def stop(self):
        """Stop all components of the combined engine."""
        self.logger.info("Stopping combined engine...")
        self._is_shutting_down = True

        # Stop mempool processor first
        try:
            await self.mempool_processor.stop()
            self.logger.info("Mempool processor stopped.")
        except Exception as e:
            self.logger.error(f"Error stopping mempool processor: {e}", exc_info=True)

        # Stop the main token processor (this should handle its internal tasks)
        try:
            if hasattr(self, 'live_token_processor'):
                 await self.live_token_processor.stop()
                 self.logger.info("Live token processor stopped.")
        except Exception as e:
            self.logger.error(f"Error stopping live token processor: {e}", exc_info=True)

        # Cancel all our tasks
        tasks_to_cancel = [
            getattr(self, '_main_token_processing_task', None),
            getattr(self, '_scam_detection_task', None),
            getattr(self, '_pool_update_task', None),
            getattr(self, 'token_processor_task', None)
        ]

        for task in tasks_to_cancel:
            if task and not task.done():
                task_name = task.get_name() if hasattr(task, 'get_name') else 'Unnamed Task'
                self.logger.debug(f"Cancelling task: {task_name}")
                task.cancel()
                try:
                    await task
                except asyncio.CancelledError:
                    self.logger.debug(f"Task {task_name} cancelled successfully.")
                except Exception as e:
                    self.logger.error(f"Error cancelling task {task_name} during shutdown: {e}", exc_info=True)

        # Final save of strategy results if enabled
        if self.save_strategy_results and self.results_writer:
            try:
                await self._save_strategy_results_to_database(None)
                self.logger.info("Final strategy results saved.")
            except Exception as e:
                 self.logger.error(f"Error saving final results: {e}", exc_info=True)

        self.logger.info("Combined engine shutdown complete.")

    async def _process_token_updates(self):
        """Process token updates, execute strategies, save results, AND trigger pool updates."""
        self.logger.info("Starting main token update processing loop")
        while not self._is_shutting_down:
            try:
                await self.live_token_processor.new_updates_event.wait()
                self.live_token_processor.new_updates_event.clear()

                while not self.live_token_processor.unprocessed_token_updates.empty():
                    try:
                        result = await self.live_token_processor.unprocessed_token_updates.get()
                        block_number, updated_tokens = result
                        if updated_tokens:
                            tasks = []
                            for strategy_name, position_manager in self.strategy_position_managers.items():
                                if strategy_name not in self.updated_positions_by_strategy:
                                    self.updated_positions_by_strategy[strategy_name] = set()
                                else:
                                    self.updated_positions_by_strategy[strategy_name].clear()

                                tasks.append(self._update_strategy_positions(
                                    strategy_name,
                                    position_manager,
                                    updated_tokens,
                                    self.updated_positions_by_strategy
                                ))
                            await asyncio.gather(*tasks)
                            await self._pool_updates_queue.put((block_number, updated_tokens))
                            self._pool_update_event.set()
                            
                            # Save Strategy Results
                            if self.save_strategy_results and self.results_writer:
                                await self._save_strategy_results_to_database(block_number)
                            # Add PnL writing here
                            if self.live_token_processor.add_pnl_to_db:
                                updated_token_addresses = list(updated_tokens.keys())
                                await self._schedule_pnl_writes_for_tokens(updated_token_addresses, block_number)
                            
                        self.live_token_processor.unprocessed_token_updates.task_done()

                    except Exception as e:
                        self.logger.error(f"Error processing individual token update item: {e}", exc_info=True)
                        if not self.live_token_processor.unprocessed_token_updates.empty():
                             try:
                                 self.live_token_processor.unprocessed_token_updates.task_done()
                             except ValueError: pass

            except asyncio.CancelledError:
                break
            except Exception as e:
                self.logger.error(f"Error in main token update processing loop: {e}", exc_info=True)
                await asyncio.sleep(1)

    async def _update_strategy_positions(self, strategy_name, position_manager, updated_tokens, updated_positions_by_strategy):
        """Update positions for a single strategy based on new token data."""
        try:
            updated_positions_dict = await position_manager.update_updated_tokens_positions(updated_tokens)

            if updated_positions_dict:
                for token_address, pool_positions in updated_positions_dict.items():
                    for pool_address, position in pool_positions.items():
                        if position:
                            composite_key = f"{token_address}-{pool_address}"
                            updated_positions_by_strategy[strategy_name].add(composite_key)
            
            return updated_positions_dict
        except Exception as e:
            self.logger.error(f"Error updating positions for strategy {strategy_name}: {e}", exc_info=True)
            return {}

    async def _save_strategy_results_to_database(self, block_number):
        """Save current strategy results to the database (if enabled)."""
        if not self.save_strategy_results or not self.results_writer:
            return

        for strategy_name, updated_positions in self.updated_positions_by_strategy.items():
            if not updated_positions:
                continue

            try:
                # Get or create a run ID for this strategy if it's the first save
                if strategy_name not in self.strategy_run_ids:
                    strategy_run_name = f"LIVE_{strategy_name}"
                    strategy_params = self.strategy_engines[strategy_name].investment_strategy.to_dict()
                    run_id = await asyncio.to_thread(
                        self.results_writer.create_strategy_run,
                        strategy_run_name,
                        strategy_params,
                        self.live_token_processor.start_block,
                        datetime.now()
                    )
                    self.strategy_run_ids[strategy_name] = run_id
                    self.logger.info(f"Created new strategy run record: {strategy_run_name} (ID: {run_id})")

                run_id = self.strategy_run_ids[strategy_name]
                position_manager = self.strategy_position_managers[strategy_name]
                
                # Filter positions that need updating
                positions_to_save = {
                    pos_key: position_manager.open_positions[pos_key]
                    for pos_key in updated_positions
                    if pos_key in position_manager.open_positions
                }

                if positions_to_save:
                    await asyncio.to_thread(
                        self.results_writer.write_token_position_updates,
                        run_id,
                        positions_to_save
                    )
            except Exception as e:
                self.logger.error(f"Error saving results for strategy {strategy_name}: {e}", exc_info=True)

    async def _update_pool_levels_from_tokens(self):
        """Task to update internal pool ETH levels based on token updates."""
        self.logger.info("Starting pool ETH level update task.")
        try:
            while not self._is_shutting_down:
                await self._pool_update_event.wait()
                if self._is_shutting_down:
                    break
                self._pool_update_event.clear()

                while not self._pool_updates_queue.empty():
                    try:
                        _, updated_tokens = self._pool_updates_queue.get_nowait()
                        if updated_tokens:
                            await self._update_pool_levels(updated_tokens)
                        self._pool_updates_queue.task_done()
                    except asyncio.QueueEmpty:
                        break # Should not happen with this logic, but for safety
                    except Exception as e:
                        self.logger.error(f"Error processing item from pool updates queue: {e}", exc_info=True)
                        if not self._pool_updates_queue.empty():
                            try:
                                self._pool_updates_queue.task_done()
                            except ValueError: pass

        except asyncio.CancelledError:
            self.logger.info("Pool level update task cancelled.")
        except Exception as e:
            self.logger.error(f"Pool level update task failed: {e}", exc_info=True)
        finally:
            self.logger.info("Pool level update task finished.")

    async def _update_pool_levels(self, updated_tokens: Dict[str, Any]):
        """Update internal records of pool addresses and their ETH levels."""
        for token_address, token in updated_tokens.items():
            if token.has_pool:
                for pool_address in token.pool_addresses:
                    if pool_address not in self._pool_token_map:
                        self._pool_token_map[pool_address] = token_address
                    
                    try:
                        # Fetch the reserves directly from the token data object
                        denom_reserve = token.get_pool_reserve(pool_address)
                        if denom_reserve is not None:
                            self._pool_eth_levels[pool_address] = denom_reserve
                    except Exception as e:
                        self.logger.error(f"Could not get reserves for pool {pool_address} of token {token_address}: {e}")

    async def _monitor_for_scams(self):
        """Periodically check the mempool for potential scam transactions."""
        self.logger.info("Starting mempool scam monitoring task.")
        try:
            while not self._is_shutting_down:
                await self._check_for_scam_transactions()
                await asyncio.sleep(self.poll_interval)
        except asyncio.CancelledError:
            self.logger.info("Mempool scam monitoring task cancelled.")
        except Exception as e:
            self.logger.error(f"Mempool scam monitoring task failed: {e}", exc_info=True)
        finally:
            self.logger.info("Mempool scam monitoring task finished.")

    async def _check_for_scam_transactions(self):
        """Check for transactions in the mempool that could potentially be scams."""
        try:
            current_levels = self._get_current_pool_eth_levels()
            if not current_levels:
                return

            pending_diffs = {}
            
            all_pool_addresses = list(self._pool_eth_levels.keys())
            sample_size = min(20, len(all_pool_addresses))
            sampled_pools = all_pool_addresses[:sample_size]

            for pool_address in sampled_pools:
                try:
                    state_diff = self.mempool_processor.get_address_state_diffs(pool_address)
                    if state_diff:
                        pending_diffs[pool_address] = state_diff
                    await asyncio.sleep(0.05)
                except Exception as e:
                    self.logger.debug(f"Scam Check: Error checking state diff for pool {pool_address}...: {str(e)}")
                    continue
            
            if pending_diffs:
                simulated_levels_with_hashes = self._simulate_pool_eth_levels(pending_diffs)
                suspicious_pools = self._detect_suspicious_pools(simulated_levels_with_hashes, current_levels)

                for pool_address, current_level, sim_level, tx_hash in suspicious_pools:
                    token_address = self._pool_token_map.get(pool_address, 'unknown')
                    warning_msg = (
                        f"[URGENT SCAM ALERT] PENDING tx - "
                        f"Token: {token_address}, "
                        f"Pool: {pool_address}, TxHash: {tx_hash}, "
                        f"Current ETH level: {current_level:.6f}, "
                        f"Simulated ETH level: {sim_level:.6f}, "
                        f"Threshold: {self.eth_threshold}"
                    )
                    if self.results_writer and token_address != 'unknown':
                        try:
                            prediction_block_number = self.live_token_processor.latest_processed_block
                            await asyncio.to_thread(
                                self.results_writer.write_mempool_scam_prediction,
                                token_address,
                                pool_address,
                                prediction_block_number,
                                current_level,
                                sim_level,
                                self.eth_threshold,
                            )
                        except Exception as e:
                            self.logger.error(f"Failed to write mempool scam prediction for token {token_address}, pool {pool_address}: {e}", exc_info=True)
                    self.logger.warning(warning_msg)
                    
        except Exception as e:
            self.logger.error(f"Error in _check_for_scam_transactions: {e}", exc_info=True)
            await asyncio.sleep(2)

    def _get_current_pool_eth_levels(self) -> Dict[str, float]:
        """Return a copy of the current pool ETH levels."""
        return self._pool_eth_levels.copy()

    def _simulate_pool_eth_levels(self, mempool_state_diffs: Dict[str, Dict]) -> Dict[str, Tuple[float, str]]:
        """
        Simulate the ETH levels for pools and return the level and associated transaction hash.
        """
        current_levels = self._get_current_pool_eth_levels()
        simulated_levels = {pool: (level, None) for pool, level in current_levels.items()}

        for pool_address, tx_diffs in mempool_state_diffs.items():
            if pool_address in simulated_levels and tx_diffs:
                for tx_hash, diff_details in tx_diffs.items():
                    if not diff_details or 'balance' not in diff_details:
                        continue
                    
                    current_level, _ = simulated_levels[pool_address]
                    new_level = current_level 
                    try:
                        balance_diff = diff_details['balance']
                        if 'change' in balance_diff:
                            change_in_wei = int(balance_diff['change'], 16)
                            change_in_eth = Web3.from_wei(change_in_wei, 'ether')
                            new_level += float(change_in_eth)
                        elif 'after' in balance_diff:
                            new_level_in_wei = int(balance_diff['after'], 16)
                            new_level = float(Web3.from_wei(new_level_in_wei, 'ether'))
                        
                        simulated_levels[pool_address] = (new_level, tx_hash)

                    except (ValueError, TypeError, KeyError) as e:
                        self.logger.error(f"Error simulating pool level for {pool_address} from tx {tx_hash}: {e}")
        
        return simulated_levels

    def _detect_suspicious_pools(self,
                                 simulated_levels: Dict[str, Tuple[float, str]],
                                 current_levels: Dict[str, float]) -> List[Tuple[str, float, float, str]]:
        """
        Detects pools whose ETH level would drop below the threshold.
        Returns a list of (pool_address, current_level, simulated_level, tx_hash).
        """
        suspicious_pools = []
        
        for pool_address, (sim_level, tx_hash) in simulated_levels.items():
            current_level = current_levels.get(pool_address, 0.0)

            if tx_hash and isinstance(sim_level, (int, float)) and sim_level < self.eth_threshold and current_level >= self.eth_threshold:
                suspicious_pools.append((pool_address, current_level, sim_level, tx_hash))
                token_address = self._pool_token_map.get(pool_address)
                self.logger.debug(
                   f"Suspicious pool detected: {pool_address} (token: {token_address}...) "
                    f"Current: {current_level:.3f} ETH -> Simulated: {sim_level:.3f} ETH by Tx: {tx_hash}"
                )

        return suspicious_pools

    async def _schedule_pnl_writes_for_tokens(self, updated_token_addresses, block_number):
        """
        Schedules the writing of PnL data for a list of tokens.
        """
        if not self.live_token_processor.add_pnl_to_db:
            return
            
        try:
            for token_address in updated_token_addresses:
                try:    
                    # Write PnL data directly (the method is already thread-safe)
                    self.live_token_processor.live_tokens_cache._write_token_pnl(token_address)
                except Exception as e:
                    self.logger.error(f"Error scheduling PnL write for token {token_address}: {e}")
        except Exception as e:
            self.logger.error(f"Error scheduling PnL writes for block {block_number}: {e}", exc_info=True)