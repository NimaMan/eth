# 2026-05-19 Mainnet V1

This run folder is the first mainnet canary deployment package for
`UniswapV2TradingVault`.

The temporary test address used for owner, treasury, deployer, and ETH tx executor
`from` is
[`0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27`](https://etherscan.io/address/0x2348E8a3A21DBe64Ace84853D7b4B696E8A1fC27).

The vault contract was deployed on Ethereum mainnet:

| Field | Value |
| --- | --- |
| Vault | [`0x28474cbCd780AeEb3ED1501B68254bEd87cF5597`](https://etherscan.io/address/0x28474cbCd780AeEb3ED1501B68254bEd87cF5597) |
| Deployment tx | [`0x8cfdddfba66d0c5651ee60042acd27978b35d6088b65d568f76de9d53208c886`](https://etherscan.io/tx/0x8cfdddfba66d0c5651ee60042acd27978b35d6088b65d568f76de9d53208c886) |
| Block | `25130558` |
| Gas used | `771,639` |
| Effective gas price | `253,330,951 wei` |
| ETH spent | `0.000195480041698689 ETH` |

Post-deploy smoke passed and confirmed owner, treasury, WETH, and Uniswap V2
router match the expected values.
