# Pre-Iteration 02 Signal Review

Status: investigation complete; do not train iteration 02 until these findings are folded into the feature plan.

Objective: find signals that could warn about an upcoming scam before the scam/removal event, using only evidence visible at or before each observation block.

## Case Set

| Case                              | Category                          | Selected block | 4-block score | Scam block | Mechanism                   | Mempool rows |
| --------------------------------- | --------------------------------- | -------------- | ------------- | ---------- | --------------------------- | ------------ |
| uliq_holdout_backdoor_miss        | holdout_false_negative            | 25144059       | 1.14%         | 25144063   | pair_balance_backdoor_drain | 0            |
| direct_lp_true_positive           | true_positive                     | 25130547       | 96.33%        | 25130549   | direct_lp_liquidity_removal | 0            |
| backdoor_true_positive            | true_positive_backdoor            | 25127850       | 90.92%        | 25127852   | direct_lp_liquidity_removal | 0            |
| backdoor_false_negative           | false_negative_backdoor           | 25124408       | 0.43%         | 25124411   | pair_balance_backdoor_drain | 0            |
| near_horizon_false_positive       | near_horizon_false_positive       | 25125020       | 80.12%        | 25125031   | direct_lp_liquidity_removal | 0            |
| unlabeled_backdoor_false_positive | unlabeled_false_positive_backdoor | 25130111       | 58.91%        | -          | -                           | 0            |

## Cross-Case Findings

- Mempool coverage is not useful in this sample yet: `0` selected evidence rows had `mempool_first_seen_ms`.
- `4` of `6` selected cases reached `lp_removable_pct_as_of >= 90`; this is the common direct-removal signal the current model already understands.
- `4` of `6` selected cases showed token-control/backdoor flags before or at the selected block; several were still scored low, so this needs a separate branch instead of relying on the generic model.
- Network snapshots surfaced creator+owner labelled control wallets for `backdoor_false_negative`, `uliq_holdout_backdoor_miss`; those should become investigation features once we can make them as-of safe.
- The weakest positive case in this review is `backdoor_false_negative` with max positive 4-block score `0.43%`; that is the main miss to explain before iteration 02.
- Next iteration should model two warning families separately: direct LP-removal readiness and token-control/pair-balance backdoor behavior.

## Cases

### ULIQ holdout backdoor miss

- Case: `uliq_holdout_backdoor_miss`
- Token: `0x0ca72b24abf950be8bb145f5c68f7004485192b2`
- Pool: `0x57ca70b663ead85122589f345835d8f8209608dc`
- Why selected: Known fast ULIQ token-control backdoor; included to see what was visible before the scam.
- Scam mechanism: `pair_balance_backdoor_drain`
- Selected row: block `25144059`, score `1.14%`, time-to-scam `4` chain blocks.

Model score summary:

| Target                           | Rows | Positive rows | Max score | Max score block | First positive block |
| -------------------------------- | ---- | ------------- | --------- | --------------- | -------------------- |
| scam_within_10_chain_block_delta | 8    | 4             | 25.05%    | 25144048        | 25144053             |
| scam_within_1_chain_block_delta  | 8    | 0             | 8.88%     | 25144059        | -                    |
| scam_within_2_chain_block_delta  | 8    | 0             | 35.15%    | 25144048        | -                    |
| scam_within_3_chain_block_delta  | 8    | 0             | 53.43%    | 25144047        | -                    |
| scam_within_4_chain_block_delta  | 8    | 1             | 2.16%     | 25144047        | 25144059             |

Event sequence:

| Block    | Kind                 | Mechanism                   | Tx                  | Mempool first seen |
| -------- | -------------------- | --------------------------- | ------------------- | ------------------ |
| 25144047 | lp_approval          | -                           | 0x36850a34...d2248f | -                  |
| 25144057 | token_control_signal | -                           | 0xc28c751c...e52b93 | -                  |
| 25144063 | scam                 | pair_balance_backdoor_drain | 0x07ee7718...01c1b4 | -                  |
| 25144063 | token_control_signal | -                           | 0x22e921be...fafe75 | -                  |

Signal blocks:

| Signal                                 | First block |
| -------------------------------------- | ----------- |
| LP approval event                      | 25144047    |
| LP removable >= 90%                    | -           |
| Creator LP balance >= 90%              | 25144047    |
| Token-control event                    | 25144057    |
| Pair-balance backdoor flag             | 25144057    |
| Holder-to-burn transferFrom flag       | 25144057    |
| transferFrom without Transfer log flag | 25144057    |
| Pair-token-to-control flag             | -           |

Observation timeline:

| Block    | dt scam | tx | LP rem % | creator LP % | direct rm | backdoor | holder burn | no log | pair->control | price/init |
| -------- | ------- | -- | -------- | ------------ | --------- | -------- | ----------- | ------ | ------------- | ---------- |
| 25144047 | 16      | 3  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1          |
| 25144048 | 15      | 1  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1          |
| 25144051 | 12      | 4  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 2.331      |
| 25144052 | 11      | 5  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 4.227      |
| 25144053 | 10      | 3  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 2.74       |
| 25144054 | 9       | 5  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 4.553      |
| 25144057 | 6       | 11 | 0.00%    | 100.00%      | -         | Y        | Y           | Y      | -             | 17.3       |
| 25144059 | 4       | 5  | 0.00%    | 100.00%      | -         | Y        | Y           | Y      | -             | 22.39      |

Network snapshot:

- Status: `complete`
- Token: `ULIQ` / `Uniliquid`
- Creator address from metadata: `-`
- Graph: `81` nodes, `273` edges, `43` addresses.
- Labelled network nodes: `0x51e70799c78e6ccdd473a97a97ddfce577e5a643` (wallet,creator,owner); `0x011306901bce24af026c4c685f08ac241a98cd8f` (wallet); `0x078bce331c4e7cae5e06385b383a45b3bc362904` (wallet); `0x423be6a2330c217b46748ab74cd339fdd85c05dd` (wallet); `0x6603ca7f491d930e454ccfef9b71bd0e52af6f9a` (wallet).
- Top context edges: `0x3328f7f4...309c49->0x57ca70b6...9608dc` direct_denom_flow w=16; `0x7a250d56...f2488d->0x57ca70b6...9608dc` direct_denom_flow w=16; `0x01130690...98cd8f->0x3328f7f4...309c49` direct_denom_flow w=2; `0x01130690...98cd8f->0x7e7565b2...9ce473` direct_denom_flow w=2; `0x078bce33...362904->0x3328f7f4...309c49` direct_denom_flow w=2.

Feature lesson:

- Backdoor timing should be explicit: first backdoor flag was `25144057`, `6` chain blocks before scam.
- Add `blocks_since_first_backdoor_signal`: it was `2` at the selected row.
- The holder-to-burn `transferFrom` and pair-balance backdoor flags co-arrived; add an interaction feature instead of leaving them as independent booleans.

### Direct LP true positive

- Case: `direct_lp_true_positive`
- Token: `0xc56097ceba350e0dc29daffa1f7c806adfb0103d`
- Pool: `0x360561e9f567845cb8941a517213847611cc6816`
- Why selected: Highest 4-block true positive without a pair-balance backdoor flag.
- Scam mechanism: `direct_lp_liquidity_removal`
- Selected row: block `25130547`, score `96.33%`, time-to-scam `2` chain blocks.

Model score summary:

| Target                           | Rows | Positive rows | Max score | Max score block | First positive block |
| -------------------------------- | ---- | ------------- | --------- | --------------- | -------------------- |
| scam_within_10_chain_block_delta | 9    | 1             | 96.14%    | 25130547        | 25130547             |
| scam_within_1_chain_block_delta  | 9    | 0             | 91.23%    | 25130547        | -                    |
| scam_within_2_chain_block_delta  | 9    | 1             | 92.62%    | 25130547        | 25130547             |
| scam_within_3_chain_block_delta  | 9    | 1             | 94.61%    | 25130547        | 25130547             |
| scam_within_4_chain_block_delta  | 9    | 1             | 96.33%    | 25130547        | 25130547             |

Event sequence:

| Block    | Kind        | Mechanism                   | Tx                  | Mempool first seen |
| -------- | ----------- | --------------------------- | ------------------- | ------------------ |
| 25130547 | lp_approval | -                           | 0x7d466153...fae907 | -                  |
| 25130549 | scam        | direct_lp_liquidity_removal | 0xc354f8c9...d220a3 | -                  |

Signal blocks:

| Signal                                 | First block |
| -------------------------------------- | ----------- |
| LP approval event                      | 25130547    |
| LP removable >= 90%                    | 25130547    |
| Creator LP balance >= 90%              | 25126542    |
| Token-control event                    | -           |
| Pair-balance backdoor flag             | -           |
| Holder-to-burn transferFrom flag       | -           |
| transferFrom without Transfer log flag | -           |
| Pair-token-to-control flag             | -           |

Observation timeline:

| Block    | dt scam | tx | LP rem % | creator LP % | direct rm | backdoor | holder burn | no log | pair->control | price/init |
| -------- | ------- | -- | -------- | ------------ | --------- | -------- | ----------- | ------ | ------------- | ---------- |
| 25126531 | 4018    | 1  | 0.00%    | -            | -         | -        | -           | -      | -             | -          |
| 25130547 | 2       | 1  | 100.00%  | 100.00%      | -         | -        | -           | -      | -             | 1.054      |

Network snapshot:

- Status: `complete`
- Token: `CJ` / `Get CJ Neuralink`
- Creator address from metadata: `-`
- Graph: `10` nodes, `14` edges, `5` addresses.
- Labelled network nodes: `0xd46f2e413c24a2c840d2abb6bf57887ed9df8eb5` (wallet).
- Top context edges: `0x7a250d56...f2488d->0xc02aaa39...756cc2` direct_denom_flow w=65; `0x3328f7f4...309c49->0xc02aaa39...756cc2` direct_denom_flow w=55; `0xc02aaa39...756cc2->0x7a250d56...f2488d` direct_denom_flow w=54; `0xc02aaa39...756cc2->0xf8277254...da081f` direct_denom_flow w=46; `0x7a250d56...f2488d->0x2e0eb5f5...b80198` direct_denom_flow w=40.

Feature lesson:

- LP removability timing should be explicit: first >=90% removable block was `25130547`, `2` chain blocks before scam.

### Backdoor true positive

- Case: `backdoor_true_positive`
- Token: `0xf6c37ee76a513b3472a88c3bafe4f0441016a054`
- Pool: `0xa9a39de9602c5512e193e77c9a206d8217b2e575`
- Why selected: Highest 4-block true positive with pair-balance backdoor signal already visible.
- Scam mechanism: `direct_lp_liquidity_removal`
- Selected row: block `25127850`, score `90.92%`, time-to-scam `2` chain blocks.

Model score summary:

| Target                           | Rows | Positive rows | Max score | Max score block | First positive block |
| -------------------------------- | ---- | ------------- | --------- | --------------- | -------------------- |
| scam_within_10_chain_block_delta | 9    | 1             | 96.14%    | 25127850        | 25127850             |
| scam_within_1_chain_block_delta  | 9    | 0             | 91.14%    | 25127850        | -                    |
| scam_within_2_chain_block_delta  | 9    | 1             | 96.36%    | 25127850        | 25127850             |
| scam_within_3_chain_block_delta  | 9    | 1             | 93.17%    | 25127850        | 25127850             |
| scam_within_4_chain_block_delta  | 9    | 1             | 90.92%    | 25127850        | 25127850             |

Event sequence:

| Block    | Kind        | Mechanism                   | Tx                  | Mempool first seen |
| -------- | ----------- | --------------------------- | ------------------- | ------------------ |
| 25127850 | lp_approval | -                           | 0xd550abd6...7bb1fb | -                  |
| 25127852 | scam        | direct_lp_liquidity_removal | 0xe4f51b49...5d5ba0 | -                  |

Signal blocks:

| Signal                                 | First block |
| -------------------------------------- | ----------- |
| LP approval event                      | 25127850    |
| LP removable >= 90%                    | 25127850    |
| Creator LP balance >= 90%              | 25127055    |
| Token-control event                    | -           |
| Pair-balance backdoor flag             | 25127820    |
| Holder-to-burn transferFrom flag       | -           |
| transferFrom without Transfer log flag | -           |
| Pair-token-to-control flag             | 25127820    |

Observation timeline:

| Block    | dt scam | tx | LP rem % | creator LP % | direct rm | backdoor | holder burn | no log | pair->control | price/init |
| -------- | ------- | -- | -------- | ------------ | --------- | -------- | ----------- | ------ | ------------- | ---------- |
| 25127055 | 797     | 1  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1          |
| 25127820 | 32      | 1  | 0.00%    | 100.00%      | -         | Y        | -           | -      | Y             | 1.388      |
| 25127850 | 2       | 1  | 100.00%  | 100.00%      | -         | Y        | -           | -      | Y             | 1.388      |

Network snapshot:

- Status: `complete`
- Token: `WORLDCUP` / `WORLD CUP COIN`
- Creator address from metadata: `-`
- Graph: `11` nodes, `19` edges, `5` addresses.
- Labelled network nodes: `0xd9c9be98e47401dd3a0c3a9aec05538ab26dcea6` (wallet).
- Top context edges: `0x7a250d56...f2488d->0xc02aaa39...756cc2` direct_denom_flow w=101; `0xc02aaa39...756cc2->0x7a250d56...f2488d` direct_denom_flow w=57; `0x28b1dc1a...b2a183->0xc02aaa39...756cc2` direct_denom_flow w=49; `0xc02aaa39...756cc2->0x5703b683...9d4757` direct_denom_flow w=45; `0x7f54f056...a3be8a->0xc02aaa39...756cc2` direct_denom_flow w=40.

Feature lesson:

- Backdoor timing should be explicit: first backdoor flag was `25127820`, `32` chain blocks before scam.
- LP removability timing should be explicit: first >=90% removable block was `25127850`, `2` chain blocks before scam.
- Add `blocks_since_first_backdoor_signal`: it was `30` at the selected row.

### Backdoor false negative

- Case: `backdoor_false_negative`
- Token: `0x6e8755282d7145827f3c75de74f648423a4a73e5`
- Pool: `0xac0c5221146cdb3f99f2e81081317863b12499dd`
- Why selected: Lowest-scored 4-block positive row where pair-balance backdoor signal was already visible.
- Scam mechanism: `pair_balance_backdoor_drain`
- Selected row: block `25124408`, score `0.43%`, time-to-scam `3` chain blocks.

Model score summary:

| Target                           | Rows | Positive rows | Max score | Max score block | First positive block |
| -------------------------------- | ---- | ------------- | --------- | --------------- | -------------------- |
| scam_within_10_chain_block_delta | 13   | 7             | 8.69%     | 25124408        | 25124401             |
| scam_within_1_chain_block_delta  | 13   | 0             | 13.20%    | 25124408        | -                    |
| scam_within_2_chain_block_delta  | 13   | 0             | 35.38%    | 25124396        | -                    |
| scam_within_3_chain_block_delta  | 13   | 1             | 47.39%    | 25124400        | 25124408             |
| scam_within_4_chain_block_delta  | 13   | 1             | 1.19%     | 25124401        | 25124408             |

Event sequence:

| Block    | Kind                 | Mechanism                   | Tx                  | Mempool first seen |
| -------- | -------------------- | --------------------------- | ------------------- | ------------------ |
| 25124390 | lp_approval          | -                           | 0xa61e5450...794087 | -                  |
| 25124396 | token_control_signal | -                           | 0xdff08376...8fe097 | -                  |
| 25124403 | lp_approval          | -                           | 0x8858d2e1...cb09da | -                  |
| 25124411 | scam                 | pair_balance_backdoor_drain | 0x2d335f58...056e7c | -                  |
| 25124411 | token_control_signal | -                           | 0x6f9a7685...b1ed34 | -                  |

Signal blocks:

| Signal                                 | First block |
| -------------------------------------- | ----------- |
| LP approval event                      | 25124390    |
| LP removable >= 90%                    | -           |
| Creator LP balance >= 90%              | 25124390    |
| Token-control event                    | 25124396    |
| Pair-balance backdoor flag             | 25124396    |
| Holder-to-burn transferFrom flag       | 25124396    |
| transferFrom without Transfer log flag | 25124396    |
| Pair-token-to-control flag             | -           |

Observation timeline:

| Block    | dt scam | tx | LP rem % | creator LP % | direct rm | backdoor | holder burn | no log | pair->control | price/init |
| -------- | ------- | -- | -------- | ------------ | --------- | -------- | ----------- | ------ | ------------- | ---------- |
| 25124390 | 21      | 13 | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1.564      |
| 25124393 | 18      | 4  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 2.313      |
| 25124394 | 17      | 7  | 0.00%    | 99.95%       | -         | -        | -           | -      | -             | 3.948      |
| 25124396 | 15      | 1  | 0.00%    | 99.95%       | -         | Y        | Y           | Y      | -             | 3.948      |
| 25124398 | 13      | 4  | 0.00%    | 99.95%       | -         | Y        | Y           | Y      | -             | 7.201      |
| 25124400 | 11      | 6  | 0.00%    | 99.95%       | -         | Y        | Y           | Y      | -             | 4.56       |
| 25124401 | 10      | 5  | 0.00%    | 99.95%       | -         | Y        | Y           | Y      | -             | 7.453      |
| 25124402 | 9       | 6  | 0.00%    | 99.95%       | -         | Y        | Y           | Y      | -             | 15.35      |
| 25124403 | 8       | 12 | 3.76%    | 99.95%       | -         | Y        | Y           | Y      | -             | 7.021      |
| 25124404 | 7       | 2  | 3.76%    | 99.95%       | -         | Y        | Y           | Y      | -             | 10.09      |
| 25124405 | 6       | 4  | 3.76%    | 99.95%       | -         | Y        | Y           | Y      | -             | 14.98      |
| 25124406 | 5       | 4  | 3.76%    | 99.95%       | -         | Y        | Y           | Y      | -             | 20.28      |
| 25124408 | 3       | 12 | 3.76%    | 99.95%       | -         | Y        | Y           | Y      | -             | 10.09      |

Network snapshot:

- Status: `complete`
- Token: `shabang` / `shabang`
- Creator address from metadata: `-`
- Graph: `131` nodes, `497` edges, `64` addresses.
- Labelled network nodes: `0x6d7c9af246e2df47939d66899fb38297d5093bea` (wallet,creator,owner); `0x11550f9491b07c6bb95053393b084a0659b54b4e` (wallet); `0x12deba11348d57be94b11ecfe621f9d1600794bb` (wallet); `0x13c711c91dc7e9fb27ff9f7fcac6edb042f12a29` (wallet); `0x2291e541f703927e22b201ee802ee3989e241b75` (wallet).
- Top context edges: `0x3328f7f4...309c49->0xac0c5221...2499dd` direct_denom_flow w=34; `0x7a250d56...f2488d->0xac0c5221...2499dd` direct_denom_flow w=15; `0xac0c5221...2499dd->0x7a250d56...f2488d` direct_denom_flow w=14; `0xac0c5221...2499dd->0x7a250d56...f2488d` shared_sink w=14; `0x11550f94...b54b4e->0x3328f7f4...309c49` direct_denom_flow w=2.

Feature lesson:

- Backdoor timing should be explicit: first backdoor flag was `25124396`, `15` chain blocks before scam.
- Add `blocks_since_first_backdoor_signal`: it was `12` at the selected row.
- The holder-to-burn `transferFrom` and pair-balance backdoor flags co-arrived; add an interaction feature instead of leaving them as independent booleans.

### Near-horizon false positive

- Case: `near_horizon_false_positive`
- Token: `0x14b440b285f9606eb91393e319e18a20dd21d401`
- Pool: `0x7d306dae4fe677c229b4074f33eb5f8038476064`
- Why selected: Highest-scored 4-block false positive that did scam soon after the 4-block window.
- Scam mechanism: `direct_lp_liquidity_removal`
- Selected row: block `25125020`, score `80.12%`, time-to-scam `11` chain blocks.

Model score summary:

| Target                           | Rows | Positive rows | Max score | Max score block | First positive block |
| -------------------------------- | ---- | ------------- | --------- | --------------- | -------------------- |
| scam_within_10_chain_block_delta | 30   | 0             | 55.35%    | 25125020        | -                    |
| scam_within_1_chain_block_delta  | 30   | 0             | 75.14%    | 25125020        | -                    |
| scam_within_2_chain_block_delta  | 30   | 0             | 77.52%    | 25125020        | -                    |
| scam_within_3_chain_block_delta  | 30   | 0             | 72.66%    | 25125020        | -                    |
| scam_within_4_chain_block_delta  | 30   | 0             | 80.12%    | 25125020        | -                    |

Event sequence:

| Block    | Kind        | Mechanism                   | Tx                  | Mempool first seen |
| -------- | ----------- | --------------------------- | ------------------- | ------------------ |
| 25124949 | lp_approval | -                           | 0x93963900...033ca2 | -                  |
| 25125020 | lp_approval | -                           | 0xee5a38d9...a54a89 | -                  |
| 25125031 | scam        | direct_lp_liquidity_removal | 0xb5defd2a...2a7c1d | -                  |

Signal blocks:

| Signal                                 | First block |
| -------------------------------------- | ----------- |
| LP approval event                      | 25124949    |
| LP removable >= 90%                    | 25125020    |
| Creator LP balance >= 90%              | 25124949    |
| Token-control event                    | -           |
| Pair-balance backdoor flag             | -           |
| Holder-to-burn transferFrom flag       | -           |
| transferFrom without Transfer log flag | -           |
| Pair-token-to-control flag             | -           |

Observation timeline:

| Block    | dt scam | tx | LP rem % | creator LP % | direct rm | backdoor | holder burn | no log | pair->control | price/init |
| -------- | ------- | -- | -------- | ------------ | --------- | -------- | ----------- | ------ | ------------- | ---------- |
| 25124949 | 82      | 2  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1.148      |
| 25124951 | 80      | 10 | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1.179      |
| 25124952 | 79      | 26 | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1.177      |
| 25124968 | 63      | 47 | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1.902      |
| 25124969 | 62      | 34 | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 2.305      |
| 25124970 | 61      | 36 | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 0.9655     |
| 25124971 | 60      | 20 | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1.05       |
| 25124972 | 59      | 6  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1.162      |
| 25124973 | 58      | 7  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1.234      |
| 25124974 | 57      | 5  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1.161      |
| 25124975 | 56      | 3  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1.161      |
| 25124976 | 55      | 3  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1.104      |
| 25124978 | 53      | 1  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1.104      |
| 25124980 | 51      | 2  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1.101      |
| 25125020 | 11      | 3  | 100.00%  | 100.00%      | -         | -        | -           | -      | -             | 1.183      |

Network snapshot:

- Status: `complete`
- Token: `ENSO` / `Enso Research`
- Creator address from metadata: `-`
- Graph: `1230` nodes, `3301` edges, `271` addresses.
- Labelled network nodes: `0x01cc6f2eb39fc2852be361b77800fd11ff22edbe` (wallet); `0x1e467dfde1d23b54d74333fbade18b1902f07412` (wallet); `0x2ac8a1c87728b82029627abb81827f8928e77899` (wallet); `0x5904d43874a27185ba47ac39f442cdd0357634f2` (wallet); `0x6e95d13d190a814aeaa864d5a19fce405654e4b8` (wallet).
- Top context edges: `0x7a250d56...f2488d->0x97c322ab...8d1495` direct_denom_flow w=62; `0x18dd3c14...f68e25->0x6e95d13d...54e4b8` direct_denom_flow w=2; `0x6e95d13d...54e4b8->0xef6fc636...2e5042` direct_denom_flow w=2; `0x14b440b2...21d401->0x7a250d56...f2488d` direct_denom_flow w=1.

Feature lesson:

- LP removability timing should be explicit: first >=90% removable block was `25125020`, `11` chain blocks before scam.

### Unlabeled backdoor false positive

- Case: `unlabeled_backdoor_false_positive`
- Token: `0xff8f98fa9cf9deb1b369785d2bf2778df6c0f609`
- Pool: `0xbb2a3a1529636325330835373def729344a97b6a`
- Why selected: Highest-scored 4-block false positive with backdoor signal and no scam label in the test run.
- Scam mechanism: `none in test run`
- Selected row: block `25130111`, score `58.91%`, time-to-scam `-` chain blocks.

Model score summary:

| Target                           | Rows | Positive rows | Max score | Max score block | First positive block |
| -------------------------------- | ---- | ------------- | --------- | --------------- | -------------------- |
| scam_within_10_chain_block_delta | 1353 | 0             | 55.50%    | 25130111        | -                    |
| scam_within_1_chain_block_delta  | 1354 | 0             | 58.70%    | 25130111        | -                    |
| scam_within_2_chain_block_delta  | 1354 | 0             | 67.88%    | 25130111        | -                    |
| scam_within_3_chain_block_delta  | 1354 | 0             | 73.42%    | 25130111        | -                    |
| scam_within_4_chain_block_delta  | 1353 | 0             | 58.91%    | 25130111        | -                    |

Event sequence:

| Block    | Kind        | Mechanism | Tx                  | Mempool first seen |
| -------- | ----------- | --------- | ------------------- | ------------------ |
| 25130111 | lp_approval | -         | 0xc32cc530...924b3a | -                  |

Signal blocks:

| Signal                                 | First block |
| -------------------------------------- | ----------- |
| LP approval event                      | 25130111    |
| LP removable >= 90%                    | 25130111    |
| Creator LP balance >= 90%              | 25129807    |
| Token-control event                    | -           |
| Pair-balance backdoor flag             | 25129813    |
| Holder-to-burn transferFrom flag       | -           |
| transferFrom without Transfer log flag | -           |
| Pair-token-to-control flag             | 25129813    |

Observation timeline:

| Block    | dt scam | tx | LP rem % | creator LP % | direct rm | backdoor | holder burn | no log | pair->control | price/init |
| -------- | ------- | -- | -------- | ------------ | --------- | -------- | ----------- | ------ | ------------- | ---------- |
| 25129807 | -       | 1  | 0.00%    | 100.00%      | -         | -        | -           | -      | -             | 1          |
| 25129813 | -       | 1  | 0.00%    | 100.00%      | -         | Y        | -           | -      | Y             | 1.361      |
| 25130000 | -       | 1  | 0.00%    | 100.00%      | -         | Y        | -           | -      | Y             | 1.724      |
| 25130111 | -       | 2  | 100.00%  | 100.00%      | -         | Y        | -           | -      | Y             | 2.503      |
| 25143903 | -       | 1  | 0.00%    | 0.00%        | -         | -        | -           | -      | -             | 0.9987     |
| 25143909 | -       | 1  | 0.00%    | 0.00%        | -         | -        | -           | -      | -             | 1.003      |
| 25143926 | -       | 1  | 0.00%    | 0.00%        | -         | -        | -           | -      | -             | 1          |
| 25143955 | -       | 1  | 0.00%    | 0.00%        | -         | -        | -           | -      | -             | 1          |
| 25143967 | -       | 1  | 0.00%    | 0.00%        | -         | -        | -           | -      | -             | 1          |
| 25143978 | -       | 1  | 0.00%    | 0.00%        | -         | -        | -           | -      | -             | 1          |
| 25143980 | -       | 2  | 0.00%    | 0.00%        | -         | -        | -           | -      | -             | 1          |
| 25143990 | -       | 1  | 0.00%    | 0.00%        | -         | -        | -           | -      | -             | 0.9961     |
| 25144017 | -       | 1  | 0.00%    | 0.00%        | -         | -        | -           | -      | -             | 1.001      |
| 25144031 | -       | 1  | 0.00%    | 0.00%        | -         | -        | -           | -      | -             | 0.9962     |
| 25144054 | -       | 1  | 0.00%    | 0.00%        | -         | -        | -           | -      | -             | 1          |
| 25144078 | -       | 1  | 0.00%    | 0.00%        | -         | -        | -           | -      | -             | 0.9959     |

Network snapshot:

- Status: `complete`
- Token: `WORLDCUP26` / `WORLD CUP 2026`
- Creator address from metadata: `-`
- Graph: `290` nodes, `637` edges, `245` addresses.
- Labelled network nodes: `0x0a8cc6e198cad7c466a8091ac89bacd15b51e688` (wallet); `0x0d1b6284516ab1cd4993524137875a8159f59999` (wallet); `0x47b8472804daefdcf260502f15ab8a4f1a951120` (wallet); `0x4e9eaaf3f9e965ed684b1e612824f5e5f817cfde` (wallet); `0x69e0ca277ea790bd54fc3a57a965ce4f118a16c5` (wallet).
- Top context edges: `0x4313c378...96b27f->0xbb2a3a15...a97b6a` direct_denom_flow w=4; `0x47b84728...951120->0x4c82d1fb...0a2cca` direct_denom_flow w=4; `0x4e9eaaf3...17cfde->0x3328f7f4...309c49` direct_denom_flow w=4; `0x89510133...f77492->0x4313c378...96b27f` direct_denom_flow w=4; `0x4313c378...96b27f->0xbb2a3a15...a97b6a` shared_funder w=4.

Feature lesson:

- Add `blocks_since_first_backdoor_signal`: it was `298` at the selected row.
