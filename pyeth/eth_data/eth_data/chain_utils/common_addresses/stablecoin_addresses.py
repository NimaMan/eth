

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
