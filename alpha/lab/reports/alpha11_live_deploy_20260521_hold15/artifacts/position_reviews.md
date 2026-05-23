# Position Reviews

Result set: `live-alpha11-live-univ2-lp30-pool-update-block-hold-sweep-chain-sim-20260521-105807`
Strategy: `alpha11-live-univ2-lp30-pool-update-block-hold15`

Each section states how the position performed, whether the backtest could have done better, and the exact next-block priority evidence for its submitted orders.

## 01. `trd_mpf9gbf5_9klg_2`

- token: `0x3340486fe0B95D72858bCc907648e184936b3234`
- pool: `0x3340486fe0b95d72858bcc907648e184936b3234:0x62bdb1da6506cb4f988372268ded2183d631c56a`
- state: `sell_confirmed`; blocks: `25142582` -> `25142614`
- PnL: `0.003952` ETH; ROI: `39.52%`; current value: `0.000000` ETH
- peak/trough: max `0.007033` ETH at block `25142593`, min `0.000601` ETH at block `25142605`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25142613`
- after priority: top25 `0.003403` ETH, top10 `0.003264` ETH, fixed 2 gwei `0.003403` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Peak mark was 0.007033 ETH at block 25142593, so a trailing or take-profit exit would likely have improved this position.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142581->25142582 | 118467 | 2.000000 gwei | 2.510000 gwei | 15 | 0.000236934000 ETH |
| sell | 25142613->25142614 | 156377 | 2.000000 gwei | 2.500000 gwei | 12 | 0.000312754000 ETH |

## 02. `trd_mpf9h2u7_9klg_b`

- token: `0x68241707430e58350367a3f14a5285FDa1667734`
- pool: `0x68241707430e58350367a3f14a5285fda1667734:0xd723b8e13677122974c42edabacc21030316d79c`
- state: `buy_confirmed`; blocks: `25142583` -> `open`
- PnL: `-0.003096` ETH; ROI: `-30.96%`; current value: `0.006920` ETH
- peak/trough: max `-0.000123` ETH at block `25142583`, min `-0.004163` ETH at block `25142648`
- sell reason: `-`
- after priority: top25 `-0.003408` ETH, top10 `-0.003439` ETH, fixed 2 gwei `-0.003408` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits. The least-bad mark was still negative and occurred earlier; a time-stop or stricter continuation filter would have reduced the current drawdown.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142582->25142583 | 155970 | 2.000000 gwei | 2.198680 gwei | 16 | 0.000311940000 ETH |

## 03. `trd_mpf9h2pk_9klg_5`

- token: `0x0f4f69358dA5a86Cf9Bdd5742F320613d282bA37`
- pool: `0x0f4f69358da5a86cf9bdd5742f320613d282ba37:0xe46ec84baaa99e0fc597dfb0429b35acbb9166d9`
- state: `sell_confirmed`; blocks: `25142585` -> `25142633`
- PnL: `0.003075` ETH; ROI: `30.75%`; current value: `0.000000` ETH
- peak/trough: max `0.003955` ETH at block `25142618`, min `0.000021` ETH at block `25142586`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25142632`
- after priority: top25 `0.002637` ETH, top10 `0.002366` ETH, fixed 2 gwei `0.002366` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142584->25142585 | 155031 | 0.254600 gwei | 2.000000 gwei | 6 | 0.000310062000 ETH |
| sell | 25142632->25142633 | 199222 | 2.000000 gwei | 2.000000 gwei | 8 | 0.000398444000 ETH |

## 04. `trd_mpf9h2s6_9klg_8`

- token: `0x3821EE8F32f4E2e5e87f76e145e100352a291110`
- pool: `0x3821ee8f32f4e2e5e87f76e145e100352a291110:0x95252e44b58818cdc32ff8dd11d6848543f41b72`
- state: `sell_confirmed`; blocks: `25142585` -> `25142650`
- PnL: `-0.003738` ETH; ROI: `-37.38%`; current value: `0.000000` ETH
- peak/trough: max `0.000523` ETH at block `25142588`, min `-0.010018` ETH at block `25142609`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25142649`
- after priority: top25 `-0.004191` ETH, top10 `-0.004467` ETH, fixed 2 gwei `-0.004467` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. The position had a deep adverse mark and then recovered; a hard stop should be paired with scam-specific evidence, not only mark-to-market drawdown.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142584->25142585 | 158162 | 0.254600 gwei | 2.000000 gwei | 6 | 0.000316324000 ETH |
| sell | 25142649->25142650 | 206710 | 2.000000 gwei | 2.000000 gwei | 8 | 0.000413420000 ETH |

## 05. `trd_mpf9h4mf_9klg_e`

- token: `0xFf8F98fA9Cf9dEB1b369785d2BF2778Df6C0F609`
- pool: `0xff8f98fa9cf9deb1b369785d2bf2778df6c0f609:0xbb2a3a1529636325330835373def729344a97b6a`
- state: `sell_confirmed`; blocks: `25142587` -> `25142785`
- PnL: `-0.000557` ETH; ROI: `-5.57%`; current value: `0.000000` ETH
- peak/trough: max `-0.000094` ETH at block `25142622`, min `-0.000557` ETH at block `25142785`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25142784`
- after priority: top25 `-0.001112` ETH, top10 `-0.001246` ETH, fixed 2 gwei `-0.001112` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142586->25142587 | 118904 | 2.000000 gwei | 2.989605 gwei | 15 | 0.000237808000 ETH |
| sell | 25142784->25142785 | 158681 | 2.000000 gwei | 2.100000 gwei | 13 | 0.000317362000 ETH |

## 06. `trd_mpf9ili1_9klg_h`

- token: `0x9c12D7e7F82d9715E9F151D1a21eC624f49656Dd`
- pool: `0x9c12d7e7f82d9715e9f151d1a21ec624f49656dd:0x4c2cdeb8610d434046e86bf76b10c43bf4d0efe7`
- state: `sell_confirmed`; blocks: `25142592` -> `25142594`
- PnL: `-0.000040` ETH; ROI: `-0.40%`; current value: `0.000000` ETH
- peak/trough: max `-0.000040` ETH at block `25142594`, min `-0.000266` ETH at block `25142593`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25142593`; pending tx: `0x319355...0e636f`
- after priority: top25 `-0.000484` ETH, top10 `-0.000669` ETH, fixed 2 gwei `-0.000559` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Risk evidence observed: mempool trading_enabled pre_entry block 25142590 pending 0xdd1133...fdbe22; mempool lp_approval in_position block 25142593 pending 0x319355...0e636f; mempool liquidity_removal post_exit block 25142677 pending 0x487ef5...a6cad9.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142591->25142592 | 109287 | 2.000000 gwei | 3.000000 gwei | 19 | 0.000218574000 ETH |
| sell | 25142593->25142594 | 150595 | 1.500000 gwei | 2.000000 gwei | 10 | 0.000301190000 ETH |

## 07. `trd_mpf9qbdu_9klg_k`

- token: `0xe8353c0DA7a39380D6d967940467a91Bf81cE479`
- pool: `0xe8353c0da7a39380d6d967940467a91bf81ce479:0xc02028e9d665d11d355b98fbc0d73221fbc74a0e`
- state: `sell_confirmed`; blocks: `25142623` -> `25142659`
- PnL: `0.027107` ETH; ROI: `271.07%`; current value: `0.000000` ETH
- peak/trough: max `0.027107` ETH at block `25142659`, min `-0.000265` ETH at block `25142623`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25142658`
- after priority: top25 `0.026729` ETH, top10 `0.026571` ETH, fixed 2 gwei `0.026571` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25142620 pending 0xc9909f...afc2d4; mempool lp_approval in_position block 25142623 pending 0xef8d3b...d93270; mempool liquidity_removal post_exit block 25142703 pending 0xafa493...11bb9d.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142622->25142623 | 109217 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000218434000 ETH |
| sell | 25142658->25142659 | 158946 | 1.005298 gwei | 2.000000 gwei | 5 | 0.000317892000 ETH |

## 08. `trd_mpf9rvd2_9klg_n`

- token: `0x8EC7bf6075a786bcAbC617a4C3d31Ea050cAb693`
- pool: `0x8ec7bf6075a786bcabc617a4c3d31ea050cab693:0xc76bfff294a36b8b3613eb08d48b270186c59052`
- state: `sell_confirmed`; blocks: `25142628` -> `25143254`
- PnL: `0.000121` ETH; ROI: `1.21%`; current value: `0.000000` ETH
- peak/trough: max `0.000948` ETH at block `25142776`, min `-0.000789` ETH at block `25143181`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25143253`
- after priority: top25 `-0.000387` ETH, top10 `-0.000532` ETH, fixed 2 gwei `-0.000532` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142627->25142628 | 148105 | 1.019957 gwei | 2.000000 gwei | 4 | 0.000296210000 ETH |
| sell | 25143253->25143254 | 178736 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000357472000 ETH |

## 09. `trd_mpf9tg0u_9klg_q`

- token: `0xC2FF63C29bB1de82b321bcD786B47500d02037A8`
- pool: `0xc2ff63c29bb1de82b321bcd786b47500d02037a8:0x75470cfbca844b4f82124331a10bf2de802a4d4d`
- state: `buy_confirmed`; blocks: `25142635` -> `open`
- PnL: `-0.000320` ETH; ROI: `-3.20%`; current value: `0.009698` ETH
- peak/trough: max `-0.000309` ETH at block `25142635`, min `-0.000320` ETH at block `25142828`
- sell reason: `-`
- after priority: top25 `-0.000650` ETH, top10 `-0.000666` ETH, fixed 2 gwei `-0.000650` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142634->25142635 | 164589 | 2.000000 gwei | 2.100000 gwei | 13 | 0.000329178000 ETH |

## 10. `trd_mpfa7a96_9klg_t`

- token: `0xE53D61848e70D816644f3A9545Dd7D47Ec92D7A2`
- pool: `0xe53d61848e70d816644f3a9545dd7d47ec92d7a2:0xb25cd3904e0e7af9b8e569c767bc088b80bd7072`
- state: `sell_confirmed`; blocks: `25142688` -> `25142690`
- PnL: `-0.000282` ETH; ROI: `-2.82%`; current value: `0.000000` ETH
- peak/trough: max `-0.000265` ETH at block `25142689`, min `-0.000282` ETH at block `25142690`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25142689`; pending tx: `0x8b7d77...b56594`
- after priority: top25 `-0.000739` ETH, top10 `-0.000829` ETH, fixed 2 gwei `-0.000819` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Risk evidence observed: mempool trading_enabled pre_entry block 25142686 pending 0xf7bd4e...b822ad; mempool lp_approval in_position block 25142689 pending 0x8b7d77...b56594; mempool liquidity_removal post_exit block 25142734 pending 0x8dddb2...4aae00.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142687->25142688 | 109217 | 2.000000 gwei | 2.100000 gwei | 14 | 0.000218434000 ETH |
| sell | 25142689->25142690 | 158946 | 1.500000 gwei | 2.000000 gwei | 7 | 0.000317892000 ETH |

## 11. `trd_mpfab543_9klg_w`

- token: `0x8BE3C3dE329de79C2B267bfE31DfD8E5d79931aa`
- pool: `0x8be3c3de329de79c2b267bfe31dfd8e5d79931aa:0xc799aa450ef8ce569b1935bcc4516f63d2a933c6`
- state: `sell_confirmed`; blocks: `25142704` -> `25143555`
- PnL: `0.002873` ETH; ROI: `28.73%`; current value: `0.000000` ETH
- peak/trough: max `0.002873` ETH at block `25143555`, min `-0.003496` ETH at block `25143278`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25143554`
- after priority: top25 `0.002256` ETH, top10 `0.002256` ETH, fixed 2 gwei `0.002256` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142703->25142704 | 128912 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000257824000 ETH |
| sell | 25143554->25143555 | 179409 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000358818000 ETH |

## 12. `trd_mpfae98p_9klg_z`

- token: `0x98f97144F48d0a27D47F8D6348BE84162Ea885df`
- pool: `0x98f97144f48d0a27d47f8d6348be84162ea885df:0x9f9745ab19d43b6cbecc9dfd6829ade99314e059`
- state: `sell_confirmed`; blocks: `25142715` -> `25142717`
- PnL: `0.000140` ETH; ROI: `1.40%`; current value: `0.000000` ETH
- peak/trough: max `0.000156` ETH at block `25142716`, min `0.000140` ETH at block `25142717`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25142716`; pending tx: `0x30135b...4f692f`
- after priority: top25 `-0.000396` ETH, top10 `-0.000476` ETH, fixed 2 gwei `-0.000396` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25142713 pending 0x6550a9...3fff07; mempool lp_approval in_position block 25142716 pending 0x30135b...4f692f; mempool liquidity_removal post_exit block 25142800 pending 0xf169f4...b901ea.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142714->25142715 | 109287 | 2.000000 gwei | 2.000000 gwei | 8 | 0.000218574000 ETH |
| sell | 25142716->25142717 | 158906 | 2.000000 gwei | 2.500000 gwei | 12 | 0.000317812000 ETH |

## 13. `trd_mpfaivyu_9klg_12`

- token: `0xaAeE0c030C85C7dB1142D9c6B825C759aFe619f6`
- pool: `0xaaee0c030c85c7db1142d9c6b825c759afe619f6:0x62094ce8c9fdd5cc6146c62ad1cc72cfadad1da1`
- state: `buy_confirmed`; blocks: `25142733` -> `open`
- PnL: `0.003413` ETH; ROI: `34.13%`; current value: `0.013444` ETH
- peak/trough: max `0.004114` ETH at block `25144044`, min `-0.000274` ETH at block `25142734`
- sell reason: `-`
- after priority: top25 `0.003107` ETH, top10 `0.003030` ETH, fixed 2 gwei `0.003107` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Still open, so final quality is not known; deployment review should discount this mark until it exits.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142732->25142733 | 152955 | 2.000000 gwei | 2.503223 gwei | 16 | 0.000305910000 ETH |

## 14. `trd_mpfanhoz_9klg_15`

- token: `0x12900A57187cAF77B26cdE8a464f760cfcC3203E`
- pool: `0x12900a57187caf77b26cde8a464f760cfcc3203e:0x7404ab4e583b7edf1b1b624432101d4ab332956d`
- state: `sell_confirmed`; blocks: `25142751` -> `25142753`
- PnL: `-0.000339` ETH; ROI: `-3.39%`; current value: `0.000000` ETH
- peak/trough: max `-0.000289` ETH at block `25142752`, min `-0.000339` ETH at block `25142753`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25142752`; pending tx: `0x621338...c9dcbc`
- after priority: top25 `-0.000875` ETH, top10 `-0.000880` ETH, fixed 2 gwei `-0.000875` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Risk evidence observed: mempool trading_enabled pre_entry block 25142747 pending 0x147ebc...e09618; mempool lp_approval in_position block 25142752 pending 0x621338...c9dcbc; mempool liquidity_removal post_exit block 25142830 pending 0x097711...77d1d5.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142750->25142751 | 109287 | 2.000000 gwei | 2.000000 gwei | 9 | 0.000218574000 ETH |
| sell | 25142752->25142753 | 158928 | 2.000000 gwei | 2.031004 gwei | 13 | 0.000317856000 ETH |

## 15. `trd_mpfav74o_9klg_18`

- token: `0x512637b2Cb34D9c80D7F2fE7858EE3c443C811F6`
- pool: `0x512637b2cb34d9c80d7f2fe7858ee3c443c811f6:0xbbc9de5f723506d46b427295f3d3a58f3d606e07`
- state: `sell_confirmed`; blocks: `25142780` -> `25142783`
- PnL: `0.002101` ETH; ROI: `21.01%`; current value: `0.000000` ETH
- peak/trough: max `0.002101` ETH at block `25142783`, min `0.001543` ETH at block `25142782`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25142782`; pending tx: `0x0bd51d...6a3f7f`
- after priority: top25 `0.001586` ETH, top10 `0.001563` ETH, fixed 2 gwei `0.001586` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Risk evidence observed: mempool trading_enabled pre_entry block 25142778 pending 0x67de2e...3c81d0; mempool lp_approval in_position block 25142782 pending 0x0bd51d...6a3f7f; mempool liquidity_removal post_exit block 25144343 pending 0x5d9265...ca8083.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142779->25142780 | 108386 | 2.000000 gwei | 2.100000 gwei | 11 | 0.000216772000 ETH |
| sell | 25142782->25142783 | 148898 | 2.000000 gwei | 2.081565 gwei | 12 | 0.000297796000 ETH |

## 16. `trd_mpfbani5_9klg_1b`

- token: `0x89613C294DcE9205E720926a1eF2f918a0903856`
- pool: `0x89613c294dce9205e720926a1ef2f918a0903856:0xa7f94b17835330d6f2facf71941237d032bd9b92`
- state: `sell_confirmed`; blocks: `25142842` -> `25142874`
- PnL: `0.028972` ETH; ROI: `289.72%`; current value: `0.000000` ETH
- peak/trough: max `0.030194` ETH at block `25142869`, min `-0.000265` ETH at block `25142842`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25142873`
- after priority: top25 `0.028435` ETH, top10 `0.028326` ETH, fixed 2 gwei `0.028435` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25142840 pending 0xb70152...6a5c8f; mempool lp_approval in_position block 25142842 pending 0xd326d2...111080; mempool liquidity_removal post_exit block 25142918 pending 0xd435d7...2b0114.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142841->25142842 | 109287 | 2.000000 gwei | 3.000000 gwei | 19 | 0.000218574000 ETH |
| sell | 25142873->25142874 | 158928 | 2.000000 gwei | 2.000000 gwei | 7 | 0.000317856000 ETH |

## 17. `trd_mpfbse4a_9klg_1e`

- token: `0x91946f4a1069FE5Ee0DaeD38b9B99C966222427E`
- pool: `0x91946f4a1069fe5ee0daed38b9b99c966222427e:0xbcadd0f8d43f3d0fbeafa3d2c56990a5ae13e4d6`
- state: `sell_confirmed`; blocks: `25142909` -> `25142912`
- PnL: `-0.000280` ETH; ROI: `-2.80%`; current value: `0.000000` ETH
- peak/trough: max `-0.000263` ETH at block `25142911`, min `-0.000280` ETH at block `25142912`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25142911`; pending tx: `0x1ec307...ba724d`
- after priority: top25 `-0.000737` ETH, top10 `-0.000832` ETH, fixed 2 gwei `-0.000816` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Risk evidence observed: mempool trading_enabled pre_entry block 25142907 pending 0x5c5cfa...140174; mempool lp_approval in_position block 25142911 pending 0x1ec307...ba724d; mempool liquidity_removal post_exit block 25142962 pending 0xd7b761...db9ae6.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142908->25142909 | 109287 | 2.000000 gwei | 2.000000 gwei | 6 | 0.000218574000 ETH |
| sell | 25142911->25142912 | 158906 | 1.500000 gwei | 2.100000 gwei | 11 | 0.000317812000 ETH |

## 18. `trd_mpfbx1kj_9klg_1h`

- token: `0x85E7a9dF397d3AF59Bb184577231CcDB4d08FABE`
- pool: `0x85e7a9df397d3af59bb184577231ccdb4d08fabe:0x1e5423e4b702fd38c456df1f4cb153a623abbbd1`
- state: `buy_confirmed`; blocks: `25142927` -> `open`
- PnL: `-0.007997` ETH; ROI: `-79.97%`; current value: `0.002019` ETH
- peak/trough: max `-0.007972` ETH at block `25142929`, min `-0.007997` ETH at block `25144440`
- sell reason: `-`
- after priority: top25 `-0.008312` ETH, top10 `-0.008313` ETH, fixed 2 gwei `-0.008312` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142926->25142927 | 157229 | 2.000000 gwei | 2.006359 gwei | 11 | 0.000314458000 ETH |

## 19. `trd_mpfc2f3e_9klg_1k`

- token: `0x2fc9Edd615CfCdA82A711D597A12f4A7da541EAC`
- pool: `0x2fc9edd615cfcda82a711d597a12f4a7da541eac:0xb3d2e539f46a831ea03b9c54bec7cef51c869fee`
- state: `sell_confirmed`; blocks: `25142950` -> `25142978`
- PnL: `0.009918` ETH; ROI: `99.18%`; current value: `0.000000` ETH
- peak/trough: max `0.012729` ETH at block `25142966`, min `-0.000128` ETH at block `25142950`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25142977`
- after priority: top25 `0.009385` ETH, top10 `0.009315` ETH, fixed 2 gwei `0.009315` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25142948 pending 0x88b294...9f7211; mempool lp_approval in_position block 25142950 pending 0xe42999...352af5; mempool lp_approval in_position block 25142950 pending 0x64ddc1...b26cca.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142949->25142950 | 152929 | 2.000000 gwei | 2.000000 gwei | 6 | 0.000305858000 ETH |
| sell | 25142977->25142978 | 148358 | 1.526845 gwei | 2.000000 gwei | 10 | 0.000296716000 ETH |

## 20. `trd_mpfc40m2_9klg_1n`

- token: `0x98E2b91455F96e988276508c7fad47F47467edb4`
- pool: `0x98e2b91455f96e988276508c7fad47f47467edb4:0x4466f519e7ce5355e8fcfbf65674eb69d6720364`
- state: `buy_confirmed`; blocks: `25142954` -> `open`
- PnL: `0.002797` ETH; ROI: `27.97%`; current value: `0.012808` ETH
- peak/trough: max `0.002797` ETH at block `25143956`, min `-0.000200` ETH at block `25142956`
- sell reason: `-`
- after priority: top25 `0.002570` ETH, top10 `0.002547` ETH, fixed 2 gwei `0.002559` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Still open, so final quality is not known; deployment review should discount this mark until it exits.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142953->25142954 | 118974 | 1.907631 gwei | 2.100000 gwei | 14 | 0.000237948000 ETH |

## 21. `trd_mpfcawmw_9klg_1q`

- token: `0xa8346F00325deDAE1991Cf6C158A7e83D118d821`
- pool: `0xa8346f00325dedae1991cf6c158a7e83d118d821:0x71dfbaa8be4a9c2bdf74e463a1af217bf7f44035`
- state: `sell_confirmed`; blocks: `25142981` -> `25142984`
- PnL: `0.000447` ETH; ROI: `4.47%`; current value: `0.000000` ETH
- peak/trough: max `0.000464` ETH at block `25142983`, min `0.000447` ETH at block `25142984`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25142983`; pending tx: `0x71a058...38c6c5`
- after priority: top25 `0.000020` ETH, top10 `-0.000105` ETH, fixed 2 gwei `-0.000089` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25142980 pending 0x88afd7...9aa2b0; mempool lp_approval in_position block 25142983 pending 0x71a058...38c6c5; mempool liquidity_removal post_exit block 25143068 pending 0x0414f9...01556a.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25142980->25142981 | 109275 | 1.000000 gwei | 2.000000 gwei | 6 | 0.000218550000 ETH |
| sell | 25142983->25142984 | 158896 | 2.000000 gwei | 2.100000 gwei | 11 | 0.000317792000 ETH |

## 22. `trd_mpfch2qg_9klg_1t`

- token: `0x455b9814208E86B56f18e60771Fc4f48239B75Ab`
- pool: `0x455b9814208e86b56f18e60771fc4f48239b75ab:0x4f06ae436d7e2037c620337927251515ba2b6f0e`
- state: `sell_confirmed`; blocks: `25143006` -> `25143064`
- PnL: `0.001827` ETH; ROI: `18.27%`; current value: `0.000000` ETH
- peak/trough: max `0.006700` ETH at block `25143022`, min `-0.002961` ETH at block `25143006`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25143063`
- after priority: top25 `0.000841` ETH, top10 `0.000620` ETH, fixed 2 gwei `0.000841` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Peak mark was 0.006700 ETH at block 25143022, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled pre_entry block 25143004 pending 0x61d793...d0bb5e; mempool lp_approval post_exit block 25143092 pending 0x68582b...859a69; mempool liquidity_removal post_exit block 25143094 pending 0x7b32a1...71e6af.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143005->25143006 | 207638 | 2.000000 gwei | 2.092794 gwei | 11 | 0.000415276000 ETH |
| sell | 25143063->25143064 | 285136 | 2.000000 gwei | 2.708384 gwei | 18 | 0.000570272000 ETH |

## 23. `trd_mpfclppw_9klg_1w`

- token: `0xaA7F268384665DD32C82ccEfF93abAf0adcE4E2F`
- pool: `0xaa7f268384665dd32c82cceff93abaf0adce4e2f:0xfeb3ba03589ef8138793bad9d71ef066c4bac117`
- state: `sell_confirmed`; blocks: `25143024` -> `25143026`
- PnL: `0.001062` ETH; ROI: `10.62%`; current value: `0.000000` ETH
- peak/trough: max `0.001062` ETH at block `25143026`, min `0.000156` ETH at block `25143025`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143025`; pending tx: `0x51baa2...a6e752`
- after priority: top25 `0.000590` ETH, top10 `0.000515` ETH, fixed 2 gwei `0.000543` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25143022 pending 0x2c30ac...8ec5b2; mempool lp_approval in_position block 25143025 pending 0x51baa2...a6e752; mempool liquidity_removal post_exit block 25143054 pending 0xefb435...a75b34.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143023->25143024 | 109287 | 1.567000 gwei | 2.000000 gwei | 9 | 0.000218574000 ETH |
| sell | 25143025->25143026 | 150573 | 2.000000 gwei | 2.184705 gwei | 12 | 0.000301146000 ETH |

## 24. `trd_mpfcy20x_9klg_1z`

- token: `0x70E1AA4864a46Cb7fc40ee1F8415cc8A1FCA673D`
- pool: `0x70e1aa4864a46cb7fc40ee1f8415cc8a1fca673d:0x49098cd7b11c4019335af5b57b0f00535bf7f543`
- state: `sell_confirmed`; blocks: `25143071` -> `25143074`
- PnL: `0.001241` ETH; ROI: `12.41%`; current value: `0.000000` ETH
- peak/trough: max `0.001241` ETH at block `25143074`, min `0.000700` ETH at block `25143073`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143073`; pending tx: `0x2d7de2...110eba`
- after priority: top25 `0.000719` ETH, top10 `0.000522` ETH, fixed 2 gwei `0.000722` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25143070 pending 0x7c111b...5c6fd5; mempool lp_approval in_position block 25143073 pending 0x2d7de2...110eba; mempool liquidity_removal post_exit block 25143138 pending 0x3d678a...a5277f.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143070->25143071 | 109287 | 2.011000 gwei | 2.500000 gwei | 36 | 0.000218574000 ETH |
| sell | 25143073->25143074 | 150595 | 2.011000 gwei | 2.960000 gwei | 27 | 0.000301190000 ETH |

## 25. `trd_mpfd0mi6_9klg_22`

- token: `0x04537Dd4cAB3a8C3FC1e0C81ed91C9bF8E75Edf7`
- pool: `0x04537dd4cab3a8c3fc1e0c81ed91c9bf8e75edf7:0xa019eedbb023043924d5300321c6995f5c9c2bdb`
- state: `sell_confirmed`; blocks: `25143080` -> `25143083`
- PnL: `0.001857` ETH; ROI: `18.57%`; current value: `0.000000` ETH
- peak/trough: max `0.001890` ETH at block `25143082`, min `0.001857` ETH at block `25143083`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143082`; pending tx: `0x2293fb...f821a6`
- after priority: top25 `0.001321` ETH, top10 `0.001317` ETH, fixed 2 gwei `0.001321` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25143077 pending 0x506b75...6c263b; mempool lp_approval in_position block 25143082 pending 0x2293fb...f821a6; mempool liquidity_removal post_exit block 25143142 pending 0xb8b592...26c5a3.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143079->25143080 | 109287 | 2.000000 gwei | 2.032000 gwei | 16 | 0.000218574000 ETH |
| sell | 25143082->25143083 | 158884 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000317768000 ETH |

## 26. `trd_mpfdg2gw_9klg_25`

- token: `0x6E0E42FEAA321b6406d206C5D565245a463112E9`
- pool: `0x6e0e42feaa321b6406d206c5d565245a463112e9:0xc9296d46ba380bef937485d8e1cd929f1f0748ff`
- state: `buy_confirmed`; blocks: `25143141` -> `open`
- PnL: `-0.000392` ETH; ROI: `-3.92%`; current value: `0.009623` ETH
- peak/trough: max `-0.000392` ETH at block `25143142`, min `-0.000392` ETH at block `25143142`
- sell reason: `-`
- after priority: top25 `-0.000684` ETH, top10 `-0.000700` ETH, fixed 2 gwei `-0.000684` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143140->25143141 | 145792 | 2.000000 gwei | 2.110123 gwei | 12 | 0.000291584000 ETH |

## 27. `trd_mpfdjx2l_9klg_28`

- token: `0x7e0a44cC1D90A42b316c38E4254f5CbBa5fC4Bf1`
- pool: `0x7e0a44cc1d90a42b316c38e4254f5cbba5fc4bf1:0xffd7fc7b46a41fbb3260fa369dad457197197f85`
- state: `sell_confirmed`; blocks: `25143157` -> `25143192`
- PnL: `0.011771` ETH; ROI: `117.71%`; current value: `0.000000` ETH
- peak/trough: max `0.016743` ETH at block `25143186`, min `-0.000137` ETH at block `25143157`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25143191`
- after priority: top25 `0.011152` ETH, top10 `0.010922` ETH, fixed 2 gwei `0.011152` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Peak mark was 0.016743 ETH at block 25143186, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled pre_entry block 25143155 pending 0xf8258a...6ec5c7; mempool lp_approval in_position block 25143157 pending 0xff80ce...6bbe43; mempool lp_approval in_position block 25143157 pending 0xff36cd...6a16f8.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143156->25143157 | 152907 | 2.000000 gwei | 2.600000 gwei | 16 | 0.000305814000 ETH |
| sell | 25143191->25143192 | 156643 | 2.000000 gwei | 2.882116 gwei | 17 | 0.000313286000 ETH |

## 28. `trd_mpfdjx4q_9klg_2b`

- token: `0xFaCa514F47989c37E5CE167Cda6f9B6F79F0C8C2`
- pool: `0xfaca514f47989c37e5ce167cda6f9b6f79f0c8c2:0x4aa3d63b605e0394d2ce82d89a3f3d4fa52c10fd`
- state: `sell_confirmed`; blocks: `25143157` -> `25143188`
- PnL: `0.020422` ETH; ROI: `204.22%`; current value: `0.000000` ETH
- peak/trough: max `0.020422` ETH at block `25143188`, min `-0.000266` ETH at block `25143157`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25143187`
- after priority: top25 `0.020053` ETH, top10 `0.019837` ETH, fixed 2 gwei `0.019903` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25143155 pending 0xce51cd...030d5f; mempool lp_approval in_position block 25143157 pending 0x214bf8...4960bd; mempool liquidity_removal post_exit block 25143228 pending 0x5bc41d...cb561c.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143156->25143157 | 109217 | 2.000000 gwei | 2.600000 gwei | 16 | 0.000218434000 ETH |
| sell | 25143187->25143188 | 150635 | 1.000000 gwei | 2.000000 gwei | 8 | 0.000301270000 ETH |

## 29. `trd_mpfdkok4_9klg_2e`

- token: `0xbc464E867C0b31E0B9ab5C3947BC06D8176fFB15`
- pool: `0xbc464e867c0b31e0b9ab5c3947bc06d8176ffb15:0xb760c2f9fb184a0e1738366dc15ee000b0d909c6`
- state: `sell_confirmed`; blocks: `25143159` -> `25143161`
- PnL: `0.001635` ETH; ROI: `16.35%`; current value: `0.000000` ETH
- peak/trough: max `0.001635` ETH at block `25143161`, min `0.000079` ETH at block `25143160`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143160`; pending tx: `0x0e34a7...887242`
- after priority: top25 `0.001115` ETH, top10 `0.000953` ETH, fixed 2 gwei `0.001115` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25143157 pending 0xa05622...a426f9; mempool lp_approval in_position block 25143160 pending 0x0e34a7...887242; mempool liquidity_removal post_exit block 25143196 pending 0xbcacb0...100694.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143158->25143159 | 109287 | 2.000000 gwei | 2.100000 gwei | 14 | 0.000218574000 ETH |
| sell | 25143160->25143161 | 150552 | 2.000000 gwei | 3.000000 gwei | 17 | 0.000301104000 ETH |

## 30. `trd_mpfdxuhv_9klg_2h`

- token: `0xFc1E735eFDDfc144CB569aD1EF6F02c50A9A6640`
- pool: `0xfc1e735efddfc144cb569ad1ef6f02c50a9a6640:0xcc8d6d6ff44f940fa4c86a3646aaa09c769f04b5`
- state: `sell_confirmed`; blocks: `25143209` -> `25143212`
- PnL: `-0.000282` ETH; ROI: `-2.82%`; current value: `0.000000` ETH
- peak/trough: max `-0.000266` ETH at block `25143211`, min `-0.000282` ETH at block `25143212`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143211`; pending tx: `0x862139...6438ab`
- after priority: top25 `-0.000725` ETH, top10 `-0.000831` ETH, fixed 2 gwei `-0.000819` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Risk evidence observed: mempool trading_enabled pre_entry block 25143207 pending 0x692c03...577a52; mempool lp_approval in_position block 25143211 pending 0x862139...6438ab; mempool liquidity_removal post_exit block 25143242 pending 0xd3c009...81acf0.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143208->25143209 | 109217 | 1.137240 gwei | 2.000000 gwei | 4 | 0.000218434000 ETH |
| sell | 25143211->25143212 | 158946 | 2.000000 gwei | 2.079228 gwei | 12 | 0.000317892000 ETH |

## 31. `trd_mpfe72ab_9klg_2k`

- token: `0x88637C919655Dc4DB4C6b1464779D5222B46f410`
- pool: `0x88637c919655dc4db4c6b1464779d5222b46f410:0x2ee815f57ef65da6e549644c3bfe1e1b98296c1d`
- state: `sell_confirmed`; blocks: `25143245` -> `25143248`
- PnL: `-0.000282` ETH; ROI: `-2.82%`; current value: `0.000000` ETH
- peak/trough: max `-0.000266` ETH at block `25143247`, min `-0.000282` ETH at block `25143248`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143247`; pending tx: `0xadb94f...742e90`
- after priority: top25 `-0.000543` ETH, top10 `-0.000709` ETH, fixed 2 gwei `-0.000819` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Risk evidence observed: mempool trading_enabled pre_entry block 25143243 pending 0xc9d129...4786d3; mempool lp_approval in_position block 25143247 pending 0xadb94f...742e90; mempool liquidity_removal post_exit block 25143314 pending 0xd41a7b...162aa0.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143244->25143245 | 109287 | 0.201797 gwei | 1.000000 gwei | 4 | 0.000218574000 ETH |
| sell | 25143247->25143248 | 158928 | 1.500000 gwei | 2.000000 gwei | 7 | 0.000317856000 ETH |

## 32. `trd_mpfe9dnl_9klg_2n`

- token: `0xB5ea0aB812098E74e93b4e5E9817adde15b5e5A0`
- pool: `0xb5ea0ab812098e74e93b4e5e9817adde15b5e5a0:0x3b4efc8a94c60001993032a6f7053673c0dbeea3`
- state: `sell_confirmed`; blocks: `25143254` -> `25143257`
- PnL: `0.001370` ETH; ROI: `13.70%`; current value: `0.000000` ETH
- peak/trough: max `0.001370` ETH at block `25143257`, min `0.000564` ETH at block `25143256`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143256`; pending tx: `0xb4b723...e91639`
- after priority: top25 `0.000851` ETH, top10 `0.000836` ETH, fixed 2 gwei `0.000851` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25143252 pending 0xeb7832...dde19a; mempool lp_approval in_position block 25143256 pending 0xb4b723...e91639; mempool liquidity_removal post_exit block 25143346 pending 0xd3e2d7...47be82.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143253->25143254 | 109287 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000218574000 ETH |
| sell | 25143256->25143257 | 150573 | 2.000000 gwei | 2.100000 gwei | 13 | 0.000301146000 ETH |

## 33. `trd_mpferv8h_9klg_2q`

- token: `0x1DcB59017f72DEF186E7492408D93028D951a34a`
- pool: `0x1dcb59017f72def186e7492408d93028d951a34a:0x36fcfe73ac522ee064050f785405f378084bb696`
- state: `sell_confirmed`; blocks: `25143328` -> `25143347`
- PnL: `-0.009817` ETH; ROI: `-98.17%`; current value: `0.000000` ETH
- peak/trough: max `0.011908` ETH at block `25143340`, min `-0.009817` ETH at block `25143347`
- sell reason: `Exit: mempool liquidity removal signal`; trigger: `mempool liquidity_removal`; source: `mempool_signal`; signal block: `25143346`; pending tx: `0x503cbc...8e7a94`
- after priority: top25 `-0.010173` ETH, top10 `-0.010445` ETH, fixed 2 gwei `-0.010353` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Peak mark was 0.011908 ETH at block 25143340, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled pre_entry block 25143326 pending 0x39b457...d756fa; mempool lp_approval in_position block 25143328 pending 0x18b471...26bea7; mempool liquidity_removal in_position block 25143346 pending 0x503cbc...8e7a94.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143327->25143328 | 109287 | 0.349321 gwei | 1.387444 gwei | 3 | 0.000218574000 ETH |
| sell | 25143346->25143347 | 158906 | 2.000000 gwei | 2.998041 gwei | 25 | 0.000317812000 ETH |

## 34. `trd_mpfetfte_9klg_2t`

- token: `0xa4275c1326BEc80A0A2f50302b7CeCae7fdBce20`
- pool: `0xa4275c1326bec80a0a2f50302b7cecae7fdbce20:0x9d0ba97097592c907f21d6e2459ab87cce3f6c35`
- state: `buy_confirmed`; blocks: `25143332` -> `open`
- PnL: `-0.005457` ETH; ROI: `-54.57%`; current value: `0.004559` ETH
- peak/trough: max `-0.000225` ETH at block `25143334`, min `-0.005953` ETH at block `25144225`
- sell reason: `-`
- after priority: top25 `-0.005763` ETH, top10 `-0.005763` ETH, fixed 2 gwei `-0.005763` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits. The least-bad mark was still negative and occurred earlier; a time-stop or stricter continuation filter would have reduced the current drawdown.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143331->25143332 | 152955 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000305910000 ETH |

## 35. `trd_mpfexcdd_9klg_2w`

- token: `0xFA93a45ccEC4931c78d1312e56F9D0f9Dc5133A1`
- pool: `0xfa93a45ccec4931c78d1312e56f9d0f9dc5133a1:0x7b7c7c223ec3e69912a36657b0f13af79ab05c7a`
- state: `buy_confirmed`; blocks: `25143348` -> `open`
- PnL: `-0.000399` ETH; ROI: `-3.99%`; current value: `0.009619` ETH
- peak/trough: max `-0.000399` ETH at block `25143349`, min `-0.000399` ETH at block `25143349`
- sell reason: `-`
- after priority: top25 `-0.000701` ETH, top10 `-0.000718` ETH, fixed 2 gwei `-0.000718` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143347->25143348 | 159811 | 1.891485 gwei | 2.000000 gwei | 9 | 0.000319622000 ETH |

## 36. `trd_mpfeyuhd_9klg_2z`

- token: `0xaad87D2A3c93a094f5C9743716233a306490033b`
- pool: `0xaad87d2a3c93a094f5c9743716233a306490033b:0xfec3c91ad0fc56ad416641ffc24fcdef1ea72ceb`
- state: `sell_confirmed`; blocks: `25143354` -> `25143356`
- PnL: `0.000249` ETH; ROI: `2.49%`; current value: `0.000000` ETH
- peak/trough: max `0.000249` ETH at block `25143356`, min `-0.000265` ETH at block `25143355`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143355`; pending tx: `0x6ac309...a37e6c`
- after priority: top25 `-0.000270` ETH, top10 `-0.000432` ETH, fixed 2 gwei `-0.000270` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25143352 pending 0xa88c1c...b793f3; mempool lp_approval in_position block 25143355 pending 0x6ac309...a37e6c; mempool liquidity_removal post_exit block 25143430 pending 0x17739e...f22ddf.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143353->25143354 | 109287 | 2.000000 gwei | 2.100000 gwei | 15 | 0.000218574000 ETH |
| sell | 25143355->25143356 | 150552 | 2.000000 gwei | 3.000000 gwei | 17 | 0.000301104000 ETH |

## 37. `trd_mpff0evt_9klg_32`

- token: `0x01e47e6997D29d1A1E0d42B963CC68f73147D59F`
- pool: `0x01e47e6997d29d1a1e0d42b963cc68f73147d59f:0x55832e3bb216bbc5f90d9bb402aa385270de0d36`
- state: `sell_confirmed`; blocks: `25143359` -> `25144251`
- PnL: `0.001750` ETH; ROI: `17.50%`; current value: `0.000000` ETH
- peak/trough: max `0.004791` ETH at block `25144154`, min `-0.000311` ETH at block `25143361`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25144250`
- after priority: top25 `0.001253` ETH, top10 `0.000996` ETH, fixed 2 gwei `0.001088` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Peak mark was 0.004791 ETH at block 25144154, so a trailing or take-profit exit would likely have improved this position.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143358->25143359 | 150078 | 0.904309 gwei | 2.000000 gwei | 7 | 0.000300156000 ETH |
| sell | 25144250->25144251 | 180867 | 2.000000 gwei | 2.510000 gwei | 18 | 0.000361734000 ETH |

## 38. `trd_mpff8vho_9klg_35`

- token: `0x20e55Dd176A8fBA7fcbd7BCb634847e1DfC9D94C`
- pool: `0x20e55dd176a8fba7fcbd7bcb634847e1dfc9d94c:0x16b168cd507c1d31495f23e58a311058f0de2586`
- state: `buy_confirmed`; blocks: `25143394` -> `open`
- PnL: `-0.010016` ETH; ROI: `-100.16%`; current value: `0.000000` ETH
- peak/trough: max `0.009952` ETH at block `25143409`, min `-0.010016` ETH at block `25143587`
- sell reason: `-`
- after priority: top25 `-0.010295` ETH, top10 `-0.010326` ETH, fixed 2 gwei `-0.010311` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. Peak mark was 0.009952 ETH at block 25143409, so a trailing or take-profit exit would likely have improved this position.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143393->25143394 | 147700 | 1.894821 gwei | 2.100000 gwei | 15 | 0.000295400000 ETH |

## 39. `trd_mpffdi7l_9klg_38`

- token: `0x6897781d99150B25dAA8da836164558e9346D0B4`
- pool: `0x6897781d99150b25daa8da836164558e9346d0b4:0xfd0e81248339e46c8a3ba9767204369ef276d71d`
- state: `buy_confirmed`; blocks: `25143410` -> `open`
- PnL: `-0.010014` ETH; ROI: `-100.14%`; current value: `0.000001` ETH
- peak/trough: max `0.058778` ETH at block `25143427`, min `-0.010014` ETH at block `25143431`
- sell reason: `-`
- after priority: top25 `-0.010302` ETH, top10 `-0.010316` ETH, fixed 2 gwei `-0.010316` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. Peak mark was 0.058778 ETH at block 25143427, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled in_position block 25143412 pending 0x3f67e1...73ab6c.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143409->25143410 | 151188 | 1.902384 gwei | 2.000000 gwei | 10 | 0.000302376000 ETH |

## 40. `trd_mpffl9my_9klg_3b`

- token: `0x161231058a0D00ee8ce5c58d2F34b9e829974454`
- pool: `0x161231058a0d00ee8ce5c58d2f34b9e829974454:0x32619cbb29aab769e6c7e12231122feb5e7c7812`
- state: `buy_confirmed`; blocks: `25143442` -> `open`
- PnL: `-0.000459` ETH; ROI: `-4.59%`; current value: `0.009559` ETH
- peak/trough: max `0.000813` ETH at block `25143824`, min `-0.001574` ETH at block `25143611`
- sell reason: `-`
- after priority: top25 `-0.000778` ETH, top10 `-0.000778` ETH, fixed 2 gwei `-0.000778` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143441->25143442 | 159869 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000319738000 ETH |

## 41. `trd_mpfflzpn_9klg_3e`

- token: `0x2A2e399769C680aB3f6b8BEc131286Db549086F9`
- pool: `0x2a2e399769c680ab3f6b8bec131286db549086f9:0xf0968f90163b34cb45fd1790c9674c252de73027`
- state: `sell_confirmed`; blocks: `25143443` -> `25143446`
- PnL: `0.000208` ETH; ROI: `2.08%`; current value: `0.000000` ETH
- peak/trough: max `0.000208` ETH at block `25143446`, min `-0.000266` ETH at block `25143445`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143445`; pending tx: `0x4aa704...d4a48f`
- after priority: top25 `-0.000299` ETH, top10 `-0.000518` ETH, fixed 2 gwei `-0.000311` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25143442 pending 0x27476a...bc6900; mempool lp_approval in_position block 25143445 pending 0x4aa704...d4a48f; mempool liquidity_removal post_exit block 25143530 pending 0xa2e517...0b6f6b.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143442->25143443 | 109287 | 1.888651 gwei | 2.510000 gwei | 13 | 0.000218574000 ETH |
| sell | 25143445->25143446 | 150573 | 2.000000 gwei | 2.999997 gwei | 24 | 0.000301146000 ETH |

## 42. `trd_mpffs5z0_9klg_3h`

- token: `0x9FE4Cf8295689Dc96e9eF0fd7d7072dFaAf01C66`
- pool: `0x9fe4cf8295689dc96e9ef0fd7d7072dfaaf01c66:0x0e94770d6fa00330a75a9fbe8ffac6953f9d6cad`
- state: `sell_confirmed`; blocks: `25143468` -> `25143470`
- PnL: `-0.000280` ETH; ROI: `-2.80%`; current value: `0.000000` ETH
- peak/trough: max `-0.000265` ETH at block `25143469`, min `-0.000280` ETH at block `25143470`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143469`; pending tx: `0x41ca7d...d40f5a`
- after priority: top25 `-0.000737` ETH, top10 `-0.000816` ETH, fixed 2 gwei `-0.000816` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Risk evidence observed: mempool trading_enabled pre_entry block 25143466 pending 0x9fcf5f...341b39; mempool lp_approval in_position block 25143469 pending 0x41ca7d...d40f5a; mempool liquidity_removal post_exit block 25143541 pending 0x075cab...01267f.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143467->25143468 | 109287 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000218574000 ETH |
| sell | 25143469->25143470 | 158906 | 1.500000 gwei | 2.000000 gwei | 6 | 0.000317812000 ETH |

## 43. `trd_mpffwt9o_9klg_3k`

- token: `0x04ce19771E9D093687bc1dEF8bD3A7DFBcf1Cebd`
- pool: `0x04ce19771e9d093687bc1def8bd3a7dfbcf1cebd:0x95ce7823daa41c23e633141819b1adf1aad11401`
- state: `buy_confirmed`; blocks: `25143486` -> `open`
- PnL: `-0.000228` ETH; ROI: `-2.28%`; current value: `0.009789` ETH
- peak/trough: max `-0.000228` ETH at block `25143901`, min `-0.001553` ETH at block `25143517`
- sell reason: `-`
- after priority: top25 `-0.000547` ETH, top10 `-0.000628` ETH, fixed 2 gwei `-0.000547` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143485->25143486 | 159453 | 2.000000 gwei | 2.510000 gwei | 15 | 0.000318906000 ETH |

## 44. `trd_mpfg2yp2_9klg_3n`

- token: `0xC089E611418AfCf5b61c1d5e86CDBD88547aD619`
- pool: `0xc089e611418afcf5b61c1d5e86cdbd88547ad619:0x9507159839f3404cd710f7acc574c1bdb8a00c07`
- state: `buy_confirmed`; blocks: `25143510` -> `open`
- PnL: `-0.010007` ETH; ROI: `-100.07%`; current value: `0.000007` ETH
- peak/trough: max `-0.003585` ETH at block `25143510`, min `-0.010007` ETH at block `25144038`
- sell reason: `-`
- after priority: top25 `-0.010303` ETH, top10 `-0.010317` ETH, fixed 2 gwei `-0.010303` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. The least-bad mark was still negative and occurred earlier; a time-stop or stricter continuation filter would have reduced the current drawdown.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143509->25143510 | 147630 | 2.000000 gwei | 2.100000 gwei | 17 | 0.000295260000 ETH |

## 45. `trd_mpfgbfzn_9klg_3q`

- token: `0xe8A39427e438815c600831bAe4CE42050F6FEc5B`
- pool: `0xe8a39427e438815c600831bae4ce42050f6fec5b:0x2117e47d4431567284671e4a82c2814bfa034bba`
- state: `sell_confirmed`; blocks: `25143542` -> `25143545`
- PnL: `0.000893` ETH; ROI: `8.93%`; current value: `0.000000` ETH
- peak/trough: max `0.000893` ETH at block `25143545`, min `-0.000131` ETH at block `25143544`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143544`; pending tx: `0x5d503e...fab014`
- after priority: top25 `0.000291` ETH, top10 `0.000144` ETH, fixed 2 gwei `0.000291` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25143541 pending 0x1ac034...ec9df8; mempool lp_approval in_position block 25143544 pending 0x5d503e...fab014; mempool lp_approval in_position block 25143544 pending 0x89f49f...910f58.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143541->25143542 | 152666 | 2.000000 gwei | 2.100000 gwei | 13 | 0.000305332000 ETH |
| sell | 25143544->25143545 | 148081 | 2.000000 gwei | 2.891319 gwei | 16 | 0.000296162000 ETH |

## 46. `trd_mpfgejwb_9klg_3t`

- token: `0x216501245C04Db61EdBB43dF6477CD33b1d1bCCC`
- pool: `0x216501245c04db61edbb43df6477cd33b1d1bccc:0xcdb8226f08ea54de6fdaae2f14df01a235c68890`
- state: `sell_confirmed`; blocks: `25143554` -> `25143557`
- PnL: `0.003323` ETH; ROI: `33.23%`; current value: `0.000000` ETH
- peak/trough: max `0.003323` ETH at block `25143557`, min `0.002130` ETH at block `25143556`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143556`; pending tx: `0x738e25...cd1c61`
- after priority: top25 `0.002879` ETH, top10 `0.002665` ETH, fixed 2 gwei `0.002803` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Risk evidence observed: mempool trading_enabled pre_entry block 25143553 pending 0x566d1c...b50e1c; mempool lp_approval in_position block 25143556 pending 0x738e25...cd1c61; mempool liquidity_removal post_exit block 25143631 pending 0xa5f660...412289.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143553->25143554 | 109287 | 1.309670 gwei | 2.035261 gwei | 11 | 0.000218574000 ETH |
| sell | 25143556->25143557 | 150595 | 2.000000 gwei | 2.888836 gwei | 13 | 0.000301190000 ETH |

## 47. `trd_mpfguqi6_9klg_3w`

- token: `0x9a95361A959e01d7F55Ab43D1D76a7190eDF8D4a`
- pool: `0x9a95361a959e01d7f55ab43d1d76a7190edf8d4a:0xab97ddc7b6ace0339aaa77379596e85ed9fd1cff`
- state: `sell_confirmed`; blocks: `25143619` -> `25143728`
- PnL: `0.031826` ETH; ROI: `318.26%`; current value: `0.000000` ETH
- peak/trough: max `0.031826` ETH at block `25143728`, min `-0.003910` ETH at block `25143619`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25143727`
- after priority: top25 `0.030935` ETH, top10 `0.030903` ETH, fixed 2 gwei `0.030935` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled in_position block 25143706 pending 0x38203a...85c2fa.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143618->25143619 | 153146 | 2.000000 gwei | 2.017073 gwei | 11 | 0.000306292000 ETH |
| sell | 25143727->25143728 | 292615 | 2.000000 gwei | 2.100000 gwei | 15 | 0.000585230000 ETH |

## 48. `trd_mpfh1oig_9klg_3z`

- token: `0xC432e2c687e2759824876FfF3dA4fa27Fb135e5b`
- pool: `0xc432e2c687e2759824876fff3da4fa27fb135e5b:0x8ec83bf16b10916f4c596817a20669c884950256`
- state: `sell_confirmed`; blocks: `25143645` -> `25143647`
- PnL: `-0.000283` ETH; ROI: `-2.83%`; current value: `0.000000` ETH
- peak/trough: max `-0.000266` ETH at block `25143646`, min `-0.000283` ETH at block `25143647`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143646`; pending tx: `0x8bb259...d18706`
- after priority: top25 `-0.000819` ETH, top10 `-0.000830` ETH, fixed 2 gwei `-0.000819` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Risk evidence observed: mempool trading_enabled pre_entry block 25143643 pending 0xf3b426...64ed1c; mempool lp_approval in_position block 25143646 pending 0x8bb259...d18706; mempool liquidity_removal post_exit block 25143720 pending 0xb23a9f...91cadc.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143644->25143645 | 109217 | 2.000000 gwei | 2.100000 gwei | 19 | 0.000218434000 ETH |
| sell | 25143646->25143647 | 158946 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000317892000 ETH |

## 49. `trd_mpfh387t_9klg_42`

- token: `0x8DEEde7F69babE599C274Df02D278BC6E466d79e`
- pool: `0x8deede7f69babe599c274df02d278bc6e466d79e:0x0efb4f1b96e2809609f04327053248cf18794592`
- state: `sell_confirmed`; blocks: `25143650` -> `25143653`
- PnL: `0.001410` ETH; ROI: `14.10%`; current value: `0.000000` ETH
- peak/trough: max `0.001426` ETH at block `25143652`, min `0.001410` ETH at block `25143653`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143652`; pending tx: `0x9ddb02...7cda39`
- after priority: top25 `0.000874` ETH, top10 `0.000863` ETH, fixed 2 gwei `0.000874` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25143649 pending 0x9c0397...bbe12d; mempool lp_approval in_position block 25143652 pending 0x9ddb02...7cda39; mempool liquidity_removal post_exit block 25143701 pending 0x067259...893c6b.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143649->25143650 | 109287 | 2.000000 gwei | 2.100000 gwei | 12 | 0.000218574000 ETH |
| sell | 25143652->25143653 | 158906 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000317812000 ETH |

## 50. `trd_mpfha6gi_9klg_45`

- token: `0x1f7f0D6254320988a5b4D154733a1fb35854Dff7`
- pool: `0x1f7f0d6254320988a5b4d154733a1fb35854dff7:0xb194ed1b4d4b44b58b598f124255e5b5ecf8ef35`
- state: `buy_confirmed`; blocks: `25143678` -> `open`
- PnL: `-0.000479` ETH; ROI: `-4.79%`; current value: `0.009538` ETH
- peak/trough: max `-0.000479` ETH at block `25143679`, min `-0.000479` ETH at block `25143679`
- sell reason: `-`
- after priority: top25 `-0.000780` ETH, top10 `-0.000796` ETH, fixed 2 gwei `-0.000780` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143677->25143678 | 150859 | 2.000000 gwei | 2.100000 gwei | 13 | 0.000301718000 ETH |

## 51. `trd_mpfhkzyd_9klg_48`

- token: `0xf9f504ACD4845b2cC37554C248C7133D44B6e1D9`
- pool: `0xf9f504acd4845b2cc37554c248c7133d44b6e1d9:0x1c0640562f10f3809a8f62fedd9e85963e2637dd`
- state: `sell_confirmed`; blocks: `25143721` -> `25143734`
- PnL: `-0.009986` ETH; ROI: `-99.86%`; current value: `0.000000` ETH
- peak/trough: max `0.002928` ETH at block `25143730`, min `-0.009986` ETH at block `25143734`
- sell reason: `Exit: mempool liquidity removal signal`; trigger: `mempool liquidity_removal`; source: `mempool_signal`; signal block: `25143733`; pending tx: `0xa05c3e...0659fe`
- after priority: top25 `-0.010369` ETH, top10 `-0.010523` ETH, fixed 2 gwei `-0.010522` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Peak mark was 0.002928 ETH at block 25143730, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled pre_entry block 25143718 pending 0x2b827a...3fc421; mempool lp_approval in_position block 25143721 pending 0x2248ee...db574f; mempool liquidity_removal in_position block 25143733 pending 0xa05c3e...0659fe.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143720->25143721 | 109217 | 0.600000 gwei | 2.000000 gwei | 8 | 0.000218434000 ETH |
| sell | 25143733->25143734 | 158946 | 2.000000 gwei | 2.005000 gwei | 11 | 0.000317892000 ETH |

## 52. `trd_mpfho2nh_9klg_4b`

- token: `0x3e9b8A65541e559459251f08327Bc2E818267D27`
- pool: `0x3e9b8a65541e559459251f08327bc2e818267d27:0x146eb6dcbf30f372de3b1fe703177328b5198a75`
- state: `sell_confirmed`; blocks: `25143732` -> `25143772`
- PnL: `0.005408` ETH; ROI: `54.08%`; current value: `0.000000` ETH
- peak/trough: max `0.005408` ETH at block `25143772`, min `0.000104` ETH at block `25143733`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25143771`
- after priority: top25 `0.005002` ETH, top10 `0.004745` ETH, fixed 2 gwei `0.004761` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25143730 pending 0xa5ef7e...03e036; mempool lp_approval post_exit block 25144145 pending 0x9144d9...503fdb; mempool liquidity_removal post_exit block 25144146 pending 0xf510a5...e0d73e.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143731->25143732 | 160549 | 0.500000 gwei | 2.000000 gwei | 6 | 0.000321098000 ETH |
| sell | 25143771->25143772 | 163202 | 2.000000 gwei | 2.100000 gwei | 11 | 0.000326404000 ETH |

## 53. `trd_mpfhousr_9klg_4e`

- token: `0xE2d399722DDa939ceCf2819E54cB637Ff7011aa3`
- pool: `0xe2d399722dda939cecf2819e54cb637ff7011aa3:0x34271f829cd27afcfc3a508f39873255b9fd52c0`
- state: `sell_confirmed`; blocks: `25143734` -> `25143737`
- PnL: `0.000173` ETH; ROI: `1.73%`; current value: `0.000000` ETH
- peak/trough: max `0.000173` ETH at block `25143737`, min `-0.000269` ETH at block `25143736`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143736`; pending tx: `0xe0fe99...c1f5ab`
- after priority: top25 `-0.000436` ETH, top10 `-0.000452` ETH, fixed 2 gwei `-0.000436` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25143733 pending 0x6052be...accb26; mempool lp_approval in_position block 25143736 pending 0xe0fe99...c1f5ab; mempool liquidity_removal post_exit block 25143823 pending 0xce0c73...0d80f3.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143733->25143734 | 153833 | 2.000000 gwei | 2.005000 gwei | 11 | 0.000307666000 ETH |
| sell | 25143736->25143737 | 150613 | 2.000000 gwei | 2.100000 gwei | 12 | 0.000301226000 ETH |

## 54. `trd_mpfhr6hw_9klg_4h`

- token: `0x5e3b49D87D049986b92c566B6f6fF9aD527318E7`
- pool: `0x5e3b49d87d049986b92c566b6f6ff9ad527318e7:0xcb2420d943002cea73144f9571bd9724e3da2cf3`
- state: `sell_confirmed`; blocks: `25143744` -> `25143746`
- PnL: `0.001143` ETH; ROI: `11.43%`; current value: `0.000000` ETH
- peak/trough: max `0.001160` ETH at block `25143745`, min `0.001143` ETH at block `25143746`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143745`; pending tx: `0xb26bcf...881691`
- after priority: top25 `0.000517` ETH, top10 `0.000025` ETH, fixed 2 gwei `0.000517` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25143742 pending 0xc7ee6e...5c13ec; mempool lp_approval in_position block 25143745 pending 0xb26bcf...881691; mempool liquidity_removal post_exit block 25143832 pending 0x00a3d9...9c68bf.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143743->25143744 | 153903 | 2.000000 gwei | 2.100000 gwei | 14 | 0.000307806000 ETH |
| sell | 25143745->25143746 | 158906 | 2.000000 gwei | 5.000000 gwei | 19 | 0.000317812000 ETH |

## 55. `trd_mpfib86v_9klg_4k`

- token: `0xb3077b258202A2068EEa136971781aBD97C79FF1`
- pool: `0xb3077b258202a2068eea136971781abd97c79ff1:0xf86c15c5d1d62ca05528d6de21dce23e76655129`
- state: `sell_confirmed`; blocks: `25143823` -> `25143857`
- PnL: `0.009981` ETH; ROI: `99.81%`; current value: `0.000000` ETH
- peak/trough: max `0.017209` ETH at block `25143847`, min `-0.000148` ETH at block `25143823`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25143856`
- after priority: top25 `0.009355` ETH, top10 `0.009183` ETH, fixed 2 gwei `0.009355` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Peak mark was 0.017209 ETH at block 25143847, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled pre_entry block 25143821 pending 0xf47892...5ec59e; mempool lp_approval in_position block 25143823 pending 0xf33e37...cfb57f; mempool lp_approval in_position block 25143823 pending 0x1cd384...38dc59.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143822->25143823 | 154179 | 2.000000 gwei | 2.856063 gwei | 15 | 0.000308358000 ETH |
| sell | 25143856->25143857 | 158726 | 2.000000 gwei | 2.250000 gwei | 12 | 0.000317452000 ETH |

## 56. `trd_mpfif2ps_9klg_4n`

- token: `0x3D1BF44c6B27A32a64E40aAD5e08E426243e0179`
- pool: `0x3d1bf44c6b27a32a64e40aad5e08e426243e0179:0x7bec7d19513379b769a97ed0f008e9c0b4f32bd5`
- state: `sell_confirmed`; blocks: `25143837` -> `25143839`
- PnL: `0.000582` ETH; ROI: `5.82%`; current value: `0.000000` ETH
- peak/trough: max `0.000582` ETH at block `25143839`, min `0.000174` ETH at block `25143838`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143838`; pending tx: `0x983719...ff0bca`
- after priority: top25 `-0.000042` ETH, top10 `-0.000549` ETH, fixed 2 gwei `-0.000027` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25143835 pending 0x8c4bba...1b423e; mempool lp_approval in_position block 25143838 pending 0x983719...ff0bca; mempool liquidity_removal post_exit block 25143911 pending 0x71bffd...179b10.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143836->25143837 | 153903 | 2.100000 gwei | 5.000000 gwei | 33 | 0.000307806000 ETH |
| sell | 25143838->25143839 | 150573 | 2.000000 gwei | 2.400000 gwei | 16 | 0.000301146000 ETH |

## 57. `trd_mpfiglz8_9klg_4q`

- token: `0x6E638Df2b8237dd8a77768c4Fcfb234b47421F1C`
- pool: `0x6e638df2b8237dd8a77768c4fcfb234b47421f1c:0x736ce335678878546346f9368354d1c7261c9564`
- state: `sell_confirmed`; blocks: `25143844` -> `25143879`
- PnL: `0.029027` ETH; ROI: `290.27%`; current value: `0.000000` ETH
- peak/trough: max `0.029027` ETH at block `25143879`, min `-0.000271` ETH at block `25143844`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25143878`
- after priority: top25 `0.028401` ETH, top10 `0.028242` ETH, fixed 2 gwei `0.028401` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25143841 pending 0x398026...34c2f4; mempool lp_approval in_position block 25143844 pending 0x833165...85720d; mempool liquidity_removal post_exit block 25143926 pending 0xd139ee...cc9484.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143843->25143844 | 153903 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000307806000 ETH |
| sell | 25143878->25143879 | 158906 | 2.000000 gwei | 2.998118 gwei | 25 | 0.000317812000 ETH |

## 58. `trd_mpfip3ks_9klg_4t`

- token: `0xEdd436AEEE230D5465590A1e7541DA02f598f516`
- pool: `0xedd436aeee230d5465590a1e7541da02f598f516:0xb4263024321e69d1504a8c9c01478c6f30dfe654`
- state: `sell_confirmed`; blocks: `25143876` -> `25143918`
- PnL: `0.007878` ETH; ROI: `78.78%`; current value: `0.000000` ETH
- peak/trough: max `0.009684` ETH at block `25143907`, min `0.001527` ETH at block `25143897`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25143917`
- after priority: top25 `0.006972` ETH, top10 `0.006479` ETH, fixed 2 gwei `0.007044` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25143874 pending 0xb554d9...872e20.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143875->25143876 | 142469 | 2.500000 gwei | 5.000000 gwei | 29 | 0.000284938000 ETH |
| sell | 25143917->25143918 | 274479 | 2.000000 gwei | 2.500000 gwei | 15 | 0.000548958000 ETH |

## 59. `trd_mpfirf48_9klg_4w`

- token: `0x1e5e64456E9ACD1C5B8895aBc42B2d2a4a542795`
- pool: `0x1e5e64456e9acd1c5b8895abc42b2d2a4a542795:0x1b9fb2cac34f1e34bad8b5d9fea463dbe2bb80f1`
- state: `buy_confirmed`; blocks: `25143884` -> `open`
- PnL: `-0.010021` ETH; ROI: `-100.21%`; current value: `0.000000` ETH
- peak/trough: max `-0.000275` ETH at block `25143885`, min `-0.010021` ETH at block `25143946`
- sell reason: `-`
- after priority: top25 `-0.010227` ETH, top10 `-0.010443` ETH, fixed 2 gwei `-0.010423` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. The least-bad mark was still negative and occurred earlier; a time-stop or stricter continuation filter would have reduced the current drawdown.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143883->25143884 | 200720 | 1.023846 gwei | 2.100000 gwei | 11 | 0.000401440000 ETH |

## 60. `trd_mpfj0nxo_9klg_4z`

- token: `0x9514Ad4467D9118b9192d5C30d9D405dC48bb551`
- pool: `0x9514ad4467d9118b9192d5c30d9d405dc48bb551:0x0a90f67f06f3666a88509c904ef809d5d9cb120c`
- state: `sell_confirmed`; blocks: `25143922` -> `25143956`
- PnL: `0.029955` ETH; ROI: `299.55%`; current value: `0.000000` ETH
- peak/trough: max `0.029955` ETH at block `25143956`, min `-0.000264` ETH at block `25143922`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25143955`
- after priority: top25 `0.029403` ETH, top10 `0.029260` ETH, fixed 2 gwei `0.029419` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25143920 pending 0xf2235c...27117a; mempool lp_approval in_position block 25143922 pending 0x1d8ad4...734b0a; mempool liquidity_removal post_exit block 25144011 pending 0x6a97da...c0166a.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143921->25143922 | 109287 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000218574000 ETH |
| sell | 25143955->25143956 | 158928 | 2.100000 gwei | 3.000000 gwei | 28 | 0.000317856000 ETH |

## 61. `trd_mpfj1g6e_9klg_52`

- token: `0xedaD978B1aE3E1acC601859F95c2fFf6A1a4C74a`
- pool: `0xedad978b1ae3e1acc601859f95c2fff6a1a4c74a:0x506e766e37a3dc301ec1104498bc65d34ea85533`
- state: `sell_confirmed`; blocks: `25143924` -> `25143956`
- PnL: `-0.001039` ETH; ROI: `-10.39%`; current value: `0.000000` ETH
- peak/trough: max `0.001503` ETH at block `25143949`, min `-0.003646` ETH at block `25143940`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25143955`
- after priority: top25 `-0.001917` ETH, top10 `-0.002422` ETH, fixed 2 gwei `-0.001876` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Risk evidence observed: mempool trading_enabled pre_entry block 25143922 pending 0xad0fdc...d84bd0; mempool trading_enabled post_exit block 25144801 pending 0xe6a961...8cd743.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143923->25143924 | 125363 | 2.091866 gwei | 4.017294 gwei | 27 | 0.000250726000 ETH |
| sell | 25143955->25143956 | 293100 | 2.100000 gwei | 3.000000 gwei | 28 | 0.000586200000 ETH |

## 62. `trd_mpfj6ubc_9klg_55`

- token: `0x20a8D189c2588d00c01f482B4f47835AD625B96F`
- pool: `0x20a8d189c2588d00c01f482b4f47835ad625b96f:0xdb179f6126d278d4ef739c171a833ee8aab2d68a`
- state: `sell_confirmed`; blocks: `25143944` -> `25143947`
- PnL: `0.001916` ETH; ROI: `19.16%`; current value: `0.000000` ETH
- peak/trough: max `0.001939` ETH at block `25143946`, min `0.001916` ETH at block `25143947`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25143946`; pending tx: `0xc888eb...bcf393`
- after priority: top25 `0.001380` ETH, top10 `0.001364` ETH, fixed 2 gwei `0.001380` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25143943 pending 0xff05fe...ff281a; mempool lp_approval in_position block 25143946 pending 0xc888eb...bcf393; mempool liquidity_removal post_exit block 25144033 pending 0x8886f9...a21bd1.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25143943->25143944 | 109275 | 2.000000 gwei | 2.000000 gwei | 5 | 0.000218550000 ETH |
| sell | 25143946->25143947 | 158896 | 2.000000 gwei | 2.100000 gwei | 12 | 0.000317792000 ETH |

## 63. `trd_mpfjrxxe_9klg_58`

- token: `0x5377303Ba6a46dad2f5F66c1478Aa11d6E467Cef`
- pool: `0x5377303ba6a46dad2f5f66c1478aa11d6e467cef:0x3aebd9e5247c8d96f5466d801451a1927404240e`
- state: `sell_confirmed`; blocks: `25144025` -> `25144028`
- PnL: `-0.000287` ETH; ROI: `-2.87%`; current value: `0.000000` ETH
- peak/trough: max `-0.000268` ETH at block `25144027`, min `-0.000287` ETH at block `25144028`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25144027`; pending tx: `0x3d8e22...c6d2a7`
- after priority: top25 `-0.000823` ETH, top10 `-0.000993` ETH, fixed 2 gwei `-0.000823` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Risk evidence observed: mempool trading_enabled pre_entry block 25144022 pending 0xd564eb...ca25c9; mempool lp_approval in_position block 25144027 pending 0x3d8e22...c6d2a7; mempool liquidity_removal post_exit block 25144109 pending 0x972170...9e3ed2.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144024->25144025 | 109287 | 2.000000 gwei | 2.100000 gwei | 11 | 0.000218574000 ETH |
| sell | 25144027->25144028 | 158928 | 2.000000 gwei | 3.000000 gwei | 22 | 0.000317856000 ETH |

## 64. `trd_mpfjxd66_9klg_5b`

- token: `0x0ca72b24abf950be8BB145f5c68f7004485192B2`
- pool: `0x0ca72b24abf950be8bb145f5c68f7004485192b2:0x57ca70b663ead85122589f345835d8f8209608dc`
- state: `buy_confirmed`; blocks: `25144048` -> `open`
- PnL: `-0.010017` ETH; ROI: `-100.17%`; current value: `0.000002` ETH
- peak/trough: max `0.200569` ETH at block `25144059`, min `-0.010017` ETH at block `25144063`
- sell reason: `-`
- after priority: top25 `-0.010299` ETH, top10 `-0.010594` ETH, fixed 2 gwei `-0.010299` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. Peak mark was 0.200569 ETH at block 25144059, so a trailing or take-profit exit would likely have improved this position.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144047->25144048 | 140794 | 2.000000 gwei | 4.097050 gwei | 18 | 0.000281588000 ETH |

## 65. `trd_mpfjyvq4_9klg_5e`

- token: `0xc2bEF0cFE92FF5e0879A799A1aB56263ef4AD8A5`
- pool: `0xc2bef0cfe92ff5e0879a799a1ab56263ef4ad8a5:0xff7ad39e5d8393c3b273f2765cba51541b2a9303`
- state: `sell_confirmed`; blocks: `25144052` -> `25144055`
- PnL: `0.000461` ETH; ROI: `4.61%`; current value: `0.000000` ETH
- peak/trough: max `0.000461` ETH at block `25144055`, min `-0.000268` ETH at block `25144054`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25144054`; pending tx: `0x9e29df...74bca8`
- after priority: top25 `-0.000294` ETH, top10 `-0.000989` ETH, fixed 2 gwei `-0.000059` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25144050 pending 0xf3da76...bd32e3; mempool lp_approval in_position block 25144054 pending 0x9e29df...74bca8; mempool liquidity_removal post_exit block 25144153 pending 0xe5ac4c...4167c0.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144051->25144052 | 109217 | 2.962998 gwei | 5.000000 gwei | 39 | 0.000218434000 ETH |
| sell | 25144054->25144055 | 150592 | 2.860402 gwei | 6.000000 gwei | 36 | 0.000301184000 ETH |

## 66. `trd_mpfkiyad_9klg_5h`

- token: `0xD63838bFdCE6E376742D086eeEAE18DDA5C0e06D`
- pool: `0xd63838bfdce6e376742d086eeeae18dda5c0e06d:0x2df2ec6bd78e2a402436be970e92d044245a0a83`
- state: `sell_confirmed`; blocks: `25144130` -> `25144133`
- PnL: `0.001557` ETH; ROI: `15.57%`; current value: `0.000000` ETH
- peak/trough: max `0.001557` ETH at block `25144133`, min `0.001031` ETH at block `25144132`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25144132`; pending tx: `0xed30d7...cf485e`
- after priority: top25 `0.001083` ETH, top10 `0.000942` ETH, fixed 2 gwei `0.001037` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25144128 pending 0x514a85...bc3168; mempool lp_approval in_position block 25144132 pending 0xed30d7...cf485e; mempool liquidity_removal post_exit block 25144138 pending 0x1ff5e3...c8bc9b.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144129->25144130 | 109217 | 2.000000 gwei | 2.869284 gwei | 23 | 0.000218434000 ETH |
| sell | 25144132->25144133 | 150613 | 1.691236 gwei | 2.000000 gwei | 8 | 0.000301226000 ETH |

## 67. `trd_mpfknkmv_9klg_5k`

- token: `0x700051BA108884AB58c0bd20A9BB05b10eb42063`
- pool: `0x700051ba108884ab58c0bd20a9bb05b10eb42063:0x6d45b4fd7e5bb32c3d501b0bc04969ed22f0d2a9`
- state: `sell_confirmed`; blocks: `25144148` -> `25144151`
- PnL: `0.003029` ETH; ROI: `30.29%`; current value: `0.000000` ETH
- peak/trough: max `0.003029` ETH at block `25144151`, min `0.000502` ETH at block `25144150`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25144150`; pending tx: `0x11b3ed...2fa0c1`
- after priority: top25 `0.002494` ETH, top10 `0.002183` ETH, fixed 2 gwei `0.002509` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Risk evidence observed: mempool trading_enabled pre_entry block 25144146 pending 0xec6120...aca52d; mempool lp_approval in_position block 25144150 pending 0x11b3ed...2fa0c1; mempool liquidity_removal post_exit block 25144225 pending 0x8f2511...7b5400.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144147->25144148 | 109275 | 2.000000 gwei | 2.106444 gwei | 12 | 0.000218550000 ETH |
| sell | 25144150->25144151 | 150564 | 2.100000 gwei | 4.092115 gwei | 31 | 0.000301128000 ETH |

## 68. `trd_mpfkpv2e_9klg_5n`

- token: `0xBF0fF25F4C4A121CD14056e4c8a7C99593a67786`
- pool: `0xbf0ff25f4c4a121cd14056e4c8a7c99593a67786:0x974ce1ad92a1cf8a6ecf3901213e5425035a3871`
- state: `sell_confirmed`; blocks: `25144157` -> `25144202`
- PnL: `0.005565` ETH; ROI: `55.65%`; current value: `0.000000` ETH
- peak/trough: max `0.005565` ETH at block `25144202`, min `-0.000092` ETH at block `25144157`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25144201`
- after priority: top25 `0.005007` ETH, top10 `0.004904` ETH, fixed 2 gwei `0.005007` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25144156 pending 0xd73ae4...f3665c; mempool lp_approval post_exit block 25145185 pending 0x577087...d24f3f; mempool liquidity_removal post_exit block 25145185 pending 0xf746a2...00c118.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144156->25144157 | 115933 | 2.000000 gwei | 2.887734 gwei | 17 | 0.000231866000 ETH |
| sell | 25144201->25144202 | 163202 | 2.000000 gwei | 2.000000 gwei | 9 | 0.000326404000 ETH |

## 69. `trd_mpfl30i4_9klg_5q`

- token: `0x15a530AefB61Ec6F2052F839a8FD2b2ECa950683`
- pool: `0x15a530aefb61ec6f2052f839a8fd2b2eca950683:0x577154dcd99c288f408e832dfc916c50cb6f8653`
- state: `buy_confirmed`; blocks: `25144208` -> `open`
- PnL: `-0.010019` ETH; ROI: `-100.19%`; current value: `0.000000` ETH
- peak/trough: max `0.229659` ETH at block `25144219`, min `-0.010019` ETH at block `25144230`
- sell reason: `-`
- after priority: top25 `-0.010308` ETH, top10 `-0.010433` ETH, fixed 2 gwei `-0.010308` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. Peak mark was 0.229659 ETH at block 25144219, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled in_position block 25144213 pending 0x4f6e66...975544.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144207->25144208 | 144462 | 2.000000 gwei | 2.870023 gwei | 18 | 0.000288924000 ETH |

## 70. `trd_mpflca95_9klg_5t`

- token: `0xB4d9a824Be198C12C204027951C0546e288E5ACC`
- pool: `0xb4d9a824be198c12c204027951c0546e288e5acc:0x9f2bf060d5f9dcc8628a6789912d18688186d376`
- state: `sell_confirmed`; blocks: `25144246` -> `25144279`
- PnL: `0.016547` ETH; ROI: `165.47%`; current value: `0.000000` ETH
- peak/trough: max `0.016547` ETH at block `25144279`, min `-0.000267` ETH at block `25144246`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25144278`
- after priority: top25 `0.015983` ETH, top10 `0.015792` ETH, fixed 2 gwei `0.016010` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25144244 pending 0xbc271f...79e645; mempool lp_approval in_position block 25144246 pending 0x1df68d...6d1412; mempool liquidity_removal post_exit block 25144317 pending 0xdedcf6...94ae95.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144245->25144246 | 109287 | 2.250000 gwei | 4.000000 gwei | 27 | 0.000218574000 ETH |
| sell | 25144278->25144279 | 158906 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000317812000 ETH |

## 71. `trd_mpfld2ce_9klg_5w`

- token: `0x87FcBbc742C36a805A1Ae3a8e65cC31AA899E69e`
- pool: `0x87fcbbc742c36a805a1ae3a8e65cc31aa899e69e:0xc99da678abf60cc1f2c03e44124d314e64577c2b`
- state: `buy_confirmed`; blocks: `25144247` -> `open`
- PnL: `-0.010019` ETH; ROI: `-100.19%`; current value: `0.000000` ETH
- peak/trough: max `0.081716` ETH at block `25144260`, min `-0.010019` ETH at block `25144272`
- sell reason: `-`
- after priority: top25 `-0.010319` ETH, top10 `-0.010453` ETH, fixed 2 gwei `-0.010308` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. Peak mark was 0.081716 ETH at block 25144260, so a trailing or take-profit exit would likely have improved this position.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144246->25144247 | 144572 | 2.073688 gwei | 3.000000 gwei | 26 | 0.000289144000 ETH |

## 72. `trd_mpfljz8q_9klg_5z`

- token: `0xE43ab49b3fcD599A52d747B415014b1cdDafb1e5`
- pool: `0xe43ab49b3fcd599a52d747b415014b1cddafb1e5:0x5ae3bab5010a2131d1827698989fb0ff3fc83731`
- state: `buy_confirmed`; blocks: `25144275` -> `open`
- PnL: `-0.010017` ETH; ROI: `-100.17%`; current value: `0.000000` ETH
- peak/trough: max `0.312711` ETH at block `25144293`, min `-0.010017` ETH at block `25144311`
- sell reason: `-`
- after priority: top25 `-0.010306` ETH, top10 `-0.010434` ETH, fixed 2 gwei `-0.010306` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. Peak mark was 0.312711 ETH at block 25144293, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled in_position block 25144277 pending 0xb811bc...bab1f3.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144274->25144275 | 144525 | 2.000000 gwei | 2.882759 gwei | 19 | 0.000289050000 ETH |

## 73. `trd_mpflliwm_9klg_62`

- token: `0x0604b4D463Ed76D1Ae04b479b50b38d80EdF075b`
- pool: `0x0604b4d463ed76d1ae04b479b50b38d80edf075b:0x343791d381f489984a479434864e500f2e0f9b56`
- state: `buy_confirmed`; blocks: `25144280` -> `open`
- PnL: `-0.010017` ETH; ROI: `-100.17%`; current value: `0.000000` ETH
- peak/trough: max `0.004292` ETH at block `25144291`, min `-0.010017` ETH at block `25144296`
- sell reason: `-`
- after priority: top25 `-0.010327` ETH, top10 `-0.010465` ETH, fixed 2 gwei `-0.010327` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. Peak mark was 0.004292 ETH at block 25144291, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled in_position block 25144293 pending 0x55f33a...6d959e.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144279->25144280 | 154900 | 2.000000 gwei | 2.890500 gwei | 17 | 0.000309800000 ETH |

## 74. `trd_mpflzeo7_9klg_65`

- token: `0x19821240401a700246E93b692AA72522DEa0955c`
- pool: `0x19821240401a700246e93b692aa72522dea0955c:0x4cb4ec9018f6c5244b3bdef73d0c6697b5cba99f`
- state: `sell_confirmed`; blocks: `25144334` -> `25144337`
- PnL: `0.000478` ETH; ROI: `4.78%`; current value: `0.000000` ETH
- peak/trough: max `0.000478` ETH at block `25144337`, min `0.000047` ETH at block `25144336`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25144336`; pending tx: `0x5390a1...9b11e7`
- after priority: top25 `-0.000042` ETH, top10 `-0.000172` ETH, fixed 2 gwei `-0.000042` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25144332 pending 0x54f044...c97da6; mempool lp_approval in_position block 25144336 pending 0x5390a1...9b11e7; mempool liquidity_removal post_exit block 25144399 pending 0x2680fb...136b96.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144333->25144334 | 109287 | 2.000000 gwei | 3.000000 gwei | 17 | 0.000218574000 ETH |
| sell | 25144336->25144337 | 150573 | 2.000000 gwei | 2.135326 gwei | 15 | 0.000301146000 ETH |

## 75. `trd_mpfm42ts_9klg_68`

- token: `0x9928C024330af84373b05c9002960D02dE654F28`
- pool: `0x9928c024330af84373b05c9002960d02de654f28:0x2206861e07a486f7a083b3e20780bbef09b04062`
- state: `sell_confirmed`; blocks: `25144354` -> `25144384`
- PnL: `0.025303` ETH; ROI: `253.03%`; current value: `0.000000` ETH
- peak/trough: max `0.029022` ETH at block `25144380`, min `-0.000266` ETH at block `25144354`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25144383`
- after priority: top25 `0.024800` ETH, top10 `0.024331` ETH, fixed 2 gwei `0.024789` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Peak mark was 0.029022 ETH at block 25144380, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled pre_entry block 25144352 pending 0xc4ac45...59f201; mempool lp_approval in_position block 25144354 pending 0x99afcb...1693aa.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144353->25144354 | 108386 | 1.891799 gwei | 2.100000 gwei | 12 | 0.000216772000 ETH |
| sell | 25144383->25144384 | 148898 | 2.000000 gwei | 5.000000 gwei | 24 | 0.000297796000 ETH |

## 76. `trd_mpfm8o8v_9klg_6b`

- token: `0xCc05c8962B85E9600373093dcCe4fB0e46E3e089`
- pool: `0xcc05c8962b85e9600373093dcce4fb0e46e3e089:0x0172d4fe937ae59074767b49a45b0d9d7f3e0d05`
- state: `sell_confirmed`; blocks: `25144372` -> `25144410`
- PnL: `0.030302` ETH; ROI: `303.02%`; current value: `0.000000` ETH
- peak/trough: max `0.030302` ETH at block `25144410`, min `-0.000267` ETH at block `25144372`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25144409`
- after priority: top25 `0.029765` ETH, top10 `0.029671` ETH, fixed 2 gwei `0.029765` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25144370 pending 0xa73379...fa7154; mempool lp_approval in_position block 25144372 pending 0x97ff0e...21968a; mempool liquidity_removal post_exit block 25144512 pending 0xdccfd8...1f9fd0.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144371->25144372 | 109217 | 2.000000 gwei | 2.500000 gwei | 13 | 0.000218434000 ETH |
| sell | 25144409->25144410 | 158924 | 2.000000 gwei | 2.250000 gwei | 18 | 0.000317848000 ETH |

## 77. `trd_mpfmkim7_9klg_6e`

- token: `0xf8d1a0731435Ce84B8fdf57d332819790b82F45a`
- pool: `0xf8d1a0731435ce84b8fdf57d332819790b82f45a:0x405536998daac1a0987648fda54a133114c48e70`
- state: `sell_confirmed`; blocks: `25144416` -> `25144418`
- PnL: `0.000561` ETH; ROI: `5.61%`; current value: `0.000000` ETH
- peak/trough: max `0.000561` ETH at block `25144418`, min `-0.000284` ETH at block `25144417`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25144417`; pending tx: `0x5cc738...c2435a`
- after priority: top25 `0.000042` ETH, top10 `0.000010` ETH, fixed 2 gwei `0.000042` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25144414 pending 0x29b957...aa50a9; mempool lp_approval in_position block 25144417 pending 0x5cc738...c2435a; mempool liquidity_removal post_exit block 25144603 pending 0xda84bc...cfe06d.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144415->25144416 | 109217 | 2.000000 gwei | 2.123000 gwei | 16 | 0.000218434000 ETH |
| sell | 25144417->25144418 | 150635 | 2.000000 gwei | 2.119000 gwei | 18 | 0.000301270000 ETH |

## 78. `trd_mpfmqobi_9klg_6h`

- token: `0x48742c8450e21e566Fc4d06ec350D550De963ac6`
- pool: `0x48742c8450e21e566fc4d06ec350d550de963ac6:0x28ceb9e468aedbc792ba8534a211d7bdced04c5e`
- state: `sell_confirmed`; blocks: `25144441` -> `25144469`
- PnL: `-0.001842` ETH; ROI: `-18.42%`; current value: `0.000000` ETH
- peak/trough: max `0.006460` ETH at block `25144463`, min `-0.010051` ETH at block `25144445`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25144468`
- after priority: top25 `-0.002662` ETH, top10 `-0.003008` ETH, fixed 2 gwei `-0.002661` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Peak mark was 0.006460 ETH at block 25144463, so a trailing or take-profit exit would likely have improved this position. The position had a deep adverse mark and then recovered; a hard stop should be paired with scam-specific evidence, not only mark-to-market drawdown.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144440->25144441 | 138283 | 2.000000 gwei | 3.000000 gwei | 22 | 0.000276566000 ETH |
| sell | 25144468->25144469 | 271324 | 2.002826 gwei | 2.768602 gwei | 26 | 0.000542648000 ETH |

## 79. `trd_mpfmvb2j_9klg_6k`

- token: `0xB433c1C5f3A7E8BF538A08370F28df7ac759aBEd`
- pool: `0xb433c1c5f3a7e8bf538a08370f28df7ac759abed:0x751e3523ef01ec55b02173e8aec4166cf275f07d`
- state: `buy_confirmed`; blocks: `25144457` -> `open`
- PnL: `-0.010081` ETH; ROI: `-100.81%`; current value: `0.000000` ETH
- peak/trough: max `0.031109` ETH at block `25144467`, min `-0.010081` ETH at block `25144488`
- sell reason: `-`
- after priority: top25 `-0.010370` ETH, top10 `-0.010514` ETH, fixed 2 gwei `-0.010370` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. Peak mark was 0.031109 ETH at block 25144467, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled in_position block 25144459 pending 0x7277ed...41afb1.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144456->25144457 | 144573 | 2.000000 gwei | 3.000000 gwei | 23 | 0.000289146000 ETH |

## 80. `trd_mpfnkt0q_9klg_6n`

- token: `0x9239DAa217CaAba8Ed599FF8406A20820154a017`
- pool: `0x9239daa217caaba8ed599ff8406a20820154a017:0xc366491a457703a987561b245544505f6e77bcd9`
- state: `sell_confirmed`; blocks: `25144556` -> `25144559`
- PnL: `0.000278` ETH; ROI: `2.78%`; current value: `0.000000` ETH
- peak/trough: max `0.000322` ETH at block `25144558`, min `0.000278` ETH at block `25144559`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25144558`; pending tx: `0x59bdff...304f6c`
- after priority: top25 `-0.000258` ETH, top10 `-0.000375` ETH, fixed 2 gwei `-0.000258` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25144554 pending 0xc4e998...929f08; mempool lp_approval in_position block 25144558 pending 0x59bdff...304f6c; mempool liquidity_removal post_exit block 25144642 pending 0xc0f3a9...068b1d.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144555->25144556 | 109287 | 2.000000 gwei | 2.707911 gwei | 14 | 0.000218574000 ETH |
| sell | 25144558->25144559 | 158906 | 2.000000 gwei | 2.250000 gwei | 17 | 0.000317812000 ETH |

## 81. `trd_mpfnn3x0_9klg_6q`

- token: `0x1ae560e95d0c8ec6B338DeAD8f44A3BAeE48d4e9`
- pool: `0x1ae560e95d0c8ec6b338dead8f44a3baee48d4e9:0xa8e56206a0ad40997b23bd678b5d68a7d6f7aa4c`
- state: `buy_confirmed`; blocks: `25144567` -> `open`
- PnL: `-0.000190` ETH; ROI: `-1.90%`; current value: `0.009837` ETH
- peak/trough: max `-0.000190` ETH at block `25144567`, min `-0.000190` ETH at block `25144567`
- sell reason: `-`
- after priority: top25 `-0.000411` ETH, top10 `-0.000439` ETH, fixed 2 gwei `-0.000411` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits. Risk evidence observed: mempool token_supply_risk pre_entry block 25143862 pending 0x7a70c2...123bd7; mempool token_supply_risk pre_entry block 25144010 pending 0xd150ef...9ff2d4; mempool trading_enabled pre_entry block 25144565 pending 0x8176f2...cf64a3.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144566->25144567 | 110713 | 2.000000 gwei | 2.250000 gwei | 14 | 0.000221426000 ETH |

## 82. `trd_mpfnwd1w_9klg_6t`

- token: `0xf9180452ae7Bd986632B976A3e0C6b141364b80F`
- pool: `0xf9180452ae7bd986632b976a3e0c6b141364b80f:0x7133b52c9b3999ec5e366e412d64c3413b03a0d0`
- state: `buy_confirmed`; blocks: `25144602` -> `open`
- PnL: `-0.010023` ETH; ROI: `-100.23%`; current value: `0.000001` ETH
- peak/trough: max `0.154284` ETH at block `25144621`, min `-0.010023` ETH at block `25144634`
- sell reason: `-`
- after priority: top25 `-0.010305` ETH, top10 `-0.010376` ETH, fixed 2 gwei `-0.010305` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. Peak mark was 0.154284 ETH at block 25144621, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled in_position block 25144603 pending 0x3f2a78...687020.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144601->25144602 | 140636 | 2.000000 gwei | 2.510000 gwei | 12 | 0.000281272000 ETH |

## 83. `trd_mpfo1qjh_9klg_6w`

- token: `0x63110d641dFE388236a5fF4B0FF76b70A5e2F348`
- pool: `0x63110d641dfe388236a5ff4b0ff76b70a5e2f348:0x12f9d21235719686dba6826b6581bd244c6873f7`
- state: `sell_confirmed`; blocks: `25144624` -> `25144631`
- PnL: `-0.009975` ETH; ROI: `-99.75%`; current value: `0.000000` ETH
- peak/trough: max `0.004951` ETH at block `25144628`, min `-0.009975` ETH at block `25144631`
- sell reason: `Exit: mempool liquidity removal signal`; trigger: `mempool liquidity_removal`; source: `mempool_signal`; signal block: `25144630`; pending tx: `0xfe0716...29733e`
- after priority: top25 `-0.010494` ETH, top10 `-0.010494` ETH, fixed 2 gwei `-0.010494` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Peak mark was 0.004951 ETH at block 25144628, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled pre_entry block 25144622 pending 0x5c57e4...6c837e; mempool lp_approval in_position block 25144624 pending 0xe320c0...88a471; mempool liquidity_removal in_position block 25144630 pending 0xfe0716...29733e.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144623->25144624 | 109287 | 2.000000 gwei | 2.000000 gwei | 8 | 0.000218574000 ETH |
| sell | 25144630->25144631 | 150595 | 2.000000 gwei | 2.000000 gwei | 6 | 0.000301190000 ETH |

## 84. `trd_mpfo7xym_9klg_6z`

- token: `0xc63d3a33C49439897DD4EB8c2C5fc62E3208FB7C`
- pool: `0xc63d3a33c49439897dd4eb8c2c5fc62e3208fb7c:0x0f20b903b35a849d92bf2dc493a623b34915bfce`
- state: `sell_confirmed`; blocks: `25144648` -> `25144796`
- PnL: `0.036990` ETH; ROI: `369.90%`; current value: `0.000000` ETH
- peak/trough: max `0.036990` ETH at block `25144796`, min `-0.000268` ETH at block `25144648`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25144795`
- after priority: top25 `0.036470` ETH, top10 `0.036443` ETH, fixed 2 gwei `0.036470` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25144646 pending 0xbc086d...4fa41d; mempool lp_approval in_position block 25144648 pending 0x69a008...9b9dc3; mempool liquidity_removal post_exit block 25144834 pending 0xe7a373...86b25e.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144647->25144648 | 109217 | 2.000000 gwei | 2.000000 gwei | 8 | 0.000218434000 ETH |
| sell | 25144795->25144796 | 150635 | 2.000000 gwei | 2.184000 gwei | 20 | 0.000301270000 ETH |

## 85. `trd_mpfo9g91_9klg_72`

- token: `0x999afbdD2D8b7F47073Ddb4C7d5373f133Bba750`
- pool: `0x999afbdd2d8b7f47073ddb4c7d5373f133bba750:0xc0ac056f2fea32b3a161db940571f33f5d74c9ec`
- state: `sell_confirmed`; blocks: `25144654` -> `25144803`
- PnL: `0.020517` ETH; ROI: `205.17%`; current value: `0.000000` ETH
- peak/trough: max `0.020517` ETH at block `25144803`, min `-0.000269` ETH at block `25144654`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25144802`
- after priority: top25 `0.019981` ETH, top10 `0.019921` ETH, fixed 2 gwei `0.019981` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25144652 pending 0x855dbb...d710a9; mempool lp_approval in_position block 25144654 pending 0x2397a7...b18790; mempool liquidity_removal post_exit block 25144831 pending 0xb26cad...476de3.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144653->25144654 | 109287 | 2.000000 gwei | 2.100000 gwei | 12 | 0.000218574000 ETH |
| sell | 25144802->25144803 | 158906 | 2.000000 gwei | 2.303858 gwei | 21 | 0.000317812000 ETH |

## 86. `trd_mpfpizdo_9klg_75`

- token: `0x03fE6D0eF0CB447E3b599D50E99C8abaAB1D307e`
- pool: `0x03fe6d0ef0cb447e3b599d50e99c8abaab1d307e:0x53ebad984f6bacce7d1e924e345df9806fb3a82f`
- state: `buy_confirmed`; blocks: `25144829` -> `open`
- PnL: `-0.003831` ETH; ROI: `-38.31%`; current value: `0.006211` ETH
- peak/trough: max `-0.003429` ETH at block `25144831`, min `-0.003831` ETH at block `25145334`
- sell reason: `-`
- after priority: top25 `-0.004230` ETH, top10 `-0.004336` ETH, fixed 2 gwei `-0.004230` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144828->25144829 | 199545 | 2.000000 gwei | 2.535653 gwei | 18 | 0.000399090000 ETH |

## 87. `trd_mpfplasi_9klg_78`

- token: `0x480A7601379e306aDd546c16E5578d021EeB0d13`
- pool: `0x480a7601379e306add546c16e5578d021eeb0d13:0xb7790a5e55434bb243d8a50cf6ff296b6f3fbf73`
- state: `sell_confirmed`; blocks: `25144840` -> `25144873`
- PnL: `0.031832` ETH; ROI: `318.32%`; current value: `0.000000` ETH
- peak/trough: max `0.031832` ETH at block `25144873`, min `-0.000275` ETH at block `25144840`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25144872`
- after priority: top25 `0.031295` ETH, top10 `0.031142` ETH, fixed 2 gwei `0.031295` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25144838 pending 0xc142f8...dfdc7d; mempool lp_approval in_position block 25144840 pending 0x6f45f4...b604cc; mempool liquidity_removal post_exit block 25144912 pending 0x3fce03...30283f.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144839->25144840 | 109287 | 2.000000 gwei | 2.172967 gwei | 15 | 0.000218574000 ETH |
| sell | 25144872->25144873 | 158906 | 2.000000 gwei | 2.846240 gwei | 16 | 0.000317812000 ETH |

## 88. `trd_mpfq002t_9klg_7b`

- token: `0xe192Eddd1D9bD098cdeB6FEC3b8ca0De056b1034`
- pool: `0xe192eddd1d9bd098cdeb6fec3b8ca0de056b1034:0xc8ad1d6c55af41797c73f896ddc3b0b583a9229c`
- state: `sell_confirmed`; blocks: `25144896` -> `25144898`
- PnL: `-0.000289` ETH; ROI: `-2.89%`; current value: `0.000000` ETH
- peak/trough: max `-0.000268` ETH at block `25144897`, min `-0.000289` ETH at block `25144898`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25144897`; pending tx: `0x1f9e83...6c8db7`
- after priority: top25 `-0.000825` ETH, top10 `-0.000852` ETH, fixed 2 gwei `-0.000825` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Risk evidence observed: mempool trading_enabled pre_entry block 25144894 pending 0xba32e1...ca3de3; mempool lp_approval in_position block 25144897 pending 0x1f9e83...6c8db7; mempool liquidity_removal post_exit block 25144981 pending 0x118cd4...e3fd77.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144895->25144896 | 109217 | 2.000000 gwei | 2.100000 gwei | 11 | 0.000218434000 ETH |
| sell | 25144897->25144898 | 158968 | 2.000000 gwei | 2.100000 gwei | 13 | 0.000317936000 ETH |

## 89. `trd_mpfq64oy_9klg_7e`

- token: `0x49a1aceF7dB0E759c69b4eE515F95300C5a7a671`
- pool: `0x49a1acef7db0e759c69b4ee515f95300c5a7a671:0x5504748b01e0637bf475e5daec0bab5799354dca`
- state: `sell_confirmed`; blocks: `25144921` -> `25144961`
- PnL: `0.017139` ETH; ROI: `171.39%`; current value: `0.000000` ETH
- peak/trough: max `0.017139` ETH at block `25144961`, min `-0.000270` ETH at block `25144921`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25144960`
- after priority: top25 `0.016533` ETH, top10 `0.016033` ETH, fixed 2 gwei `0.016603` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25144919 pending 0x6dc16b...66a413; mempool lp_approval in_position block 25144921 pending 0x386e5a...41f2c8; mempool liquidity_removal post_exit block 25145011 pending 0x6a1532...abee3b.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144920->25144921 | 109275 | 2.000000 gwei | 2.854648 gwei | 16 | 0.000218550000 ETH |
| sell | 25144960->25144961 | 158896 | 2.438671 gwei | 5.000000 gwei | 31 | 0.000317792000 ETH |

## 90. `trd_mpfq64qs_9klg_7h`

- token: `0x67B3061F2ee0443dB5E72262adbedD70E49fF653`
- pool: `0x67b3061f2ee0443db5e72262adbedd70e49ff653:0x25366dbec20afccd5e604a0c2e20917b24f3dec9`
- state: `buy_confirmed`; blocks: `25144921` -> `open`
- PnL: `-0.005029` ETH; ROI: `-50.29%`; current value: `0.004992` ETH
- peak/trough: max `-0.000357` ETH at block `25144921`, min `-0.005029` ETH at block `25144923`
- sell reason: `-`
- after priority: top25 `-0.005325` ETH, top10 `-0.005452` ETH, fixed 2 gwei `-0.005325` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits. The least-bad mark was still negative and occurred earlier; a time-stop or stricter continuation filter would have reduced the current drawdown.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144920->25144921 | 147907 | 2.000000 gwei | 2.854648 gwei | 16 | 0.000295814000 ETH |

## 91. `trd_mpfqd2lk_9klg_7k`

- token: `0xE5C7B9e4d20032D5D7d137C2e5d7633FE7268aC8`
- pool: `0xe5c7b9e4d20032d5d7d137c2e5d7633fe7268ac8:0xbfaced0714af69f2cd097c4844cdb121066b7087`
- state: `sell_confirmed`; blocks: `25144946` -> `25144973`
- PnL: `0.107690` ETH; ROI: `1076.90%`; current value: `0.000000` ETH
- peak/trough: max `0.137070` ETH at block `25144970`, min `-0.010022` ETH at block `25144954`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25144972`
- after priority: top25 `0.107034` ETH, top10 `0.106706` ETH, fixed 2 gwei `0.107034` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Peak mark was 0.137070 ETH at block 25144970, so a trailing or take-profit exit would likely have improved this position. The position had a deep adverse mark and then recovered; a hard stop should be paired with scam-specific evidence, not only mark-to-market drawdown.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144945->25144946 | 147735 | 2.000000 gwei | 3.000000 gwei | 20 | 0.000295470000 ETH |
| sell | 25144972->25144973 | 180217 | 2.000000 gwei | 3.000000 gwei | 25 | 0.000360434000 ETH |

## 92. `trd_mpfqn411_9klg_7n`

- token: `0xFb26F0B8D3775Aee1b656d03e99e5D000079a6dB`
- pool: `0xfb26f0b8d3775aee1b656d03e99e5d000079a6db:0x04706f51a8c4cdc4110314d935a16a019232ed12`
- state: `sell_failed`; blocks: `25144985` -> `open`
- PnL: `-0.010044` ETH; ROI: `-100.44%`; current value: `0.000000` ETH
- peak/trough: max `0.674384` ETH at block `25145056`, min `-0.010044` ETH at block `25145005`
- sell reason: `Exit: tax risk`; source: `mempool_signal`; signal block: `25145002`
- after priority: top25 `-0.010622` ETH, top10 `-0.010825` ETH, fixed 2 gwei `-0.010595` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. Peak mark was 0.674384 ETH at block 25145056, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled in_position block 25144990 pending 0x4c2c35...81d68c; mempool honeypot in_position block 25145002 pending 0xafec73...1d1f05.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144984->25144985 | 144488 | 2.188000 gwei | 3.499081 gwei | 37 | 0.000288976000 ETH |
| sell | 25145002->25145003 | 130827 | 2.000000 gwei | 2.100000 gwei | 15 | 0.000261654000 ETH |

## 93. `trd_mpfqona6_9klg_7q`

- token: `0x04850E7a1a6944C2D2ac632CCdc7E3918f92C52F`
- pool: `0x04850e7a1a6944c2d2ac632ccdc7e3918f92c52f:0xeb674ffd7cf603600a4f64a2ac7f7d3e7c0aa8cc`
- state: `buy_confirmed`; blocks: `25144991` -> `open`
- PnL: `-0.010023` ETH; ROI: `-100.23%`; current value: `0.000000` ETH
- peak/trough: max `0.088035` ETH at block `25145005`, min `-0.010023` ETH at block `25145377`
- sell reason: `-`
- after priority: top25 `-0.010305` ETH, top10 `-0.010445` ETH, fixed 2 gwei `-0.010305` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. Peak mark was 0.088035 ETH at block 25145005, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled in_position block 25144996 pending 0x745236...8d4f94.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144990->25144991 | 140706 | 2.000000 gwei | 3.000000 gwei | 21 | 0.000281412000 ETH |

## 94. `trd_mpfqpg86_9klg_7t`

- token: `0x2efe89B5977166CbFAF5Fb732f1261173E29E803`
- pool: `0x2efe89b5977166cbfaf5fb732f1261173e29e803:0xe2df8c0e4f2b64c9a20f50aa421cab2bdb3612d8`
- state: `buy_confirmed`; blocks: `25144994` -> `open`
- PnL: `-0.005657` ETH; ROI: `-56.57%`; current value: `0.004368` ETH
- peak/trough: max `0.000982` ETH at block `25144996`, min `-0.005657` ETH at block `25145043`
- sell reason: `-`
- after priority: top25 `-0.006145` ETH, top10 `-0.006610` ETH, fixed 2 gwei `-0.005937` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144993->25144994 | 139785 | 3.489550 gwei | 6.816218 gwei | 34 | 0.000279570000 ETH |

## 95. `trd_mpfqpgbx_9klg_7w`

- token: `0xC7c7936Ea8A3EeF178F8ef03a275608CCc1aBe73`
- pool: `0xc7c7936ea8a3eef178f8ef03a275608ccc1abe73:0x001692a251632161787b93a066c52faf4b7b57e9`
- state: `sell_confirmed`; blocks: `25144994` -> `25145054`
- PnL: `0.154638` ETH; ROI: `1546.38%`; current value: `0.000000` ETH
- peak/trough: max `0.188231` ETH at block `25145045`, min `-0.000280` ETH at block `25144994`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25145053`
- after priority: top25 `0.153761` ETH, top10 `0.153184` ETH, fixed 2 gwei `0.153971` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Peak mark was 0.188231 ETH at block 25145045, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled in_position block 25144999 pending 0xdc9a28...408361.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144993->25144994 | 140702 | 3.489550 gwei | 6.816218 gwei | 34 | 0.000281404000 ETH |
| sell | 25145053->25145054 | 192790 | 2.000000 gwei | 2.562362 gwei | 17 | 0.000385580000 ETH |

## 96. `trd_mpfqpgf8_9klg_7z`

- token: `0xfc3EeC8a7177F5aAb4fCD60aCc874484660F1d28`
- pool: `0xfc3eec8a7177f5aab4fcd60acc874484660f1d28:0xd67877ada9173d7652aff9a77d839e41df5a3a35`
- state: `sell_confirmed`; blocks: `25144994` -> `25144997`
- PnL: `0.000976` ETH; ROI: `9.76%`; current value: `0.000000` ETH
- peak/trough: max `0.000976` ETH at block `25144997`, min `0.000486` ETH at block `25144996`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25144996`; pending tx: `0x876de3...ced5bf`
- after priority: top25 `0.000138` ETH, top10 `-0.000826` ETH, fixed 2 gwei `0.000367` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25144993 pending 0x3f7fdd...7210c9; mempool lp_approval in_position block 25144996 pending 0x876de3...ced5bf; mempool liquidity_removal post_exit block 25145170 pending 0x223c7b...832f99.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25144993->25144994 | 153833 | 3.489550 gwei | 6.816218 gwei | 34 | 0.000307666000 ETH |
| sell | 25144996->25144997 | 150613 | 2.000000 gwei | 5.000000 gwei | 21 | 0.000301226000 ETH |

## 97. `trd_mpfqxwha_9klg_82`

- token: `0x3926aF1f253FD99fFb99Fa89B520302fC284e117`
- pool: `0x3926af1f253fd99ffb99fa89b520302fc284e117:0xf2efd9f021cb87fb0359c5d7889088dd8c68429e`
- state: `sell_confirmed`; blocks: `25145028` -> `25145030`
- PnL: `-0.000289` ETH; ROI: `-2.89%`; current value: `0.000000` ETH
- peak/trough: max `-0.000269` ETH at block `25145029`, min `-0.000289` ETH at block `25145030`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25145029`; pending tx: `0x9aaddb...cf7409`
- after priority: top25 `-0.000825` ETH, top10 `-0.000934` ETH, fixed 2 gwei `-0.000825` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Risk evidence observed: mempool trading_enabled pre_entry block 25145026 pending 0x787968...44a0fb; mempool lp_approval in_position block 25145029 pending 0x9aaddb...cf7409; mempool liquidity_removal post_exit block 25145215 pending 0x0067b1...356e5d.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25145027->25145028 | 109287 | 2.000000 gwei | 3.000000 gwei | 16 | 0.000218574000 ETH |
| sell | 25145029->25145030 | 158906 | 2.000000 gwei | 2.000000 gwei | 10 | 0.000317812000 ETH |

## 98. `trd_mpfr9hn1_9klg_85`

- token: `0x70cDE595444E1b6ADD10a4461935e51cf432336b`
- pool: `0x70cde595444e1b6add10a4461935e51cf432336b:0x366bc09e121c0b388f2c21aed9e0be316a271c10`
- state: `sell_confirmed`; blocks: `25145074` -> `25145118`
- PnL: `0.029207` ETH; ROI: `292.07%`; current value: `0.000000` ETH
- peak/trough: max `0.033782` ETH at block `25145109`, min `-0.000361` ETH at block `25145074`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25145117`
- after priority: top25 `0.028409` ETH, top10 `0.028154` ETH, fixed 2 gwei `0.028409` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Peak mark was 0.033782 ETH at block 25145109, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled in_position block 25145076 pending 0x6fbcda...54bc75.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25145073->25145074 | 192384 | 2.000000 gwei | 2.250000 gwei | 12 | 0.000384768000 ETH |
| sell | 25145117->25145118 | 206623 | 2.000000 gwei | 3.000000 gwei | 23 | 0.000413246000 ETH |

## 99. `trd_mpfs2kg0_9klg_88`

- token: `0x4AE88093F3e1b6Bdc7F896d1A696aab4a5F7b78b`
- pool: `0x4ae88093f3e1b6bdc7f896d1a696aab4a5f7b78b:0x19ccac99bb26f90d235ad32a87b69435f631f849`
- state: `sell_confirmed`; blocks: `25145184` -> `25145186`
- PnL: `0.000599` ETH; ROI: `5.99%`; current value: `0.000000` ETH
- peak/trough: max `0.000639` ETH at block `25145185`, min `0.000599` ETH at block `25145186`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25145185`; pending tx: `0x5eaa23...2350aa`
- after priority: top25 `0.000063` ETH, top10 `-0.000742` ETH, fixed 2 gwei `0.000063` ETH
- review: Small winner; live execution should cap priority because the residual edge after bribe is thin. Risk evidence observed: mempool trading_enabled pre_entry block 25145181 pending 0xb39fe3...b0461b; mempool lp_approval in_position block 25145185 pending 0x5eaa23...2350aa; mempool liquidity_removal post_exit block 25145257 pending 0x76caff...cd4ff3.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25145183->25145184 | 109287 | 2.000000 gwei | 5.000000 gwei | 25 | 0.000218574000 ETH |
| sell | 25145185->25145186 | 158906 | 2.000000 gwei | 5.000000 gwei | 24 | 0.000317812000 ETH |

## 100. `trd_mpfs4uz6_9klg_8b`

- token: `0x0dFA509081B50DB651c6BD011AcDBbB6ee03FC63`
- pool: `0x0dfa509081b50db651c6bd011acdbbb6ee03fc63:0x6bbf3c4eae536e63fb310d6fc3126d13648243fb`
- state: `sell_confirmed`; blocks: `25145194` -> `25145239`
- PnL: `0.005532` ETH; ROI: `55.32%`; current value: `0.000000` ETH
- peak/trough: max `0.005532` ETH at block `25145239`, min `-0.000108` ETH at block `25145194`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25145238`
- after priority: top25 `0.004974` ETH, top10 `0.004933` ETH, fixed 2 gwei `0.004974` ETH
- review: Useful winner; net edge is present, but it is small enough that aggressive priority can consume a noticeable share. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25145192 pending 0x5f00bc...287610.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25145193->25145194 | 115933 | 2.000000 gwei | 2.213940 gwei | 13 | 0.000231866000 ETH |
| sell | 25145238->25145239 | 163202 | 2.000000 gwei | 2.095294 gwei | 11 | 0.000326404000 ETH |

## 101. `trd_mpfsddmi_9klg_8e`

- token: `0xa80B608C9E99101CA7Fa02938a26b4c557a4Feee`
- pool: `0xa80b608c9e99101ca7fa02938a26b4c557a4feee:0xa5744e35de6126a497c3105a4f486031dfe0a543`
- state: `sell_confirmed`; blocks: `25145227` -> `25145261`
- PnL: `0.025591` ETH; ROI: `255.91%`; current value: `0.000000` ETH
- peak/trough: max `0.025591` ETH at block `25145261`, min `-0.000282` ETH at block `25145227`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25145260`
- after priority: top25 `0.025011` ETH, top10 `0.024769` ETH, fixed 2 gwei `0.024966` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25145221 pending 0x1be8d9...0b09ff; mempool lp_approval in_position block 25145227 pending 0x180983...eda9b2; mempool liquidity_removal post_exit block 25145374 pending 0xd21063...49f11c.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25145226->25145227 | 153903 | 2.000000 gwei | 3.277349 gwei | 22 | 0.000307806000 ETH |
| sell | 25145260->25145261 | 158906 | 1.716568 gwei | 2.000000 gwei | 8 | 0.000317812000 ETH |

## 102. `trd_mpfsoy4u_9klg_8h`

- token: `0x1B8c0D9D952C90Ee4A688e0d6DbDb95D8D901403`
- pool: `0x1b8c0d9d952c90ee4a688e0d6dbdb95d8d901403:0x9200f0d5fa37e856f688dfa4d6d87b9a585deb07`
- state: `sell_confirmed`; blocks: `25145270` -> `25145273`
- PnL: `-0.000362` ETH; ROI: `-3.62%`; current value: `0.000000` ETH
- peak/trough: max `-0.000304` ETH at block `25145272`, min `-0.000362` ETH at block `25145273`
- sell reason: `Exit: LP approval`; trigger: `mempool lp_approval`; source: `mempool_signal`; signal block: `25145272`; pending tx: `0x5545c8...b528e3`
- after priority: top25 `-0.001124` ETH, top10 `-0.001168` ETH, fixed 2 gwei `-0.001047` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Risk evidence observed: mempool trading_enabled pre_entry block 25145267 pending 0x870016...44a501; mempool lp_approval in_position block 25145272 pending 0x5545c8...b528e3.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25145269->25145270 | 153903 | 2.500000 gwei | 2.500000 gwei | 391 | 0.000307806000 ETH |
| sell | 25145272->25145273 | 188803 | 2.000000 gwei | 2.234000 gwei | 22 | 0.000377606000 ETH |

## 103. `trd_mpftasem_9klg_8k`

- token: `0xAcAF83243C269948F6aAFD14B999e0C5f459D21f`
- pool: `0xacaf83243c269948f6aafd14b999e0c5f459d21f:0xae236530638012214b07721f993384598bfeb555`
- state: `buy_confirmed`; blocks: `25145355` -> `open`
- PnL: `-0.010063` ETH; ROI: `-100.63%`; current value: `0.000000` ETH
- peak/trough: max `0.054430` ETH at block `25145372`, min `-0.010063` ETH at block `25145377`
- sell reason: `-`
- after priority: top25 `-0.010344` ETH, top10 `-0.010359` ETH, fixed 2 gwei `-0.010344` ETH
- review: Major loss; this needs an avoid/fast-exit rule rather than just gas tuning. Still open near full loss; live version needs an emergency exit/scam guard and should not assume max-hold will recover value. Peak mark was 0.054430 ETH at block 25145372, so a trailing or take-profit exit would likely have improved this position. Risk evidence observed: mempool trading_enabled in_position block 25145356 pending 0x92201c...f10d86.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25145354->25145355 | 140772 | 2.000000 gwei | 2.100000 gwei | 13 | 0.000281544000 ETH |

## 104. `trd_mpftbk5v_9klg_8n`

- token: `0x10ebD8BEcEbA5A58BA65EA602f4A17b65236d757`
- pool: `0x10ebd8beceba5a58ba65ea602f4a17b65236d757:0xfc86dfef88ac19e505f3be97bcd983b895bc06d6`
- state: `buy_confirmed`; blocks: `25145358` -> `open`
- PnL: `-0.000515` ETH; ROI: `-5.15%`; current value: `0.009559` ETH
- peak/trough: max `-0.000515` ETH at block `25145359`, min `-0.000515` ETH at block `25145359`
- sell reason: `-`
- after priority: top25 `-0.000864` ETH, top10 `-0.001039` ETH, fixed 2 gwei `-0.000864` ETH
- review: Small loss; most of these need either a higher entry-quality threshold or cheaper inclusion. Still open, so final quality is not known; deployment review should discount this mark until it exits.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25145357->25145358 | 174753 | 2.000000 gwei | 3.000000 gwei | 23 | 0.000349506000 ETH |

## 105. `trd_mpftjat4_9klg_8q`

- token: `0xfce57D6E59BdD3566547Af2F34eD54EB7Fd44CF1`
- pool: `0xfce57d6e59bdd3566547af2f34ed54eb7fd44cf1:0x637f0483992534999160eaaadab881d3380f6934`
- state: `sell_confirmed`; blocks: `25145389` -> `25145427`
- PnL: `0.030609` ETH; ROI: `306.09%`; current value: `0.000000` ETH
- peak/trough: max `0.030609` ETH at block `25145427`, min `-0.000283` ETH at block `25145389`
- sell reason: `Exit: max hold`; source: `market`; signal block: `25145426`
- after priority: top25 `0.030153` ETH, top10 `0.029924` ETH, fixed 2 gwei `0.030000` ETH
- review: Large winner; the entry signal was strong and the main live question is preserving inclusion without overpaying priority. Max-hold exit was acceptable here; there is no obvious need to exit earlier from the persisted marks alone. Risk evidence observed: mempool trading_enabled pre_entry block 25145386 pending 0xdd5c77...a99fc1; mempool lp_approval in_position block 25145389 pending 0xdd4780...717233.

| side | submit -> confirm | gas | top25 tip | top10 tip | rank at 2 gwei | 2 gwei spend |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| buy | 25145388->25145389 | 153833 | 1.000000 gwei | 2.000000 gwei | 6 | 0.000307666000 ETH |
| sell | 25145426->25145427 | 150592 | 2.000000 gwei | 2.500000 gwei | 13 | 0.000301184000 ETH |
