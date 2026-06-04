# Evidence Requirements

Each mined-validation attempt must leave enough evidence to reconstruct the
decision path without relying on terminal scrollback.

## Required Artifacts

- Strategy run id.
- Strategy name.
- ETH tx executor status snapshot before public broadcast.
- ETH tx executor policy snapshot.
- Signer status or configured signer cap summary.
- Exact final request body or hash of the request body.
- Exact pre-submit simulation metadata.
- ETH tx executor journal rows for received, accepted, signed, dry-run,
  broadcast, broadcast-error, or rejected decisions.
- Signer journal rows for signed or rejected decisions.
- Alpha DB rows for order intent, trade, position, snapshots, and lifecycle
  state transitions.
- RPC receipts for buy and sell tx hashes.
- Deployed-vault events used by receipt reconciliation.
- Vault and treasury balance checks when applicable.
- Asena strategy page URL and validation/assessment URL.

## Required Mined Fields

For every confirmed buy or sell:

- tx hash;
- mined block number;
- mined block hash;
- transaction index;
- receipt status;
- gas used;
- cumulative gas used;
- effective gas price;
- paid gas cost;
- decoded deployed-vault event name and amounts;
- confirmation count used for first settlement;
- confirmation depth used for recheck.

## Failure Evidence

Failed gates are useful when they are precise. Record:

- rejecting component: alpha, ETH tx executor policy, signer policy, tx executor, RPC,
  receipt reconciliation, or strategy lifecycle;
- exact rejection reason;
- whether any transaction was signed;
- whether any transaction was broadcast;
- whether spend reservation was released;
- whether alpha correctly avoided a confirmed lifecycle state.
