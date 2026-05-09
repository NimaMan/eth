# Chain Truth

Initial observations from the range `25,041,048..25,041,137`.

## Key Reserve Points

| Block | Event | Token Reserve | WETH Reserve | Notes |
| --- | --- | ---: | ---: | --- |
| 25,041,048 | initial LP sync | 1,000,000,000.000000000 | 1.000000000000000000 | First pool sync. |
| 25,041,049 | first tracked buy | 835,138,624.660307288 | 1.198000000000000000 | First price point used by saved build. |
| 25,041,117 | pre-manipulation | 119,838,970.280179918 | 8.594042892376489107 | Pool still has meaningful token and WETH reserves. |
| 25,041,123 | creator sync | 119.838970281 | 8.594042892376489107 | Token reserve collapses by roughly 1,000,000x without WETH moving. |
| 25,041,123 | WETH drain sell | 119,838,970.280179918 | 0.000008619902574299 | A sell-like tx drains WETH. |
| 25,041,137 | later buy | 697.712149384 | 1.485008619902574267 | Pool has WETH but only dust token reserve. |

## Key Transactions

- `0x1d8bcaff714a59ba96aba2db3ac58e7862723382d02394aaac6d541d2bcbbe5e`
  creates the pair, sends `1,000,000,000` SSS and `1 WETH`, then syncs.
- `0x620827c84f54de392802cbc7b7fb7fdf565591d26481d4eb1ec1e7d47d3bc660`
  is a direct pool call from the creator that emits a `Sync` after the token
  reserve collapsed.
- `0x2c7f9398b329df81b5a526ab8f400a8b90c81206e47b4550f6c8b00efcaaac1b`
  transfers SSS into the pool and drains about `8.594 WETH`.
- `0x43ea4fbb24b326397e5f782c556be4b47f0406107bf263709e2f0c9250bb5c96`
  buys after the WETH drain and leaves the pool with only `697.712149384` SSS.

## Immediate Interpretation

The high price ratio is mathematically consistent with pool reserves. It is not
evidence of healthy price appreciation. It is a reserve-quality problem: WETH is
present while token reserve is dust relative to total supply.
