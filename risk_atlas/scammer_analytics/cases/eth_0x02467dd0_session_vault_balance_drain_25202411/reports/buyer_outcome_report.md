# Session Buyer Outcome Report

## Scope

- token: `0xc3A640bD249381F8097f44C1b61C46172068cDff`
- pool: `0x77a43d235C261436f0ad4577ce575fF0991aDc1a`
- suspect: `0x02467dd05200e5B3FBcdddbF43735aE4d0681a59`
- control contract: `0xD8e11826e82619bf49C58c05F26C8e00B0B64eA4`
- audit block: `25202575`
- active blocks replayed: `44`

## Buyer Outcome Summary

- buyers: `67`
- confiscated: `5`
- sold normally: `0`
- partial exit still holding: `11`
- still holding at audit: `51`
- transferred or burned normally: `0`
- direct creator/control connections found: `1`
- shared-funder clusters: `1`

## Interpretation

The custody verdict is split by evidence type. `confiscated` means the emitted-transfer ledger expected a material buyer balance but on-chain `balanceOf` at the audit block was missing at least 90%. `sold_normally` and `partial_exit_still_holding` are ordinary emitted pool-out/pool-in paths. `still_holding_at_audit` means no state loss was observed through the latest indexed token activity block used by this report.

Hidden creator linkage is conservative. `direct` requires the buyer itself, its nearest pre-buy funder, or a post-buy ETH flow to touch the suspect/control address directly. `weak` or `strong` shared-funder clusters are not attribution by themselves; they are follow-up leads.

## Buyer Rows

| Buyer | Class | Bought | Expected | Actual | Missing | Funder | Link |
| --- | --- | ---: | ---: | ---: | ---: | --- | --- |
| `0x02467d...681a59` | `confiscated` | `168137175.6409` | `1000000000.0000` | `0.0000` | `1000000000.0000` | `-` | `buyer_is_suspect` |
| `0x4d8b6d...de234a` | `confiscated` | `35857786.4872` | `35857786.4872` | `180.0000` | `35857606.4872` | `-` | `no_direct_creator_link_in_window` |
| `0x9ec2f5...6ba663` | `confiscated` | `27041891.0980` | `27041891.0980` | `131.0000` | `27041760.0980` | `0x99c1c2...ee85f5` | `no_direct_creator_link_in_window` |
| `0x28474c...cf5597` | `confiscated` | `9871580.3440` | `9871487.3440` | `0.0000` | `9871487.3440` | `0x2348e8...a1fc27` | `no_direct_creator_link_in_window` |
| `0xb003b0...712b34` | `confiscated` | `349007.7669` | `349007.7669` | `30.0000` | `348977.7669` | `-` | `no_direct_creator_link_in_window` |
| `0x063676...7e95a0` | `partial_exit_still_holding` | `129032018.6661` | `4817589.9648` | `4817589.9648` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x10929a...4be50e` | `partial_exit_still_holding` | `91553930.1948` | `15701499.0284` | `15701499.0284` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xe12af9...31c20a` | `partial_exit_still_holding` | `84435747.9768` | `5112922.1748` | `5112922.1748` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xce0e56...5111c8` | `partial_exit_still_holding` | `61583554.1930` | `10561579.5441` | `10561579.5441` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x750819...0af5db` | `partial_exit_still_holding` | `39442540.9968` | `19326845.0884` | `19326845.0884` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xd377fd...f33c21` | `partial_exit_still_holding` | `38441509.2686` | `26909056.4880` | `26909056.4880` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xfde8ca...73fdd3` | `partial_exit_still_holding` | `34097050.0860` | `16707554.5421` | `16707554.5421` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x7a0fec...e16052` | `partial_exit_still_holding` | `25190834.6201` | `17633584.2341` | `17633584.2341` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x7c5a0f...f06a92` | `partial_exit_still_holding` | `23754181.9103` | `16627927.3372` | `16627927.3372` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x7fce3e...a22009` | `partial_exit_still_holding` | `11984476.8865` | `9664992.5629` | `9664992.5629` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xdb7f5b...b6341f` | `partial_exit_still_holding` | `9670064.9829` | `7921376.0670` | `7921376.0670` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x257cda...f4b943` | `still_holding_at_audit` | `45087040.0548` | `45087040.0548` | `45087040.0548` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xe3f81b...07fbb3` | `still_holding_at_audit` | `37951634.4641` | `37951634.4641` | `37951634.4641` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x46ddf3...76f6c9` | `still_holding_at_audit` | `35366231.7967` | `35366231.7967` | `35366231.7967` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x6e659b...67454b` | `still_holding_at_audit` | `33822305.3999` | `33822305.3999` | `33822305.3999` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x2f1a91...c491f0` | `still_holding_at_audit` | `32602704.5686` | `32602704.5686` | `32602704.5686` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x263161...018cee` | `still_holding_at_audit` | `28163914.7997` | `28163914.7997` | `28163914.7997` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x9de749...dd22b2` | `still_holding_at_audit` | `27914168.7238` | `27914168.7238` | `27914168.7238` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xa37eae...86d90a` | `still_holding_at_audit` | `21953371.7099` | `21953371.7099` | `21953371.7099` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xefafec...3cf766` | `still_holding_at_audit` | `20106576.6109` | `20106576.6109` | `20106576.6109` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x3f4cf4...83948b` | `still_holding_at_audit` | `20001677.7851` | `20001677.7851` | `20001677.7851` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x71fbac...0e2905` | `still_holding_at_audit` | `18993674.1276` | `18993674.1276` | `18993674.1276` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x529158...2ab77d` | `still_holding_at_audit` | `16238411.3405` | `16238411.3405` | `16238411.3405` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x0adb41...ed8071` | `still_holding_at_audit` | `15441861.4066` | `15441861.4066` | `15441861.4066` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x2b28ab...046062` | `still_holding_at_audit` | `14757394.1035` | `14757394.1035` | `14757394.1035` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x11aa5e...bdd46c` | `still_holding_at_audit` | `14483175.1574` | `14483175.1574` | `14483175.1574` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xf7b096...d4fd00` | `still_holding_at_audit` | `13556516.4214` | `13556516.4214` | `13556516.4214` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xba82d8...17cf49` | `still_holding_at_audit` | `12889271.3037` | `12889271.3037` | `12889271.3037` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x7426be...208fdc` | `still_holding_at_audit` | `12669419.9927` | `12669419.9927` | `12669419.9927` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x4ba861...9b6f80` | `still_holding_at_audit` | `12102503.6152` | `12102503.6152` | `12102503.6152` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x7d41f4...3567d3` | `still_holding_at_audit` | `9312882.2220` | `9312882.2220` | `9312882.2220` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x8343d9...97eee7` | `still_holding_at_audit` | `8459680.9124` | `8459680.9124` | `8459680.9124` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x20323d...27a2cf` | `still_holding_at_audit` | `8158392.7948` | `8158392.7948` | `8158392.7948` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xb7a2c1...31c841` | `still_holding_at_audit` | `8019228.6429` | `8019228.6429` | `8019228.6429` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xd5902c...f9408c` | `still_holding_at_audit` | `7747343.3608` | `7747343.3608` | `7747343.3608` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xf409fc...c5e200` | `still_holding_at_audit` | `7649973.8743` | `7649973.8743` | `7649973.8743` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x870a82...154a87` | `still_holding_at_audit` | `7360005.7030` | `7360005.7030` | `7360005.7030` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x3bf217...e54839` | `still_holding_at_audit` | `7277086.1528` | `7277086.1528` | `7277086.1528` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xeadc53...97d602` | `still_holding_at_audit` | `7186518.3782` | `7186518.3782` | `7186518.3782` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x90f427...5b83cc` | `still_holding_at_audit` | `6295848.6411` | `6295848.6411` | `6295848.6411` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xe635e0...89b2fd` | `still_holding_at_audit` | `6250575.1853` | `6250575.1853` | `6250575.1853` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x1ca0d0...17ef3f` | `still_holding_at_audit` | `6045102.7038` | `6045102.7038` | `6045102.7038` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x43076a...c82255` | `still_holding_at_audit` | `5886664.5678` | `5886664.5678` | `5886664.5678` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xff5202...106ccd` | `still_holding_at_audit` | `5755928.0240` | `5755928.0240` | `5755928.0240` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xb64ace...bcaa9a` | `still_holding_at_audit` | `5515121.2216` | `5515121.2216` | `5515121.2216` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x7a1de4...5561f9` | `still_holding_at_audit` | `5424590.4048` | `5424590.4048` | `5424590.4048` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xa1e1d9...050fef` | `still_holding_at_audit` | `5272052.7023` | `5272052.7023` | `5272052.7023` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xcbb3ba...bb926c` | `still_holding_at_audit` | `4949102.9547` | `4949102.9547` | `4949102.9547` | `0.0000` | `0xee5a7a...5f6f7a` | `shared_funder_cluster` |
| `0xe632ab...653e24` | `still_holding_at_audit` | `4863783.3676` | `4863783.3676` | `4863783.3676` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x29dfe9...5dfd04` | `still_holding_at_audit` | `4854250.7515` | `4854250.7515` | `4854250.7515` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xbc8a46...50a8e3` | `still_holding_at_audit` | `4610055.7520` | `4610055.7520` | `4610055.7520` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x3e517d...713084` | `still_holding_at_audit` | `4591078.2831` | `4591078.2831` | `4591078.2831` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x7f3d82...afed3e` | `still_holding_at_audit` | `4244304.8207` | `4244304.8207` | `4244304.8207` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x77b7fd...4c7fa5` | `still_holding_at_audit` | `4135116.6357` | `4135116.6357` | `4135116.6357` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xc1e67f...2fdf79` | `still_holding_at_audit` | `4077066.0275` | `4077066.0275` | `4077066.0275` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xe862f4...ca8b3e` | `still_holding_at_audit` | `3648110.2886` | `3648110.2886` | `3648110.2886` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x93468d...25c3c9` | `still_holding_at_audit` | `3410289.4849` | `3410289.4849` | `3410289.4849` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xa8b78f...4ac232` | `still_holding_at_audit` | `3298142.7018` | `3298142.7018` | `3298142.7018` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x596a5e...70f494` | `still_holding_at_audit` | `2854554.2740` | `2854554.2740` | `2854554.2740` | `0.0000` | `0xee5a7a...5f6f7a` | `shared_funder_cluster` |
| `0x445461...fb74fe` | `still_holding_at_audit` | `2724324.4734` | `2724324.4734` | `2724324.4734` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0x5355c7...e9b224` | `still_holding_at_audit` | `2137960.7689` | `2137960.7689` | `2137960.7689` | `0.0000` | `-` | `no_direct_creator_link_in_window` |
| `0xa18d64...787478` | `still_holding_at_audit` | `1639053.7841` | `1639053.7841` | `1639053.7841` | `0.0000` | `-` | `no_direct_creator_link_in_window` |

## Address Clusters

| Cluster | Confidence | Hub | Members |
| --- | --- | --- | ---: |
| `shared_funder_0xee5a7a...5f6f7a` | `weak` | `0xee5a7a...5f6f7a` | `2` |

## Artifacts

- `artifacts/buyer_token_outcomes.json`
- `artifacts/buyer_token_outcomes.csv`
- `artifacts/buyer_pnl.csv`
- `artifacts/tx_fund_flow/buyer_eth_edges.json`
- `artifacts/tx_fund_flow/buyer_eth_edges.csv`
- `artifacts/tx_fund_flow/address_clusters.json`
