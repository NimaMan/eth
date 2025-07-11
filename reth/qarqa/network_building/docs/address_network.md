# Address Network 

## Algorithm: Building Address Network via State Changes

**Objective**: Construct a directed graph representing significant fund flows starting from a seed address to 
    - identify potential malicious activity sources.
    - identify the most important addresses in the network.

### Inputs
- `seed_address`: Ethereum address to analyze (e.g., an ERC20 contract creator)
- `eth_threshold`: minimum ETH change to track (e.g., 0.001 ETH)
- `token_threshold`: minimum token change to track (e.g., 0.001 tokens)
- `max_depth`: maximum exploration depth to bound graph size
- `max_nodes`: cap on total nodes to limit complexity
- `start_block`, `end_block`: optional block range filter

### Detailed Steps
1. Initialize:
   - `G = MultiDiGraph()` to store nodes and edges.
   - `Q = deque([(seed_address, 0)])` as BFS queue.
   - `visited = {seed_address}` to avoid revisiting.

2. BFS Exploration:
   While `Q` not empty and `len(G.nodes()) < max_nodes`:
   - Pop `(addr, depth)` from `Q`.
   - If `depth >= max_depth`, skip expansion.
   - Load all transactions for `addr` from the database.
   - For each transaction `tx`:
     a. Extract ETH and ERC20 transfer records.
     b. Compute state diffs via `AddressStateChangeCalculator`.
     c. Filter out related addresses whose net changes are below thresholds.
     d. For each significant `related_addr`:
        - Add node with metadata (category, color, shape).  # serves rich visualization and grouping
        - Add directed edge(s) capturing `amount`, `type`, and `tx_hash`.  # records flow details
        - If not in `visited`, add to `visited` and `Q.append((related_addr, depth+1))`.

3. Termination:
   - Stop when queue is empty or node limit reached.  # bounds computation and noise

4. Output Formatting:
   - Convert `G` into a dictionary `{ nodes: [...], links: [...] }`  # ready for frontend consumption

**Requirement Rationale**:
- **BFS Exploration** ensures systematic lineage tracing of fund movements.
- **Threshold Filtering** focuses on meaningful flows, reducing noise.
- **Depth & Node Limits** bound computational cost and graph complexity.
- **Rich Metadata** supports visual identification of suspicious patterns. 

## Implementation
The approach builds on transaction state changes similar to the token network model, focusing on actual value movement between addresses:

1. **Block Loading and Caching**: Efficiently cache transaction blocks for the target address
2. **State Change Calculation**: Calculate meaningful state changes (ETH and token transfers)
3. **Graph Construction**: Build a directed graph based on value flows between addresses

### Key Components
#### 1. Shared Block Cache

A shared block cache in `AddressDataFetcher` will store and manage blocks relevant to the target address:

```python
def _cache_address_blocks(self, address, start_block=None, end_block=None):
    """Cache blocks containing transactions involving the target address."""
    # Get blocks where address has activity
    address_blocks = self.get_address_blocks(address, start_block, end_block)
    
    # Cache each block that isn't already cached
    for block_num in address_blocks:
        if block_num not in self._block_cache:
            block_data = self.fetch_block_data(block_num)
            self._block_cache[block_num] = block_data
    
    return address_blocks
```

#### 2. State Change Calculator

The `AddressStateChangeCalculator` will calculate meaningful state changes in transactions:

```python
def calculate_state_changes(self, txn_hash, from_address, eth_transfers, erc20_transfers):
    """Calculate state changes including ETH and ERC20 token transfers."""
    # Track ETH movements
    for transfer in eth_transfers:
        self._track_movement('eth', 
                           transfer['from_address'], 
                           transfer['to_address'], 
                           transfer['amount'])
    
    # Track ERC20 movements
    for transfer in erc20_transfers:
        self._track_movement('token', 
                           transfer['from_address'], 
                           transfer['to_address'], 
                           transfer['amount'],
                           token_address=transfer['token_address'])
    
    # Calculate net changes for each address
    return self.get_net_changes(from_address)
```

#### 3. Enhanced Address Network Builder

The enhanced `AddressNetworkBuilder` will construct a network based on state changes:

```python
def build_network(self, target_address_str, max_nodes=3000, max_level=2):
    """Build address network based on state changes."""
    # Initialize graph
    # Cache relevant blocks for the target address
    cached_blocks = self.data_fetcher._cache_address_blocks(target_address_str)
    
    # Process transactions from cached blocks
    for block_num in cached_blocks:
        block_data = self.data_fetcher._block_cache[block_num]
        for tx in block_data['transactions']:
            if target_address_str in [tx['from_address']] + tx.get('involved_addresses', []):
                # Process transaction state changes
                state_changes = self.state_change_calculator.calculate_state_changes(
                    tx['hash'],
                    tx['from_address'],
                    tx.get('eth_transfers', []),
                    tx.get('erc20_transfers', [])
                )
                
                # Update network with state changes
                for address, changes in state_changes.items():
                    self._update_network_with_state_change(address, changes)
    
    # Process additional levels if requested
    # ...
    
    return self._format_network_for_output()
```

### Benefits of the Enhanced Approach

1. **Higher Signal-to-Noise Ratio**: Focuses on meaningful value exchanges rather than mere co-participation
2. **Direction and Magnitude**: Captures both the direction (sender/receiver) and magnitude (amount) of transfers
3. **Special Case Handling**: Properly handles special cases like WETH conversions and fee payments
4. **Multi-Token Support**: Can represent both ETH and ERC20 token movements in a single network
5. **Filtering Capabilities**: Can filter out insignificant interactions based on configurable thresholds

### Implementation Considerations

1. **Efficient Block Caching**: Minimize duplicate block fetching by implementing a shared block cache
2. **Configurable Thresholds**: Allow configuration of significance thresholds for different types of transfers
3. **Progressive Loading**: Support incremental network building for large datasets
4. **Edge Attributes**: Store transfer type (ETH/ERC20) and amount in edge attributes for visualization
5. **Node Categorization**: Maintain compatibility with the address categorization scheme

## Integration with Frontend

The enhanced network model will provide richer data to the frontend visualization:

1. **Edge Thickness**: Can represent transfer magnitude
2. **Edge Color/Type**: Can represent transfer type (ETH vs. different token types)
3. **Directional Arrows**: Clearly show the direction of value flow
4. **Filtering Options**: Allow users to filter by transaction type or transfer threshold
