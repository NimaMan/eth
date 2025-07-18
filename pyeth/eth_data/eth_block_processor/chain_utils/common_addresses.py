from web3 import Web3


DENOM_ADDRESSES = {
    # Stablecoins
    '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48': 'USDC',
    '0xdAC17F958D2ee523a2206206994597C13D831ec7': 'USDT',
    '0x6B175474E89094C44Da98b954EedeAC495271d0F': 'DAI',
    '0x4Fabb145d64652a948d72533023f6E7A623C7C53': 'BUSD',
    '0x8E870D67F660D95d5be530380D0eC0bd388289E1': 'PAX',
    '0x956F47F50A910163D8BF957Cf5846D573E7f87CA': 'FEI',
    '0x853d955aCEf822Db058eb8505911ED77F175b99e': 'FRAX',
    '0x5f98805A4E8be255a32880FDeC7F6728C6568bA0': 'LUSD',
    
    # Wrapped Tokens
    '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2': 'WETH',
    '0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599': 'WBTC',
    '0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c': 'WBNB',
    
    # Other Major Tokens
    '0x7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0': 'MATIC',
    '0x514910771AF9Ca656af840dff83E8264EcF986CA': 'LINK',
    '0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984': 'UNI',
    '0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9': 'AAVE',
    
    # Wrapped Versions of Stablecoins
    '0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643': 'cDAI',
    '0x39AA39c021dfbaE8faC545936693aC917d5E7563': 'cUSDC',
    '0x4Ddc2D193948926D02f9B1fE9e1daa0718270ED5': 'cETH',
    
    # Curve LP Tokens
    '0x6c3F90f043a72FA612cbac8115EE7e52BDe6E490': '3Crv',
    '0x06325440D014e39736583c165C2963BA99fAf14E': 'stETH-ETH',
    
    # Yearn Tokens
    '0xdA816459F1AB5631232FE5e97a05BBBb94970c95': 'yvDAI',
    '0xa354F35829Ae975e850e23e9615b11Da1B3dC4DE': 'yvUSDC',
    
    # Liquid Staking Derivatives
    '0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84': 'stETH',
    '0xBe9895146f7AF43049ca1c1AE358B0541Ea49704': 'cbETH',
    '0xae78736Cd615f374D3085123A210448E74Fc6393': 'rETH',
}


DENOM_NAMES_TO_ADDRESS = {v: k for k, v in DENOM_ADDRESSES.items()}


# Complete decimals mapping for all tokens
ERC20_TOKEN_DECIMALS = {
    # Stablecoins
    'USDC': 6,
    'USDT': 6,
    "USD1": 18,
    'DAI': 18,
    'BUSD': 18,
    'PAX': 18,
    'FEI': 18,
    'FRAX': 18,
    'LUSD': 18,
    'BRZ': 18,
    'CADC': 18,
    'DOLA': 18,
    'EUROC': 6,
    'EURCV': 18,
    'EURS': 2,
    'EURT': 6,
    'FDUSD': 18,
    'GHO': 18,
    'GUSD': 2,
    'GYEN': 6,
    'IDRT': 2,
    'JPYC': 18,
    'MIM': 18,
    'MKUSD': 18,
    'OUSD': 18,
    'PAXG': 18,
    'PYUSD': 6,
    'RAI': 18,
    'sUSD': 18,
    'TUSD': 18,
    'USD0': 18,
    'USDP': 18,
    'USDS': 6,
    'USDD': 18,
    'USDE': 18,
    'XAUt': 6,
    'XIDR': 6,
    'XSGD': 6,
    'XUSD': 6,
    'ZUSD': 6,
    
    # Wrapped Tokens
    'WETH': 18,
    'WBTC': 8,
    'WBNB': 18,
    
    # Other Major Tokens
    'MATIC': 18,
    'LINK': 18,
    'UNI': 18,
    'AAVE': 18,
    
    # Wrapped Versions of Stablecoins
    'cDAI': 8,    # Compound DAI
    'cUSDC': 8,   # Compound USDC
    'cETH': 8,    # Compound ETH
    
    # Curve LP Tokens
    '3Crv': 18,   # 3pool LP token
    'stETH-ETH': 18,  # stETH-ETH LP token
    
    # Yearn Tokens
    'yvDAI': 18,  # Yearn DAI vault
    'yvUSDC': 6,  # Yearn USDC vault
    
    # Liquid Staking Derivatives
    'stETH': 18,  # Lido staked ETH
    'cbETH': 18,  # Coinbase staked ETH
    'rETH': 18,   # Rocket Pool ETH
}


STABLECOIN_UNIT_BY_NAME = {
    # Commodity‑backed
    "PAXG": "Gold (troy ounce)",      # Pax Gold 1‑oz token  [oai_citation:0‡Paxos](https://www.paxos.com/pax-gold?utm_source=chatgpt.com)
    "XAUt": "Gold (troy ounce)",      # Tether Gold 1‑oz token  [oai_citation:1‡Tether](https://gold.tether.to/?utm_source=chatgpt.com)

    # Non‑USD fiat units
    "BRZ":  "Brazilian Real",         # Transfero’s BRZ peg  [oai_citation:2‡Transfero](https://transfero.com/stablecoins/brz/?utm_source=chatgpt.com)
    "CADC": "Canadian Dollar",        # (Fiat‑backed CADC)  [oai_citation:3‡Getting Started | Synthetix Docs](https://docs.synthetix.io/exchange/perps-v3-base/multi-collateral-margin/collateral-types?utm_source=chatgpt.com)
    "EUROC": "Euro",                  # Circle’s euro coin   [oai_citation:4‡Circle](https://www.circle.com/eurc?utm_source=chatgpt.com)
    "EURCV": "Euro",                  # Société Générale EUR CoinVertible  [oai_citation:5‡SG Forge](https://www.sgforge.com/product/coinvertible/?utm_source=chatgpt.com)
    "EURS":  "Euro",                  # Stasis EURS stable coin (info in docs/stasis)  [oai_citation:6‡CoinMarketCap](https://coinmarketcap.com/currencies/eur-coinvertible/?utm_source=chatgpt.com)
    "EURT":  "Euro",                  # Tether Euro token (Tether docs)  [oai_citation:7‡CoinMarketCap](https://coinmarketcap.com/currencies/eur-coinvertible/?utm_source=chatgpt.com)
    "GYEN":  "Japanese Yen",          # GMO Trust’s GYEN peg  [oai_citation:8‡stablecoin.z.com](https://stablecoin.z.com/gyen/?utm_source=chatgpt.com)
    "JPYC":  "Japanese Yen",          # JPYC pegged to JPY (jpyc.jp)  [oai_citation:9‡stablecoin.z.com](https://stablecoin.z.com/gyen/?utm_source=chatgpt.com)
    "IDRT":  "Indonesian Rupiah",     # Rupiah Token IDRT  [oai_citation:10‡rupiahtoken.com](https://rupiahtoken.com/?utm_source=chatgpt.com)
    "XIDR":  "Indonesian Rupiah",     # StraitsX IDR token info (straitsx)  [oai_citation:11‡rupiahtoken.com](https://rupiahtoken.com/?utm_source=chatgpt.com)
    "XSGD":  "Singapore Dollar",      # StraitsX XSGD peg  [oai_citation:12‡CoinMarketCap](https://coinmarketcap.com/currencies/xsgd/?utm_source=chatgpt.com)

    # USD‑denominated (fiat‑backed, crypto‑collateralised or algorithmic)
    "BUSD": "US Dollar",
    "DAI":  "US Dollar",              # MakerDAO white‑paper peg  [oai_citation:13‡makerdao.com](https://makerdao.com/?utm_source=chatgpt.com)
    "DOLA": "US Dollar",
    "FDUSD": "US Dollar",
    "FEI":  "US Dollar",
    "FRAX": "US Dollar",
    "GHO":  "US Dollar",              # Aave GHO docs  [oai_citation:14‡aave.com](https://aave.com/gho?utm_source=chatgpt.com)
    "GUSD": "US Dollar",
    "LUSD": "US Dollar",
    "MIM":  "US Dollar",
    "MKUSD": "US Dollar",             # Prisma mkUSD docs  [oai_citation:15‡The Big Whale](https://www.thebigwhale.io/tokens/prisma-mkusd?utm_source=chatgpt.com)
    "OUSD": "US Dollar",
    "PYUSD": "US Dollar",             # PayPal PYUSD launch (Reuters)  [oai_citation:16‡Reuters](https://www.reuters.com/business/coinbase-waives-fees-paypals-stablecoin-crypto-payments-push-2025-04-24/?utm_source=chatgpt.com)
    "RAI":  "RAI Protocol",
    "sUSD": "US Dollar",              # Synthetix docs  [oai_citation:17‡Getting Started | Synthetix Docs](https://docs.synthetix.io/user-docs/v2-user-docs/dao/elections-and-voting?utm_source=chatgpt.com)
    "TUSD": "US Dollar",
    "USDC": "US Dollar",              # Circle reserves description  [oai_citation:18‡Getting Started | Synthetix Docs](https://docs.synthetix.io/exchange/perps-v3-base/multi-collateral-margin/collateral-types?utm_source=chatgpt.com)
    "USD0": "US Dollar",              # Usual USD0 peg (CoinMarketCap)  [oai_citation:19‡CoinMarketCap](https://coinmarketcap.com/currencies/usual-usd/?utm_source=chatgpt.com)
    "USD1": "US Dollar",
    "USDP": "US Dollar",              # Pax Dollar docs  [oai_citation:20‡docs.paxos.com](https://docs.paxos.com/stablecoin?utm_source=chatgpt.com)
    "USDS": "US Dollar",
    "USDT": "US Dollar",
    "USDD": "US Dollar",
    "USDE": "US Dollar",              # Ethena synthetic USD  [oai_citation:21‡Ethena](https://ethena.fi/?utm_source=chatgpt.com)
    "XUSD": "US Dollar",              # StraitsX USD token  [oai_citation:22‡straitsx.com](https://www.straitsx.com/xusd?utm_source=chatgpt.com)
    "ZUSD": "US Dollar",              # ZUSD peg (CoinMarketCap)  [oai_citation:23‡CoinMarketCap](https://coinmarketcap.com/currencies/zusd/?utm_source=chatgpt.com)
}


STABLECOINS_ADDRESS_BY_NAME = {
    "BRZ": "0x01D33Fd36ec67C6adA32Cf36B31E88Ee190b1839",
    "BUSD": "0x4fabb145d64652a948d72533023f6e7a623c7c53",
    "CADC": "0xcaDC0aCD4B445166f12D2C07EaC6E2544FbE2Eef",
    "DAI": "0x6B175474E89094C44Da98b954EedeAC495271d0F",
    "DOLA": "0x865377367054516e17014ccded1e7d814edc9ce4",
    "EUROC": "0x1aBaEA1f7C830bd89Acc67EC4af516284b1bC33c",
    "EURCV": "0x5F7827FDeb7c20b443265Fc2F40845B715385Ff2",
    "EURS": "0xdb25f211ab05b1c97d595516f45794528a807ad8",
    "EURT": "0xC581b735A1688071A1746c968e0798D642EDE491",
    "FDUSD": "0xc5f0F7B66764F6EC8c8dFF7Ba683102295E16409",
    "FEI": "0x956F47F50A910163D8BF957Cf5846D573E7f87CA",
    "FRAX": "0x853d955aCEf822Db058eb8505911ED77F175b99e",
    "GHO": "0x40D16FC0246aD3160Ccc09B8D0D3A2cD28aE6C2f",
    "GUSD": "0x056FD409E1d7A124BD7017459dFEa2F387B6d5Cd",
    "GYEN": "0xC08512927D12348F6620a698105e1BAac6EcD911",
    "IDRT": "0x998FFE1E43fAcffb941dc337dD0468d52BA5B48A",
    "JPYC": "0x2370f9d504C7A6E775bf6E14B3F12846b594cD53",
    "LUSD": "0x5f98805A4E8be255a32880FDeC7F6728C6568bA0",
    "MKUSD": "0x4591DBfF62656E7859Afe5e45f6f47D3669fBB28",
    "OUSD": "0x2A8e1E676Ec238d8A992307B495b45B3fEAa5e86",
    "PAXG": "0x45804880De22913dAFE09f4980848ECE6EcbAf78",
    "PYUSD": "0x6c3EA9036406852006290770BEdFcAbA0e23A0E8",
    "RAI": "0x03ab458634910Aad20Ef5f1C8eE96f1d6Ac54919",
    "sUSD": "0x57ab1eC28D129707052DF4DF418D58A2D46d5f51",
    "TUSD": "0x0000000000085d4780B73119b644AE5ecd22b376",
    "USDC": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    "USD0": "0x73a15fed60bf67631dc6cd7bc5b6e8da8190acf5",
    "USD1": "0xC824Bf014539F6bdE6b81ABAaca0D626C2AC5985",
    "USDP": "0x8E870D67F660D95D5be530380D0eC0bd388289E1",
    "USDS": "0xA4BDB11dC0a2beC88d24A3AA1e6bb17201112EBE",
    "USDT": "0xdAC17F958D2ee523a2206206994597C13D831ec7",
    "USDD": "0x0C10BF8FCB7BF5412187A595aB97A3609160B5C6",
    "USDE": "0x4C9EDD5852cD905F086c759e8383E09BFF1E68B3",
    "XAUt": "0x68749665FF8D2d112Fa859AA293F07A622782F38",
    "XIDR": "0xebF2096E01455108bAdCbAF86cE30b6e5A72aa52",
    "XSGD": "0x70e8dE73cE538DA2bEEd35d14187F6959a8ecA96",
    "XUSD": "0xC08e7E23C235073C6807C2eFe7021304cB7C2815",
    "ZUSD": "0xc56c2b7e71B54d38Aab6d52E94a04Cbfa8F604fA"
}
STABLECOINS_NAME_BY_ADDRESS = {v: k for k, v in STABLECOINS_ADDRESS_BY_NAME.items()}


addresses_by_name = {
    'zero_address': '0x0000000000000000000000000000000000000000',
    'dead_address': '0x000000000000000000000000000000000000dEaD',
    'WETH': '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2',
    'TrustSwap: Team Finance Lock': '0xE2fE530C047f2d85298b07D9333C05737f1435fB',
    'UNCX Network Security: LP Lockers': '0x663A5C229c09b049E36dCc11a9B0d4a8Eb9db214',
    'UNCX Network Security: Token Vesting': '0xDba68f07d1b7Ca219f78ae8582C213d975c25cAf',
    'UNCX Network Lockers: V3 Proof of Reserves 2': '0x7f5C649856F900d15C83741f45AE46f5C6858234',
    'UNCX Network Lockers: V3 Proof of Reserves 3': '0xFD235968e65B0990584585763f837A5b5330e6DE',
    'PinkLock02': '0x71B5759d73262FBb223956913ecF4ecC51057641',
    'UniswapV2Router02': '0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D',
    'UniswapV2Factory': '0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f',
    'UniswapV3Factory': '0x1F98431c8aD98523631AE4a59f267346ea31F984',
    "UniswapUniversalRouter": "0x3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD",
    'NonfungiblePositionManager': '0xC36442b4a4522E871399CD717aBDD847Ab11FE88',
    'WETH_USDT': '0x11b815efB8f581194ae79006d24E0d814B7697F6', 
    'Banana Gun: Router 2': '0x3328F7f4A1D1C57c35df56bBf0c9dCAFCA309C49',
    'Banana Gun: Deployer': '0xBCd3a47e4d0000cf170E25d1bD3d53F7C08be0A6',
    'Banana Gun: Deployer 2': '0x35fC556d6f8675B26fDF1542e6E894100155B34E',
    'Banana Gun': '0xC465CC50B7D5A29b9308968f870a4B242A8e1873',
    'Maestro: Router 2': '0x80a64c6D7f12C47B7c66c5B4E20E72bc1FCd5d9e',
    'Maestro: Deployer': '0x6599aE06914f1f5Ec0053d3F475348D40E608442',
    'Unibot': '0x5c9321e92Ba4eb43f2901c4952358e132163a85A',
    'Sigma / Alphaman': '0xe76014c179F19dA26Bb30A0f085FF0A466B92829',
    "Metamask: Swap Router": "0x881D40237659C251811CEC9c364ef91dC08D300C",
    "1inch v5: Aggregation Router": "0x1111111254EEB25477B68fb85Ed929f73A960582",
}

addresses_by_name = {**addresses_by_name, **STABLECOINS_ADDRESS_BY_NAME}

names_by_address = {v: k for k, v in addresses_by_name.items()}

byte_addresses_by_name = {k: Web3.to_bytes(hexstr=v) for k, v in addresses_by_name.items()}

names_by_byte_addresses = {v: k for k, v in byte_addresses_by_name.items()}

denominator_addresses_by_name = {key: addresses_by_name[key] for key in ['WETH', 'USDC', 'USDT', 'DAI'] if key in addresses_by_name}

denominator_names_by_address = {v: k for k, v in denominator_addresses_by_name.items()}

denominator_byte_addresses_by_name = {key: byte_addresses_by_name[key] for key in denominator_addresses_by_name}

denominator_names_by_byte_address = {v: k for k, v in denominator_byte_addresses_by_name.items()}

denominator_balance_slots = {
    'WETH': 3,
    'USDC': 9,
    'USDT': 2,
    'DAI': 2
}

denominator_allowance_slots = {
    'WETH': 4,
    'DAI': 3
}

known_denom_decimals = {
    'WETH': 18,
    'USDC': 6,
    'USDT': 6,
    'DAI': 18,
    'UNI-V2': 18
}

denominator_decimal_by_address = {k: known_denom_decimals[v] for k, v in denominator_names_by_address.items()}


fee_recipients = {
    '0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5': 'beaverbuild',
    '0x1f9090aaE28b8a3dCeaDf281B0F12828e676c326': 'rsync-builder.eth',
    '0xDAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5': 'Flashbots: Builder',
    '0x4675C7e5BaAFBFFbca748158bEcBA61ef3b0a263': 'MEV Builder: 0x467...263',
    '0x690B9A9E9aa1C9dB991C7721a92d351Db4FaC990': 'builder0x69',
    '0x4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97': 'Titan Builder',
    '0xF2f5C73fa04406b1995e397B55c24aB1f3eA726C': 'bloXroute: Max Profit Builder',
    '0xBaF6dC2E647aeb6F510f9e318856A1BCd66C5e19': 'MEV Builder: 0xBaF...e19',
    '0xFeebabE6b0418eC13b30aAdF129F5DcDd4f70CeA': 'eth-builder',
    '0x5124fcC2B3F99F571AD67D075643C743F38f1C34': 'Faith Builder',
    '0x88c6C46EBf353A52Bdbab708c23D0c81dAA8134A': 'MEV Builder: 0x88c...34A',
    '0x199D5ED7F45F4eE35960cF22EAde2076e95B253F': 'bloXroute: Regulated Builder',
    '0x473780deAF4a2Ac070BBbA936B0cdefe7F267dFc': 'MEV Builder: 0x473...dFc',
    '0xAAB27b150451726EC7738aa1d0A94505c8729bd1': 'Eden Network: Builder',
    '0xbd3Afb0bB76683eCb4225F9DBc91f998713C3b01': 'BuildAI.net',
    '0x7e2a2FA2a064F693f0a55C5639476d913Ff12D05': 'MEV Builder: 0x7e2...D05',
    '0x5F927395213ee6b95dE97bDdCb1b2B1C0F16844F': 'Manta-builder',
    '0x0Aa8EBb6aD5A8e499E550ae2C461197624c6e667': 'MEV Builder: 0x0Aa...667',
    '0x333333f332a06ECB5D20D35da44ba07986D6E203': 'MEV Builder: 0x333...203',
    '0xf573d99385C05c23B24ed33De616ad16a43a0919': 'bloXroute: Ethical Builder',
    '0xCE0BaBc8398144Aa98D9210d595E3A9714910748': 'payload',
    '0x965Df5Ff6116C395187E288e5C87fb96CfB8141c': 'bloXroute: Builder 1',
    '0x77777A6C097a1cE65C61A96a49bd1100F660eC94': 'MEV Builder: 0x777...C94',
    '0xd2090025857B9C7B24387741f120538E928A3a59': 'lightspeedbuilder 2',
    '0x3b64216AD1a58f61538b4fA1B27327675Ab7ED67': 'Boba Builder',
    '0x3b7fAEc3181114A99c243608BC822c5436441FfF': 'MEV Builder: 0x3b...FfF',
    '0xcDBF58a9A9b54a2C43800c50C7192946dE858321': 'MEV Builder: 0xcDB...321',
    '0x7dA0aEf1B75035cbf364a690411BCCa7E7859dF8': 'MEV Builder: 0x7dA...dF8',
    '0x3Bee5122E2a2FbE11287aAfb0cB918e22aBB5436': 'MEV Builder: 0x3B...436',
    '0xcDA9D71bdfAe59b89Cee131eD3079f8AC4c77062': 'MEV Builder: 0xcDA...062',
    '0xB279d48442aAfCF8F2af6d9E7d5d9C23f63b4e16': 'MEV Builder: 0xB27...e16',
    '0x2194331af2cF9dE9Adb36cf09654faD65cafb58b': 'MEV Builder: 0x219...58b',
    '0xb4c9E4617a16Be36B92689b9e07e9F64757c1792': 'MEV Builder: 0xb4c...792',
    '0xDccA982701a264e8d629A6E8CFBa9C1a27912623': 'MEV Builder: 0xDcc...623',
    '0xd11D7D2cb0aFF72A61Df37fD016EE1bd9F180633': 'MEV Builder: 0xd11...633',
    '0xb646D87963Da1FB9D192Ddba775f24f33e857128': 'MEV Builder: 0xb64…128',
    '0xb64a30399f7F6b0C154c2E7Af0a3ec7B0A5b131a': 'Flashbots: Old Builder',
    '0x57865ba267D48671A41431F471933aEC32a7c7d1': 'Manifold Finance: Builder',
    '0x57af10eD3469b2351AE60175d3C9B3740E1Bb649': 'MEV Builder: 0x57...649',
    '0xd1A0b5843F384f92a6759015c742fc12d1d579a1': 'MEV Builder: 0xd1...9a1',
    '0x7aDc0e867EBc337E2d20c44DB181c067fA08637b': 'blockbeelder',
    '0x0b1Ddf6D1DA69532Ad4198470679b0b49176c68f': 'MEV Builder: 0x0b1...68f',
    '0xC6108744c9D5db8b30f8004053a16D5683cD3489': 'MEV Builder: 0xC61...489',
    '0x25D88437dF70730122b73Ef35462435d187C466f': 'MEV Builder: 0x25D...66f',
    '0xae08c571e771F360c35f5715E36407ECc89D91ed': 'MEV Builder: 0xae...1ed',
    '0x036C9c0aaE7a8268F332bA968dac5963c6aDAca5': 'MEV Builder: 0x036...ca5',
    '0x1e54945FBf1872e34D76B7d72151B861704Df8B2': 'MEV Builder: 0x1e54...8B2',
    '0xf0Ef0B3D1CE0a2C303e76200213B3AD5dE61a4B7': 'Titanbuilder:0xf0E...4B7',
    '0x3E3753491f224571dd8d7E925B73cc685ab0aae4': 'MEV Builder: 0x3E3...ae4',
    '0x5638cbdC72bd8554055883D309CFc70357190CF3': 'MEV Builder: 0x563...cf3',
    '0x229b8325bb9Ac04602898B7e8989998710235d5f': 'MEV Builder: 0x22...d5f',
    '0xc9D945721ed37c6451E457b3C7F1e0ceC42417fb': 'antbuilder',
    '0xaC7EA48093B61f2E217b9d077d69D9d55CA1B106': 'MEV Builder: 0xaC7...106',
    '0xc83dad6e38BF7F2d79f2a51dd3C4bE3f530965D6': 'Flashbots: SGX Builder',
    '0x3c496DF419762533607f30BB2143aFF77bEBc36A': 'MEV Builder: 0x3c...36A',
    '0x5c8D0eeD35a9e632BB8c0AbE4662B6aB3326850b': 'MEV Builder: 0x5c8...50b',
    '0x1324c0fB6F45f3bF1AAA1fCdC08f17431F53DeD7': 'MEV Builder: 0x13...eD7',
    '0x25B71878850D008ec4237C55f0A59198BCC72b43': 'MEV Builder: 0x25B…b43',
    '0x8D5998A27b3CdF33479B65B18F075E20a7aa05b9': 'MEV Builder: 0x8D...5b9',
    '0xdA795b000C29e6F47C2b2A5F3A35c5647695e301': 'MEV Builder: 0xda7...301',
    '0x7AdE2D98420d1735EA2Ad3C17Ef46bF11500Fc4f': 'MEV Builder: 0x7Ad...c4f',
    '0xEeEE8Db5fC7d505e99970945a9220Ab7992050E3': 'MEV Builder: 0xEe...0E3',
    '0x70B6c88f608AC228Fd767d05094967eb91d02583': 'MEV Builder: 0x70b...583',
    '0x3B6c26116749a6F9D194172d56299377E61bB0aE': 'MEV Builder: 0x3B6...0aE',
    '0x4A55474EACb48CEFe25D7656Db1976AA7AE70E3C': 'MEV Builder: 0x4A5...E3C',
    '0x6aF43cC73c4a871274767887e8E39Eeb540582A3': 'MEV Builder: 0x6a...2A3',
    '0xE821377b30C63a873Be0eb7fE0c6f31911285d38': 'MEV Builder: 0xE82...d38',
    '0xA7FdCa7AA0B69927a34ec48DdcFe3d4C66fF0d94': 'MEV Builder: 0xA7F...d94',
    '0x7316b4E0f0D4B19b4aC13895224cD522D785e51D': 'lightspeedbuilder 1',
    '0x089780A88f35B58144Aa8a9BE654207A1aFe7959': 'MEV Builder: 0x089...959',
    '0xbEED9A1A750945966cfc8800Abd4Cf7eECD22500': 'MEV Bot: 0xbEE...500',
    '0x9D8e2dc5615c674F329d18786D52AF10a65Af08b': 'MEV Builder: 0x9D8...08b',
    '0x11a8961fbD55e67Fe4Ab99c7ca47616Dcf3D0010': 'MEV Builder: 0x11a...010',
    '0x195d0F5F00833bcE2F40920DE0DB1D92a8808886': 'MEV Builder: 0x195...886',
    '0x47fE0AEe392D59Ccaa7Cc3B162F629eCb0f2671F': 'MEV Builder: 0x47f...71F',
    '0x5F525f637759FCa7C9d1C0C4f9d479D6E8D8ceF5': 'MEV Builder: 0x5F...eF5',
    '0x8E57bC446f76B2054089CC5c8fA6F0F5B72fC59a': 'Titanbuilder: 0x8E5...59a',
    '0x1d0124FeE8Dbe21884Ab97adCCBF5C55d768886e': 'MEV Builder: 0x1d0...86e',
    '0x000000000000d3B2C76221467d2f8c8f1dE832A2': 'Builder Smith',
    '0x9FE3bC4A1A4116c6Dc1fFD61226E262c3f2bc561': 'Titanbuilder: 0x9FE...561',
    '0x5A266F52802e846ddf93B87e520404B4fe778411': 'MEV Builder: 0x5A2...411',
    '0x5416f0dd6C29bFF6a4f32BaFB5c5dA6365472973': 'MEV Builder: 0x541...973',
    '0xC4b7a6008d8e2C2E1b5F8B743E71d2c0495cd777': 'MEV Builder: 0xC4b...777',
    '0x29F94b27Fe0B410e4546b6021E4044ff985dc252': 'MEV Builder: 0x29F...252',
    '0x3Ca601b21D62790308298E3274Fd852669Fdfc08': 'MEV Builder: 0x3Ca...c08',
    '0x1b887Aa026f7A90a0f7173C2f0722d263c7CeC45': 'MEV Builder: 0x1b8...C45',
    '0x24b1D27B0f6B5A2Aa052Acf59817a8D9e7A8600A': 'Titanbuilder: 0x24b...00A',
    '0x795e17B08f45cd06E833138a2236Fa8C7aA0b3AC': 'MEV Builder: 0x795...3AC',
    '0x185A5012067d8F0f6ab2de1C78B96f15Aa552043': 'MEV Builder: 0x185...043',
    '0x000E633ddeF00DA46aBd5044779257a64ead9bce': 'MEV Builder: 0x000...bce',
    '0xc1612dc56C3E7e00D86c668dF03904B7E59616C5': 'MEV Builder: 0xc16...6C5',
    '0x14F1a856A821B3CE5F59d3bA813D614546125C15': 'MEV Builder: 0x14F...C15',
    '0x00066282d9FAc206F8F0fd0b935958ae55E13333': 'MEV Builder: 0x000...333',
    '0x418211EFaf54e6A9b376f6Bfd9E0AE304E064CBb': 'MEV Builder: 0x418...CBb'
    }

fee_recipients_set = set(fee_recipients.keys())



sandwich_attackers = [
    "0xae2Fc483527B8EF99EB5D9B44875F005ba1FaE13",   
    ]


# https://www.loock.io/blog/mrbeast-investigation
alleged_mr_beast_wallet = ["0x9e67D018488aD636B538e4158E9e7577F2ECac12", 
                           "0x3640f50C46632E03F2677f85Ec0372a8Dd70b8f4",
                           "0xED3F5d401a270416e5008ce35E07Eb0721D6f8B4"
                           "0x949cC70bAa140f5b55717ca938E3c7e4C3b3A016"
                           "0xb5bf6777e3524aD0ffCC5a37375cc49a4BE92F64", 
                           "0x2c071Af9dCeFB7155659B662480CbB8679977394",
                           "0x4f7B657a2cAe7A8808Df1D889838d5Da33007ae8",
]
