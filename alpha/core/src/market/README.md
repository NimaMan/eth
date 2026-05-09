# market

Confirmed-chain market facts consumed by the alpha engine.

## Owns

- `MarketEvent`
- `TokenSnapshot`
- `PoolSnapshot`
- block-scoped market update metadata
- denomination address/symbol metadata needed by strategies that use different liquidity floors for ETH/WETH and USD-stable pools

## Does Not Own

- Portfolio positions.
- Strategy decisions.
- Mempool predictions.
- Redis read/write implementation.

## Python Lesson

Python passed full `ERC20Token` objects through the portfolio layer. Rust should pass smaller immutable snapshots/events so strategies cannot accidentally mutate canonical market state.
