# Transaction Data Models

This package defines the canonical representation of processed Ethereum
transactions. The goal is twofold:

1. Capture every on-chain transaction deterministically in a single `ProcessedTransaction`
   schema using lossless types.
2. Keep that schema byte-for-byte aligned with the Rust `ProcessedTransaction` used by
   PyReth so transactions can move between runtimes without coercion or precision loss.

## Core Types

### ProcessedTransaction (top-level envelope)
- `hash: str` – 0x-prefixed transaction hash  
- `block_number: int`  
- `block_timestamp: int` – seconds since epoch  
- `tx_index: int`  
- `from_address: ChecksumAddress`  
- `to_address: Optional[ChecksumAddress]`  
- `contract_address: Optional[ChecksumAddress]` – deployment target for CREATE / CREATE2  
- `value: int` – wei  
- `status: bool` – `True` on success, `False` on revert  
- `nonce: int`  
- `tx_type: str` – classifier string (swap, mint, liquidity_add, …)  
- `actions: list[str]` – high-level tags used by alerting/analytics  
- `fees: TransactionFees`  
- `bribe_amount: int` – wei paid outside of canonical gas accounting  
- `unique_addresses: set[ChecksumAddress]` – all addresses touched by the tx  
- `erc20_contracts / erc721_contracts / erc1155_contracts: set[ChecksumAddress]`  
- Transfer collections (see below)  
- Event collections (Uniswap, approvals, owner changes, etc.)  
- `other_events: list[dict]` – catch-all for protocols we have not modelled yet  
- `address_balance_changes: dict[str, Any]` – currency/token deltas keyed by checksum address  
- `latest_states: dict[str, Any]` – optional snapshot payloads provided by the processor  
- `input: str` – 0x calldata string

### TransactionFees
- `gas_price: int` – effective gas price (wei)  
- `gas_used: int` – total gas consumed  
- `tx_fee: int` – `gas_price * gas_used` (wei)  
- `protocol_type: str` – `legacy | eip1559 | eip2930 | unknown`  
- `max_fee_per_gas: Optional[int]` – user-specified cap (wei)  
- `max_priority_fee: Optional[int]` – user tip cap (wei)

### Transfer Models (receipt-derived)
All transfer/approval structs surface decoded log data for common token standards; every amount is an integer in native token units.

- `ETHTransferEvent` – simple ether movement  
  - `from_address`, `to_address`: `ChecksumAddress`  
  - `amount: int` – wei
- `ERC20TransferEvent`  
  - `token_address`, `from_address`, `to_address`: `ChecksumAddress`  
  - `amount: int` – raw token units (respect decimals externally)  
  - `log_index: int`
- `ERC20ApprovalEvent`  
  - `owner`, `spender`, `token_address`: `ChecksumAddress`  
  - `amount: int` – allowance set  
  - `log_index: int`
- `ERC721TransferEvent`, `ERC721ApprovalEvent` – mirror the ERC20 forms but carry `token_id: int`
- `ERC1155TransferEvent`  
  - `token_address`: `ChecksumAddress`  
  - `operator`, `from_address`, `to_address`: `ChecksumAddress`  
  - `token_ids: list[int]`, `amounts: list[int]`  
  - `log_index: int`

### Trace Models (call tree & miner data)
Derived from execution traces and miner bundles; integer magnitudes express wei or token quantities depending on context.

- `InternalTransaction`  
  - `from_address`, `to_address`: `ChecksumAddress`  
  - `value: int` – wei  
  - `trace_type: str` – CALL | STATICCALL | CREATE | …  
  - `call_type: Optional[str]` – delegatecall / callcode  
  - `gas_used: int`, `depth: int`
- `UniswapV2MintEvent`, `UniswapV2BurnEvent`, `DepositEvent`, `WithdrawEvent` – ERC20/ERC721/LP-specific lifecycle events constructed from traces.
- `Permit2Event` – EIP-2612 style approvals captured via traces/logs.

### Address Balance Changes
`address_balance_changes` is a nested map summarising net balance movement per address:

- top level key: checksum address  
- value payload:  
  - `eth_net: int` – wei delta (positive for inflow)  
  - `token_net: dict[str, int]` – ERC20/ERC721/ERC1155 deltas keyed by token address  
  - `movements: dict` – optional breakdown of raw in/out flows (`tokens.in/out`, `denom.in/out`) used by downstream analytics


### DEX Events (aligned with Rust naming)
Each Uniswap structure surfaces the fields required to replay liquidity and swap state; naming keeps the protocol prefix.

- Uniswap V2  
  - `UniswapV2SyncEvent` – reserves update (`reserve0`, `reserve1`, `pair_address`, `log_index`)  
  - `UniswapV2SwapEvent` – swap event with in/out amounts and `sender`/`to`  
  - `UniswapV2PairCreatedEvent` – metadata (owner changes, pair creation)  
  - `TradingEnabledEvent`, `TradingDisabledEvent`
- Uniswap V3  
  - `UniswapV3PoolCreatedEvent`, `UniswapV3InitializeEvent` – pool bootstrapping (`pool_address`, `token0`, `token1`, `fee`)  
  - `UniswapV3MintEvent`, `UniswapV3BurnEvent` – position liquidity deltas (`liquidity`, `tick_lower`, `tick_upper`)  
  - `UniswapV3SwapEvent` – swap path, amounts specified in token0/token1  
  - `UniswapV3PositionEvent`, `UniswapV3IncreaseLiquidityEvent`, `UniswapV3DecreaseLiquidityEvent`
- Uniswap V4  
  - `UniswapV4InitializeEvent`, `UniswapV4ModifyLiquidityEvent`, `UniswapV4SwapEvent` – PoolManager event family; fields mirror the Rust definitions (`pool_manager_address`, `amount0`, `amount1`, `sqrt_price_x96`, `liquidity`, `log_index`)

Additional protocol-specific structures can be added in the same style: `<Protocol>_<Entity>` with integer quantities and checksum addresses. When introducing a new venue, document it here so cross-runtime consumers know where to look.

## Design Principles

- **Integer everywhere (in-memory)**: All value-carrying fields are integer wei or token
  units within the Python/Rust data structures. When we serialise to transports with
  64-bit limits (RabbitMQ JSON payloads, etc.) the publisher converts values that exceed
  signed 64-bit range into decimal strings so the message gets through without truncation.
- **Canonical status**: `status` is a boolean (Rust `bool`, Python `bool`).
- **Checksum addresses**: every address exposed to callers is `ChecksumAddress`.
- **Stable field names**: Python and Rust share the same keys. 
- **Machine-scale first**: All comparisons, thresholds, and persistence happen in raw node
  units (wei, token base units). Human formatting happens at the very edge.
- **No silent coercion**: legacy converters that accepted floats, hex-without-prefix,
  or mixed sets are being eliminated. Producers must emit canonical data; consumers can
  rely on it.

## Alignment With Rust

- Rust `ProcessedTransaction` (`tx_processor::ProcessedTransaction`) mirrors this
  schema: `U256` for value/bribe/fee fields, `bool` status, `Vec`/`HashSet` containers.
- PyO3 bridge (`PyProcessedTransaction`) converts each `U256` to Python `int` and
  maintains checksum casing. No precision is lost during serialisation.
- Python-to-Rust handoff (`processed_tx_bridge.rs`) expects canonical ints/booleans and
  refuses legacy floats/strings; we normalise through this schema before sending data to Rust.
- `ProcessedTransaction.to_dict()` retains the machine-scale integers; `to_json()` uses
  `orjson` for fast transport encodings once the canonical dict is built. Downstream
  publishers run the same normaliser that stringifies out-of-range integers before they
  leave the Python runtime so consumers on either side of the bridge can recover the full
  magnitude easily.

Following this contract ensures the live pipeline (transaction processors, simulators,
alerts) handles transactions consistently across languages and avoids the precision bugs
we hit when floats slipped into the path.

### Derived Summaries
- `DexSwapEvent` – summary object pairing token-in/out ERC20 transfers for high-level swap analytics.
