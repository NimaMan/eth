# Address Enrichment

Populates `NetworkLabelKind` labels that are currently unpopulated:
`Cex`, `Bridge`, `Router`, and other known-address classifications.

## Sources

1. **External databases** — Etherscan labelcloud, Arkham entity graph, Nansen labels
2. **Heuristics** — High out-degree + no token holdings = Router; receives from many + sends to few large balances = CEX deposit
3. **Community contributions** — Manual labels stored in a local JSON/DB

## Modules

- `known_book.rs` — External known-address database integration
- `heuristics.rs` — On-chain heuristic label assignment
