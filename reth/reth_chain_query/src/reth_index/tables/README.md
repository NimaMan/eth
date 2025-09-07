RethIndex Tables — Design Guide

Purpose
- Document two separate concerns and how we store/query them efficiently:
  1) Mempool arrival timestamp for mined transactions (minimal, per-tx number)
  2) Fast lookup for “all tx related to X” (address/contract/token) — optional and scoped

Scope and Philosophy
- Keep disk usage minimal and leverage the archive Reth node for heavy, indexed queries where possible.
- Preserve correct intra-block ordering by operating at the block level (process full blocks when necessary).
- Only persist what we repeatedly need at low latency; compute the rest on demand via Reth + tx_processor.

1) Mempool Arrival Times (Minimal)
- Goal: Know if a mined tx passed through mempool and when it first arrived.
- Storage: single compact table keyed by TxNumber.
  - Table: `tx_arrival_by_txnum`
  - Key: `tx_number` (u64, big‑endian)
  - Value: `first_seen_ns` (u64)
- Write path:
  - In mempool processor, keep an in‑memory map `tx_hash → first_seen_ns`.
  - When a tx is included (mined), resolve `tx_number` from Reth (via `TransactionHashNumbers: TxHash → TxNumber`) and write one row: `tx_number → first_seen_ns`.
  - Acceptable trade‑off: if the process restarts before inclusion, some arrivals are lost (keeps design minimal).
- Why TxNumber:
  - Matches Reth’s primary sequencing and composes with other tables (e.g., block mapping via `TransactionBlocks`, position via `BlockBodyIndices`).

2) “All TX related to X” — Address/Token Lookup

Prefer Archive‑Node Queries When Possible
- Token‑centric (ERC20) queries:
  - Use logs on the token contract (e.g., `Transfer`, `Approval`) for token‑initiated activity.
  - Use logs on the token’s pool contracts (e.g., `Swap`) for swap activity involving the token.
  - From logs you get block numbers (and `transactionIndex`);
    fetch full blocks and re‑process them via `tx_processor` to derive `ProcessedTransaction` and `unique_addresses` as needed.
- Arbitrary EOA/contract (generic address) queries:
  - If needed rarely, you can perform on‑demand scans using logs/receipts/traces and then block re‑processing.
  - If needed frequently at low latency, maintain a compact reverse index in MDBX (below).

Optional Reverse Index (Enable Only If Needed)
- Table: `address_to_txs` — maps address → list of TxNumbers using MDBX dupsort.
  - Key: 20‑byte `address`
  - Value: 8‑byte `tx_number` (u64, big‑endian)
  - DB flags: `DUPSORT | DUPFIXED` (compact, append‑friendly, easy pagination)
- Write path: upon inclusion of a tx, append each `(address, tx_number)` for `unique_addresses` in that tx.
- Benefits: tiny on disk, fast range iteration per address, no large blobs.
- When to use: only if you need fast, repeated generic address→tx queries.

Preserving Ordering
- Within a block: ordering is by `transactionIndex` or by TxNumber, where `tx_number = first_tx_num(block) + tx_index` using `BlockBodyIndices`.
- For token/address reconstruction that requires causality within a block, process the full block and feed transactions through `tx_processor` in block order.

Minimal Table Set (Recommended Now)
- `tx_arrival_by_txnum` (required):
  - Key: BE u64 (TxNumber)
  - Value: u64 (first_seen_ns)
- `address_to_txs` (optional):
  - Key: 20‑byte address; Value: BE u64 TxNumber, dupsort+dupfixed
- Existing curated tables (optional depending on usage):
  - `tokens`, `pools`, `trades`, `address_metrics` — keep lean; prefer computing dynamic views via Reth + tx_processor.

Query Patterns
- By tx hash → arrival time:
  - Reth `TransactionHashNumbers`: `hash → tx_number` → lookup `tx_arrival_by_txnum[tx_number]`.
- By token → blocks → processed txs:
  - Logs on token (+pool addresses) → union block numbers → fetch full blocks → run `tx_processor` → filter by token/pool/participants.
- By generic address (EOA/contract):
  - If MDBX reverse index enabled: iterate `address_to_txs[address]` dupset for direct TxNumbers.
  - Else: on‑demand logs/receipts/traces to collect candidate blocks, then full block processing as needed.

Postgres De‑scoping
- Avoid writing a large relational `tx_participants` table.
- Keep only curated relational entities you truly need in Postgres (e.g., addresses metadata, minimal transactions for reports).

Reth Tables We Rely On
- `TransactionHashNumbers`: `TxHash → TxNumber` (hash resolution)
- `TransactionBlocks`: `TxNumber → BlockNumber` (block mapping)
- `BlockBodyIndices`: `BlockNumber → { first_tx_num, tx_count }` (ordering/position)
- Logs via JSON‑RPC (archive node): block and `transactionIndex` from event logs

Future Enhancements
- Optional pre‑inclusion persistence (e.g., `tx_arrivals_by_hash` with TTL) if you later want to recover arrivals after restarts.
- Batch append APIs for dupsort tables to reduce write amplification on hot addresses.

