# Mempool Management Summary

Generated: `2026-05-20T20:05:37+00:00`

## Counts

- Liquidity-removal pools: `161`
- Strict mempool-managed pools: `97` (60.2%)
- Creator main tx public before removal: `75` (46.6%)
- LP-control tx public before removal: `97` (60.2%)
- Both creator and LP-control public before removal: `75` (46.6%)
- Same manager/remover actor overlap: `97` (60.2%)
- Exit signal available from liquidity-removal mempool event: `161` (100.0%)
- Only removal signal, no prior public management signal: `64` (39.8%)

## Timing

- Median seconds from LP approval to liquidity removal: `900`
- Min/max seconds from LP approval to liquidity removal: `8` / `13102`
- Median seconds from trading enabled to liquidity removal: `934`
- Min/max seconds from trading enabled to liquidity removal: `141` / `13111`

## Latest Pools

| First removal | Pool | Strict flag | LP window seconds | Creator window seconds |
| --- | --- | --- | ---: | ---: |
| 2026-05-19 23:45:58+02:00 | `0xe99cccf8d64fa69d5458568c8f13b7838b9c7103` | `True` | 883 | 890 |
| 2026-05-19 23:32:12+02:00 | `0xd65480f85e2b5df4d724e661f9d05af900cda244` | `True` | 889 | 900 |
| 2026-05-19 23:28:01+02:00 | `0xebc28a5a800a6e0bf9498b7782ae6ce652859de9` | `True` | 836 | 845 |
| 2026-05-19 23:15:22+02:00 | `0xc3001e0c5ac3a3a84504eef390010f4ea8b9a31e` | `True` | 247 | 255 |
| 2026-05-19 23:11:35+02:00 | `0x9d4dec90de25f2d75a5505f724638d151b20efa0` | `True` | 954 | 961 |
| 2026-05-19 23:09:28+02:00 | `0x6d8de2748439c79189662c98c3a86a9499b02dd0` | `True` | 1061 | 1075 |
| 2026-05-19 23:09:23+02:00 | `0x326c3f70665fec415f71cf1cc32684286ed9ba1b` | `True` | 381 | 393 |
| 2026-05-19 23:01:16+02:00 | `0x2f93f8c0fd632dc17584d02af85072319e565bf2` | `None` | None | None |
| 2026-05-19 22:51:27+02:00 | `0x714fd9605c1cbdc749be5b75eeb91b5f662c8821` | `True` | 873 | 893 |
| 2026-05-19 22:48:56+02:00 | `0xefdc7bd71dbc4054d256287bdfa7d7275fe3f222` | `True` | 735 | 744 |

## Interpretation

Use `pool_mempool_managed_pre_removal` as the persistent behavior flag. `pool_mempool_exit_signal_available` is an event-level fact: it means the actual removal tx was seen in the mempool, not that the pool was known to be public-mempool managed before that event.
