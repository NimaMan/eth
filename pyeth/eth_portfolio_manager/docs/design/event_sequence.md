# Live Backtest Engine with Mempool - Event Sequence

## Objective

To understand the flow of data and control signals within the `LiveBacktestEngineWithMempool` system, covering both the processing of confirmed blocks and the concurrent monitoring of the mempool for scam detection.

## Core Components

*   **`LiveBacktestEngineWithMempool`**: The main orchestrator running multiple asyncio tasks. Manages strategies, results, pool levels, and scam detection logic.
*   **`LiveBlockTokenProcessor`**: Processes historical and live blocks, identifies relevant token updates, manages `LiveTokensCache`.
*   **`BlockSubscriber`**: Underlying component (used by `LiveBlockTokenProcessor`) that listens for new blocks (e.g., via RabbitMQ or `eth_subscribe`).
*   **`LiveTokensCache`**: In-memory cache holding `LiveERC20Token` objects, managed by `LiveBlockTokenProcessor`. Can optionally contain `TokenPnLWriter`.
*   **`MempoolProcessor`**: Independent component polling the mempool and simulating transactions to get state diffs.
*   **Strategy Engines / Position Managers**: Components applying trading logic based on confirmed token updates.
*   **`TokenPnLWriter`**: Component responsible for writing token PnL data to the database (lives inside `LiveTokensCache`).
*   **`LiveResultsWriter`**: Component responsible for writing strategy results to the database.

## Key Queues & Events (Signals)

*   **`LiveBlockTokenProcessor.unprocessed_token_updates`** (`asyncio.PriorityQueue`):
    *   **Content:** `(priority, (block_number, updated_tokens_dict))`
    *   **Flow:** `LiveBlockTokenProcessor` -> `LiveBacktestEngineWithMempool._process_token_updates` task.
    *   **Purpose:** Passes processed token updates from the block processor to the main engine loop. Priority likely based on `block_number`.
*   **`LiveBlockTokenProcessor.block_processed_event`** (`asyncio.Event`):
    *   **Flow:** Internal to `LiveBlockTokenProcessor`. Set by `process_block_live`, waited on by `_monitor_token_updates`.
    *   **Purpose:** Synchronizes internal block processing with the task that queues updates for the engine.
*   **`LiveBlockTokenProcessor.new_updates_event`** (`asyncio.Event`):
    *   **Flow:** Set by `LiveBlockTokenProcessor._monitor_token_updates`, waited on by `LiveBacktestEngineWithMempool._process_token_updates`.
    *   **Purpose:** Notifies the main engine loop that new items are available in `unprocessed_token_updates`.
*   **`LiveBacktestEngineWithMempool._pool_updates_queue`** (`asyncio.Queue`):
    *   **Content:** `(block_number, updated_tokens_dict)`
    *   **Flow:** `_process_token_updates` task -> `_update_pool_levels_from_tokens` task.
    *   **Purpose:** Passes token updates to the dedicated task responsible for updating the engine's internal pool ETH levels.
*   **`LiveBacktestEngineWithMempool._pool_update_event`** (`asyncio.Event`):
    *   **Flow:** Set by `_process_token_updates`, waited on by `_update_pool_levels_from_tokens`.
    *   **Purpose:** Notifies the pool update task that new items are available in `_pool_updates_queue`.

## Flow A: Confirmed Block Processing

1.  **Block Arrival:** `BlockSubscriber` receives a new block (e.g., from RabbitMQ).
2.  **Processor Callback (`LiveBlockTokenProcessor`)**: `process_block_live(block_data)` is called.
3.  **Core Block Processing (`LiveBlockTokenProcessor`)**:
    *   Calls `self.process_block(block_data)` (inherited method).
    *   This fetches transactions, processes logs/traces, updates `LiveERC20Token` objects within `self.live_tokens_cache`, and identifies which tokens were updated in `self.updated_tokens`.
    *   Sets `self.latest_processed_block = block_number`.
    *   Sets `self.block_processed_event` upon completion.
4.  **Processor Monitoring (`LiveBlockTokenProcessor`)**: The `_monitor_token_updates` task waits on `self.block_processed_event`. Upon seeing it set:
    *   Checks if `self.updated_tokens` is not empty.
    *   Puts `(current_block, (current_block, self.updated_tokens.copy()))` onto `self.unprocessed_token_updates` queue.
    *   Sets `self.new_updates_event` to signal the engine.
    *   Clears `self.block_processed_event`.
5.  **Engine Main Loop (`LiveBacktestEngineWithMempool`)**: The `_process_token_updates` task waits on `self.new_updates_event`. Upon seeing it set:
    *   Clears `self.new_updates_event`.
    *   Gets `(priority, (block_number, updated_tokens))` from `self.unprocessed_token_updates`.
    *   Logs `INFO - Processing block {block_number}...`.
    *   **Strategy Execution**: Calls `self._update_strategy_positions` for each strategy, potentially concurrently using `asyncio.gather`. This updates `StrategyPositionManager` states.
    *   **Pool Update Trigger**: Puts `(block_number, updated_tokens)` onto `self._pool_updates_queue`.
    *   **Pool Update Signal**: Sets `self._pool_update_event`.
    *   **Strategy Results**: (Optional) Calls `self._save_strategy_results_to_database(block_number)` to write strategy position data. Logs `INFO - {block_number}: Saved strategy positions...`.
    *   **PnL Writing**: (Optional) Calls `self._schedule_pnl_writes_for_tokens(updated_tokens, block_number)` to write token PnL data.
    *   Marks the item from `unprocessed_token_updates` as done (`task_done()`).
6.  **Engine Pool Update (`LiveBacktestEngineWithMempool`)**: The `_update_pool_levels_from_tokens` task waits on `self._pool_update_event`. Upon seeing it set:
    *   Clears `self._pool_update_event`.
    *   Gets `(block_number, updated_tokens)` from `self._pool_updates_queue`.
    *   Calls `self._update_pool_levels(updated_tokens)`, which iterates through tokens, calls `token.token_data.get_pool_reserve(pool_address)`, and updates the engine's internal `self._pool_eth_levels` dictionary.
    *   Logs `INFO - Processing block {block_number}... in Pool Update Task`.
    *   Marks the item from `_pool_updates_queue` as done (`task_done()`).

## Flow B: Mempool Monitoring & Scam Detection (Concurrent)

1.  **Periodic Check (`LiveBacktestEngineWithMempool`)**: The `_monitor_for_scams` task wakes up periodically (based on `poll_interval`).
2.  **Scam Check Logic**: Calls `self._check_for_scam_transactions`.
3.  **Read Confirmed State**: Reads the *current state* of `self._pool_eth_levels`. **Note:** This state reflects the last *completed* update by the `_update_pool_levels_from_tokens` task.
4.  **Query Mempool Processor**: Calls `self.mempool_processor.get_address_state_diffs(pool_address)` for relevant pools.
5.  **Mempool Processor (Internals)**: Concurrently and independently polls the node's mempool, simulates new transactions using `StateDiffProcessor` (`trace_call`), and stores/provides the resulting state diffs (potential ETH balance changes).
6.  **Simulate Impact**: `_check_for_scam_transactions` calls `self._simulate_pool_eth_levels`, applying the *pending* state diffs from the mempool to the *confirmed* levels read in step 3.
7.  **Detect Suspicious**: Calls `self._detect_suspicious_pools`, comparing the simulated levels against the confirmed levels and the `eth_threshold`.
8.  **Alert**: Logs `WARNING - [URGENT SCAM ALERT]...` if a pool's simulated level drops below the threshold based on the potentially stale confirmed level read in step 3.

## PnL Writing Placement Discussion

Based on this flow:

*   **Previous:** PnL writing happened in `LiveBlockTokenProcessor._monitor_token_updates` immediately after queuing updates for the engine. This ensured it used the state immediately after block processing.
*   **Current:** PnL writing now occurs in `LiveBacktestEngineWithMempool._process_token_updates` after strategy execution, pool updates, and strategy results writing. This ensures all processing is completed first, providing more current and consistent data.
*   **Alternative (Dedicated Task):** Creating a new queue and task specifically for PnL writing would be cleaner architecturally but would add additional components.

The current placement prioritizes critical operations (strategy execution and pool level updates) before less time-sensitive database operations (strategy results and PnL writing), improving the responsiveness of the mempool monitoring system.

## Database Writing & Transaction Management

### Token PnL Writing Process

1. **Trigger Point:** PnL writing now occurs in `LiveBacktestEngineWithMempool._process_token_updates` after pool updates and strategy results are written:
   * For each token in `updated_tokens`, it attempts to get the token from cache
   * It then schedules `live_tokens_cache._write_token_pnl(token_address)` in a separate thread

2. **PnL Writing Flow (`_write_token_pnl`):**
   * Checks if the token has trading activity (`token_trading_age_blocks is not None`)
   * Calls `pnl_writer.write_token_pnl_to_db(token)` which:
     * Collects user activity data from token's network
     * Ensures token exists in database via `_ensure_token_in_db`
     * Writes user trade PnL data via `_write_user_trade_pnl_data`

3. **Critical Path - Token Database Insertion (`_ensure_token_in_db`):**
   * **Transaction Verification:** Checks if required transactions (`creation_txn`, `trading_enabled_txn`) exist in the database first
   * **Retry Mechanism:** Uses exponential backoff to retry up to 3 times if transactions aren't yet in the database
   * **Deduplication:** Handles potential `UniqueViolation` errors by resyncing the token cache from database

### Strategy Results Writing Process

1. **Trigger Point:** Strategy results writing occurs in `_process_token_updates` after pool updates are triggered:
   * Calls `_save_strategy_results_to_database(block_number)` if `save_strategy_results` is enabled

2. **Results Writing Flow (`_save_strategy_results_to_database`):**
   * For each strategy, creates or updates a strategy run record
   * Identifies which positions have changed in the current block
   * Writes only updated positions to the database via `results_writer.update_token_positions`

3. **Position Writing Management (`update_token_positions`):**
   * Synchronizes in-memory cache of saved tokens with database on first access to a run
   * Separates tokens into new (INSERT) and existing (UPDATE) operations
   * Handles duplicate key violations by resyncing and reassessing operations

### Potential Race Conditions & Solutions

1. **Block Processor vs. Token System:**
   * **Issue:** The block processor writes transactions to DB concurrently with token system trying to reference those transactions
   * **Solution:** Retry mechanism in `_ensure_token_in_db` allows token system to wait for transactions to appear

2. **In-Memory Cache vs. Database State:**
   * **Issue:** The in-memory tracking of saved tokens can get out of sync with actual database state
   * **Solution:** Database synchronization on first access and error recovery with cache resyncing
   
3. **Transaction Management:**
   * **Issue:** Failed transactions can enter aborted state causing cascading failures
   * **Solution:** Proper transaction rollback and exception handling in all database operations

## End-to-End Database Write Sequence

To clarify the complete sequence across components in a single block processing iteration:

1. **Block Processing & TokenData Updates**
   * New block → BlockSubscriber → LiveBlockTokenProcessor.process_block_live()
   * Token states updated in LiveTokensCache (in-memory)

2. **Token Update Notification**
   * LiveBlockTokenProcessor._monitor_token_updates() queues updated tokens
   * Sets new_updates_event to notify engine

3. **Strategy Processing**
   * LiveBacktestEngineWithMempool._process_token_updates() processes token updates
   * Updates strategy positions based on token updates

4. **Pool Level Updates**
   * Pool update triggered immediately after strategy execution
   * Updates internal ETH levels for mempool monitoring (critical for scam detection)

5. **Strategy Results Writing**
   * LiveBacktestEngineWithMempool._save_strategy_results_to_database() writes strategy results
   * Writes only updated positions to the database

6. **PnL Database Writing**
   * LiveBacktestEngineWithMempool._schedule_pnl_writes_for_tokens() writes token PnL data
   * Executes after all processing and critical updates are complete