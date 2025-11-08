//! DENOM token addresses (major tokens and currencies)
//!
//! This file is auto-generated from Python address files.
//! Do not edit manually - regenerate using scripts/convert_addresses_to_rust.py

use crate::common_addresses::wallets::WALLET_ADDRESSES;
use alloy_primitives::{address, Address};
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// DENOM_ADDRESSES - Major tokens and currencies tracked by the system
/// Maps token contract addresses to their symbols
pub static DENOM_ADDRESSES: Lazy<HashMap<Address, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"), "USDC");
    m.insert(address!("dAC17F958D2ee523a2206206994597C13D831ec7"), "USDT");
    m.insert(address!("6B175474E89094C44Da98b954EedeAC495271d0F"), "DAI");
    m.insert(address!("4Fabb145d64652a948d72533023f6E7A623C7C53"), "BUSD");
    m.insert(address!("8E870D67F660D95d5be530380D0eC0bd388289E1"), "PAX");
    m.insert(address!("956F47F50A910163D8BF957Cf5846D573E7f87CA"), "FEI");
    m.insert(address!("853d955aCEf822Db058eb8505911ED77F175b99e"), "FRAX");
    m.insert(
        address!("5E8422345238F34275888049021821E8E08CAa1f"),
        "frxETH",
    );
    m.insert(address!("5f98805A4E8be255a32880FDeC7F6728C6568bA0"), "LUSD");
    m.insert(address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"), "WETH");
    m.insert(address!("2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"), "WBTC");
    m.insert(address!("bb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"), "WBNB");
    m.insert(
        address!("7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0"),
        "MATIC",
    );
    m.insert(address!("514910771AF9Ca656af840dff83E8264EcF986CA"), "LINK");
    m.insert(address!("1f9840a85d5aF5bf1D1762F925BDADdC4201F984"), "UNI");
    m.insert(address!("7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9"), "AAVE");
    m.insert(address!("5d3a536E4D6DbD6114cc1Ead35777bAB948E3643"), "cDAI");
    m.insert(
        address!("39AA39c021dfbaE8faC545936693aC917d5E7563"),
        "cUSDC",
    );
    m.insert(address!("4Ddc2D193948926D02f9B1fE9e1daa0718270ED5"), "cETH");
    m.insert(address!("6c3F90f043a72FA612cbac8115EE7e52BDe6E490"), "3Crv");
    m.insert(
        address!("06325440D014e39736583c165C2963BA99fAf14E"),
        "stETH-ETH",
    );
    m.insert(
        address!("dA816459F1AB5631232FE5e97a05BBBb94970c95"),
        "yvDAI",
    );
    m.insert(
        address!("a354F35829Ae975e850e23e9615b11Da1B3dC4DE"),
        "yvUSDC",
    );
    m.insert(
        address!("ae7ab96520DE3A18E5e111B5EaAb095312D7fE84"),
        "stETH",
    );
    m.insert(
        address!("Be9895146f7AF43049ca1c1AE358B0541Ea49704"),
        "cbETH",
    );
    m.insert(address!("ae78736Cd615f374D3085123A210448E74Fc6393"), "rETH");
    m.insert(address!("9f8F72aA9304c8B593d555F12eF6589cC3A579A2"), "MKR");
    m.insert(address!("D533a949740bb3306d119CC777fa900bA034cd52"), "CRV");
    m.insert(address!("C011a73ee8576Fb46F5E1c5751cA3B9Fe0af2a6F"), "SNX");
    m.insert(address!("c00e94Cb662C3520282E6f5717214004A7f26888"), "COMP");
    m.insert(address!("0bc529c00C6401aEF6D220BE8C6Ea1667F6Ad93e"), "YFI");
    m.insert(
        address!("6B3595068778DD592e39A122f4f5a5cF09C90fE2"),
        "SUSHI",
    );
    m.insert(address!("ba100000625a3754423978a60c9317c58a424e3D"), "BAL");
    m.insert(
        address!("111111111117dC0aa78b770fA6A738034120C302"),
        "1INCH",
    );
    m.insert(address!("B50721BCf8d664c30412Cfbc6cf7a15145234ad1"), "ARB");
    m.insert(address!("4200000000000000000000000000000000000042"), "OP");
    m.insert(address!("95aD61b0a150d79219dCF64E1E6Cc01f0B64C4cE"), "SHIB");
    m.insert(address!("6982508145454Ce325dDbE47a25d4ec3d2311933"), "PEPE");
    m.insert(address!("01D33Fd36ec67C6adA32Cf36B31E88Ee190b1839"), "BRZ");
    m.insert(address!("4fabb145d64652a948d72533023f6e7a623c7c53"), "BUSD");
    m.insert(address!("caDC0aCD4B445166f12D2C07EaC6E2544FbE2Eef"), "CADC");
    m.insert(address!("865377367054516e17014ccded1e7d814edc9ce4"), "DOLA");
    m.insert(
        address!("1aBaEA1f7C830bd89Acc67EC4af516284b1bC33c"),
        "EUROC",
    );
    m.insert(
        address!("5F7827FDeb7c20b443265Fc2F40845B715385Ff2"),
        "EURCV",
    );
    m.insert(address!("db25f211ab05b1c97d595516f45794528a807ad8"), "EURS");
    m.insert(address!("C581b735A1688071A1746c968e0798D642EDE491"), "EURT");
    m.insert(
        address!("c5f0F7B66764F6EC8c8dFF7Ba683102295E16409"),
        "FDUSD",
    );
    m.insert(address!("40D16FC0246aD3160Ccc09B8D0D3A2cD28aE6C2f"), "GHO");
    m.insert(address!("056FD409E1d7A124BD7017459dFEa2F387B6d5Cd"), "GUSD");
    m.insert(address!("C08512927D12348F6620a698105e1BAac6EcD911"), "GYEN");
    m.insert(address!("998FFE1E43fAcffb941dc337dD0468d52BA5B48A"), "IDRT");
    m.insert(address!("2370f9d504C7A6E775bf6E14B3F12846b594cD53"), "JPYC");
    m.insert(
        address!("4591DBfF62656E7859Afe5e45f6f47D3669fBB28"),
        "MKUSD",
    );
    m.insert(address!("2A8e1E676Ec238d8A992307B495b45B3fEAa5e86"), "OUSD");
    m.insert(address!("45804880De22913dAFE09f4980848ECE6EcbAf78"), "PAXG");
    m.insert(
        address!("6c3EA9036406852006290770BEdFcAbA0e23A0E8"),
        "PYUSD",
    );
    m.insert(address!("03ab458634910Aad20Ef5f1C8eE96f1d6Ac54919"), "RAI");
    m.insert(address!("57ab1eC28D129707052DF4DF418D58A2D46d5f51"), "sUSD");
    m.insert(address!("0000000000085d4780B73119b644AE5ecd22b376"), "TUSD");
    m.insert(address!("73a15fed60bf67631dc6cd7bc5b6e8da8190acf5"), "USD0");
    m.insert(address!("C824Bf014539F6bdE6b81ABAaca0D626C2AC5985"), "USD1");
    m.insert(address!("8E870D67F660D95D5be530380D0eC0bd388289E1"), "USDP");
    m.insert(address!("A4BDB11dC0a2beC88d24A3AA1e6bb17201112EBE"), "USDS");
    m.insert(address!("0C10BF8FCB7BF5412187A595aB97A3609160B5C6"), "USDD");
    m.insert(address!("4C9EDD5852cD905F086c759e8383E09BFF1E68B3"), "USDE");
    m.insert(address!("68749665FF8D2d112Fa859AA293F07A622782F38"), "XAUt");
    m.insert(address!("ebF2096E01455108bAdCbAF86cE30b6e5A72aa52"), "XIDR");
    m.insert(address!("70e8dE73cE538DA2bEEd35d14187F6959a8ecA96"), "XSGD");
    m.insert(address!("C08e7E23C235073C6807C2eFe7021304cB7C2815"), "XUSD");
    m.insert(address!("c56c2b7e71B54d38Aab6d52E94a04Cbfa8F604fA"), "ZUSD");
    // Extended token set used across pool simulations
    m.insert(address!("0D8775F648430679A709E98d2b0Cb6250d2887EF"), "BAT");
    m.insert(address!("5283D291DBCF85356A21bA090E6db59121208b44"), "BLUR");
    m.insert(address!("9813037ee2218799597d83D4a5B6F3b6778218d9"), "BONE");
    m.insert(address!("4206931337dc273a630d328dA6441786BfaD668f"), "DOGE");
    m.insert(address!("761D38e5ddf6ccf6Cf7c55759d5210750B5D60F3"), "ELON");
    m.insert(address!("F629cBd94d3791C9250152BD8dfBDF380E2a3B9c"), "ENJ");
    m.insert(address!("C18360217D8F7Ab5e7c516566761Ea12Ce7F9D72"), "ENS");
    m.insert(address!("aea46A60368A7bD060eec7DF8CBa43b7EF41Ad85"), "FET");
    m.insert(
        address!("cf0C122c6b73ff809C693DB761e7BaeBe62b6a2E"),
        "FLOKI",
    );
    m.insert(address!("50D1c9771902476076eCFc8B2A83Ad6b9355a4c9"), "FTT");
    m.insert(address!("d1d2Eb1B1e90B638588728b4130137D262C87cae"), "GALA");
    m.insert(address!("6810e776880C02933D47DB1b9fc05908e5386b96"), "GNO");
    m.insert(address!("c944E90C64B2c07662A292be6244BDf05Cda44a7"), "GRT");
    m.insert(
        address!("A2b4C0Af19cC16a6CfAcCe81F192B024d625817D"),
        "KISHU",
    );
    m.insert(address!("5A98FcBEA516Cf06857215779Fd812CA3beF1B32"), "LDO");
    m.insert(address!("BBbbCA6A901c926F240b89EacB641d8Aec7AEafD"), "LRC");
    m.insert(address!("0F5D2fB29fb7d3CFeE444a200298f468908cC942"), "MANA");
    m.insert(address!("99D8a9C45b2ecA8864373A26D1459e3Dff1e17F3"), "MIM");
    m.insert(address!("3432B6A60D23Ca0dFCa7761B7ab56459D9C964D0"), "FXS");
    m.insert(address!("4d224452801ACEd8B2F0aebE155379bb5D594381"), "APE");
    m.insert(address!("6De037ef9aD2725EB40118Bb1702EBb27e4Aeb24"), "RNDR");
    m.insert(address!("D33526068D116cE69F19A9ee46F0bd304F21A51f"), "RPL");
    m.insert(
        address!("42981d0bfbAf196529376EE702F2a9Eb9092fcB5"),
        "SAFEMOON",
    );
    m.insert(address!("3845badAde8e6dFF049820680d1F14bD3903a5d0"), "SAND");
    m.insert(address!("77777FeDdddFfC19Ff86DB637967013e6C6A116C"), "TORN");
    m.insert(
        address!("3301Ee63Fb29F863f2333Bd4466acb46CD8323E6"),
        "AKITA",
    );
    m.insert(address!("BB0E17EF65F82Ab018d8EDd776e8DD940327B28b"), "AXS");
    m.insert(
        address!("AC57De9C1A09FeC648E93EB98875B212DB0d460B"),
        "BABYDOGE",
    );
    m
});

/// ERC20_TOKEN_DECIMALS - Decimal places for known tokens
pub static ERC20_TOKEN_DECIMALS: Lazy<HashMap<&'static str, u8>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("USDC", 6);
    m.insert("USDT", 6);
    m.insert("USD1", 18);
    m.insert("DAI", 18);
    m.insert("BUSD", 18);
    m.insert("PAX", 18);
    m.insert("FEI", 18);
    m.insert("FRAX", 18);
    m.insert("frxETH", 18);
    m.insert("LUSD", 18);
    m.insert("BRZ", 18);
    m.insert("CADC", 18);
    m.insert("DOLA", 18);
    m.insert("EUROC", 6);
    m.insert("EURCV", 18);
    m.insert("EURS", 2);
    m.insert("EURT", 6);
    m.insert("FDUSD", 18);
    m.insert("GHO", 18);
    m.insert("GUSD", 2);
    m.insert("GYEN", 6);
    m.insert("IDRT", 2);
    m.insert("JPYC", 18);
    m.insert("MIM", 18);
    m.insert("MKUSD", 18);
    m.insert("OUSD", 18);
    m.insert("PAXG", 18);
    m.insert("PYUSD", 6);
    m.insert("RAI", 18);
    m.insert("sUSD", 18);
    m.insert("TUSD", 18);
    m.insert("USD0", 18);
    m.insert("USDP", 18);
    m.insert("USDS", 6);
    m.insert("USDD", 18);
    m.insert("USDE", 18);
    m.insert("XAUt", 6);
    m.insert("XIDR", 6);
    m.insert("XSGD", 6);
    m.insert("XUSD", 6);
    m.insert("ZUSD", 6);
    m.insert("WETH", 18);
    m.insert("ETH", 18);
    m.insert("WBTC", 8);
    m.insert("WBNB", 18);
    m.insert("MATIC", 18);
    m.insert("LINK", 18);
    m.insert("UNI", 18);
    m.insert("AAVE", 18);
    m.insert("cDAI", 8);
    m.insert("cUSDC", 8);
    m.insert("cETH", 8);
    m.insert("3Crv", 18);
    m.insert("stETH-ETH", 18);
    m.insert("yvDAI", 18);
    m.insert("yvUSDC", 6);
    m.insert("stETH", 18);
    m.insert("cbETH", 18);
    m.insert("rETH", 18);
    m.insert("MKR", 18);
    m.insert("CRV", 18);
    m.insert("SNX", 18);
    m.insert("COMP", 18);
    m.insert("YFI", 18);
    m.insert("SUSHI", 18);
    m.insert("BAL", 18);
    m.insert("1INCH", 18);
    m.insert("ARB", 18);
    m.insert("OP", 18);
    m.insert("SHIB", 18);
    m.insert("PEPE", 18);
    m.insert("BAT", 18);
    m.insert("BLUR", 18);
    m.insert("BONE", 18);
    m.insert("DOGE", 8);
    m.insert("ELON", 18);
    m.insert("ENJ", 18);
    m.insert("ENS", 18);
    m.insert("FET", 18);
    m.insert("FLOKI", 9);
    m.insert("FTT", 18);
    m.insert("GALA", 8);
    m.insert("GNO", 18);
    m.insert("GRT", 18);
    m.insert("KISHU", 9);
    m.insert("LDO", 18);
    m.insert("LRC", 18);
    m.insert("MANA", 18);
    m.insert("MIM", 18);
    m.insert("FXS", 18);
    m.insert("APE", 18);
    m.insert("RNDR", 18);
    m.insert("RPL", 18);
    m.insert("SAFEMOON", 9);
    m.insert("SAND", 18);
    m.insert("TORN", 18);
    m.insert("AKITA", 18);
    m.insert("AXS", 18);
    m.insert("BABYDOGE", 9);
    m
});

/// SYMBOL_TO_ADDRESS - quick lookup for token addresses by symbol
pub static SYMBOL_TO_ADDRESS: Lazy<HashMap<&'static str, Address>> = Lazy::new(|| {
    let mut m = HashMap::new();
    for (address, symbol) in DENOM_ADDRESSES.iter() {
        m.insert(*symbol, *address);
    }
    m.insert("ETH", address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"));
    m
});

/// Retrieve token address by symbol (case-sensitive)
pub fn token_address(symbol: &str) -> Option<Address> {
    SYMBOL_TO_ADDRESS.get(symbol).copied()
}

/// Common addresses by name (routers, factories, etc.)
pub static ADDRESSES_BY_NAME: Lazy<HashMap<&'static str, Address>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(
        "zero_address",
        address!("0000000000000000000000000000000000000000"),
    );
    m.insert(
        "dead_address",
        address!("000000000000000000000000000000000000dEaD"),
    );
    m.insert("WETH", address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"));
    m.insert("ETH", address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"));
    m.insert("WBTC", address!("2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"));
    m.insert(
        "TrustSwap: Team Finance Lock",
        address!("E2fE530C047f2d85298b07D9333C05737f1435fB"),
    );
    m.insert(
        "UNCX Network Security: LP Lockers",
        address!("663A5C229c09b049E36dCc11a9B0d4a8Eb9db214"),
    );
    m.insert(
        "UNCX Network Security: Token Vesting",
        address!("Dba68f07d1b7Ca219f78ae8582C213d975c25cAf"),
    );
    m.insert(
        "UNCX Network Lockers: V3 Proof of Reserves 2",
        address!("7f5C649856F900d15C83741f45AE46f5C6858234"),
    );
    m.insert(
        "UNCX Network Lockers: V3 Proof of Reserves 3",
        address!("FD235968e65B0990584585763f837A5b5330e6DE"),
    );
    m.insert(
        "PinkLock02",
        address!("71B5759d73262FBb223956913ecF4ecC51057641"),
    );
    m.insert(
        "UniswapV2Router02",
        address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"),
    );
    m.insert(
        "UniswapV2Factory",
        address!("5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"),
    );
    m.insert(
        "UniswapV3Factory",
        address!("1F98431c8aD98523631AE4a59f267346ea31F984"),
    );
    m.insert(
        "UniswapUniversalRouter",
        address!("3fC91A3afd70395Cd496C647d5a6CC9D4B2b7FAD"),
    );
    m.insert(
        "NonfungiblePositionManager",
        address!("C36442b4a4522E871399CD717aBDD847Ab11FE88"),
    );
    m.insert(
        "WETH_USDT",
        address!("11b815efB8f581194ae79006d24E0d814B7697F6"),
    );
    m.insert(
        "Banana Gun: Router 2",
        address!("3328F7f4A1D1C57c35df56bBf0c9dCAFCA309C49"),
    );
    m.insert(
        "Banana Gun: Deployer",
        address!("BCd3a47e4d0000cf170E25d1bD3d53F7C08be0A6"),
    );
    m.insert(
        "Banana Gun: Deployer 2",
        address!("35fC556d6f8675B26fDF1542e6E894100155B34E"),
    );
    m.insert(
        "Banana Gun",
        address!("C465CC50B7D5A29b9308968f870a4B242A8e1873"),
    );
    m.insert(
        "Maestro: Router 2",
        address!("80a64c6D7f12C47B7c66c5B4E20E72bc1FCd5d9e"),
    );
    m.insert(
        "Maestro: Deployer",
        address!("6599aE06914f1f5Ec0053d3F475348D40E608442"),
    );
    m.insert(
        "Unibot",
        address!("5c9321e92Ba4eb43f2901c4952358e132163a85A"),
    );
    m.insert(
        "Sigma / Alphaman",
        address!("e76014c179F19dA26Bb30A0f085FF0A466B92829"),
    );
    m.insert(
        "Metamask: Swap Router",
        address!("881D40237659C251811CEC9c364ef91dC08D300C"),
    );
    m.insert(
        "1inch v5: Aggregation Router",
        address!("1111111254EEB25477B68fb85Ed929f73A960582"),
    );
    m.insert("BRZ", address!("01D33Fd36ec67C6adA32Cf36B31E88Ee190b1839"));
    m.insert("BUSD", address!("4fabb145d64652a948d72533023f6e7a623c7c53"));
    m.insert("CADC", address!("caDC0aCD4B445166f12D2C07EaC6E2544FbE2Eef"));
    m.insert("DAI", address!("6B175474E89094C44Da98b954EedeAC495271d0F"));
    m.insert("DOLA", address!("865377367054516e17014ccded1e7d814edc9ce4"));
    m.insert(
        "EUROC",
        address!("1aBaEA1f7C830bd89Acc67EC4af516284b1bC33c"),
    );
    m.insert(
        "EURCV",
        address!("5F7827FDeb7c20b443265Fc2F40845B715385Ff2"),
    );
    m.insert("EURS", address!("db25f211ab05b1c97d595516f45794528a807ad8"));
    m.insert("EURT", address!("C581b735A1688071A1746c968e0798D642EDE491"));
    m.insert(
        "FDUSD",
        address!("c5f0F7B66764F6EC8c8dFF7Ba683102295E16409"),
    );
    m.insert("FEI", address!("956F47F50A910163D8BF957Cf5846D573E7f87CA"));
    m.insert("FRAX", address!("853d955aCEf822Db058eb8505911ED77F175b99e"));
    m.insert("GHO", address!("40D16FC0246aD3160Ccc09B8D0D3A2cD28aE6C2f"));
    m.insert("GUSD", address!("056FD409E1d7A124BD7017459dFEa2F387B6d5Cd"));
    m.insert("GYEN", address!("C08512927D12348F6620a698105e1BAac6EcD911"));
    m.insert("IDRT", address!("998FFE1E43fAcffb941dc337dD0468d52BA5B48A"));
    m.insert("JPYC", address!("2370f9d504C7A6E775bf6E14B3F12846b594cD53"));
    m.insert("LUSD", address!("5f98805A4E8be255a32880FDeC7F6728C6568bA0"));
    m.insert(
        "MKUSD",
        address!("4591DBfF62656E7859Afe5e45f6f47D3669fBB28"),
    );
    m.insert("OUSD", address!("2A8e1E676Ec238d8A992307B495b45B3fEAa5e86"));
    m.insert("PAXG", address!("45804880De22913dAFE09f4980848ECE6EcbAf78"));
    m.insert(
        "PYUSD",
        address!("6c3EA9036406852006290770BEdFcAbA0e23A0E8"),
    );
    m.insert("RAI", address!("03ab458634910Aad20Ef5f1C8eE96f1d6Ac54919"));
    m.insert("sUSD", address!("57ab1eC28D129707052DF4DF418D58A2D46d5f51"));
    m.insert("TUSD", address!("0000000000085d4780B73119b644AE5ecd22b376"));
    m.insert("USDC", address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"));
    m.insert("USD0", address!("73a15fed60bf67631dc6cd7bc5b6e8da8190acf5"));
    m.insert("USD1", address!("C824Bf014539F6bdE6b81ABAaca0D626C2AC5985"));
    m.insert("USDP", address!("8E870D67F660D95D5be530380D0eC0bd388289E1"));
    m.insert("USDS", address!("A4BDB11dC0a2beC88d24A3AA1e6bb17201112EBE"));
    m.insert("USDT", address!("dAC17F958D2ee523a2206206994597C13D831ec7"));
    m.insert("USDD", address!("0C10BF8FCB7BF5412187A595aB97A3609160B5C6"));
    m.insert("USDE", address!("4C9EDD5852cD905F086c759e8383E09BFF1E68B3"));
    m.insert("XAUt", address!("68749665FF8D2d112Fa859AA293F07A622782F38"));
    m.insert("XIDR", address!("ebF2096E01455108bAdCbAF86cE30b6e5A72aa52"));
    m.insert("XSGD", address!("70e8dE73cE538DA2bEEd35d14187F6959a8ecA96"));
    m.insert("XUSD", address!("C08e7E23C235073C6807C2eFe7021304cB7C2815"));
    m.insert("ZUSD", address!("c56c2b7e71B54d38Aab6d52E94a04Cbfa8F604fA"));
    m.insert(
        "Binance",
        address!("001866Ae5B3de6cAa5a51543FD9fB64f524F5478"),
    );
    m.insert(
        "Binance US",
        address!("211Ee0129A67e7D44514152eB43D9f31103Ac46B"),
    );
    m.insert(
        "Binance US_1",
        address!("21d45650db732cE5dF77685d6021d7D5d1da807f"),
    );
    m.insert(
        "Binance US_2",
        address!("34ea4138580435B5A521E460035edb19Df1938c1"),
    );
    m.insert(
        "Binance US_3",
        address!("43c5b1C2bE8EF194a509cF93Eb1Ab3Dbd07B97eD"),
    );
    m.insert(
        "Binance US_4",
        address!("61189Da79177950A7272c88c6058b96d4bcD6BE2"),
    );
    m.insert(
        "Binance US_5",
        address!("9223C017a39d4806d1D92c15046Ae28c32c6D8E7"),
    );
    m.insert(
        "Binance US_6",
        address!("b14A67c63BDA5024D2EffD53Aa16A00bB7F9A30a"),
    );
    m.insert(
        "Binance US_7",
        address!("b650B0b1f183D59343dA753d94C068bFEA92693D"),
    );
    m.insert(
        "Binance US_8",
        address!("D5C08681719445A5Fdce2Bda98b341A49050d821"),
    );
    m.insert(
        "Binance US_9",
        address!("f60c2Ea62EDBfE808163751DD0d8693DCb30019c"),
    );
    m.insert(
        "Binance_1",
        address!("001cEb373C83ae75b9f5CF78Fc2aBa3e185d09E2"),
    );
    m.insert(
        "Binance_10",
        address!("141FeF8cd8397a390AFe94846c8bD6F4ab981c48"),
    );
    m.insert(
        "Binance_100",
        address!("E0F0CfDe7Ee664943906f17F7f14342E76A5CeC7"),
    );
    m.insert(
        "Binance_101",
        address!("e2fc31F816A9b94326492132018C3aEcC4a93aE1"),
    );
    m.insert(
        "Binance_102",
        address!("e7804c37c13166fF0b37F5aE0BB07A3aEbb6e245"),
    );
    m.insert(
        "Binance_103",
        address!("Eb25DF7c79a85640c4420680461DCDFD91F0dfAd"),
    );
    m.insert(
        "Binance_104",
        address!("EB2d2F1b8c558a40207669291Fda468E50c8A0bB"),
    );
    m.insert(
        "Binance_105",
        address!("ef7fb88F709aC6148C07D070BC71d252E8E13b92"),
    );
    m.insert(
        "Binance_106",
        address!("F17ACEd3c7A8DAA29ebb90Db8D1b6efD8C364a18"),
    );
    m.insert(
        "Binance_107",
        address!("f2DE20Dbf4b224Af77AA4FF446F43318800bD6b4"),
    );
    m.insert(
        "Binance_108",
        address!("F3084ed5596c3eF9fCf53689da3B998e621a34C4"),
    );
    m.insert(
        "Binance_109",
        address!("f92402bB795Fd7CD08fb83839689DB79099C8c9C"),
    );
    m.insert(
        "Binance_11",
        address!("15ecE0d7de25436bCfcF3D62A9085Ddc7838aeE9"),
    );
    m.insert(
        "Binance_110",
        address!("F977814e90dA44bFA03b6295A0616a897441aceC"),
    );
    m.insert(
        "Binance_111",
        address!("Fc19E4Ce0e0a27B09f2011eF0512669A0F76367A"),
    );
    m.insert(
        "Binance_112",
        address!("FDD2Ba77DB02Caa6a9869735dAC577d809CaDd11"),
    );
    m.insert(
        "Binance_113",
        address!("fE9e8709d3215310075d67E3ed32A380CCf451C8"),
    );
    m.insert(
        "Binance_12",
        address!("161bA15A5f335c9f06BB5BbB0A9cE14076FBb645"),
    );
    m.insert(
        "Binance_13",
        address!("1763F1A93815Ee6e6bc3C4475d31cC9570716dB2"),
    );
    m.insert(
        "Binance_14",
        address!("17B692ae403a8Ff3a3B2eD7676cF194310ddE9Af"),
    );
    m.insert(
        "Binance_15",
        address!("19184aB45C40c2920B0E0e31413b9434ABD243eD"),
    );
    m.insert(
        "Binance_16",
        address!("1B5B4e441F5A22bfd91B7772C780463F66A74b35"),
    );
    m.insert(
        "Binance_17",
        address!("1D40B233CdF2cC0CDC347d5401D5b02c2831A0c1"),
    );
    m.insert(
        "Binance_18",
        address!("1FBe2AcEe135D991592f167Ac371f3DD893A508B"),
    );
    m.insert(
        "Binance_19",
        address!("21a31Ee1afC51d94C2eFcCAa2092aD1028285549"),
    );
    m.insert(
        "Binance_2",
        address!("00799bbc833D5B168F0410312d2a8fD9e0e3079c"),
    );
    m.insert(
        "Binance_20",
        address!("25681Ab599B4E2CEea31F8B498052c53FC2D74db"),
    );
    m.insert(
        "Binance_21",
        address!("28C6c06298d514Db089934071355E5743bf21d60"),
    );
    m.insert(
        "Binance_22",
        address!("290275e3db66394C52272398959845170E4DCb88"),
    );
    m.insert(
        "Binance_23",
        address!("294B9B133cA7Bc8ED2CdD03bA661a4C6d3a834D9"),
    );
    m.insert(
        "Binance_24",
        address!("29bDfbf7D27462a2d115748ace2bd71A2646946c"),
    );
    m.insert(
        "Binance_25",
        address!("29Fe6c66097F7972d8e47c4f691576327Fcf9A12"),
    );
    m.insert(
        "Binance_26",
        address!("2E581a5aE722207Aa59aCD3939771E7c7052DD3d"),
    );
    m.insert(
        "Binance_27",
        address!("2f47A1c2Db4a3B78CDA44eADE915c3b19107DDcc"),
    );
    m.insert(
        "Binance_28",
        address!("2f7e209e0F5F645c7612D7610193Fe268F118b28"),
    );
    m.insert(
        "Binance_29",
        address!("3304E22DDaa22bCdC5fCa2269b418046aE7b566A"),
    );
    m.insert(
        "Binance_3",
        address!("01C952174C24E1210d26961D456A77A39e1F0BB0"),
    );
    m.insert(
        "Binance_30",
        address!("345D8e3A1F62eE6B1D483890976fD66168e390F2"),
    );
    m.insert(
        "Binance_31",
        address!("370b8EAad4e5970a853d25cD26a499150FD38274"),
    );
    m.insert(
        "Binance_32",
        address!("3931dAb967C3E2dbb492FE12460a66d0fe4cC857"),
    );
    m.insert(
        "Binance_33",
        address!("3c783c21a0383057D128bae431894a5C19F9Cf06"),
    );
    m.insert(
        "Binance_34",
        address!("3CDfB47b0E910d9190eD788726cD72489bf10499"),
    );
    m.insert(
        "Binance_35",
        address!("3f5CE5FBFe3E9af3971dD833D26bA9b5C936f0bE"),
    );
    m.insert(
        "Binance_36",
        address!("417850c1Cd0fB428eb63649E9DC4C78edE9A34E8"),
    );
    m.insert(
        "Binance_37",
        address!("44592b81c05b4c35Efb8424eB9D62538b949eBbF"),
    );
    m.insert(
        "Binance_38",
        address!("47ac0Fb4F2D84898e4D9E7b4DaB3C24507a6D503"),
    );
    m.insert(
        "Binance_39",
        address!("4976A4A02f38326660D17bf34b431dC6e2eb2327"),
    );
    m.insert(
        "Binance_4",
        address!("0681d8Db095565FE8A346fA0277bFfdE9C0eDBBF"),
    );
    m.insert(
        "Binance_40",
        address!("4A9E49A45A4b2545Cb177F79C7381A30e1Dc261F"),
    );
    m.insert(
        "Binance_41",
        address!("4aeFa39caEAdD662aE31ab0CE7c8C2c9c0a013E8"),
    );
    m.insert(
        "Binance_42",
        address!("4D072A68d0428A9A3054e03Ad7Ee61C557b537ab"),
    );
    m.insert(
        "Binance_43",
        address!("4D9fF50EF4dA947364BB9650892B2554e7BE5E2B"),
    );
    m.insert(
        "Binance_44",
        address!("4E9ce36E442e55EcD9025B9a6E0D88485d628A67"),
    );
    m.insert(
        "Binance_45",
        address!("50460c4CD74094CD591F455caD457e99c4AB8Be0"),
    );
    m.insert(
        "Binance_46",
        address!("505e71695E9bc45943c58adEC1650577BcA68fD9"),
    );
    m.insert(
        "Binance_47",
        address!("50d669F43b484166680Ecc3670E4766cdb0945CE"),
    );
    m.insert(
        "Binance_48",
        address!("515b72Ed8a97F42C568D6A143232775018f133C8"),
    );
    m.insert(
        "Binance_49",
        address!("564286362092D8e7936f0549571a803B203aAceD"),
    );
    m.insert(
        "Binance_5",
        address!("06a0048079ec6571Cd1b537418869CDE6191d42D"),
    );
    m.insert(
        "Binance_50",
        address!("56Eddb7aa87536c09CCc2793473599fD21A8b17F"),
    );
    m.insert(
        "Binance_51",
        address!("5a52E96BAcdaBb82fd05763E25335261B270Efcb"),
    );
    m.insert(
        "Binance_52",
        address!("5D7F34372FA8708E09689D400A613EeE67F75543"),
    );
    m.insert(
        "Binance_53",
        address!("631Fc1EA2270e98fbD9D92658eCe0F5a269Aa161"),
    );
    m.insert(
        "Binance_54",
        address!("66F791456b82921CBc3F89A98c24Ea21784973a1"),
    );
    m.insert(
        "Binance_55",
        address!("6bE5A267B04E9f24CdC1824fd38d63c436be91aB"),
    );
    m.insert(
        "Binance_56",
        address!("6D8bE5cdf0d7DEE1f04E25FD70B001AE3B907824"),
    );
    m.insert(
        "Binance_57",
        address!("708396f17127c42383E3b9014072679b2F60B82f"),
    );
    m.insert(
        "Binance_58",
        address!("73f5ebe90f27B46ea12e5795d16C4b408B19cc6F"),
    );
    m.insert(
        "Binance_59",
        address!("7a8A34DB9acD10C3b6277473b192FE47192569cA"),
    );
    m.insert(
        "Binance_6",
        address!("07B664C8aF37EdDAa7e3b6030ed1F494975e9DFB"),
    );
    m.insert(
        "Binance_60",
        address!("7Ab33AD1E91dDF6d5edf69a79D5d97a9c49015D4"),
    );
    m.insert(
        "Binance_61",
        address!("7AeD074cA56F5050D5A2E512eCc5bf7103937d76"),
    );
    m.insert(
        "Binance_62",
        address!("7DFe9A368B6Cf0C0309b763bb8d16da326e8F46e"),
    );
    m.insert(
        "Binance_63",
        address!("7E278a68A35D76A7E4b2C9d8b778aCD775C6d832"),
    );
    m.insert(
        "Binance_64",
        address!("835678a611B28684005a5e2233695fB6cbbB0007"),
    );
    m.insert(
        "Binance_65",
        address!("85b931A32a0725Be14285B66f1a22178c672d69B"),
    );
    m.insert(
        "Binance_66",
        address!("87917D879ba83CE3Ada6e02d49A10c1eC1988062"),
    );
    m.insert(
        "Binance_67",
        address!("8894E0a0c962CB723c1976a4421c95949bE2D4E3"),
    );
    m.insert(
        "Binance_68",
        address!("892e9e24AeA3f27f4C6E9360e312Cce93cc98Ebe"),
    );
    m.insert(
        "Binance_69",
        address!("8B99F3660622e21f2910ECCA7fBe51d654a1517D"),
    );
    m.insert(
        "Binance_7",
        address!("082489A616aB4D46d1947eE3F912e080815b08DA"),
    );
    m.insert(
        "Binance_70",
        address!("8F22F2063D253846B53609231eD80FA571Bc0C8F"),
    );
    m.insert(
        "Binance_71",
        address!("8f80C66C70cBC52009babB04c1CadF9b40109289"),
    );
    m.insert(
        "Binance_72",
        address!("8fF804cc2143451F454779A40DE386F913dCff20"),
    );
    m.insert(
        "Binance_73",
        address!("923fc76cB13A14e5A87843d309C9f401EC498E2d"),
    );
    m.insert(
        "Binance_74",
        address!("9430801EBAf509Ad49202aaBC5F5Bc6fd8A3dAf8"),
    );
    m.insert(
        "Binance_75",
        address!("9696f59E4d72E237BE84fFD425DCaD154Bf96976"),
    );
    m.insert(
        "Binance_76",
        address!("972Bed5493F7E7bdc760265Fbb4d8e73ea89e453"),
    );
    m.insert(
        "Binance_77",
        address!("9CD1AC952951fe63C658589db0DdE32fc55b815B"),
    );
    m.insert(
        "Binance_78",
        address!("9f8c163cBA728e99993ABe7495F06c0A3c8Ac8b9"),
    );
    m.insert(
        "Binance_79",
        address!("a180Fe01B906A1bE37BE6c534a3300785b20d947"),
    );
    m.insert(
        "Binance_8",
        address!("0b95993A39A363d99280Ac950f5E4536Ab5C5566"),
    );
    m.insert(
        "Binance_80",
        address!("a344c7aDA83113B3B56941F6e85bf2Eb425949f3"),
    );
    m.insert(
        "Binance_81",
        address!("a7C0D36c4698981FAb42a7d8c783674c6Fe2592d"),
    );
    m.insert(
        "Binance_82",
        address!("A84fD90d8640FA63D194601E0B2D1c9094297083"),
    );
    m.insert(
        "Binance_83",
        address!("AB83D182f3485cf1D6ccdd34C7CFEf95b4C08da4"),
    );
    m.insert(
        "Binance_84",
        address!("acD03D601e5bB1B275Bb94076fF46ED9D753435A"),
    );
    m.insert(
        "Binance_85",
        address!("AD9ffffd4573b642959D3B854027735579555Cbc"),
    );
    m.insert(
        "Binance_86",
        address!("B1256D6b31E4Ae87DA1D56E5890C66be7f1C038e"),
    );
    m.insert(
        "Binance_87",
        address!("b32e9A84Ae0B55b8ab715e4Ac793a61B277bAFA3"),
    );
    m.insert(
        "Binance_88",
        address!("B38e8c17e38363aF6EbdCb3dAE12e0243582891D"),
    );
    m.insert(
        "Binance_89",
        address!("B3f923eaBAF178fC1BD8E13902FC5C61D3DdEF5B"),
    );
    m.insert(
        "Binance_9",
        address!("0E4158C85FF724526233c1aeB4fF6f0C46827FbE"),
    );
    m.insert(
        "Binance_90",
        address!("BD612a3f30dcA67bF60a39Fd0D35e39B7aB80774"),
    );
    m.insert(
        "Binance_91",
        address!("BdD75A97c29294FF805FB2fEe65aBd99492b32A8"),
    );
    m.insert(
        "Binance_92",
        address!("BE0eB53F46cd790Cd13851d5EFf43D12404d33E8"),
    );
    m.insert(
        "Binance_93",
        address!("c365c3315cF926351CcAf13fA7D19c8C4058C8E1"),
    );
    m.insert(
        "Binance_94",
        address!("C3C8E0A39769e2308869f7461364ca48155D1d9E"),
    );
    m.insert(
        "Binance_95",
        address!("D551234Ae421e3BCBA99A0Da6d736074f22192FF"),
    );
    m.insert(
        "Binance_96",
        address!("d88B55467f58af508dBfDC597E8Ebd2Ad2De49b3"),
    );
    m.insert(
        "Binance_97",
        address!("d9D93951896B4eF97D251334EF2A0e39F6F6D7d7"),
    );
    m.insert(
        "Binance_98",
        address!("dccF3B77dA55107280bd850ea519DF3705D1a75a"),
    );
    m.insert(
        "Binance_99",
        address!("DFd5293D8e347dFe59E90eFd55b2956a1343963d"),
    );
    m.insert(
        "Coinbase",
        address!("00aac037C2cA137972F963693d38A57d0E9f7475"),
    );
    m.insert(
        "Coinbase Prime",
        address!("1565f0c48c06bC006095591A4c3FE4A6F39712cF"),
    );
    m.insert(
        "Coinbase Prime_1",
        address!("1E7016f7C23859d097668C27B72C170eD7129A10"),
    );
    m.insert(
        "Coinbase Prime_2",
        address!("72CEf07728B199e7aD11FF03f02D2D65F91AD553"),
    );
    m.insert(
        "Coinbase Prime_3",
        address!("A86309988947559b6E72Ef716C5058F479386C0F"),
    );
    m.insert(
        "Coinbase Prime_4",
        address!("abF7503e05a9c82726Ba6d7BBfFDfF8C2f3388c6"),
    );
    m.insert(
        "Coinbase Prime_5",
        address!("CD531Ae9EFCCE479654c4926dec5F6209531Ca7b"),
    );
    m.insert(
        "Coinbase Prime_6",
        address!("ceB69F6342eCE283b2F5c9088Ff249B5d0Ae66ea"),
    );
    m.insert(
        "Coinbase Prime_7",
        address!("DfD76BbFEB9Eb8322F3696d3567e03f894C40d6c"),
    );
    m.insert(
        "Coinbase_1",
        address!("02466E547BFDAb679fC49e96bBfc62B9747D997C"),
    );
    m.insert(
        "Coinbase_10",
        address!("14AF92363379f3548958f9de1fb2e6E5DF74476e"),
    );
    m.insert(
        "Coinbase_100",
        address!("9DCaF485e72f812DEe725Ce64B6667a7A83f73Dd"),
    );
    m.insert(
        "Coinbase_101",
        address!("9eBE8AE7DbC0285B04E93dAB86a081cA32ccf52e"),
    );
    m.insert(
        "Coinbase_102",
        address!("A090e606E30bD747d4E6245a1517EbE430F0057e"),
    );
    m.insert(
        "Coinbase_103",
        address!("A14D57f5Ea867572b0d239798D2C1Dde13153902"),
    );
    m.insert(
        "Coinbase_104",
        address!("a2908F1758d1cC3990F4A2dA8DEA0aA2ecf1b913"),
    );
    m.insert(
        "Coinbase_105",
        address!("A2B57dD51c464E863a5EFc70C8116eC46791e38f"),
    );
    m.insert(
        "Coinbase_106",
        address!("a3682Fe8fD73B90A7564585A436EC2D2AEb612eE"),
    );
    m.insert(
        "Coinbase_107",
        address!("a656f7d2A93A6F5878AA768f24eB38Ec8C827fE2"),
    );
    m.insert(
        "Coinbase_108",
        address!("A7927364E94E162102E7cF2447e33E01fc629e68"),
    );
    m.insert(
        "Coinbase_109",
        address!("a818c01C2fdf04f24f8D6BDa3512e901F33a575e"),
    );
    m.insert(
        "Coinbase_11",
        address!("1553767e6Ab6d26695B34366f61340B48d8b7a62"),
    );
    m.insert(
        "Coinbase_110",
        address!("A9D1e08C7793af67e9d92fe308d5697FB81d3E43"),
    );
    m.insert(
        "Coinbase_111",
        address!("adBbE373B5b5F72C59c0311cFfBded51f0C5F434"),
    );
    m.insert(
        "Coinbase_112",
        address!("b0fa34C866e1e1E7030820B4f846BB58d6F75b04"),
    );
    m.insert(
        "Coinbase_113",
        address!("b35D425a3F49c49181eF8e72583efB070B54202E"),
    );
    m.insert(
        "Coinbase_114",
        address!("B4807865A786E9E9E26E6A9610F2078e7fc507fB"),
    );
    m.insert(
        "Coinbase_115",
        address!("b5d85CBf7cB3EE0D56b3bB207D5Fc4B82f43F511"),
    );
    m.insert(
        "Coinbase_116",
        address!("B624219480543C54603fb6b07d5eb347E51bffe0"),
    );
    m.insert(
        "Coinbase_117",
        address!("b739D0895772DBB71A89A3754A160269068f0D45"),
    );
    m.insert(
        "Coinbase_118",
        address!("b8487eeD31Cf5C559BF3f4eDD166b949553D0d11"),
    );
    m.insert(
        "Coinbase_119",
        address!("Bc8Ec259E3026aE0D87bc442D034d6882ce4a35C"),
    );
    m.insert(
        "Coinbase_12",
        address!("1846DeB34cc19c11e7DdF0de48B13FD1F231Ca7F"),
    );
    m.insert(
        "Coinbase_120",
        address!("BE3c68821D585Cf1552214897a1c091014B1EB0a"),
    );
    m.insert(
        "Coinbase_121",
        address!("C070A61D043189D99bbf4baA58226bf0991c7b11"),
    );
    m.insert(
        "Coinbase_122",
        address!("c7bf35C9A3Bdd1B1c19A6963De669cb45191A019"),
    );
    m.insert(
        "Coinbase_123",
        address!("C8373EDFaD6d5C5f600b6b2507F78431C5271fF5"),
    );
    m.insert(
        "Coinbase_124",
        address!("c9AAA6cA0e05B87d53A3E51Edbc44b406EEaF299"),
    );
    m.insert(
        "Coinbase_125",
        address!("c9ebC59a7590E52B0904817f172AC82fc66a530b"),
    );
    m.insert(
        "Coinbase_126",
        address!("cad50671877Eb564d42DdD76b5Fec46ac60EF1BD"),
    );
    m.insert(
        "Coinbase_127",
        address!("Ce352e98934499be70F641353f16A47D9E1E3aBd"),
    );
    m.insert(
        "Coinbase_128",
        address!("cF63Fc571aDceC4FD4A750ecACC3af1f5b748101"),
    );
    m.insert(
        "Coinbase_129",
        address!("D34EA7278e6BD48DefE656bbE263aEf11101469c"),
    );
    m.insert(
        "Coinbase_13",
        address!("1985EA6E9c68E1C272d8209f3B478AC2Fdb25c87"),
    );
    m.insert(
        "Coinbase_130",
        address!("D451e3919950963e9C1CA2F78A987DBD7937C0FB"),
    );
    m.insert(
        "Coinbase_131",
        address!("d5c41FD4a31Eaaf5559FfCC60Ec051fcB8eCC375"),
    );
    m.insert(
        "Coinbase_132",
        address!("D688AEA8f7d450909AdE10C47FaA95707b0682d9"),
    );
    m.insert(
        "Coinbase_133",
        address!("d6974E28eCee4005a4d03c4C6ED0Aac71fb46b94"),
    );
    m.insert(
        "Coinbase_134",
        address!("D69B42d93aF96Cf278b1149bFfB2D668D0154B7d"),
    );
    m.insert(
        "Coinbase_135",
        address!("D839C179a4606F46abD7A757f7Bb77D7593aE249"),
    );
    m.insert(
        "Coinbase_136",
        address!("ddfAbCdc4D8FfC6d5beaf154f18B778f892A0740"),
    );
    m.insert(
        "Coinbase_137",
        address!("dF61e3281FB5B1C3Da3fF92865875D7751aD830B"),
    );
    m.insert(
        "Coinbase_138",
        address!("e04Cf52e9Fafa3D9bF14c407AFfF94165EF835f7"),
    );
    m.insert(
        "Coinbase_139",
        address!("E0E65c40BCB1225D4aB3a13f3a9E21Abbd83F9d0"),
    );
    m.insert(
        "Coinbase_14",
        address!("19aB546E77d0cD3245B2AAD46bd80dc4707d6307"),
    );
    m.insert(
        "Coinbase_140",
        address!("e1597DF1F0E1920F7a296CeF27babB40BaEeabFC"),
    );
    m.insert(
        "Coinbase_141",
        address!("E1A0DDeb9b5b55E489977b438764e60e314E917c"),
    );
    m.insert(
        "Coinbase_142",
        address!("e3aaC971590635F601Ea751096f11343C70ebaDF"),
    );
    m.insert(
        "Coinbase_143",
        address!("e3D6d8BCDC4Eb4e24aD7523D98C394960e8C1d32"),
    );
    m.insert(
        "Coinbase_144",
        address!("e49916B3ff411fC6A83dC31f130E2e85Be4a9385"),
    );
    m.insert(
        "Coinbase_145",
        address!("E68Ee8A12c611fd043fB05d65E1548dC1383f2b9"),
    );
    m.insert(
        "Coinbase_146",
        address!("E7Ee701BdAA5b446C985BFeCC8933f3E5eeed867"),
    );
    m.insert(
        "Coinbase_147",
        address!("E86F3aaA57F63B2AfeCA68178182a91bC3909962"),
    );
    m.insert(
        "Coinbase_148",
        address!("eB2629a2734e272Bcc07BDA959863f316F4bD4Cf"),
    );
    m.insert(
        "Coinbase_149",
        address!("EbA20D0f74ECc13130579d21bB53D63C96258652"),
    );
    m.insert(
        "Coinbase_15",
        address!("19d599012788b991FF542F31208bAB21Ea38403E"),
    );
    m.insert(
        "Coinbase_150",
        address!("EDc7001e99a37c3D23b5f7974F837387e09f9C93"),
    );
    m.insert(
        "Coinbase_151",
        address!("ee81B5Afc73Cf528778E0ED98622e434E5eFADb4"),
    );
    m.insert(
        "Coinbase_152",
        address!("F0eeFa46ee509eB8fAB42F62917db8a93432E2ef"),
    );
    m.insert(
        "Coinbase_153",
        address!("F27182c5568beAFb967140286E25807250EacC4C"),
    );
    m.insert(
        "Coinbase_154",
        address!("F27daFf52c38b2c373Ad2B9392652DdF433303c4"),
    );
    m.insert(
        "Coinbase_155",
        address!("F2f07ef4a923F48fC4D47F0Af115F8895177F075"),
    );
    m.insert(
        "Coinbase_156",
        address!("f491d040110384DBcf7F241fFE2A546513fD873d"),
    );
    m.insert(
        "Coinbase_157",
        address!("FF9FBB429C186029c28f1e30361271EA002847AD"),
    );
    m.insert(
        "Coinbase_16",
        address!("1b4C15b991543a6082213a88276e7a83c9985676"),
    );
    m.insert(
        "Coinbase_17",
        address!("204ad5bed7eb66e002329A018BB96d87a8dB1AA8"),
    );
    m.insert(
        "Coinbase_18",
        address!("20FE51A9229EEf2cF8Ad9E89d91CAb9312cF3b7A"),
    );
    m.insert(
        "Coinbase_19",
        address!("21bD501F86A0B5cE0907651Df3368DA905B300A9"),
    );
    m.insert(
        "Coinbase_2",
        address!("02d24cAB4f2c3Bf6e6EB07ea07e45F96baccFfE7"),
    );
    m.insert(
        "Coinbase_20",
        address!("221cEc08C3DF34763eed468705DC779Cbe750ceb"),
    );
    m.insert(
        "Coinbase_21",
        address!("251e93d51c5F2A1e60b7BC90BC8B2534b68e8f40"),
    );
    m.insert(
        "Coinbase_22",
        address!("27724B0d4fb98A89a092E6a4ADbC09154c182637"),
    );
    m.insert(
        "Coinbase_23",
        address!("281055Afc982d96fAB65b3a49cAc8b878184Cb16"),
    );
    m.insert(
        "Coinbase_24",
        address!("28C5B0445d0728bc25f143f8EbA5C5539fAe151A"),
    );
    m.insert(
        "Coinbase_25",
        address!("28E71d0b7f7f29106a1bE2A5B289cab331E7B56f"),
    );
    m.insert(
        "Coinbase_26",
        address!("292BF41E2506f88Aa3E73721Aabc75B4f08e664e"),
    );
    m.insert(
        "Coinbase_27",
        address!("2a410f11A6F520398447bF423DceDd25DFd3a568"),
    );
    m.insert(
        "Coinbase_28",
        address!("2bDCDa44D935C12c3a76972C4339975782842a12"),
    );
    m.insert(
        "Coinbase_29",
        address!("2cc5146929A893D1d73BC34Fb37815cC1a44ae33"),
    );
    m.insert(
        "Coinbase_3",
        address!("04D4876932C7C375efcaEB7aE0AD00591acF09F6"),
    );
    m.insert(
        "Coinbase_30",
        address!("333d17d3B42bf7930Dbc6e852cA7Bcf560A69003"),
    );
    m.insert(
        "Coinbase_31",
        address!("336307F2d8390035Ba926a61a86b45CA9dC91E57"),
    );
    m.insert(
        "Coinbase_32",
        address!("33AE106bc06EffA33e6dB5813b619710f2dfb5d4"),
    );
    m.insert(
        "Coinbase_33",
        address!("38024b883C72F23D6d6c1f3f869034e849e8b121"),
    );
    m.insert(
        "Coinbase_34",
        address!("382fFCe2287252F930E1C8DC9328dac5BF282bA1"),
    );
    m.insert(
        "Coinbase_35",
        address!("3C070dcEaC7f99AeC493De5b56619D71fa31E131"),
    );
    m.insert(
        "Coinbase_36",
        address!("3cD751E6b0078Be393132286c442345e5DC49699"),
    );
    m.insert(
        "Coinbase_37",
        address!("3D2e397F94e415D7773E72e44D5B5338a99E77d9"),
    );
    m.insert(
        "Coinbase_38",
        address!("3DC474a2A65507f32b05C5f80D852515B25b2134"),
    );
    m.insert(
        "Coinbase_39",
        address!("3DD1D15b3c78d6aCFD75a254e857Cbe5b9fF0aF2"),
    );
    m.insert(
        "Coinbase_4",
        address!("05e3a758FdD29d28435019ac453297eA37b61b62"),
    );
    m.insert(
        "Coinbase_40",
        address!("3DD87411a3754deea8cc52C4CF57E2fC254924Cc"),
    );
    m.insert(
        "Coinbase_41",
        address!("3f1137CF8e6468669959C3b99a9250A400a76574"),
    );
    m.insert(
        "Coinbase_42",
        address!("40EbC1Ac8d4Fedd2E144b75fe9C0420BE82750c6"),
    );
    m.insert(
        "Coinbase_43",
        address!("441CACfD43856409b163B90e094BB42aeb70a70e"),
    );
    m.insert(
        "Coinbase_44",
        address!("46Ce00Ec0866e7A045D2F2778558efee05c4fB08"),
    );
    m.insert(
        "Coinbase_45",
        address!("47D43AC7EeA761C4959B2a4318e620603bE7Fd4F"),
    );
    m.insert(
        "Coinbase_46",
        address!("4a4E859565D9B563afC8e63641542455cff0dFD2"),
    );
    m.insert(
        "Coinbase_47",
        address!("4a797A6fA930118b9BC267911751241B8c514fAA"),
    );
    m.insert(
        "Coinbase_48",
        address!("4B23d52eFf7C67F5992C2aB6D3f69b13a6a33561"),
    );
    m.insert(
        "Coinbase_49",
        address!("4D8336bDa6C11BD2a805C291Ec719BaeDD10AcB9"),
    );
    m.insert(
        "Coinbase_5",
        address!("0a74dB66f8248554D406832E305555043fE3BfD7"),
    );
    m.insert(
        "Coinbase_50",
        address!("4F86D1d365434bfBC1E818534d353FfC1A06F8Fe"),
    );
    m.insert(
        "Coinbase_51",
        address!("4FD166478B440FB2a0BC64321Ec35eD48F9CDB16"),
    );
    m.insert(
        "Coinbase_52",
        address!("503828976D22510aad0201ac7EC88293211D23Da"),
    );
    m.insert(
        "Coinbase_53",
        address!("5122E9AA635C13AFD2fc31De3953E0896bac7aB4"),
    );
    m.insert(
        "Coinbase_54",
        address!("55c4bf24450ecB6F818f6Be5881A4fc491dEb01e"),
    );
    m.insert(
        "Coinbase_55",
        address!("55E6513DBCdD1dCAf4c6710AC423C698a56Eb68A"),
    );
    m.insert(
        "Coinbase_56",
        address!("563537412ad5D49fAA7FA442b9193B8238d98c3c"),
    );
    m.insert(
        "Coinbase_57",
        address!("57a7560D0eC28065762203c0d633943298eaC7C0"),
    );
    m.insert(
        "Coinbase_58",
        address!("5FfC99B5B23c5aB8f463F6090342879c286a29bE"),
    );
    m.insert(
        "Coinbase_59",
        address!("607094ed3a8361bB5e94dD21bcBef2997b687478"),
    );
    m.insert(
        "Coinbase_6",
        address!("0Af5035a8dFf53F5a89bfC083FdEC94C7332aD7A"),
    );
    m.insert(
        "Coinbase_60",
        address!("60EB0250E3A428A51ccb1E44E0aadBD1fD213Ff3"),
    );
    m.insert(
        "Coinbase_61",
        address!("6321F9F02D9d56261c8C79131aE74D7b427ccAF5"),
    );
    m.insert(
        "Coinbase_62",
        address!("660629fe43b0825ACF3402E10a999C563075A32c"),
    );
    m.insert(
        "Coinbase_63",
        address!("67857eE12929E74082f1cAe64eF4221830c39113"),
    );
    m.insert(
        "Coinbase_64",
        address!("6AAC5E7C12D3C9259cff8E10b5bBdB1064C382a5"),
    );
    m.insert(
        "Coinbase_65",
        address!("6b76F8B1e9E59913BfE758821887311bA1805cAB"),
    );
    m.insert(
        "Coinbase_66",
        address!("6c37a68DBB44c3F437D15Dc405c67BF5778b8637"),
    );
    m.insert(
        "Coinbase_67",
        address!("6c8dd0e9cC58c07429e065178d88444B60e60b80"),
    );
    m.insert(
        "Coinbase_68",
        address!("6dcBCe46a8B494c885D0e7b6817d2b519dF64467"),
    );
    m.insert(
        "Coinbase_69",
        address!("6F52730DBA7B02beeFcAF0D6998c9AE901Ea04f9"),
    );
    m.insert(
        "Coinbase_7",
        address!("0B0A5886664376F59C351ba3f598C8A8B4D0A6f3"),
    );
    m.insert(
        "Coinbase_70",
        address!("707e8eaf4C1586fea01F8fd0242DaaC009d4f60E"),
    );
    m.insert(
        "Coinbase_71",
        address!("71660c4005BA85c37ccec55d0C4493E66Fe775d3"),
    );
    m.insert(
        "Coinbase_72",
        address!("731307f3B12cC56191aDE83ea630a377D9a941F6"),
    );
    m.insert(
        "Coinbase_73",
        address!("739120AdE7ED878FcA5bbDB806263a8258FE2360"),
    );
    m.insert(
        "Coinbase_74",
        address!("75590f473d2A7b377c1Ba991790900f28532f329"),
    );
    m.insert(
        "Coinbase_75",
        address!("760DcE7eA6e8BA224BFFBEB8a7ff4Dd1Ef122BfF"),
    );
    m.insert(
        "Coinbase_76",
        address!("76a1B88Be943BC5dC6c9b641A0835970a2cC2dc4"),
    );
    m.insert(
        "Coinbase_77",
        address!("77696bb39917C91A0c3908D577d5e322095425cA"),
    );
    m.insert(
        "Coinbase_78",
        address!("7830c87C02e56AFf27FA8Ab1241711331FA86F43"),
    );
    m.insert(
        "Coinbase_79",
        address!("7c195D981AbFdC3DDecd2ca0Fed0958430488e34"),
    );
    m.insert(
        "Coinbase_8",
        address!("0Bf5Ec06f44E9AA068732B1c203F007855A5EEd8"),
    );
    m.insert(
        "Coinbase_80",
        address!("7C310a03f4CFa19F7f3d7F36DD3E05828629fa78"),
    );
    m.insert(
        "Coinbase_81",
        address!("7c41FDceD2Ea646eD85665D1a9b28e6632b61c41"),
    );
    m.insert(
        "Coinbase_82",
        address!("7eD53F6E3dE6B2b4156FA8E618506E60D8E65843"),
    );
    m.insert(
        "Coinbase_83",
        address!("80cF6275294bddcE597789c855691cF4CDF01386"),
    );
    m.insert(
        "Coinbase_84",
        address!("8196f70b2c17Ba58d8Ef56AD62087Ee8231be33a"),
    );
    m.insert(
        "Coinbase_85",
        address!("829E3c7781a6AC8Cd864Cd8437a664Ec07DA75a8"),
    );
    m.insert(
        "Coinbase_86",
        address!("8535d5Ed435e405a881545FFb2f8D6B81F6c9d41"),
    );
    m.insert(
        "Coinbase_87",
        address!("881D4032abe4188e2237eFCD27aB435E81FC6bb1"),
    );
    m.insert(
        "Coinbase_88",
        address!("898F293E97d59b375939eC3654ac548be2759d41"),
    );
    m.insert(
        "Coinbase_89",
        address!("8af8485e1F178be06386CD3877Fde20626e0284F"),
    );
    m.insert(
        "Coinbase_9",
        address!("122fDD9fEcbc82F7d4237C0549a5057E31c8EF8D"),
    );
    m.insert(
        "Coinbase_90",
        address!("90E18a6920985DBACc3d76Cf27a3F2131923C720"),
    );
    m.insert(
        "Coinbase_91",
        address!("90e63c3d53E0Ea496845b7a03ec7548B70014A91"),
    );
    m.insert(
        "Coinbase_92",
        address!("91d66b38ae24292e9e12dd962bBB3AEcF4Ab769A"),
    );
    m.insert(
        "Coinbase_93",
        address!("93745FEEb3C42F6aB0f9890CbCe65B360145b4ab"),
    );
    m.insert(
        "Coinbase_94",
        address!("95A9bd206aE52C4BA8EecFc93d18EACDd41C88CC"),
    );
    m.insert(
        "Coinbase_95",
        address!("95f90ce2e3abaeD29eEEbDb42E1FdB146e0F848a"),
    );
    m.insert(
        "Coinbase_96",
        address!("9620e530995D4B497F16652419dBB60f7834010A"),
    );
    m.insert(
        "Coinbase_97",
        address!("9810762578aCCF1F314320CCa5B72506aE7D7630"),
    );
    m.insert(
        "Coinbase_98",
        address!("9a1eD80eBc9936ceE2d3DB944Ee6bD8D407e7f9F"),
    );
    m.insert(
        "Coinbase_99",
        address!("9b4Fc9E22b46487F0810eF5dFa230b9f139E5179"),
    );
    m.insert(
        "Kraken",
        address!("012480c08d20a14CF3Cb495e942a94dd926DCc8f"),
    );
    m.insert(
        "Kraken_1",
        address!("098cAE2DEBceDCeDcAF71e43C1c055C0Ec369492"),
    );
    m.insert(
        "Kraken_10",
        address!("24B4eAE904632c53ee231e3bd6C4444745Ce22c0"),
    );
    m.insert(
        "Kraken_11",
        address!("267be1C1D684F78cb4F6a176C4911b741E4Ffdc0"),
    );
    m.insert(
        "Kraken_12",
        address!("26a78D5b6d7a7acEEDD1e6eE3229b372A624d8b7"),
    );
    m.insert(
        "Kraken_13",
        address!("2910543Af39abA0Cd09dBb2D50200b3E800A63D2"),
    );
    m.insert(
        "Kraken_14",
        address!("29728D0efd284D85187362fAA2d4d76C2CfC2612"),
    );
    m.insert(
        "Kraken_15",
        address!("2a62C4aCcA1A166Ee582877112682cAe8Cc0ffe7"),
    );
    m.insert(
        "Kraken_16",
        address!("2C7C03cF85ec621bF997E425f550A6683D6d60f3"),
    );
    m.insert(
        "Kraken_17",
        address!("2d070ed1321871841245D8EE5B84bD2712644322"),
    );
    m.insert(
        "Kraken_18",
        address!("3FF7215004Fea03c2C745E0476e3f412050e04D1"),
    );
    m.insert(
        "Kraken_19",
        address!("43984D578803891dfa9706bDEee6078D80cFC79E"),
    );
    m.insert(
        "Kraken_2",
        address!("098cbdd8eb01b19D37539644821772e9bdE12D55"),
    );
    m.insert(
        "Kraken_20",
        address!("4442c3E6B5f22B8b4dc3c9329be6c850C5779E85"),
    );
    m.insert(
        "Kraken_21",
        address!("490b1E689Ca23be864e55B46bf038e007b528208"),
    );
    m.insert(
        "Kraken_22",
        address!("491f5512751F5dB45c5415049D423aFfAaE70392"),
    );
    m.insert(
        "Kraken_23",
        address!("4B6f17856215eab57c29ebfA18B0a0F74A3627bb"),
    );
    m.insert(
        "Kraken_24",
        address!("4C6007e38Ce164Ed80FF8Ff94192225FcdAC68CD"),
    );
    m.insert(
        "Kraken_25",
        address!("52F5F2adD61c835ff10550402A46621EBd1071D5"),
    );
    m.insert(
        "Kraken_26",
        address!("53aB4a93B31F480d17D3440a6329bDa86869458A"),
    );
    m.insert(
        "Kraken_27",
        address!("53d284357ec70cE289D6D64134DfAc8E511c8a3D"),
    );
    m.insert(
        "Kraken_28",
        address!("555e179d64335945Fc6B155B7235a31B0a595542"),
    );
    m.insert(
        "Kraken_29",
        address!("62ac55b745F9B08F1a81DCbbE630277095Cf4Be1"),
    );
    m.insert(
        "Kraken_3",
        address!("0A869d79a7052C7f1b55a8EbAbbEa3420F0D1E13"),
    );
    m.insert(
        "Kraken_30",
        address!("66c57bF505A85A74609D2C83E94Aabb26d691E1F"),
    );
    m.insert(
        "Kraken_31",
        address!("6d0Cf1F651f5Ae585d24DcaA188d44E389E93D26"),
    );
    m.insert(
        "Kraken_32",
        address!("7217F8A697713f6F7DE06CFcD80B76A2CbA375f0"),
    );
    m.insert(
        "Kraken_33",
        address!("72e93123e8b5D168246739CDC45360ea11209364"),
    );
    m.insert(
        "Kraken_34",
        address!("735FD3c55A8be1aEb3544C7e29eBa3ea23500A1C"),
    );
    m.insert(
        "Kraken_35",
        address!("79990a901281bEe059BB3F4D7Db477F7495e2049"),
    );
    m.insert(
        "Kraken_36",
        address!("808E5374106E820aE54662Fcf8a5e3CCA6aFA13D"),
    );
    m.insert(
        "Kraken_37",
        address!("89e51fA8CA5D66cd220bAed62ED01e8951aa7c40"),
    );
    m.insert(
        "Kraken_38",
        address!("8a108e4761386c94b8d2f98A5fFe13E472cFE76a"),
    );
    m.insert(
        "Kraken_39",
        address!("8aF3827a41c26C7F32C81E93bb66e837e0210D5c"),
    );
    m.insert(
        "Kraken_4",
        address!("0E33Be39B13c576ff48E14392fBf96b02F40Cd34"),
    );
    m.insert(
        "Kraken_40",
        address!("8dFeE7DbD859989bF3E80c4f64c42b7Cd283671b"),
    );
    m.insert(
        "Kraken_41",
        address!("8f9c79B9De8b0713dCAC3E535fc5A1A92DB6EA2D"),
    );
    m.insert(
        "Kraken_42",
        address!("92927a664c88449318E14D0fD582c787AE2cd934"),
    );
    m.insert(
        "Kraken_43",
        address!("9c2bd617b77961ee2c5e3038dFb0c822cb75d82a"),
    );
    m.insert(
        "Kraken_44",
        address!("9Da5812111DCBD65fF9b736874a89751A4F0a2F8"),
    );
    m.insert(
        "Kraken_45",
        address!("a054611c5B224a5F4ce93Ac2A5f8d0Ed17813402"),
    );
    m.insert(
        "Kraken_46",
        address!("A1CDBC1a4178c17116Bdb56C946e8B0757C8dcec"),
    );
    m.insert(
        "Kraken_47",
        address!("a24787320ede4CC19D800bf87B41Ab9539c4dA9D"),
    );
    m.insert(
        "Kraken_48",
        address!("A25aA6DFBf6d9bbd7a6A9eb47B9f1e57a2BD92d7"),
    );
    m.insert(
        "Kraken_49",
        address!("A2f443492bbB1b041FCEee5194e2bc133ecC6407"),
    );
    m.insert(
        "Kraken_5",
        address!("0eF6AEB825dc4c9983d551F8aFEfaAE9d79165C6"),
    );
    m.insert(
        "Kraken_50",
        address!("a40dFEE99E1C85DC97Fdc594b16A460717838703"),
    );
    m.insert(
        "Kraken_51",
        address!("a4A6A282A7fC7F939e01D62D884355d79f5046C1"),
    );
    m.insert(
        "Kraken_52",
        address!("A83B11093c858c86321FBc4c20FE82cdbd58E09E"),
    );
    m.insert(
        "Kraken_53",
        address!("a861678beE80035114B47615142e9302139a8c32"),
    );
    m.insert(
        "Kraken_54",
        address!("AdaE2f3B0dB76cb3eaFe76A8Bf99b93f099C140a"),
    );
    m.insert(
        "Kraken_55",
        address!("Ae2D4617c862309A3d75A0fFB358c7a5009c673F"),
    );
    m.insert(
        "Kraken_56",
        address!("b874005cbEa25C357b31C62145b3AEF219d105CF"),
    );
    m.insert(
        "Kraken_57",
        address!("c6bed363b30DF7F35b601a5547fE56cd31Ec63DA"),
    );
    m.insert(
        "Kraken_58",
        address!("cD0267c7F1A8Ad1b6e33B7fB801F8D935F6B557D"),
    );
    m.insert(
        "Kraken_59",
        address!("CdC8488E63A403BfD580222ea0F3719477bfea9C"),
    );
    m.insert(
        "Kraken_6",
        address!("10593a64B7b7BB0Ea29B8c01F1619ca8fF294b2F"),
    );
    m.insert(
        "Kraken_60",
        address!("cE27fC71139d02f9A3D5Cc1356Add185750660Ac"),
    );
    m.insert(
        "Kraken_61",
        address!("D0AD6ed2B2920a5744a064af3d585Ee54F528B2F"),
    );
    m.insert(
        "Kraken_62",
        address!("D4039ECC40AedA0582036437cf3ec02845DA4C13"),
    );
    m.insert(
        "Kraken_63",
        address!("d88545d0034C245857d1523bb4e8686BcED9Bb85"),
    );
    m.insert(
        "Kraken_64",
        address!("DA9dfA130Df4dE4673b89022EE50ff26f6EA73Cf"),
    );
    m.insert(
        "Kraken_65",
        address!("e6a02eeFC2612b13f2B3B914009576ce5495Ec0e"),
    );
    m.insert(
        "Kraken_66",
        address!("E7178aD747f2C12aB1F8332E61Cf6E756815D5C6"),
    );
    m.insert(
        "Kraken_67",
        address!("e84F75FC9cAA49876d0Ba18d309da4231d44E94D"),
    );
    m.insert(
        "Kraken_68",
        address!("e850b7c87F66371035e184C72d4B99e7b2ca4865"),
    );
    m.insert(
        "Kraken_69",
        address!("E853c56864A2ebe4576a807D26Fdc4A0adA51919"),
    );
    m.insert(
        "Kraken_7",
        address!("16B2b042f15564Bb8585259f535907F375Bdc415"),
    );
    m.insert(
        "Kraken_70",
        address!("e9f7eCAe3A53D2A67105292894676b00d1FaB785"),
    );
    m.insert(
        "Kraken_71",
        address!("EA578Aae84b156010b5df7759a08Cf6E6D6fC288"),
    );
    m.insert(
        "Kraken_72",
        address!("f1f7648f81F5219C36d75D24D33811f16B426DBe"),
    );
    m.insert(
        "Kraken_73",
        address!("F7Cd385CB9a442358B892B14301F6310e57CC5c9"),
    );
    m.insert(
        "Kraken_74",
        address!("Fa52274DD61E1643d2205169732f29114BC240b3"),
    );
    m.insert(
        "Kraken_75",
        address!("fCAF9c57C26566f96D23F585950Bb1c66E138890"),
    );
    m.insert(
        "Kraken_8",
        address!("16b34756653f88a89005E96C0622832D8fB6b0B5"),
    );
    m.insert(
        "Kraken_9",
        address!("1F7bc4dA1a0c2e49d7eF542F74CD46a3FE592cb1"),
    );
    m.insert(
        "Bitfinex",
        address!("0b73F67A49273fc4B9A65DBD25D7d0918E734E63"),
    );
    m.insert(
        "Bitfinex_1",
        address!("0cD76cD43992C665FdC2d8aC91B935CA3165E782"),
    );
    m.insert(
        "Bitfinex_10",
        address!("482F02e8BC15b5EAbC52C6497b425B3Ca3c821E8"),
    );
    m.insert(
        "Bitfinex_11",
        address!("4fdd5Eb2FB260149A3903859043e962Ab89D8ED4"),
    );
    m.insert(
        "Bitfinex_12",
        address!("53B36141490c419fa27ecabFEB8Be1ecAdc82431"),
    );
    m.insert(
        "Bitfinex_13",
        address!("5754284f345afc66a98fbB0a0Afe71e0F007B949"),
    );
    m.insert(
        "Bitfinex_14",
        address!("58AE42A38D6b33A1E31492B60465fA80dA595755"),
    );
    m.insert(
        "Bitfinex_15",
        address!("59448fe20378357F206880c58068f095ae63d5A5"),
    );
    m.insert(
        "Bitfinex_16",
        address!("5a710a3cDF2AF218740384c52a10852D8870626A"),
    );
    m.insert(
        "Bitfinex_17",
        address!("618F37D7ff7B140E604172466CD42D1Ec35E0544"),
    );
    m.insert(
        "Bitfinex_18",
        address!("7180EB39A6264938FDB3EfFD7341C4727c382153"),
    );
    m.insert(
        "Bitfinex_19",
        address!("742d35Cc6634C0532925a3b844Bc454e4438f44e"),
    );
    m.insert(
        "Bitfinex_2",
        address!("1151314c646Ce4E0eFD76d1aF4760aE66a9Fe30F"),
    );
    m.insert(
        "Bitfinex_20",
        address!("77134cbC06cB00b66F4c7e623D5fdBF6777635EC"),
    );
    m.insert(
        "Bitfinex_21",
        address!("7727E5113D1d161373623e5f49FD568B4F543a9E"),
    );
    m.insert(
        "Bitfinex_22",
        address!("8103683202aa8DA10536036EDef04CDd865C225E"),
    );
    m.insert(
        "Bitfinex_23",
        address!("876EabF441B2EE5B5b0554Fd502a8E0600950cFa"),
    );
    m.insert(
        "Bitfinex_24",
        address!("88037f361891A0B5De3C0C30632fBcA7DB2D341F"),
    );
    m.insert(
        "Bitfinex_25",
        address!("ab7c74abC0C4d48d1bdad5DCB26153FC8780f83E"),
    );
    m.insert(
        "Bitfinex_26",
        address!("C56fEFd1028B0534bfaDCdB580d3519b5586246E"),
    );
    m.insert(
        "Bitfinex_27",
        address!("C58B32218A746B70813A057275591966deb5920e"),
    );
    m.insert(
        "Bitfinex_28",
        address!("cAfB10eE663f465f9d10588AC44eD20eD608C11e"),
    );
    m.insert(
        "Bitfinex_29",
        address!("dcD0272462140D0A3cEd6C4bf970c7641f08CD2c"),
    );
    m.insert(
        "Bitfinex_3",
        address!("1b29DD8fF0EB3240238bF97CaFD6edeA05D5Ba82"),
    );
    m.insert(
        "Bitfinex_30",
        address!("E92d1A43df510F82C66382592a047d288f85226f"),
    );
    m.insert(
        "Bitfinex_31",
        address!("Ed9Eef56E64A8E779cFaE9ddEDb25d11Ba2B2425"),
    );
    m.insert(
        "Bitfinex_32",
        address!("f4B51B14b9EE30dc37EC970B50a486F37686E2a8"),
    );
    m.insert(
        "Bitfinex_4",
        address!("1B8766d041567EeD306940c587e21C06aB968663"),
    );
    m.insert(
        "Bitfinex_5",
        address!("28140CB1AC771d4Add91eE23788E50249C10263d"),
    );
    m.insert(
        "Bitfinex_6",
        address!("2EE3B2dF6534abc759ffE994f7b8DcDFAa02cd31"),
    );
    m.insert(
        "Bitfinex_7",
        address!("30a2EBF10f34c6C4874b0bDD5740690fD2f3B70C"),
    );
    m.insert(
        "Bitfinex_8",
        address!("36a85757645E8e8AeC062a1dEE289c7d615901Ca"),
    );
    m.insert(
        "Bitfinex_9",
        address!("3F7E77B627676763997344a1AD71aCb765fc8aC5"),
    );
    m.insert("OKX", address!("03aE1A796DFE0400439211133D065BDA774B9D3e"));
    m.insert(
        "OKX_1",
        address!("0475Dd0e4194422A8Cac486Dc69173F535D0baF4"),
    );
    m.insert(
        "OKX_10",
        address!("12A8BDC0470ab29a229D828526641b7d1f170fcF"),
    );
    m.insert(
        "OKX_100",
        address!("9e64aFC7bca5F2C6607a5B8c378bCF1EC3531C97"),
    );
    m.insert(
        "OKX_101",
        address!("A16F524a804BEaED0d791De0aa0b5836295A2a84"),
    );
    m.insert(
        "OKX_102",
        address!("a2684F75740cFFF46c29bAe79a4eCc43043C003D"),
    );
    m.insert(
        "OKX_103",
        address!("a27CEF8aF2B6575903b676e5644657FAe96F491F"),
    );
    m.insert(
        "OKX_104",
        address!("A7EFAe728D2936e78BDA97dc267687568dD593f3"),
    );
    m.insert(
        "OKX_105",
        address!("aad8AD7DfA05Bc354e011890dd61636842c2Cb96"),
    );
    m.insert(
        "OKX_106",
        address!("aE0CBABa071D58EFc278A815B2Cb652286e192ff"),
    );
    m.insert(
        "OKX_107",
        address!("B072b7DC9521d97a3f12b04bEb1e497f8875eC52"),
    );
    m.insert(
        "OKX_108",
        address!("b47A0AC7798d8308467b2b96aC632ED43c9CB6d7"),
    );
    m.insert(
        "OKX_109",
        address!("B4eC508ADEB174610B4295E233A458b3475964f7"),
    );
    m.insert(
        "OKX_11",
        address!("16b5016803bcB4915701EFC0a3B471Bd4c168d93"),
    );
    m.insert(
        "OKX_110",
        address!("B640e1e5A5f726A054Cd518C968DFDac8C421eE5"),
    );
    m.insert(
        "OKX_111",
        address!("b8351B61Fa1Eb007A9f80144C489d513e6A76b14"),
    );
    m.insert(
        "OKX_112",
        address!("B8B0b53b387B061Af2717D642961Af405c79e85A"),
    );
    m.insert(
        "OKX_113",
        address!("b9656d0393015f92bA642C3A344061E8E2478599"),
    );
    m.insert(
        "OKX_114",
        address!("b99CC7e10Fe0Acc68C50C7829F473d81e23249cc"),
    );
    m.insert(
        "OKX_115",
        address!("Ba0a39d37151fA4f938Bec51CdAE4675105760f4"),
    );
    m.insert(
        "OKX_116",
        address!("ba96A3f3d5E9F13a5f93C37001f946D42eb2E165"),
    );
    m.insert(
        "OKX_117",
        address!("BDa23B750dD04F792ad365B5F2a6F1d8593796f2"),
    );
    m.insert(
        "OKX_118",
        address!("bE787D53E09822cC42bfB4ABE1fb4492CAe3D19d"),
    );
    m.insert(
        "OKX_119",
        address!("Bf94F0AC752C739F623C463b5210a7fb2cbb420B"),
    );
    m.insert(
        "OKX_12",
        address!("17e91B98988937cB59Dc163b0b11781F9785591A"),
    );
    m.insert(
        "OKX_120",
        address!("bFbBFacCD1126A11b8F84C60b09859F80f3BD10F"),
    );
    m.insert(
        "OKX_121",
        address!("BFeF5c888bB7a0A6B14b4C3Ccc4364EA81aA573f"),
    );
    m.insert(
        "OKX_122",
        address!("c3AE71FE59f5133BA180cbBd76536a70Dec23d40"),
    );
    m.insert(
        "OKX_123",
        address!("c5451b523d5FFfe1351337a221688a62806ad91a"),
    );
    m.insert(
        "OKX_124",
        address!("c5a93444Cc4dA6EfB9e6FC6e5D3CB55A53b52396"),
    );
    m.insert(
        "OKX_125",
        address!("C68c17E6Eec0fDE3605C595C9B98De5c1a4Cc3E4"),
    );
    m.insert(
        "OKX_126",
        address!("c708A1c712bA26DC618f972ad7A187F76C8596Fd"),
    );
    m.insert(
        "OKX_127",
        address!("CB0963264231Bb08B2f680aB3ED89A49c9641Bb3"),
    );
    m.insert(
        "OKX_128",
        address!("CbA38020cd7B6F51Df6AFaf507685aDd148F6ab6"),
    );
    m.insert(
        "OKX_129",
        address!("CBA6a2397b322CF1389f6d1adc05F75F36B20116"),
    );
    m.insert(
        "OKX_13",
        address!("236F9F97e0E62388479bf9E5BA4889e46B0273C3"),
    );
    m.insert(
        "OKX_130",
        address!("Cbc767b519394A9E4682Cc9a15DCd18c46A6045b"),
    );
    m.insert(
        "OKX_131",
        address!("CbffCB2c38ecd19468d366D392AC0c1DC7F04Bb6"),
    );
    m.insert(
        "OKX_132",
        address!("cC3947059395A2DfD4eE23DfB1bB586F90d44F2E"),
    );
    m.insert(
        "OKX_133",
        address!("CD5bD47D3D1D8412b241cE9015c5032142948c12"),
    );
    m.insert(
        "OKX_134",
        address!("d266529641EeFFAA2B2A0Bc99daA5b32EF241078"),
    );
    m.insert(
        "OKX_135",
        address!("D30b438DF65f4f788563b2b3611Bd6059bFF4ad9"),
    );
    m.insert(
        "OKX_136",
        address!("d3d7DBe73BbdD5A5C7a49Ca322763c4d400fC240"),
    );
    m.insert(
        "OKX_137",
        address!("D576392CB12b7749BA33f8d223f64E65Ee32f03f"),
    );
    m.insert(
        "OKX_138",
        address!("D7efCbB86eFdD9E8dE014dafA5944AaE36E817e4"),
    );
    m.insert(
        "OKX_139",
        address!("d9a3c9BA5aa4415a53B9190bcb00f14472790a70"),
    );
    m.insert(
        "OKX_14",
        address!("24C654f6b143dc5caE3C02Fbb527cA63aa555dBC"),
    );
    m.insert(
        "OKX_140",
        address!("dad24044E36587d975D7b5BCEb6467Fac21E0c81"),
    );
    m.insert(
        "OKX_141",
        address!("db0ED345Cb52c2F2457918AFA3EB4b682E264Ad0"),
    );
    m.insert(
        "OKX_142",
        address!("dc3cE895714844B4775B6d06F0DaE513542cEE10"),
    );
    m.insert(
        "OKX_143",
        address!("DD6B3aC983AE0427D329A705108C26D2cb8F945E"),
    );
    m.insert(
        "OKX_144",
        address!("dE01974Fb4A98BAFd7cbf8A06eCf6DCc94d7283f"),
    );
    m.insert(
        "OKX_145",
        address!("deB6ad2a7820839464Da79BaC953aE6189215443"),
    );
    m.insert(
        "OKX_146",
        address!("e6eea9812CCc5981cCf0Ab333610C42c2D92D146"),
    );
    m.insert(
        "OKX_147",
        address!("e7aaFaAD7Eb2bB10771d14CdC4F62D3c7D4A3ba8"),
    );
    m.insert(
        "OKX_148",
        address!("E7F344E7b95c8A19d7d77eA27B86EE2fD6D776CF"),
    );
    m.insert(
        "OKX_149",
        address!("e9172Daf64b05B26eb18f07aC8d6D723aCB48f99"),
    );
    m.insert(
        "OKX_15",
        address!("25236E080106B5387Df201FBBd9a6b870917676D"),
    );
    m.insert(
        "OKX_150",
        address!("e95f6604A591F6ba33aCCB43a8a885C9c272108c"),
    );
    m.insert(
        "OKX_151",
        address!("e983845A04c681A295Dd9cE1FA8C2c8505932da3"),
    );
    m.insert(
        "OKX_152",
        address!("eaEd6576334B003d1c5C4797D9c7Bb025A20c038"),
    );
    m.insert(
        "OKX_153",
        address!("eB196a61f9A1E35Bf5053b65AAA57c5541dcBa86"),
    );
    m.insert(
        "OKX_154",
        address!("Ebe80f029b1c02862B9E8a70a7e5317C06F62Cae"),
    );
    m.insert(
        "OKX_155",
        address!("ED55E0d547FD2fA53Aa64F587B21A9caa9E2D90f"),
    );
    m.insert(
        "OKX_156",
        address!("Ee1c6537E589a15a15f80961f5594C57beD936fB"),
    );
    m.insert(
        "OKX_157",
        address!("f332761c673b59B21fF6dfa8adA44d78c12dEF09"),
    );
    m.insert(
        "OKX_158",
        address!("f51cD688b8744b1bfD2FBa70D050dE85EC4fb9Fb"),
    );
    m.insert(
        "OKX_159",
        address!("f59869753f41Db720127Ceb8DbB8afAF89030De4"),
    );
    m.insert(
        "OKX_16",
        address!("267b51c97632225Da2b2e49aEbE59eEF1Af32653"),
    );
    m.insert(
        "OKX_160",
        address!("f7858Da8a6617f7C6d0fF2bcAFDb6D2eeDF64840"),
    );
    m.insert(
        "OKX_161",
        address!("F7C63e75B90C60D7b343106A39658F8b3ca6e4D2"),
    );
    m.insert(
        "OKX_162",
        address!("F81233a61C0D6D13C6FE504DDbbA3E2630eA0c5c"),
    );
    m.insert(
        "OKX_163",
        address!("fcB21730ac0CD487D8701dFED1170e023B57cf7a"),
    );
    m.insert(
        "OKX_164",
        address!("Fd92F4e91d54B9EF91cc3f97C011a6aF0C2a7eDa"),
    );
    m.insert(
        "OKX_17",
        address!("276cdBa3a39aBF9cEdBa0F1948312c0681E6D5Fd"),
    );
    m.insert(
        "OKX_18",
        address!("297611B7aCcB7F24032c27B3a496465624c7Ef50"),
    );
    m.insert(
        "OKX_19",
        address!("2c8FBB630289363Ac80705A1a61273f76fD5a161"),
    );
    m.insert(
        "OKX_2",
        address!("06959153B974D0D5fDfd87D561db6d8d4FA0bb0B"),
    );
    m.insert(
        "OKX_20",
        address!("2D2cC0eB095e43204E0C087E07Dbf95909650939"),
    );
    m.insert(
        "OKX_21",
        address!("30C1DcdE81e5dbF3121D0408abC7908980e83aE2"),
    );
    m.insert(
        "OKX_22",
        address!("313Eb1C5e1970EB5CEEF6AEbad66b07c7338d369"),
    );
    m.insert(
        "OKX_23",
        address!("3b5a23f6207d87B423C6789D2625eA620423b32D"),
    );
    m.insert(
        "OKX_24",
        address!("3c5883C650d600bd543A9B5c8D9A3a6f5d16b8f4"),
    );
    m.insert(
        "OKX_25",
        address!("3D55CCb2a943d88D39dd2E62DAf767C69fD0179F"),
    );
    m.insert(
        "OKX_26",
        address!("3f482c72a2b3e777746f5755cC0Ff1323eA2Ad16"),
    );
    m.insert(
        "OKX_27",
        address!("3F83fd98d5fA84F3cBF8B275D6A10dFC5605CDa2"),
    );
    m.insert(
        "OKX_28",
        address!("4106A4be14867c70E52c51B9805514Ae2e16dE64"),
    );
    m.insert(
        "OKX_29",
        address!("41205307b6618F03bE2d95747a07311456bdb143"),
    );
    m.insert(
        "OKX_3",
        address!("06d3a30cBb00660B85a30988D197B1c282c6dCB6"),
    );
    m.insert(
        "OKX_30",
        address!("42436286A9c8d63AAfC2eEbBCA193064d68068f2"),
    );
    m.insert(
        "OKX_31",
        address!("42Cf18596EE08E877d532Df1b7cF763059A7EA57"),
    );
    m.insert(
        "OKX_32",
        address!("45CeaBB79cF4aba1E9781CEc35ce726f8a1A9309"),
    );
    m.insert(
        "OKX_33",
        address!("461249076B88189f8AC9418De28B365859E46BfD"),
    );
    m.insert(
        "OKX_34",
        address!("47Eb32dEa1ab1436187939fa72D6d5FF884A87Da"),
    );
    m.insert(
        "OKX_35",
        address!("48480aAd203e8b030F82754B3c75869C9895C6Bf"),
    );
    m.insert(
        "OKX_36",
        address!("48C4c83BE7e3884ee5043a3ABE5115eB020b5f4a"),
    );
    m.insert(
        "OKX_37",
        address!("4a11078a99b118BbFee78a5c187D98D264360433"),
    );
    m.insert(
        "OKX_38",
        address!("4A8F1F5B2A3652131eAc54a6f183A4a2cF44A9A6"),
    );
    m.insert(
        "OKX_39",
        address!("4b4e14a3773Ee558b6597070797fd51EB48606e5"),
    );
    m.insert(
        "OKX_4",
        address!("0799dDbF6F14Db566ca4dF4ff0575c4cC1e7749c"),
    );
    m.insert(
        "OKX_40",
        address!("4BCbAF34862e6480329477917Cf5cd7b9537a98D"),
    );
    m.insert(
        "OKX_41",
        address!("4D19C0a5357bC48be0017095d3C871D9aFC3F21d"),
    );
    m.insert(
        "OKX_42",
        address!("4e2757e46103556f98d4D036D8eFE18389B89f51"),
    );
    m.insert(
        "OKX_43",
        address!("4E7b110335511F662FDBB01bf958A7844118c0D4"),
    );
    m.insert(
        "OKX_44",
        address!("5041ed759Dd4aFc3a72b8192C143F72f4724081A"),
    );
    m.insert(
        "OKX_45",
        address!("52738a51882f35D6b25a3FD0C86089ddBd206821"),
    );
    m.insert(
        "OKX_46",
        address!("52b311c52436789f3754bD199Bf3886b8CCBab4c"),
    );
    m.insert(
        "OKX_47",
        address!("539C92186f7C6CC4CbF443F26eF84C595baBBcA1"),
    );
    m.insert(
        "OKX_48",
        address!("56FD42ECD77C88BDd959Be54aF10d1759b473DfF"),
    );
    m.insert(
        "OKX_49",
        address!("5793Da1b0c41C7dB8E3Eb8DbcD18fdca94A58535"),
    );
    m.insert(
        "OKX_5",
        address!("0938C63109801Ee4243a487aB84DFfA2Bba4589e"),
    );
    m.insert(
        "OKX_50",
        address!("59FAE149A8f8EC74d5bC038F8b76D25b136b9573"),
    );
    m.insert(
        "OKX_51",
        address!("5A150733cb59Bbdd5C7398a8fE7Da7F97c8a213a"),
    );
    m.insert(
        "OKX_52",
        address!("5B27e98516fD2Bd5001D4dfE3f5a2263f702f634"),
    );
    m.insert(
        "OKX_53",
        address!("5b686a7DB873AC258bbC16e8306594e3e862B3FC"),
    );
    m.insert(
        "OKX_54",
        address!("5C52cC7c96bDE8594e5B77D5b76d042CB5FaE5f2"),
    );
    m.insert(
        "OKX_55",
        address!("5F8215eE653Cb7225c741C7aA8591468d1f158b8"),
    );
    m.insert(
        "OKX_56",
        address!("5FF13e9A3EEd7a2Dbcb1Dfa21dfB1c07F3419277"),
    );
    m.insert(
        "OKX_57",
        address!("6182672EfCB2DdF094F7BFd3E92eCD10E718E4e1"),
    );
    m.insert(
        "OKX_58",
        address!("62383739D68Dd0F844103Db8dFb05a7EdED5BBE6"),
    );
    m.insert(
        "OKX_59",
        address!("65A0947BA5175359Bb457D3b34491eDf4cBF7997"),
    );
    m.insert(
        "OKX_6",
        address!("0F51A310a4Dd79d373eB8bE1c0ddd54570235443"),
    );
    m.insert(
        "OKX_60",
        address!("6667A4B7EFf4A0B86781fB3B187622CD3c257F09"),
    );
    m.insert(
        "OKX_61",
        address!("68841a1806fF291314946EebD0cdA8b348E73d6D"),
    );
    m.insert(
        "OKX_62",
        address!("68b5E9E083BFC28c33cfbf3F19D33e629015E907"),
    );
    m.insert(
        "OKX_63",
        address!("68E2EA1622AA67FB3a01a66D132daed8A48d1662"),
    );
    m.insert(
        "OKX_64",
        address!("69a722f0B5Da3aF02b4a205D6F0c285F4ed8F396"),
    );
    m.insert(
        "OKX_65",
        address!("6a4561eF7874A76B4bf0D3edca31DFCD51603414"),
    );
    m.insert(
        "OKX_66",
        address!("6b2C0c7be2048Daa9b5527982C29f48062B34D58"),
    );
    m.insert(
        "OKX_67",
        address!("6B7ba57e1d43c2C975Ba25139A04D193e64a11D0"),
    );
    m.insert(
        "OKX_68",
        address!("6cC5F688a315f3dC28A7781717a9A798a59fDA7b"),
    );
    m.insert(
        "OKX_69",
        address!("6d8c32fCF2d95ff410Ba492f6694F18CbEE55CE1"),
    );
    m.insert(
        "OKX_7",
        address!("0fF9491b236a36Cc183823E39d7532194143FdB1"),
    );
    m.insert(
        "OKX_70",
        address!("6Dc1A070425f437Ac08BB108f7093b177D7Af3a6"),
    );
    m.insert(
        "OKX_71",
        address!("6Df7E0F084D46683E811998847Da3832c9dC3b35"),
    );
    m.insert(
        "OKX_72",
        address!("6E0Ad348Ce07218f772dc4C05e6c747dA12D664e"),
    );
    m.insert(
        "OKX_73",
        address!("6e5d525FA1B207b0e42fC3FfDFB9bEd507708904"),
    );
    m.insert(
        "OKX_74",
        address!("6Fb624B48d9299674022a23d92515e76Ba880113"),
    );
    m.insert(
        "OKX_75",
        address!("728d92A8023bFbe0d4f3fdd549Ed3b4996a0EBa9"),
    );
    m.insert(
        "OKX_76",
        address!("730dF969955c0A2fA9e8F2484E9741a363afdbb3"),
    );
    m.insert(
        "OKX_77",
        address!("7332CD352a84673F1413416ef2E321e17df59844"),
    );
    m.insert(
        "OKX_78",
        address!("74ac8eC0c6fC83B4127816c23930Bea9E1a83df5"),
    );
    m.insert(
        "OKX_79",
        address!("793Aa889e19A130ee4cB8b63c79Aa3BDccC663CB"),
    );
    m.insert(
        "OKX_8",
        address!("10374bC1c4cA086e22673dBc4d702Fee74C4ffc5"),
    );
    m.insert(
        "OKX_80",
        address!("7Afbf56C48d38D732E8B71dB229a20A2eaEa8532"),
    );
    m.insert(
        "OKX_81",
        address!("7E4aA755550152a522d9578621EA22eDAb204308"),
    );
    m.insert(
        "OKX_82",
        address!("7eb6c83AB7D8D9B8618c0Ed973cbEF71d1921EF2"),
    );
    m.insert(
        "OKX_83",
        address!("868daB0b8E21EC0a48b726A1ccf25826c78C6d7F"),
    );
    m.insert(
        "OKX_84",
        address!("8734BCa44102bf8A663e1ba112308504606E1b08"),
    );
    m.insert(
        "OKX_85",
        address!("8744f9a43c22c804553835bA33c5c402af3C79D6"),
    );
    m.insert(
        "OKX_86",
        address!("88288100c2005c5cE9B06956BeD357F3CcC95b9E"),
    );
    m.insert(
        "OKX_87",
        address!("88956282d52Eee0aE1Bf8Eaf98Bc6Eac2250B681"),
    );
    m.insert(
        "OKX_88",
        address!("88C94d8e7d4203B185B3Beb0a7B15A6b4F36B2A2"),
    );
    m.insert(
        "OKX_89",
        address!("89B59726f9C42c350641182536Bb045B9C6A36cA"),
    );
    m.insert(
        "OKX_9",
        address!("11817afB29279703c5679959417015328cA6A0D1"),
    );
    m.insert(
        "OKX_90",
        address!("89FD00b8D2dCEE0f40D8699970115BB861241a54"),
    );
    m.insert(
        "OKX_91",
        address!("8c3CB9665833fD9f79eB14cbA16D82BBAB6F22D8"),
    );
    m.insert(
        "OKX_92",
        address!("8d0Abeb725Fa260521Ed54985a5F793141329aE9"),
    );
    m.insert(
        "OKX_93",
        address!("96FDC631F02207B72e5804428DeE274cF2aC0bCD"),
    );
    m.insert(
        "OKX_94",
        address!("9723b6d608D4841eB4Ab131687a5D4764eb30138"),
    );
    m.insert(
        "OKX_95",
        address!("98EC059Dc3aDFBdd63429454aEB0c990FBA4A128"),
    );
    m.insert(
        "OKX_96",
        address!("9B645675E8D64759E5c36E30Dcb766d8CEC3d34F"),
    );
    m.insert(
        "OKX_97",
        address!("9Cc548536d8Ec1D6c89e6dDd3B0CEEA350Fe73DC"),
    );
    m.insert(
        "OKX_98",
        address!("9e13bAE2256f968d02Fe0129A0F51788F4e2472f"),
    );
    m.insert(
        "OKX_99",
        address!("9e3bB2cD5a89FCC4B826230b144c4917881BEa85"),
    );
    m.insert("HTX", address!("034f854B44D28E26386c1BC37ff9B20C6380b00d"));
    m.insert(
        "HTX_1",
        address!("04645AF26b54BD85Dc02Ac65054e87362A72CB22"),
    );
    m.insert(
        "HTX_10",
        address!("119346062a580Ee98774DF7F0c1C5D1dd7f1AdC0"),
    );
    m.insert(
        "HTX_100",
        address!("aB5C66752a9e8167967685F1450532fB96d5d24f"),
    );
    m.insert(
        "HTX_101",
        address!("aC6a692d2cE44fec287084fFa65244baF2f578df"),
    );
    m.insert(
        "HTX_102",
        address!("adB2B42F6bD96F5c65920b9ac88619DcE4166f94"),
    );
    m.insert(
        "HTX_103",
        address!("afdfd157d9361e621e476036FEE62f688450692B"),
    );
    m.insert(
        "HTX_104",
        address!("B2a48f542dc56B89b24C04076cbE565b3Dc58e7b"),
    );
    m.insert(
        "HTX_105",
        address!("B48931a3673B6351D45f0d01a296F4db7148f485"),
    );
    m.insert(
        "HTX_106",
        address!("B4Cd0386d2Db86f30C1A11c2B8c4F4185c1Dade9"),
    );
    m.insert(
        "HTX_107",
        address!("B5E919c001F752501AC603D0c581e24c77165bb2"),
    );
    m.insert(
        "HTX_108",
        address!("b633a44c8464c9C625E25C18cf210529A1E4cc99"),
    );
    m.insert(
        "HTX_109",
        address!("B9a4873d8D2C22e56B8574e8605644d08E047549"),
    );
    m.insert(
        "HTX_11",
        address!("11E4184C68C3d673c96aCc85A03510073d6EB2c8"),
    );
    m.insert(
        "HTX_110",
        address!("b9F775179bcC7FcF4534700a48F09C590E390eAd"),
    );
    m.insert(
        "HTX_111",
        address!("BA5cfbc7c1e08156d9E6e0D91a66ad3bCFf7956a"),
    );
    m.insert(
        "HTX_112",
        address!("bc53B706B165D2B7E98F254095D9d342E845e5aC"),
    );
    m.insert(
        "HTX_113",
        address!("c589b275e60dDa57aD7E117C6DD837Ab524a5666"),
    );
    m.insert(
        "HTX_114",
        address!("c837F51A0eFa33F8ECA03570e3D01a4B2CF97FfD"),
    );
    m.insert(
        "HTX_115",
        address!("c9610bE2843F1618EdFeDd0860DC43551c727061"),
    );
    m.insert(
        "HTX_116",
        address!("CAc725beF4f114F728cbCfd744a731C2a463c3Fc"),
    );
    m.insert(
        "HTX_117",
        address!("Ce7Ec11a5F306c6B896526149dB1a86c7d1531E2"),
    );
    m.insert(
        "HTX_118",
        address!("Cf35a0dE7FE1671c997e32Be36BD2307BaA3DD72"),
    );
    m.insert(
        "HTX_119",
        address!("d10E08325c0E95D59c607a693483680FE5B755B3"),
    );
    m.insert(
        "HTX_12",
        address!("1205E4f0D2f02262E667fd72f95a68913b4F7462"),
    );
    m.insert(
        "HTX_120",
        address!("d3a2f775e973c1671f2047E620448B8662dCD3cA"),
    );
    m.insert(
        "HTX_121",
        address!("d3Cc0C7d40366A061397274Eae7C387D840e6ff8"),
    );
    m.insert(
        "HTX_122",
        address!("D4F7C1047E43016434B177222e0706cE71084514"),
    );
    m.insert(
        "HTX_123",
        address!("d70250731A72C33BFB93016E3D1F0CA160dF7e42"),
    );
    m.insert(
        "HTX_124",
        address!("d8a83b72377476D0a66683CDe20A8aAD0B628713"),
    );
    m.insert(
        "HTX_125",
        address!("dB0E89a9B003A28A4055ef772E345E8089987bfd"),
    );
    m.insert(
        "HTX_126",
        address!("Dc76CD25977E0a5Ae17155770273aD58648900D3"),
    );
    m.insert(
        "HTX_127",
        address!("dd3CB5c974601BC3974d908Ea4A86020f9999E0c"),
    );
    m.insert(
        "HTX_128",
        address!("DF95de30cDff4381B69F9e4FA8DDDce31a0128DF"),
    );
    m.insert(
        "HTX_129",
        address!("dFCCd61E2E919489062224989A54AC6E59F832aa"),
    );
    m.insert(
        "HTX_13",
        address!("12FA4951CBFC51102ccFd5580beb135cA3D74c54"),
    );
    m.insert(
        "HTX_130",
        address!("e0B7A39Fef902c21bAd124b144c62E7F85f5f5fA"),
    );
    m.insert(
        "HTX_131",
        address!("E195B82Df6A797551Eb1ACd506e892531824Af27"),
    );
    m.insert(
        "HTX_132",
        address!("E3314bbF3334228b257779E28228CfB86fA4261B"),
    );
    m.insert(
        "HTX_133",
        address!("E3C274D2e2180aFd89EdA4D25320Cff08AaDB680"),
    );
    m.insert(
        "HTX_134",
        address!("E4818f8fDe0C977A01DA4Fa467365B8bF22b071E"),
    );
    m.insert(
        "HTX_135",
        address!("e8D8a02601f54AcB6fB69537Be1F1D7cC76ccd8C"),
    );
    m.insert(
        "HTX_136",
        address!("E93381fB4c4F14bDa253907b18faD305D799241a"),
    );
    m.insert(
        "HTX_137",
        address!("EA0cFeF143182d7B9208FBfEda9D172c2ACED972"),
    );
    m.insert(
        "HTX_138",
        address!("EB6D43Fe241fb2320b5A3c9BE9CDfD4dd8226451"),
    );
    m.insert(
        "HTX_139",
        address!("EBA290cf248cB14442A071fbCb58a9Cc5dcdE28E"),
    );
    m.insert(
        "HTX_14",
        address!("137ad9C4777E1d36e4b605e745e8F37B2b62E9c5"),
    );
    m.insert(
        "HTX_140",
        address!("eCD8b3877D8E7cD0739dE18a5b545bc0B3538566"),
    );
    m.insert(
        "HTX_141",
        address!("Eec606A66edB6f497662Ea31b5eb1610da87AB5f"),
    );
    m.insert(
        "HTX_142",
        address!("eEe28d484628d41A82d01e21d12E2E78D69920da"),
    );
    m.insert(
        "HTX_143",
        address!("EF54F559B5e3b55b783C7Bc59850F83514B6149c"),
    );
    m.insert(
        "HTX_144",
        address!("f0458aAAf6d49192D3b4711960635D5FA2114E71"),
    );
    m.insert(
        "HTX_145",
        address!("f056F435Ba0CC4fCD2F1B17e3766549fFc404B94"),
    );
    m.insert(
        "HTX_146",
        address!("F2dbC42875E7764EDBd89732A15214A9a0Deb085"),
    );
    m.insert(
        "HTX_147",
        address!("f66852bC122fD40bFECc63CD48217E88bda12109"),
    );
    m.insert(
        "HTX_148",
        address!("f726dc178D1A4d9292A8d63f01e0fA0A1235E65C"),
    );
    m.insert(
        "HTX_149",
        address!("f775a9a0Ad44807bc15936dF0Ee68902aF1A0EEE"),
    );
    m.insert(
        "HTX_15",
        address!("170af0A02339743687aFD3dC8d48cfFd1f660728"),
    );
    m.insert(
        "HTX_150",
        address!("f7A8Af16Acb302351D7Ea26ffc380575B741724c"),
    );
    m.insert(
        "HTX_151",
        address!("F881bCB3705926cEa9C598aB05a837cf41a833a9"),
    );
    m.insert(
        "HTX_152",
        address!("FA4B5Be3f2f84f56703C42eB22142744E95a2c58"),
    );
    m.insert(
        "HTX_153",
        address!("Fd54078bAdD5653571726C3370AfB127351a6f26"),
    );
    m.insert(
        "HTX_154",
        address!("fdb16996831753d5331fF813c29a93c76834A0AD"),
    );
    m.insert(
        "HTX_16",
        address!("18709E89BD403F470088aBDAcEbE86CC60dda12e"),
    );
    m.insert(
        "HTX_17",
        address!("18916e1a2933Cb349145A280473A5DE8EB6630cb"),
    );
    m.insert(
        "HTX_18",
        address!("1B93129F05cc2E840135AAB154223C75097B69bf"),
    );
    m.insert(
        "HTX_19",
        address!("1C515Eaa87568c850043a89C2D2C2e8187Adb056"),
    );
    m.insert(
        "HTX_2",
        address!("0511509A39377F1C6c78DB4330FBfcC16D8A602f"),
    );
    m.insert(
        "HTX_20",
        address!("1D1E10e8c66B67692f4C002C0cB334De5D485e41"),
    );
    m.insert(
        "HTX_21",
        address!("2177c77A1f3c4900De7668662706633DB4688726"),
    );
    m.insert(
        "HTX_22",
        address!("229b5c097F9b35009CA1321Ad2034D4b3D5070F6"),
    );
    m.insert(
        "HTX_23",
        address!("25C6459e5c5b01694F6453E8961420CcD1EDF3b1"),
    );
    m.insert(
        "HTX_24",
        address!("27E9F4748a2eb776bE193a1F7dec2Bb6DAAfE9Cf"),
    );
    m.insert(
        "HTX_25",
        address!("2886DAC7bAe6C45B05BB483A6C709fA6501A765a"),
    );
    m.insert(
        "HTX_26",
        address!("28FFE35688fFFfd0659AEE2E34778b0ae4E193aD"),
    );
    m.insert(
        "HTX_27",
        address!("296d69073d5D2E0Dc51B768ee83c0f9F14A0bfE2"),
    );
    m.insert(
        "HTX_28",
        address!("2Abc22eb9A09EbBE7b41737CCde147F586EfeB6A"),
    );
    m.insert(
        "HTX_29",
        address!("2bB1881b1DdE050fdf756c25E025aA9367b4aFFC"),
    );
    m.insert(
        "HTX_3",
        address!("0577a79Cfc63Bbc0Df38833Ff4C4a3BF2095b404"),
    );
    m.insert(
        "HTX_30",
        address!("30741289523c2e4d2A62c7D6722686D14E723851"),
    );
    m.insert(
        "HTX_31",
        address!("32598293906b5b17c27d657dB3AD2c9b3f3E4265"),
    );
    m.insert(
        "HTX_32",
        address!("34189c75Cbb13Bdb4F5953CDa6c3045CFcA84a9e"),
    );
    m.insert(
        "HTX_33",
        address!("3634b91e54c4dCd7919772F225fC5532F91d8F92"),
    );
    m.insert(
        "HTX_34",
        address!("36d86971BebACd424AEb0BB164c629d61f4557A2"),
    );
    m.insert(
        "HTX_35",
        address!("39d9f4640b98189540A9C0edCFa95C5e657706aA"),
    );
    m.insert(
        "HTX_36",
        address!("3c979fB790c86e361738ED17588c1e8b4C4cc49A"),
    );
    m.insert(
        "HTX_37",
        address!("3cb013F6704EB8E22923f02Bb8E7C8D4Bd7541CF"),
    );
    m.insert(
        "HTX_38",
        address!("3d655889D197125fb90dcB72e4a287A8410ED1B9"),
    );
    m.insert(
        "HTX_39",
        address!("42dc966b7eCc3c6cc73e7bc04862859D5bDDCE65"),
    );
    m.insert(
        "HTX_4",
        address!("07EF60deCa209Ea0F3f3f08C1aD21a6DB5EF9D33"),
    );
    m.insert(
        "HTX_40",
        address!("46705dfff24256421A05D056c29E81Bdc09723B8"),
    );
    m.insert(
        "HTX_41",
        address!("48AB9f29795DFB44B36587C50da4b30c0E84d3ed"),
    );
    m.insert(
        "HTX_42",
        address!("49517CA7b7a50f592886D4c74175F4C07D460a70"),
    );
    m.insert(
        "HTX_43",
        address!("4d77a1144dC74f26838B69391a6D3B1e403D0990"),
    );
    m.insert(
        "HTX_44",
        address!("508Ab7C228951BEc9c33EB5613Fc9AaB726Fc74C"),
    );
    m.insert(
        "HTX_45",
        address!("53cFD2c9eB387Cce8F5d111a4352ab7aa4333b18"),
    );
    m.insert(
        "HTX_46",
        address!("5401dBf7da53e1C9Dbf484E3d69505815F2f5e6e"),
    );
    m.insert(
        "HTX_47",
        address!("56a5D322AE2E2F5AfC6716addacA4E62276aE9B6"),
    );
    m.insert(
        "HTX_48",
        address!("5861b8446A2F6e19a067874c133f04c578928727"),
    );
    m.insert(
        "HTX_49",
        address!("58c2cb4a6BeE98C309215D0d2A38d7F8aa71211c"),
    );
    m.insert(
        "HTX_5",
        address!("08DeB6278D671E2a1aDc7b00839b402B9cF3375d"),
    );
    m.insert(
        "HTX_50",
        address!("598273eA2CAbD9F798564877851788c5e0d5b7b9"),
    );
    m.insert(
        "HTX_51",
        address!("5BD5EdcCa85c0c035C9B2F5b4e22683FE37F6677"),
    );
    m.insert(
        "HTX_52",
        address!("5c8A9F68eFF7Df9f291cE8408b797C4e7B9a95CB"),
    );
    m.insert(
        "HTX_53",
        address!("5C985E89DDe482eFE97ea9f1950aD149Eb73829B"),
    );
    m.insert(
        "HTX_54",
        address!("5E9dB2797C2F2775247881d3535271b212bD79D6"),
    );
    m.insert(
        "HTX_55",
        address!("5F477d105EFa3bCc2F74f0e9A008bcAA988a8E8A"),
    );
    m.insert(
        "HTX_56",
        address!("60aA247146d47Aee2B1C136540C55261a4b8342c"),
    );
    m.insert(
        "HTX_57",
        address!("60B45F993223DcB8bdF05e3391f7630E5a51D787"),
    );
    m.insert(
        "HTX_58",
        address!("636B76AE213358b9867591299E5c62B8d014E372"),
    );
    m.insert(
        "HTX_59",
        address!("6496Be0EaeF40cd4497a6E9DCA28Fe5441b4AcD1"),
    );
    m.insert(
        "HTX_6",
        address!("0A98fB70939162725aE66E626Fe4b52cFF62c2e5"),
    );
    m.insert(
        "HTX_60",
        address!("664C900aD79A7E9e3E1BbdD238715744AC9b575B"),
    );
    m.insert(
        "HTX_61",
        address!("6663613FbD927cE78abBF7F5Ca7e2c3FE0d96d18"),
    );
    m.insert(
        "HTX_62",
        address!("6748F50f686bfbcA6Fe8ad62b22228b87F31ff2b"),
    );
    m.insert(
        "HTX_63",
        address!("6b2286FC3a9265bab3F064808022acA54dE4B6cE"),
    );
    m.insert(
        "HTX_64",
        address!("6EdB9d6547BEFc3397801C94Bb8C97d2e8087E2F"),
    );
    m.insert(
        "HTX_65",
        address!("6F48a3E70F0251d1e83a989e62aAa2281A6d5380"),
    );
    m.insert(
        "HTX_66",
        address!("6F50C6Bff08Ec925232937B204B0ae23C488402a"),
    );
    m.insert(
        "HTX_67",
        address!("70383BfF83a9504049A3759b892e4bb1Ec10b806"),
    );
    m.insert(
        "HTX_68",
        address!("73f8FC2e74302eb2EfdA125A326655aCF0DC2D1B"),
    );
    m.insert(
        "HTX_69",
        address!("74956D3Ab75af393cd74c8410129d27997f797f9"),
    );
    m.insert(
        "HTX_7",
        address!("0c6C34CDd915845376fb5407E0895196C9DD4eeC"),
    );
    m.insert(
        "HTX_70",
        address!("75a83599dE596cBC91a1821fFA618C40e22ac8CA"),
    );
    m.insert(
        "HTX_71",
        address!("76b66147709f45eDB1a95c469F8F076b2aBb8f2D"),
    );
    m.insert(
        "HTX_72",
        address!("77Fe06EF6A614d292A6342df1bc2CB7aEBBbe856"),
    );
    m.insert(
        "HTX_73",
        address!("794d28aC31bCB136294761a556b68D2634094153"),
    );
    m.insert(
        "HTX_74",
        address!("79795ecce9e9538E886Df5312088999D7Fa817bC"),
    );
    m.insert(
        "HTX_75",
        address!("7EF35bb398E0416b81b019fEa395219B65c52164"),
    );
    m.insert(
        "HTX_76",
        address!("80E63D2735789Fa1F676B7644236d501133986dD"),
    );
    m.insert(
        "HTX_77",
        address!("82D015d74670d8645b56c3f453398a3E799Ee582"),
    );
    m.insert(
        "HTX_78",
        address!("8AaBBA0077f1565Df73e9D15dd3784a2b0033DAd"),
    );
    m.insert(
        "HTX_79",
        address!("8b6a3587676719A4FeCBb24b503a3634C44A44d5"),
    );
    m.insert(
        "HTX_8",
        address!("0C92EfA186074Ba716d0E2156A6FFAbD579f8035"),
    );
    m.insert(
        "HTX_80",
        address!("8E8bc99b79488c276D6f3Ca11901E9AbD77efEa4"),
    );
    m.insert(
        "HTX_81",
        address!("90E9dDD9d8D5ae4E3763d0CF856C97594DEA7325"),
    );
    m.insert(
        "HTX_82",
        address!("90f49e24A9554126F591D28174e157CA267194Ba"),
    );
    m.insert(
        "HTX_83",
        address!("918800E018A0Eeea672740F88A60091c7D327a79"),
    );
    m.insert(
        "HTX_84",
        address!("91dFa9d9E062A50D2f351bfbA0d35A9604993DaC"),
    );
    m.insert(
        "HTX_85",
        address!("926fC576b7facF6aE2d08eE2D4734C134a743988"),
    );
    m.insert(
        "HTX_86",
        address!("956e0DBEcC0e873d34a5e39B25f364b2CA036730"),
    );
    m.insert(
        "HTX_87",
        address!("99fe5D6383289CDD56e54Fc0bAF7F67c957A8888"),
    );
    m.insert(
        "HTX_88",
        address!("9A31CFBDDd97276e43442f34e971decf6DC2D211"),
    );
    m.insert(
        "HTX_89",
        address!("9A755332D874c893111207b0b220Ce2615cd036F"),
    );
    m.insert(
        "HTX_9",
        address!("1062a747393198f70F71ec65A582423Dba7E5Ab3"),
    );
    m.insert(
        "HTX_90",
        address!("9A7ffD7F6c42ab805e0eDF16c25101964C6326B6"),
    );
    m.insert(
        "HTX_91",
        address!("9d6d492bD500DA5B33cf95A5d610a73360FcaAa0"),
    );
    m.insert(
        "HTX_92",
        address!("9Eebb2815dbA2166d8287AFa9A2C89336ba9DEaA"),
    );
    m.insert(
        "HTX_93",
        address!("9ef21bE1C270AA1c3c3d750F458442397fBFFCB6"),
    );
    m.insert(
        "HTX_94",
        address!("a03400E098F4421b34a3a44A1B4e571419517687"),
    );
    m.insert(
        "HTX_95",
        address!("A23d7dd4b8a1060344CAF18A29b42350852AF481"),
    );
    m.insert(
        "HTX_96",
        address!("a5D7F0F7027fa8F4D1BE8042E1e43bbdEc36951e"),
    );
    m.insert(
        "HTX_97",
        address!("a77ff0e1C52f58363a53282624C7BaA5fA91687D"),
    );
    m.insert(
        "HTX_98",
        address!("a8660c8ffD6D578F657B72c0c811284aef0B735e"),
    );
    m.insert(
        "HTX_99",
        address!("A91183d7DfcFe39D923071F5527552ab52DA6d44"),
    );
    m.insert(
        "KuCoin",
        address!("00F3e09Abe73AeC2D6AD7B8820049B60eBc73f94"),
    );
    m.insert(
        "KuCoin_1",
        address!("03E6FA590CAdcf15A38e86158E9b3D06FF3399Ba"),
    );
    m.insert(
        "KuCoin_10",
        address!("2B5634C42055806a59e9107ED44D43c426E58258"),
    );
    m.insert(
        "KuCoin_11",
        address!("3aD7D43702Bc2177cc9EC655b6ee724136891EF4"),
    );
    m.insert(
        "KuCoin_12",
        address!("41e29c02713929F800419AbE5770fAa8A5b4dADC"),
    );
    m.insert(
        "KuCoin_13",
        address!("439eF8dd8E50d632f6e5802869fD13af430DE84F"),
    );
    m.insert(
        "KuCoin_14",
        address!("441454b3D857FE365b7DefE8cb3E4F498EC91eAC"),
    );
    m.insert(
        "KuCoin_15",
        address!("45300136662dD4e58fc0DF61E6290DFfD992B785"),
    );
    m.insert(
        "KuCoin_16",
        address!("4ad64983349C49dEfE8d7A4686202d24b25D0CE8"),
    );
    m.insert(
        "KuCoin_17",
        address!("4E75e27e5Aa74F0c7A9D4897dC10EF651f3A3995"),
    );
    m.insert(
        "KuCoin_18",
        address!("53f78A071d04224B8e254E243fFfc6D9f2f3Fa23"),
    );
    m.insert(
        "KuCoin_19",
        address!("58edF78281334335EfFa23101bBe3371b6a36A51"),
    );
    m.insert(
        "KuCoin_2",
        address!("061F7937B7b2bc7596539959804F86538b6368dC"),
    );
    m.insert(
        "KuCoin_20",
        address!("635308e731A878741bfeC299e67f5fD28c7553D9"),
    );
    m.insert(
        "KuCoin_21",
        address!("689C56AEf474Df92D44A1B70850f808488F9769C"),
    );
    m.insert(
        "KuCoin_22",
        address!("738cF6903E6c4e699D1C2dd9AB8b67fcDb3121eA"),
    );
    m.insert(
        "KuCoin_23",
        address!("7491f26A0FCb459111b3a1db2fbFC4035D096933"),
    );
    m.insert(
        "KuCoin_24",
        address!("77f59b595CaC829575E262B4c8bBCB17abAdB33A"),
    );
    m.insert(
        "KuCoin_25",
        address!("7b403C20ADB5674e31FBBC040945999739A874c8"),
    );
    m.insert(
        "KuCoin_26",
        address!("7b915c27a0Ed48E2Ce726Ee40F20B2bF8a88a1b3"),
    );
    m.insert(
        "KuCoin_27",
        address!("83C41363cBee0081dab75cB841FA24f3dB46627e"),
    );
    m.insert(
        "KuCoin_28",
        address!("88Bd4D3e2997371BCEEFE8D9386c6B5B4dE60346"),
    );
    m.insert(
        "KuCoin_29",
        address!("899B5d52671830f567BF43A14684Eb14e1f945fe"),
    );
    m.insert(
        "KuCoin_3",
        address!("14EA40648fC8C1781D19363F5B9Cc9A877ac2469"),
    );
    m.insert(
        "KuCoin_30",
        address!("9AC5637d295FEA4f51E086C329d791cC157B1C84"),
    );
    m.insert(
        "KuCoin_31",
        address!("9f4Cf329f4cF376B7ADED854D6054859dd102a2A"),
    );
    m.insert(
        "KuCoin_32",
        address!("a152F8bb749c55E9943A3a0A3111D18ee2B3f94E"),
    );
    m.insert(
        "KuCoin_33",
        address!("A1CE37506eadf62d2Be3741C644AA52a009d3A8b"),
    );
    m.insert(
        "KuCoin_34",
        address!("a1D8d972560C2f8144AF871Db508F0B0B10a3fBf"),
    );
    m.insert(
        "KuCoin_35",
        address!("a3f45e619cE3AAe2Fa5f8244439a66B203b78bCc"),
    );
    m.insert(
        "KuCoin_36",
        address!("a649fFC455AC7C5acc1bc35726FcE54e25Eb59f9"),
    );
    m.insert(
        "KuCoin_37",
        address!("B9F79Fc4B7A2F5fB33493aB5D018dB811c9c2f02"),
    );
    m.insert(
        "KuCoin_38",
        address!("BF7AEBe0A571CF621F59aD48333a5c75fBf9d5E4"),
    );
    m.insert(
        "KuCoin_39",
        address!("c91FbeC25F454E1BFc782d215aDA56b3007276D0"),
    );
    m.insert(
        "KuCoin_4",
        address!("1692E170361cEFD1eb7240ec13D048Fd9aF6d667"),
    );
    m.insert(
        "KuCoin_40",
        address!("caD621da75a66c7A8f4FF86D30A2bF981Bfc8FdD"),
    );
    m.insert(
        "KuCoin_41",
        address!("cB014880de8b1E5f6c90CBcD2c232970cF3Aec32"),
    );
    m.insert(
        "KuCoin_42",
        address!("cD5F3c15120a1021155174719Ec5FCf2c75aDf5b"),
    );
    m.insert(
        "KuCoin_43",
        address!("ce0B6bfd578A5e90fB827ce6F86Aa06355277F8c"),
    );
    m.insert(
        "KuCoin_44",
        address!("cE0d2213A0eAFF4176D90B39879b7B4F870fA428"),
    );
    m.insert(
        "KuCoin_45",
        address!("D6216fC19DB775Df9774a6E33526131dA7D19a2c"),
    );
    m.insert(
        "KuCoin_46",
        address!("d89350284c7732163765b23338f2ff27449E0Bf5"),
    );
    m.insert(
        "KuCoin_47",
        address!("dd07813c45CA55731dd12f6e5De59Ce9FE5304ad"),
    );
    m.insert(
        "KuCoin_48",
        address!("e59Cd29be3BE4461d79C0881D238Cbe87D64595A"),
    );
    m.insert(
        "KuCoin_49",
        address!("E66845FD840FC7e489bcb61241FFf5B7fc5f1f0e"),
    );
    m.insert(
        "KuCoin_5",
        address!("17A30350771d02409046A683b18Fe1C13cCFC4A8"),
    );
    m.insert(
        "KuCoin_50",
        address!("EBb8EA128BbdFf9a1780A4902A9380022371d466"),
    );
    m.insert(
        "KuCoin_51",
        address!("eC30d02f10353f8EFC9601371f56e808751f396F"),
    );
    m.insert(
        "KuCoin_52",
        address!("f16E9B0D03470827A95CDfd0Cb8a8A3b46969B91"),
    );
    m.insert(
        "KuCoin_53",
        address!("F3F094484eC6901FfC9681bCb808B96bAFd0b8a8"),
    );
    m.insert(
        "KuCoin_54",
        address!("f8Ba3EC49212Ca45325A2335a8Ab1279770dF6c0"),
    );
    m.insert(
        "KuCoin_55",
        address!("F8dA05c625A6E601281110cbA52b156e714E1DC2"),
    );
    m.insert(
        "KuCoin_56",
        address!("f97DeB1C0BB4536ff16617D29E5F4B340fE231Df"),
    );
    m.insert(
        "KuCoin_57",
        address!("fB6a733bf7eC9CE047c1c5199F18401052Eb062D"),
    );
    m.insert(
        "KuCoin_6",
        address!("1dD9319a115D36bD0f71C276844f67171678E17b"),
    );
    m.insert(
        "KuCoin_7",
        address!("245654d7Db653FF134EA032f671eF2730333F42c"),
    );
    m.insert(
        "KuCoin_8",
        address!("2602669a92fCCF44e5319fF51B0F453aAb9Db021"),
    );
    m.insert(
        "KuCoin_9",
        address!("2a8c8b09bD77c13980495A959B26c1305166A57f"),
    );
    m.insert(
        "Bybit",
        address!("1e32760a3285550278aEAFA776E5641bC581C845"),
    );
    m.insert(
        "Bybit_1",
        address!("2C7DAb4B02b77603eF16a7aeA8E30F137e8C1b93"),
    );
    m.insert(
        "Bybit_10",
        address!("F5F3436A05B5CEd2490DAE07B86EB5BbD02782aA"),
    );
    m.insert(
        "Bybit_11",
        address!("F65d698D18bC37bF36e4C8D4Fe4F051EF570e2B6"),
    );
    m.insert(
        "Bybit_12",
        address!("f89d7b9c864f589bbF53a82105107622B35EaA40"),
    );
    m.insert(
        "Bybit_2",
        address!("2deD5ce31a0C61eCaf6429A1ba1A00b2bFe67099"),
    );
    m.insert(
        "Bybit_3",
        address!("3D5202A0564De9B05eCd07C955BcCA964585ea03"),
    );
    m.insert(
        "Bybit_4",
        address!("4230C402c08cB66DCf3820649A115e54661FCe9D"),
    );
    m.insert(
        "Bybit_5",
        address!("88a1493366D48225fc3cEFbdae9eBb23E323Ade3"),
    );
    m.insert(
        "Bybit_6",
        address!("a95B83af96d0B8A90BD507f2Bd82aD8F3dbb86BC"),
    );
    m.insert(
        "Bybit_7",
        address!("ab97925eB84fe0260779F58B7cb08d77dcB1ee2B"),
    );
    m.insert(
        "Bybit_8",
        address!("BaeD383EDE0e5d9d72430661f3285DAa77E9439F"),
    );
    m.insert(
        "Bybit_9",
        address!("ee5B5B923fFcE93A870B3104b7CA09c3db80047A"),
    );
    m.insert(
        "Crypto.com",
        address!("0Ecc16D3fa38E1a59c10e44CDA4e2e9d9941275A"),
    );
    m.insert(
        "Crypto.com_1",
        address!("1714400FF23dB4aF24F9fd64e7039e6597f18C2b"),
    );
    m.insert(
        "Crypto.com_10",
        address!("72A53cDBBcc1b9efa39c834A540550e23463AAcB"),
    );
    m.insert(
        "Crypto.com_11",
        address!("7758E507850dA48cd47df1fB5F875c23E3340c50"),
    );
    m.insert(
        "Crypto.com_12",
        address!("7Aad7840F119f3876EE3569e488C7C4135f695fa"),
    );
    m.insert(
        "Crypto.com_13",
        address!("8a161a996617f130d0F37478483AfC8c1914DB6d"),
    );
    m.insert(
        "Crypto.com_14",
        address!("92BD687953Da50855AeE2Df0Cff282cC2d5F226b"),
    );
    m.insert(
        "Crypto.com_15",
        address!("9a552417cfc942A5C88Ab474756d3D9962f917C0"),
    );
    m.insert(
        "Crypto.com_16",
        address!("9Fb538820D4FDe2FCC509Dc01Ae73a192f36cfcC"),
    );
    m.insert(
        "Crypto.com_17",
        address!("Ae45a8240147E6179ec7c9f92c5A18F9a97B3fCA"),
    );
    m.insert(
        "Crypto.com_18",
        address!("CFFAd3200574698b78f32232aa9D63eABD290703"),
    );
    m.insert(
        "Crypto.com_19",
        address!("D3d877fc323De661Ff9E1a38147A1AC679ce7C64"),
    );
    m.insert(
        "Crypto.com_2",
        address!("187b2d576ba7ec2141c180A96eDd0f202492f36B"),
    );
    m.insert(
        "Crypto.com_20",
        address!("D7a827FBaf38c98E8336C5658E4BcbCD20a4fd2d"),
    );
    m.insert(
        "Crypto.com_21",
        address!("f3B0073E3a7F747C7A38B36B805247B222C302A3"),
    );
    m.insert(
        "Crypto.com_22",
        address!("fa0b641678F5115ad8a8De5752016bD1359681b9"),
    );
    m.insert(
        "Crypto.com_3",
        address!("18ae7a92f5261208bb5366Fa213c966D65988C95"),
    );
    m.insert(
        "Crypto.com_4",
        address!("20fA1822A87D4e7A3CcF20f86e716Ef3772eCff1"),
    );
    m.insert(
        "Crypto.com_5",
        address!("2C2301FDB0bfA06EAABaA0122CbCEb2265337C25"),
    );
    m.insert(
        "Crypto.com_6",
        address!("46340b20830761efd32832A74d7169B29FEB9758"),
    );
    m.insert(
        "Crypto.com_7",
        address!("546553718b1B255742566f10A34D86FC22F02b1f"),
    );
    m.insert(
        "Crypto.com_8",
        address!("625b02b687Ec38f3085Af5B108Dda410775fA76a"),
    );
    m.insert(
        "Crypto.com_9",
        address!("6262998Ced04146fA42253a5C0AF90CA02dfd2A3"),
    );
    m.insert(
        "Gate.io",
        address!("05EE546c1a62f90D7aCBfFd6d846c9C54C7cF94c"),
    );
    m.insert(
        "Gate.io_1",
        address!("0D0707963952f2fBA59dD06f2b425ace40b492Fe"),
    );
    m.insert(
        "Gate.io_2",
        address!("1C4b70a3968436B9A0a9cf5205c787eb81Bb558c"),
    );
    m.insert(
        "Gate.io_3",
        address!("234EE9e35f8e9749A002fc42970D570DB716453B"),
    );
    m.insert(
        "Gate.io_4",
        address!("6596Da8B65995d5feaCfF8c2936f0b7a2051B0D0"),
    );
    m.insert(
        "Gate.io_5",
        address!("7793cD85c11a924478d358D49b05b37E91B5810F"),
    );
    m.insert(
        "Gate.io_6",
        address!("85FAa6C1F2450b9caEA300838981C2e6E120C35c"),
    );
    m.insert(
        "Gate.io_7",
        address!("C882b111A75C0c657fC507C04FbFcD2cC984F071"),
    );
    m.insert(
        "Gate.io_8",
        address!("D793281182A0e3E023116004778F45c29fc14F19"),
    );
    m.insert(
        "Gate.io_9",
        address!("eB01f8cdAE433E7B55023fF0B2DA44C4c712DCE2"),
    );
    m.insert(
        "Gemini",
        address!("07Ee55aA48Bb72DcC6E9D78256648910De513eca"),
    );
    m.insert(
        "Gemini_1",
        address!("183b1Ffb0Aa9213b9335AdFAd82E47bfb02f8d24"),
    );
    m.insert(
        "Gemini_10",
        address!("B1CCe076E720300C4E49b529a7cE0E58d3C0e8fe"),
    );
    m.insert(
        "Gemini_11",
        address!("b302BFE9c246c6e150AF70b1cAAA5e3Df60dAc05"),
    );
    m.insert(
        "Gemini_12",
        address!("d24400ae8BfEBb18cA49Be86258a3C749cf46853"),
    );
    m.insert(
        "Gemini_13",
        address!("D69B0089D9CA950640F5DC9931A41a5965f00303"),
    );
    m.insert(
        "Gemini_14",
        address!("Dd51F01D9fc0Fd084C1a4737BbFa5Becb6CEd9BC"),
    );
    m.insert(
        "Gemini_15",
        address!("F51710015536957A01f32558402902A2D9c35d82"),
    );
    m.insert(
        "Gemini_2",
        address!("3e6722f32CBE5b3C7BD3dcA7017c7FfE1b9E5A2A"),
    );
    m.insert(
        "Gemini_3",
        address!("485b9a41e8BF06E57BB64c6Ba7cB04f9d53D2d76"),
    );
    m.insert(
        "Gemini_4",
        address!("4c2F150Fc90fed3d8281114c2349f1906cdE5346"),
    );
    m.insert(
        "Gemini_5",
        address!("5f65f7b609678448494De4C87521CdF6cEf1e932"),
    );
    m.insert(
        "Gemini_6",
        address!("61EDCDf5bb737ADffE5043706e7C5bb1f1a56eEA"),
    );
    m.insert(
        "Gemini_7",
        address!("6Fc82a5fe25A5cDb58bc74600A40A69C065263f8"),
    );
    m.insert(
        "Gemini_8",
        address!("8D6F396D210d385033b348bCae9e4f9Ea4e045bD"),
    );
    m.insert(
        "Gemini_9",
        address!("9d549AD2CE668271fB1354Af19B1668fDb86d818"),
    );
    m.insert(
        "Bitstamp",
        address!("00BDb5699745f5b860228c8f939ABF1b9Ae374eD"),
    );
    m.insert(
        "Bitstamp_1",
        address!("059799F2261d37b829c2850cEe67b5b975432271"),
    );
    m.insert(
        "Bitstamp_10",
        address!("518B82370bc31eBB96922EC257D92517d7387615"),
    );
    m.insert(
        "Bitstamp_11",
        address!("538d72dEd42A76A30f730292Da939e0577f22F57"),
    );
    m.insert(
        "Bitstamp_12",
        address!("593aebEE9117EEA447279E5973F64C68D8e977a0"),
    );
    m.insert(
        "Bitstamp_13",
        address!("6dCa94b6173c28a4900ea257121e6002C0B96968"),
    );
    m.insert(
        "Bitstamp_14",
        address!("772396dD44Ce3d347838bFEC437CB32F534963F2"),
    );
    m.insert(
        "Bitstamp_15",
        address!("7E677CaCaaE0D465Cfd336869f1F575a48BF012a"),
    );
    m.insert(
        "Bitstamp_16",
        address!("808e7133C700cF3a66E6A25AAdB1fBEF6be468b4"),
    );
    m.insert(
        "Bitstamp_17",
        address!("8366DCAB4Cc14c826fC9D51bd4c16567bD07B02a"),
    );
    m.insert(
        "Bitstamp_18",
        address!("964771F6dF31EeA2D927Fa71d7bd78e81bcdce05"),
    );
    m.insert(
        "Bitstamp_19",
        address!("9A9BED3Eb03E386D66f8a29DC67dC29Bbb1ccB72"),
    );
    m.insert(
        "Bitstamp_2",
        address!("0b0F7ebF967146566799229394171FC47f1a765a"),
    );
    m.insert(
        "Bitstamp_20",
        address!("9feC89e34efaa4FC9f19c02F474c71373e6effe7"),
    );
    m.insert(
        "Bitstamp_21",
        address!("A3Fb85C3A2c50D8C0e1Dd7Fa7746F97C9E1D9591"),
    );
    m.insert(
        "Bitstamp_22",
        address!("AB7bb7959332888E44d795c6F28eE876a8469EAA"),
    );
    m.insert(
        "Bitstamp_23",
        address!("B8e73ba7C6c0b50a0cd94fe9F6622762b0401c02"),
    );
    m.insert(
        "Bitstamp_24",
        address!("BcdDebA6a9672c1F76a8b8EDD3190BDFe6D4ef11"),
    );
    m.insert(
        "Bitstamp_25",
        address!("C0AC2f4A3cF22fD504D8835B07f5acCcfa9b27F9"),
    );
    m.insert(
        "Bitstamp_26",
        address!("C20b79CFf9d2C89bA8aeb9ABf4BfEf0314Ca7bD2"),
    );
    m.insert(
        "Bitstamp_27",
        address!("C3b7336D5A5158215599572012CeDd4403A81629"),
    );
    m.insert(
        "Bitstamp_28",
        address!("c5b611f502a0DCF6C3188Fd494061aE29B2baa4f"),
    );
    m.insert(
        "Bitstamp_29",
        address!("Cddf488f1c826160eE832D4f1492f00cf8557Ff6"),
    );
    m.insert(
        "Bitstamp_3",
        address!("1522900B6daFac587d499a862861C0869Be6E428"),
    );
    m.insert(
        "Bitstamp_30",
        address!("d4FCC07a8da7d55599167991D4AB47f976d0A306"),
    );
    m.insert(
        "Bitstamp_31",
        address!("De0f7Df88678E2aee576a2f3D9B18d4DfAd0155C"),
    );
    m.insert(
        "Bitstamp_32",
        address!("e1576685451986e3f93C2fb87CCa3AEC5b5d45D0"),
    );
    m.insert(
        "Bitstamp_33",
        address!("eE9FB7A615cb76b46d26BE6EbC9114a627A81C5B"),
    );
    m.insert(
        "Bitstamp_34",
        address!("fbb23038Fe6CFa16Aa898d7DbCa7C3269bDAf258"),
    );
    m.insert(
        "Bitstamp_35",
        address!("fCA70E67b3f93f679992Cd36323eEB5a5370C8e4"),
    );
    m.insert(
        "Bitstamp_4",
        address!("182E1259eF6Ee45Dc811132eF4Ba5871F1536822"),
    );
    m.insert(
        "Bitstamp_5",
        address!("1F69d824c3b4F906ac3FC8826e2391Bcb9330E02"),
    );
    m.insert(
        "Bitstamp_6",
        address!("333C100AE1A2743a1e55D73913cAC6d95deB7F62"),
    );
    m.insert(
        "Bitstamp_7",
        address!("3F3E23249F38d35A4CdAf44eDFD99eeb4325b401"),
    );
    m.insert(
        "Bitstamp_8",
        address!("4c0907f7ad337635A7fd414A0c7a938e0d64BF4D"),
    );
    m.insert(
        "Bitstamp_9",
        address!("4c766dEf136F59f6494f0969B1355882080CF8E0"),
    );
    m.insert(
        "Poloniex",
        address!("007abbe8057433641aCB791d966D33a12cf82d01"),
    );
    m.insert(
        "Poloniex_1",
        address!("0536806df512D6cDDE913Cf95c9886f65b1D3462"),
    );
    m.insert(
        "Poloniex_10",
        address!("6B71834D65C5C4d8eD158D54B47E6Ea4Ff4E5437"),
    );
    m.insert(
        "Poloniex_11",
        address!("6F803466bCD17f44fa18975bf7c509ba64Bf3825"),
    );
    m.insert(
        "Poloniex_12",
        address!("8d451AE5ee8F557a9cE7A9D7Be8A8cb40002d5cB"),
    );
    m.insert(
        "Poloniex_13",
        address!("8fCA4adE3a517133fF23ca55CdAea29C78C990b8"),
    );
    m.insert(
        "Poloniex_14",
        address!("A910f92ACdAf488fa6eF02174fb86208Ad7722ba"),
    );
    m.insert(
        "Poloniex_15",
        address!("Aa9fa73dFE17ecAa2C89b39f0bb2779613C5Fc3b"),
    );
    m.insert(
        "Poloniex_16",
        address!("aB11204cfEacCFfa63C2D23AeF2Ea9aCCDB0a0D5"),
    );
    m.insert(
        "Poloniex_17",
        address!("b42b20ddbEabdC2a288Be7FF847fF94fB48d2579"),
    );
    m.insert(
        "Poloniex_18",
        address!("b794F5eA0ba39494cE839613fffBA74279579268"),
    );
    m.insert(
        "Poloniex_19",
        address!("Bd2Ec7c608a06fE975DBDCA729E84dEdb34eCC21"),
    );
    m.insert(
        "Poloniex_2",
        address!("209c4784AB1E8183Cf58cA33cb740efbF3FC18EF"),
    );
    m.insert(
        "Poloniex_20",
        address!("BFC39b6F805a9E40E77291afF27aeE3C96915BDD"),
    );
    m.insert(
        "Poloniex_21",
        address!("c0e30823e5e628df8bc9bf2636a347E1512F0ecb"),
    );
    m.insert(
        "Poloniex_22",
        address!("Df21fA922215B1a56f5a6D6294E6E36c85A0Acfb"),
    );
    m.insert(
        "Poloniex_23",
        address!("EaD6be34CE315940264519f250d8160f369fa5cd"),
    );
    m.insert(
        "Poloniex_24",
        address!("fbf2173154F7625713be22E0504404EBfE021eae"),
    );
    m.insert(
        "Poloniex_3",
        address!("2fA2Bc2ce6A4f92952921A4CAA46B3727D24a1ec"),
    );
    m.insert(
        "Poloniex_4",
        address!("31a2Feb9b5D3b5f4e76C71D6C92FC46eBb3cb1c1"),
    );
    m.insert(
        "Poloniex_5",
        address!("32Be343B94f860124dC4fEe278FDCBD38C102D88"),
    );
    m.insert(
        "Poloniex_6",
        address!("36B01066b7fa4a0fdb2968eA0256C848e9135674"),
    );
    m.insert(
        "Poloniex_7",
        address!("48d466B7c0d32B61E8A82Cd2bCF060F7C3F966df"),
    );
    m.insert(
        "Poloniex_8",
        address!("65F9B2e4d7aAEB40fFEA8C6F5844d5AD7Da257E0"),
    );
    m.insert(
        "Poloniex_9",
        address!("6795cf8EB25585EaDC356Ae32AC6641016c550f2"),
    );
    m.insert(
        "Bittrex",
        address!("66f820a414680B5bcda5eECA5dea238543F42054"),
    );
    m.insert(
        "Bittrex_1",
        address!("a3C1E324CA1ce40db73eD6026c4A177F099B5770"),
    );
    m.insert(
        "Bittrex_2",
        address!("E94b04a0FeD112f3664e45adb2B8915693dD5FF3"),
    );
    m.insert(
        "Bittrex_3",
        address!("FBb1b73C4f0BDa4f67dcA266ce6Ef42f520fBB98"),
    );
    m.insert("MEXC", address!("0211f3ceDbEf3143223D3ACF0e589747933e8527"));
    m.insert(
        "MEXC_1",
        address!("2e8F79aD740de90dC5F5A9F0D8D9661a60725e64"),
    );
    m.insert(
        "MEXC_10",
        address!("9b64203878F24eB0CDF55c8c6fA7D08Ba0cF77E5"),
    );
    m.insert(
        "MEXC_11",
        address!("DF90C9B995a3b10A5b8570a47101e6c6a29eb945"),
    );
    m.insert(
        "MEXC_12",
        address!("ffB3118124cdaEbD9095fA9a479895042018cac2"),
    );
    m.insert(
        "MEXC_2",
        address!("3CC936b795A188F0e246cBB2D74C5Bd190aeCF18"),
    );
    m.insert(
        "MEXC_3",
        address!("4982085C9e2F89F2eCb8131Eca71aFAD896e89CB"),
    );
    m.insert(
        "MEXC_4",
        address!("4e3ae00E8323558fA5Cac04b152238924AA31B60"),
    );
    m.insert(
        "MEXC_5",
        address!("51E3D44172868Acc60D68ca99591Ce4230bc75E0"),
    );
    m.insert(
        "MEXC_6",
        address!("576b81F0c21EDBc920ad63FeEEB2b0736b018A58"),
    );
    m.insert(
        "MEXC_7",
        address!("75e89d5979E4f6Fba9F97c104c2F0AFB3F1dcB88"),
    );
    m.insert(
        "MEXC_8",
        address!("83c1C224044Ef8573e9a728dBb91013CF80827E6"),
    );
    m.insert(
        "MEXC_9",
        address!("8E1701CFd85258DDb8DFE89Bc4c7350822B9601D"),
    );
    m.insert(
        "BitMart",
        address!("03231B778a16D2d5222b4CED947c7Ad3fEa14635"),
    );
    m.insert(
        "BitMart_1",
        address!("03Ca1829f4D3839467701592b9aDCC7bAbBD8769"),
    );
    m.insert(
        "BitMart_10",
        address!("5c7beD3Cca42e4562877eD88B9Aa0F5898Ed59B0"),
    );
    m.insert(
        "BitMart_11",
        address!("68b22215FF74E3606BD5E6c1DE8c2D68180c85F7"),
    );
    m.insert(
        "BitMart_12",
        address!("6D0D19bddDC5ED1dD501430c9621DD37ebd9062d"),
    );
    m.insert(
        "BitMart_13",
        address!("701f38C8c0eE48b4c1e5aEfc0a3C6880f1d3d445"),
    );
    m.insert(
        "BitMart_14",
        address!("7563758243A262E96880F178aeE7817DcF47Ab0f"),
    );
    m.insert(
        "BitMart_15",
        address!("79288AC3525c4E7669481571658A867F7E18f0B2"),
    );
    m.insert(
        "BitMart_16",
        address!("8c128DBA2cB66399341AA877315BE1054be75da8"),
    );
    m.insert(
        "BitMart_17",
        address!("8EaFEE3d0DF538A1e04487a43239c1C73B50032d"),
    );
    m.insert(
        "BitMart_18",
        address!("a1F54002f695E79380C7A0A27B14C8e33f1E1228"),
    );
    m.insert(
        "BitMart_19",
        address!("A9E4332448318dA58CDD398286c0809684eD9BD4"),
    );
    m.insert(
        "BitMart_2",
        address!("11FC614E2218b479F94636C234BC352EE490EFd1"),
    );
    m.insert(
        "BitMart_20",
        address!("Abb239191ab5d0482Ab7C74F412d2B117f4EeD3a"),
    );
    m.insert(
        "BitMart_21",
        address!("d11616e66b128c0b756b91cC13466deFaae67D07"),
    );
    m.insert(
        "BitMart_22",
        address!("e79eeF9b9388A4fF70ed7ec5Bccd5B928ebB8Bd1"),
    );
    m.insert(
        "BitMart_23",
        address!("eACB50a28630a4C44a884158eE85cBc10d2B3F10"),
    );
    m.insert(
        "BitMart_24",
        address!("f3d4aa3C6925B38D40C2ae4C7A935d83666Ae5f7"),
    );
    m.insert(
        "BitMart_25",
        address!("F8f21a32648a540dd8e982ff47BEF6Be2e823F9E"),
    );
    m.insert(
        "BitMart_26",
        address!("F990D51057Ee5EBd2EB627E5F179dAc90e3B2b25"),
    );
    m.insert(
        "BitMart_3",
        address!("1Aac8BC17DA523b9bC7470B0C9eD47a83760ACef"),
    );
    m.insert(
        "BitMart_4",
        address!("328130164d0F2B9D7a52edC73b3632e713ff0ec6"),
    );
    m.insert(
        "BitMart_5",
        address!("3752a0F9BeA7cDf593F46533E23853161233BD04"),
    );
    m.insert(
        "BitMart_6",
        address!("3aB28eCeDEa6cdb6feeD398E93Ae8c7b316B1182"),
    );
    m.insert(
        "BitMart_7",
        address!("3b53e17E54c64e83185954726d251c040200D80F"),
    );
    m.insert(
        "BitMart_8",
        address!("3f0A468c36E575a994e0166bdc2C62896f3A4A80"),
    );
    m.insert(
        "BitMart_9",
        address!("4bd3F43C6bfbFda03664ee3Ce4C2bcceAe2AAab0"),
    );
    m.insert(
        "Upbit",
        address!("03747F06215B44E498831dA019B27f53E483599F"),
    );
    m.insert(
        "Upbit_1",
        address!("1938A448d105D26c40A52a1Bfe99B8Ca7a745aD0"),
    );
    m.insert(
        "Upbit_2",
        address!("390dE26d772D2e2005C6d1d24afC902bae37a4bB"),
    );
    m.insert(
        "Upbit_3",
        address!("4F01001cf69785d4c37f03Fd87398849411ccbBa"),
    );
    m.insert(
        "Upbit_4",
        address!("5E032243d507C743b061eF021e2EC7fcc6d3ab89"),
    );
    m.insert(
        "Upbit_5",
        address!("BA826fEc90CEFdf6706858E5FbaFcb27A290Fbe0"),
    );
    m.insert(
        "Upbit_6",
        address!("c9cf0eC93d764f5c9571fD12f764Bae7fC87C84e"),
    );
    m.insert(
        "Bithumb",
        address!("0016C0d0343e8f2c3A7b6A51606B84B1545Ec606"),
    );
    m.insert(
        "Bithumb_1",
        address!("03599A2429871E6be1B154Fb9c24691F9D301865"),
    );
    m.insert(
        "Bithumb_10",
        address!("3052cD6BF951449A984fe4B5a38B46AEF9455c8E"),
    );
    m.insert(
        "Bithumb_11",
        address!("3154f72F0A0b9023F1c18505c2b73f9cd1990caE"),
    );
    m.insert(
        "Bithumb_12",
        address!("31D03f07178BcD74F9099AfeBD23B0AE30184ab5"),
    );
    m.insert(
        "Bithumb_13",
        address!("3B83Cd1a8e516B6Eb9f1Af992E9354b15A6F9672"),
    );
    m.insert(
        "Bithumb_14",
        address!("3fBE1f8Fc5dDb27d428aA60f661EAAaB0d2000ce"),
    );
    m.insert(
        "Bithumb_15",
        address!("47E3aC26C5A8f1715DabFE1DB00e4bf1F54aFe23"),
    );
    m.insert(
        "Bithumb_16",
        address!("5521a68D4F8253fC44BFb1490249369b3E299A4A"),
    );
    m.insert(
        "Bithumb_17",
        address!("558553D54183a8542F7832742e7B4Ba9c33Aa1E6"),
    );
    m.insert(
        "Bithumb_18",
        address!("560e389a2b032319E742A59AE8Bafa62671089fe"),
    );
    m.insert(
        "Bithumb_19",
        address!("6fC48dE8f167456b7aa27dD4ecfaBBA329EA623D"),
    );
    m.insert(
        "Bithumb_2",
        address!("0f863d0bBC760D7EF8d47c43639c13Fc4B0f2e04"),
    );
    m.insert(
        "Bithumb_20",
        address!("771a2dA83236a230c15CE3EB07be4dE7164E3cfF"),
    );
    m.insert(
        "Bithumb_21",
        address!("7Be98E1D8f29D23338E61694007D50D67144C7eF"),
    );
    m.insert(
        "Bithumb_22",
        address!("83761c6785427F5A27a07c92a9dcFa99947bC4AD"),
    );
    m.insert(
        "Bithumb_23",
        address!("88D34944cF554e9CCCf4a24292D891f620e9c94F"),
    );
    m.insert(
        "Bithumb_24",
        address!("8FA8aF91C675452200e49b4683a33Ca2E1A34e42"),
    );
    m.insert(
        "Bithumb_25",
        address!("97122dDca38c29b7653D52b07998d06a7128fa0B"),
    );
    m.insert(
        "Bithumb_26",
        address!("A0Ff1e0F30b5DDA2dc01e7e828290Bc72b71E57d"),
    );
    m.insert(
        "Bithumb_27",
        address!("a5Dab3C7a3821F6440a10d634E766bFD2750E54e"),
    );
    m.insert(
        "Bithumb_28",
        address!("a84Aa98cA1C5DBFf58F825e28FaD700653439d5F"),
    );
    m.insert(
        "Bithumb_29",
        address!("b08f3dC596fE40bE860ceE549b5c0a2f61De9f9F"),
    );
    m.insert(
        "Bithumb_3",
        address!("15878e87c685f866edFaF454BE6Dc06Fa517B35B"),
    );
    m.insert(
        "Bithumb_30",
        address!("b4460b75254ce0563Bb68eC219208344C7EA838c"),
    );
    m.insert(
        "Bithumb_31",
        address!("B470faEa0dc6E1a99DbB4fA95B5bE47D9C27e00A"),
    );
    m.insert(
        "Bithumb_32",
        address!("bb5A0408Fa54287B9074A2f47AB54c855e95EF82"),
    );
    m.insert(
        "Bithumb_33",
        address!("c1dA8F69e4881efe341600620268934ef01a3E63"),
    );
    m.insert(
        "Bithumb_34",
        address!("d273Bd546b11Bd60214A2F9d71f22A088AAfe31B"),
    );
    m.insert(
        "Bithumb_35",
        address!("d341Dd814Eb0937Caf3517Ff2203C6d26F306898"),
    );
    m.insert(
        "Bithumb_36",
        address!("D5a11A51fD0CDA5F119b78D87EaeAa970D77c55e"),
    );
    m.insert(
        "Bithumb_37",
        address!("E320D449F11560ed9ff917799CB7CF10fFD7d6Ba"),
    );
    m.insert(
        "Bithumb_38",
        address!("ed48DC0628789c2956B1E41726d062a86ec45bFF"),
    );
    m.insert(
        "Bithumb_39",
        address!("EFb2E870b14D7e555a31B392541ACf002Dae6aE9"),
    );
    m.insert(
        "Bithumb_4",
        address!("186549a4aE594fc1F70bA4CFFDAc714b405bE3F9"),
    );
    m.insert(
        "Bithumb_40",
        address!("F1BafEFDED83fe7D2b754b3393445D01A7d3573B"),
    );
    m.insert(
        "Bithumb_5",
        address!("2140eFD7Ba31169c69dfff6CDC66C542f0211825"),
    );
    m.insert(
        "Bithumb_6",
        address!("22B84d5FFeA8b801C0422AFe752377A64Aa738c2"),
    );
    m.insert(
        "Bithumb_7",
        address!("26BC3D0BB634E242359849Fe257a4cF76444D504"),
    );
    m.insert(
        "Bithumb_8",
        address!("2F41Ea745c67724FDC65FF909318EDeB73Cfd6e7"),
    );
    m.insert(
        "Bithumb_9",
        address!("2fFFb384d9bFB5F824958503B60D0d5962b080Ce"),
    );
    m.insert(
        "1xBet",
        address!("5c89724967D76a4f966b013140355789093F6c7D"),
    );
    m.insert(
        "1xBet_1",
        address!("777f415324d56e1d54fa832902d8797dB7A4c57C"),
    );
    m.insert(
        "1xBet_2",
        address!("BA3801847037ffe8dE609CCfDd8E02C2F60AdC43"),
    );
    m.insert("AAX", address!("4dCa3852323Ae417D4C7d735100A629130d50e90"));
    m.insert(
        "AAX_1",
        address!("80edADF751946A71a0131a495BB7abBC75F46f6C"),
    );
    m.insert(
        "AAX_2",
        address!("8Dc11398263ffF36eB91602A18877112eaD2EDE4"),
    );
    m.insert(
        "AAX_3",
        address!("A4EE5581fDcBAAf52c5EdB65c8fBb7D3DE78b838"),
    );
    m.insert(
        "AAX_4",
        address!("c25DC289Edce5227cf15d42539824509e826b54D"),
    );
    m.insert(
        "AAX_5",
        address!("d25f30CA035250cff6810187e2Da6F924Bd9616e"),
    );
    m.insert(
        "ABCC Exchange",
        address!("05f51AAb068CAa6Ab7eeb672f88c180f67F17eC7"),
    );
    m.insert(
        "ABCC Exchange_1",
        address!("0A26d96143f8C5754A588dE5Bef1738cF0B6A28F"),
    );
    m.insert(
        "ABCC Exchange_2",
        address!("AA9133EeC3ae5f9440C1a1E61E2D2Cc571675527"),
    );
    m.insert(
        "APROBIT",
        address!("aecBE94703Df39B49Ac440fEB177c7f1f782c064"),
    );
    m.insert(
        "ATAIX",
        address!("4dF5f3610e2471095a130D7d934D551f3ddE01ED"),
    );
    m.insert("Abra", address!("0e0066aca9ef6B8102D8Dbc66AB0091f9370a7cb"));
    m.insert(
        "Aeroswap",
        address!("0e8D02aE96b229f112f37502C2A26D66BDBcff1F"),
    );
    m.insert(
        "Aeroswap_1",
        address!("397Be73c6160E5E66408924dA4aD32eA5e9ea9db"),
    );
    m.insert(
        "Aeroswap_2",
        address!("7D411c4279A96069Fe32De0c0EAB4a964E8ED9C4"),
    );
    m.insert(
        "Aeroswap_3",
        address!("8EB871bbB6F754a04bCa23881A7D25A30aAD3f23"),
    );
    m.insert(
        "Alcumex Exchange",
        address!("2DDD202174A72514ed522E77972b461b03155525"),
    );
    m.insert(
        "Allbit",
        address!("dc1882F350b42ac9a23508996254b1915c78b204"),
    );
    m.insert(
        "Allbit_1",
        address!("Ff6b1cdfD2d3e37977d7938AA06b6d89D6675e27"),
    );
    m.insert(
        "AlphaPo",
        address!("183A6cF1Fc6504138d92C9d663094EE774f80038"),
    );
    m.insert(
        "AlphaPo_1",
        address!("6dfc34609a05bC22319fA4Cce1d1E2929548c0D7"),
    );
    m.insert(
        "AlphaPo_2",
        address!("808d0aeE8db7E7c74FaF4b264333aFE8c9cCDBA4"),
    );
    m.insert(
        "AltCoinTrader",
        address!("0Da044B16bB53AB6d6691e25b1ec7aD380Fa5Fa7"),
    );
    m.insert(
        "AltCoinTrader_1",
        address!("1BC972Db7B169C2D79719485803aF1A23645108d"),
    );
    m.insert(
        "AltCoinTrader_2",
        address!("2Cf4E5B4Fa8Afa15F8dc7B5adA853e42776f3455"),
    );
    m.insert(
        "AltCoinTrader_3",
        address!("87b7bA194eD714a871C91F6B9Ba8Dc8182eD3A5d"),
    );
    m.insert(
        "AltCoinTrader_4",
        address!("a1495F85d30aCAB5126f5C1920cD9cb1CE265263"),
    );
    m.insert(
        "AltCoinTrader_5",
        address!("a2Fe6EC4244ee94853326F70893ce5eE20AA4fFb"),
    );
    m.insert(
        "AltCoinTrader_6",
        address!("b56F9e1aecb821413C9F14822D3918A363D83226"),
    );
    m.insert(
        "AltCoinTrader_7",
        address!("c58Bb74606b73c5043B75d7Aa25ebe1D5D4E7c72"),
    );
    m.insert(
        "AltCoinTrader_8",
        address!("E1336fA8165Bc4DbaDC7FB95718A3C9DcAe23fdc"),
    );
    m.insert(
        "AltCoinTrader_9",
        address!("Ff2E0C46C7673ccD00cB5B59Dc1686229A483Fc8"),
    );
    m.insert(
        "AlterDice",
        address!("2425B5c48327DA2a8bE22E57207ae8056c3f42ee"),
    );
    m.insert(
        "AlterDice_1",
        address!("690e96f32A225F661A1881a484F858276CB82984"),
    );
    m.insert(
        "Anchorage Digital",
        address!("3161b9660cc36C00dfC36307De2B8C53960164dC"),
    );
    m.insert(
        "Anchorage Digital_1",
        address!("A44D54Ae6A00e095dAA000365C99c4A27303b6f3"),
    );
    m.insert(
        "Anchorage Digital_2",
        address!("d52055A39a3d2f7505C739f981f296Ea31B50191"),
    );
    m.insert(
        "Anycoin Direct",
        address!("3EEBBEBeCde31d36D6AA7AA4FE2A06159119b659"),
    );
    m.insert(
        "Anycoin Direct_1",
        address!("5B31Bb52bdd7006ae57F9d9506c0FF995229b63c"),
    );
    m.insert(
        "Arkham",
        address!("0323718324218dcBfF7c9f89bA5a5954F61A6c74"),
    );
    m.insert(
        "Arkham_1",
        address!("34407900475cEF87acE1597670A9A42F31961d02"),
    );
    m.insert(
        "Arkham_2",
        address!("679Fb19dEc9d66C34450a8563FfDFD29C04e615A"),
    );
    m.insert(
        "Arkham_3",
        address!("Dc2822D0685c0CcEAb07b35d6de4aC9280FB9cFF"),
    );
    m.insert(
        "Artis Turba Exchange",
        address!("94597850916a49b3B152EE374E97260B99249f5B"),
    );
    m.insert(
        "Artis Turba Exchange_1",
        address!("f0c80FB9FB22BEF8269CB6fEB9a51130288a671f"),
    );
    m.insert(
        "ArzPaya.com",
        address!("82a403c14483931B2fF6e4440c8373ccFEe698B8"),
    );
    m.insert(
        "AscendEX",
        address!("03BDf69B1322D623836aFBD27679A1C0AfA067E9"),
    );
    m.insert(
        "AscendEX_1",
        address!("09344477fDc71748216a7b8BbE7F2013B893DeF8"),
    );
    m.insert(
        "AscendEX_2",
        address!("4240781A9ebDB2EB14a183466E8820978b7DA4e2"),
    );
    m.insert(
        "AscendEX_3",
        address!("4B1a99467a284Cc690e3237bC69105956816f762"),
    );
    m.insert(
        "AscendEX_4",
        address!("80Ca27268d4603E00B8d4D98Aa309dB438127d19"),
    );
    m.insert(
        "AscendEX_5",
        address!("8FaB0D3E5eE6FFdb73589c2C47dcD3802360694f"),
    );
    m.insert(
        "AscendEX_6",
        address!("9715254754284a0b3e4C7BF8f57E790415041c1C"),
    );
    m.insert(
        "AscendEX_7",
        address!("983873529f95132BD1812A3B52c98Fb271d2f679"),
    );
    m.insert(
        "AscendEX_8",
        address!("986a2fCa9eDa0e06fBf7839B89BfC006eE2a23Dd"),
    );
    m.insert(
        "AscendEX_9",
        address!("9BD376BFce4B97c6fAe3F438d516Ae1582168596"),
    );
    m.insert(
        "AtomSolutions",
        address!("054C64741dBafDC19784505494029823D89c3b13"),
    );
    m.insert(
        "AtomSolutions_1",
        address!("112b12089611749406fde450FDa9917F7F4Ac3CB"),
    );
    m.insert(
        "AtomSolutions_2",
        address!("2F5A7b563E6C4761d478273F5f9B4A444BDd2E3C"),
    );
    m.insert(
        "AtomSolutions_3",
        address!("5EcA044b580a86e7ab4b2076330978d6D125a270"),
    );
    m.insert(
        "AtomSolutions_4",
        address!("A28d81bF8e4823cf4d6Cc2767507dEe271994e1A"),
    );
    m.insert(
        "Azbit",
        address!("22682575E073736Ed25258409B09e0e7AF6D9C61"),
    );
    m.insert(
        "Azbit_1",
        address!("36e75f48c5D67e0c619d6F56a3481A21bE57e322"),
    );
    m.insert(
        "Azbit_2",
        address!("92dBD8e0A46EdD62AA42d1f7902D0e496Bddc15A"),
    );
    m.insert(
        "B2BinPay",
        address!("20Dc0b9520CC2C2BE89F247061A2c8e310045949"),
    );
    m.insert(
        "B2BinPay_1",
        address!("849A02be4c2ec8BbD06052C5A0Cd51147994Ad96"),
    );
    m.insert(
        "B2BinPay_2",
        address!("a7fB5cA286Fc3FD67525629048a4de3bA24Cba2E"),
    );
    m.insert(
        "BTC Markets",
        address!("3BC643A841915A267eE067b580BD802a66001C1d"),
    );
    m.insert(
        "BTC Markets_1",
        address!("8a44DC02E250F0f0f388B73a257C53E3BB50321d"),
    );
    m.insert(
        "BTC Markets_2",
        address!("aecfb1af29B96011EC9AA1Ff98D8C49b49bB3dDc"),
    );
    m.insert(
        "BTC Markets_3",
        address!("C55EdDadEeB47fcDE0B3B6f25BD47D745BA7E7fa"),
    );
    m.insert(
        "BTC Markets_4",
        address!("cA46fBDC3Dfe107d033682Acb1EF212fe555E731"),
    );
    m.insert(
        "BTC-Alpha Exchange",
        address!("1c00d840ccAa67c494109F46E55cFEB2D8562F5c"),
    );
    m.insert(
        "BTC-e",
        address!("91337A300e0361BDDb2e377DD4e88CCB7796663D"),
    );
    m.insert(
        "BTC-e_1",
        address!("c73f25a029352931a64b328C82511e88188A8c96"),
    );
    m.insert(
        "BTC.com",
        address!("EEa5B82B61424dF8020f5feDD81767f2d0D25Bfb"),
    );
    m.insert(
        "BTCEX",
        address!("4f26B5961210F295542B0c5C13c4887E24F0910E"),
    );
    m.insert(
        "BTCEX_1",
        address!("59EdC943735Aa4f2b1a1e4D7bCb46ebE09F42C8C"),
    );
    m.insert(
        "BTCEX_2",
        address!("BbdaEA89Ced53Bf9E31A4cEb926832fBB1bC0bB4"),
    );
    m.insert(
        "BTCEX_3",
        address!("d020220CA4841229514d1bc02a8aBCE4C162C015"),
    );
    m.insert(
        "BTCEX_4",
        address!("fa66605B88e16c2fD011622dEe6F35A976098eDb"),
    );
    m.insert("BTSE", address!("1619d743d7DC612E99d5D94Ebd6b9695D46f0BF3"));
    m.insert(
        "BTSE_1",
        address!("661BC014Ba045A1918215cbe2aF8121ADA09638D"),
    );
    m.insert(
        "BTSE_2",
        address!("9036B1eB7630d9A45720FD80D05D46262f460529"),
    );
    m.insert(
        "BTSE_3",
        address!("b0afFFd6f6Ad77f61927803ADE6dbD47f1a1C356"),
    );
    m.insert(
        "BTSE_4",
        address!("bB4D1DC5c1ABec4Ea11166ec97E714862863aD1D"),
    );
    m.insert(
        "BTSE_5",
        address!("DDAad971BE05321FD541372CD710a7f0555972eD"),
    );
    m.insert(
        "BTSE_6",
        address!("de279a5cD86860Cd3D039AA1B74bc29E74cABB12"),
    );
    m.insert(
        "BW.com",
        address!("73957709695E73Fd175582105c44743CF0fB6f2f"),
    );
    m.insert(
        "BW.com_1",
        address!("bCDFC35b86BedF72F0Cda046A3c16829A2Ef41d1"),
    );
    m.insert("Bake", address!("94fa70d079D76279e1815ce403e9B985bcCC82AC"));
    m.insert(
        "Beaxy",
        address!("6D932cB67760F6a5343998bebAc85c0DE7C9aA10"),
    );
    m.insert(
        "Beaxy_1",
        address!("Adb72986EAd16bDbc99208086BD431C1Aa38938e"),
    );
    m.insert(
        "Beldex",
        address!("258B7B9A1BA92f47f5F4f5e733293477620a82Cb"),
    );
    m.insert(
        "BetFury",
        address!("52A258ED593C793251a89bfd36caE158EE9fC4F8"),
    );
    m.insert(
        "Bgogo",
        address!("7A10Ec7d68a048BdaE36A70E93532D31423170fA"),
    );
    m.insert(
        "Bgogo_1",
        address!("Ce1bF8E51F8b39e51c6184e059786D1c0eAF360F"),
    );
    m.insert("BiKi", address!("06BA294D190b5F9788FfCa86cE19a43CD746d36A"));
    m.insert(
        "BiKi_1",
        address!("6eFb20f61B80F6a7ebe7a107baCe58288a51FB34"),
    );
    m.insert(
        "BiKi_2",
        address!("6efF3372fa352b239Bb24ff91b423A572347000D"),
    );
    m.insert(
        "BiKi_3",
        address!("F71CBF6758aaAaF06eBCcA5447019c31bB145782"),
    );
    m.insert(
        "BiKi_4",
        address!("fd736FAA01073D35c148B61093E6AE562B2f8544"),
    );
    m.insert(
        "Bibox",
        address!("24D55BF5031D46b8Ebb656e905e3CcE759EA526F"),
    );
    m.insert(
        "Bibox_1",
        address!("76bD39DBc1cc977c03d38dc8a70ECbF21177c0Df"),
    );
    m.insert(
        "Bibox_2",
        address!("B0d3C2D9F7D3C65D83a3Af84A8584C2AD6Bee3E4"),
    );
    m.insert(
        "Bibox_3",
        address!("EA7B33d264F4B7e6fd283A8250a572f2cEEfaCD4"),
    );
    m.insert(
        "Bibox_4",
        address!("f73C3c65bde10BF26c2E1763104e609A41702EFE"),
    );
    m.insert(
        "Biconomy",
        address!("856cb5c3cBBe9e2E21293A644aA1f9363CEE11E8"),
    );
    m.insert(
        "Biconomy_1",
        address!("94D3E62151B12A12A4976F60EdC18459538FaF08"),
    );
    m.insert(
        "Biconomy_2",
        address!("c864019047B864B6ab609a968ae2725DFaee808A"),
    );
    m.insert(
        "Bidesk",
        address!("0639bFE9e40A08e7E08A04e119B13AFFbEb9AeD9"),
    );
    m.insert(
        "Bidesk_1",
        address!("0bB5DE248DbbD31eE6c402C3c4a70293024ACf74"),
    );
    m.insert(
        "Bidesk_10",
        address!("2fFe12462A7415A154a9db89266F257061c83f3D"),
    );
    m.insert(
        "Bidesk_11",
        address!("3aaD4FF78052fDF407CD4eb856923D3180e941D3"),
    );
    m.insert(
        "Bidesk_12",
        address!("3c047B9Dfa4fd8F812f264f1611b959a4DD980f8"),
    );
    m.insert(
        "Bidesk_13",
        address!("40A6C95f8809D7b0363Eb559Fbe3481d3731Df68"),
    );
    m.insert(
        "Bidesk_14",
        address!("42289749f57C0fB81f3C079291F9D3513b76FbF8"),
    );
    m.insert(
        "Bidesk_15",
        address!("45E5b30803C3a5c6f9E68451026D912CD9C9EFd6"),
    );
    m.insert(
        "Bidesk_16",
        address!("46f696DDDBb9edBF504A4e0226016190995B6dc6"),
    );
    m.insert(
        "Bidesk_17",
        address!("5388cBdB0A5B760953FBBAE2bD341c4a06b8dd79"),
    );
    m.insert(
        "Bidesk_18",
        address!("599814467C8AcBD77B761702f741325af5018a1f"),
    );
    m.insert(
        "Bidesk_19",
        address!("5D027af50Ae06f210BC8ED64f183c248Eb95eAc8"),
    );
    m.insert(
        "Bidesk_2",
        address!("0C2F8e43cc0d7184D1057620994fa32F03F64a8d"),
    );
    m.insert(
        "Bidesk_20",
        address!("6358A984302f5c7C261743423aCa4B3fF575765B"),
    );
    m.insert(
        "Bidesk_21",
        address!("6ec17B53659C71427cFeb19ACFAE5ABd29b69a16"),
    );
    m.insert(
        "Bidesk_22",
        address!("7423931617700331FA34D2Df2b58Ee809626625B"),
    );
    m.insert(
        "Bidesk_23",
        address!("7915E66B491C0e0D6e4f2d00C398866332B7c167"),
    );
    m.insert(
        "Bidesk_24",
        address!("7FeC5b3d62f357Dc0431f38ed73c8198420430D5"),
    );
    m.insert(
        "Bidesk_25",
        address!("8c699A889413893Ca1f03bA6405ae66c66F955E6"),
    );
    m.insert(
        "Bidesk_26",
        address!("8D76166C22658A144c0211d87Abf152e6a2d9D95"),
    );
    m.insert(
        "Bidesk_27",
        address!("96C604c94DCc5E356d1818F321cec9826675964b"),
    );
    m.insert(
        "Bidesk_28",
        address!("99bD796babE675B410943387b2e0Bf87a54296B9"),
    );
    m.insert(
        "Bidesk_29",
        address!("A31974408AAD18C1e5C4648d435795Fa757EA47f"),
    );
    m.insert(
        "Bidesk_3",
        address!("103f11005A9521F5CC3D6331A2C0c441ED827735"),
    );
    m.insert(
        "Bidesk_30",
        address!("a4938E606eDfe350D08813a0785Ed85D4640d365"),
    );
    m.insert(
        "Bidesk_31",
        address!("b85a1fb66A67804A017E7B9270Db5c6c06c21A3C"),
    );
    m.insert(
        "Bidesk_32",
        address!("C13CA46A9C895b6aFd8a44A1A8238a242DBd176D"),
    );
    m.insert(
        "Bidesk_33",
        address!("c2BA04E89016f417e1219Af7eF82a5B6A9214793"),
    );
    m.insert(
        "Bidesk_34",
        address!("CdFf68f58470E19c1D77643e864a9D4437754db4"),
    );
    m.insert(
        "Bidesk_35",
        address!("Ce76a14dABe64acb8b044c489374C8ca1f456837"),
    );
    m.insert(
        "Bidesk_36",
        address!("d61Ff104C480A225AB4666b95b5Ddeb101352A56"),
    );
    m.insert(
        "Bidesk_37",
        address!("De528EceC4F16cc20bFAfAD23B41388343C243C4"),
    );
    m.insert(
        "Bidesk_38",
        address!("e05b4BDf83274545E64f4f19Db75e64E6492A3b3"),
    );
    m.insert(
        "Bidesk_39",
        address!("E383E773245a93574B528b5415C752BB866AD23A"),
    );
    m.insert(
        "Bidesk_4",
        address!("1a9e7F7D49f4E9FddF063617Cde6514b711758cB"),
    );
    m.insert(
        "Bidesk_40",
        address!("Ed5cdB0D02152046E6f234aD578613831b9184D4"),
    );
    m.insert(
        "Bidesk_41",
        address!("ed643A3cf76AB65F7F9a7833656881e91097B6d2"),
    );
    m.insert(
        "Bidesk_42",
        address!("F0d04D794053936807D553319dd0988D77Ab78df"),
    );
    m.insert(
        "Bidesk_43",
        address!("F1F178e5Cf884134A72D48d07f259894390FF1f9"),
    );
    m.insert(
        "Bidesk_44",
        address!("f3385c7aE159057Bbe1Aae4eB4292E84c2679365"),
    );
    m.insert(
        "Bidesk_45",
        address!("F6cf41af1dc8aec47dBf92Eb0Ef23099b22b803E"),
    );
    m.insert(
        "Bidesk_46",
        address!("FA3F128bE5ce8c42082cA1468544e6e1F9F22824"),
    );
    m.insert(
        "Bidesk_47",
        address!("fbc865A47c741Be6a245e1Cbf9A7Fcfae048aEDb"),
    );
    m.insert(
        "Bidesk_5",
        address!("1b9e8A6caE8Dc7796cEbb40997eBC56e798E7578"),
    );
    m.insert(
        "Bidesk_6",
        address!("1Ee7C37EFF6a37d38d805A0ceDc134147a9DD3Cf"),
    );
    m.insert(
        "Bidesk_7",
        address!("1F13246D1D34f5Ae1C588DbEA54248451801065f"),
    );
    m.insert(
        "Bidesk_8",
        address!("233Da32CA8CD6CE0928C9893382216E6F81F8F16"),
    );
    m.insert(
        "Bidesk_9",
        address!("23ce3e09CeCF4D0B1C876224731F0B73B5e523bC"),
    );
    m.insert(
        "BigONE",
        address!("17Bc58b788808DaB201a9A90817fF3C168BF3d61"),
    );
    m.insert(
        "BigONE_1",
        address!("1A84b64Cc85BFab627dA6537cBE8E9E26C0B3ed0"),
    );
    m.insert(
        "BigONE_2",
        address!("493144718a78DfBEF79F825AE71b29134b5cF60E"),
    );
    m.insert(
        "BigONE_3",
        address!("8D61120Cf18069139220875C745B5A62C39Bd024"),
    );
    m.insert(
        "BigONE_4",
        address!("a30D8157911ef23c46C0eB71889eFe6a648a41F7"),
    );
    m.insert(
        "Bilaxy",
        address!("20DBb3496eBa23337F68461B6eCc3c79d4c78fd9"),
    );
    m.insert(
        "Bilaxy_1",
        address!("4bb6E665e962EF2E0fD8400C43eBc7F9d2c24435"),
    );
    m.insert(
        "Bilaxy_2",
        address!("9BA3560231e3E0aD7dde23106F5B98C72E30b468"),
    );
    m.insert(
        "Bilaxy_3",
        address!("CCE8D59AFFdd93be338FC77FA0A298C2CB65Da59"),
    );
    m.insert(
        "Bilaxy_4",
        address!("F22a4B9d9D1B20b44699E36A1e3903E78143f9Da"),
    );
    m.insert(
        "Bilaxy_5",
        address!("f7793d27A1b76CDF14Db7C83e82C772cF7C92910"),
    );
    m.insert(
        "BingX",
        address!("1651D700cD4020334bD185BA4c6E0271ffc0C732"),
    );
    m.insert(
        "BingX_1",
        address!("29F3144d84Da21F9A6788f14680d0A2aa44D6F0e"),
    );
    m.insert(
        "BingX_10",
        address!("b48C5CA99D33a8625E125f69AC8e07F3dFfE34A0"),
    );
    m.insert(
        "BingX_11",
        address!("BB936d7CD3cDc6AC01088916d939cA5207cD84c2"),
    );
    m.insert(
        "BingX_12",
        address!("C4334A9AF50C80A12C484de643149f6159Bdd110"),
    );
    m.insert(
        "BingX_13",
        address!("da43c54Ce5083885F561E05fd6220b7096bE246c"),
    );
    m.insert(
        "BingX_14",
        address!("DEc815281519F6cB080090317E0BA3E446Fafe43"),
    );
    m.insert(
        "BingX_15",
        address!("E4516477ADacc6682Cf18069475f676a5B5667f9"),
    );
    m.insert(
        "BingX_16",
        address!("ED5e461999B1Fedd00797a818Edd75C727Ed948F"),
    );
    m.insert(
        "BingX_17",
        address!("EfA02443139F31E0336608e5a2F99D26784e4bfd"),
    );
    m.insert(
        "BingX_2",
        address!("406C22b8740ae955b04fD11c2061E053807E2A69"),
    );
    m.insert(
        "BingX_3",
        address!("4597A8206978C5DE22173432B2f0Cb899EEf9Fa3"),
    );
    m.insert(
        "BingX_4",
        address!("6c69fa64EC451b1Bc5b5FBAa56CF648a281634Be"),
    );
    m.insert(
        "BingX_5",
        address!("766182bFA8B8790d61c4D7E7912C1C3A6F42cef6"),
    );
    m.insert(
        "BingX_6",
        address!("7C217Eb128337C4B44CA9093eA9a9984b1803488"),
    );
    m.insert(
        "BingX_7",
        address!("A0FCA8fA8E9C6aA77305f94bE0e03908d0a42900"),
    );
    m.insert(
        "BingX_8",
        address!("a88f86E5685FCa7C5D6de0e4D944875b007137b5"),
    );
    m.insert(
        "BingX_9",
        address!("AF1e33f8153f25e304dEC5Cb544b5B6CcC5520eD"),
    );
    m.insert(
        "Bit-Z",
        address!("0DE4b2BE45Ae233D5F782a5C70dFc8BFAB736528"),
    );
    m.insert(
        "Bit-Z_1",
        address!("0F63AF93bf5d2E786FE4b47cBd6e264669E64456"),
    );
    m.insert(
        "Bit-Z_10",
        address!("6EC1b4e6315eA30F52E5858d1F2A9Cb2E462D157"),
    );
    m.insert(
        "Bit-Z_11",
        address!("8658264955e288275E8cD8788b4a7f10ca257836"),
    );
    m.insert(
        "Bit-Z_12",
        address!("9c2cd7092c693D89E4106CE36125ccD754E589a8"),
    );
    m.insert(
        "Bit-Z_13",
        address!("9F2b734417eC00b6E2c474Bd26d7e8cC737e7C96"),
    );
    m.insert(
        "Bit-Z_14",
        address!("a1E522376F8f96A96e8Fb4722A1D4f26D6d17E2D"),
    );
    m.insert(
        "Bit-Z_15",
        address!("A399Bf13e39ecA44a943ac02Da76913FE7aE0043"),
    );
    m.insert(
        "Bit-Z_16",
        address!("a8aE6549c66C59aa55D50377948dFBE362d56B03"),
    );
    m.insert(
        "Bit-Z_17",
        address!("Af2a0a5589eC469bda06deaA938D5B3b231D5FE8"),
    );
    m.insert(
        "Bit-Z_18",
        address!("B67b5c792F6725CA3606626D679A28b551f2B4Ad"),
    );
    m.insert(
        "Bit-Z_19",
        address!("b6F8b42396B012DC124c61012D5E9354966DB9ef"),
    );
    m.insert(
        "Bit-Z_2",
        address!("237A734da47A70626B3A11e1928a5dcE12cb9E46"),
    );
    m.insert(
        "Bit-Z_20",
        address!("bA5BFfd6F8fC5A2E65Cdc37043678a990178B009"),
    );
    m.insert(
        "Bit-Z_21",
        address!("CF3618D4680817AF786a1D93465a19aB4225E69e"),
    );
    m.insert(
        "Bit-Z_22",
        address!("dFDaCDab40bc0b339E15EDAEcDaF120C389D4dAe"),
    );
    m.insert(
        "Bit-Z_23",
        address!("E65bD8fa44d9D6D37410891b98Cef29A96A84AaF"),
    );
    m.insert(
        "Bit-Z_24",
        address!("f695c9e5F0E017F82B8f7b968075d949263795aa"),
    );
    m.insert(
        "Bit-Z_25",
        address!("fC51869C1f33514bd315F1831eb17DFEA5E53C7a"),
    );
    m.insert(
        "Bit-Z_3",
        address!("2D48B865D7b322E510BCF36953A06608E20e323e"),
    );
    m.insert(
        "Bit-Z_4",
        address!("30146933A3A0BABc74eC0b3403beC69281Ba5914"),
    );
    m.insert(
        "Bit-Z_5",
        address!("3C365fFb42Ae67ed147cF413a90E887f59ba9b24"),
    );
    m.insert(
        "Bit-Z_6",
        address!("4B729cF402CfCfFd057E254924B32241AeDC1795"),
    );
    m.insert(
        "Bit-Z_7",
        address!("5633764e2299253525Ea1b9AD9032C69A2b21D76"),
    );
    m.insert(
        "Bit-Z_8",
        address!("65951c3A417e9b5e749e688b1bea1280a052c0f1"),
    );
    m.insert(
        "Bit-Z_9",
        address!("68dD1c5cb1cE86341Ca9475f3FB81Cb2C2e8Dee3"),
    );
    m.insert(
        "Bit2C",
        address!("7c49e1c0e33F3efB57d64b7690Fa287C8D15B90A"),
    );
    m.insert(
        "BitBase",
        address!("0d8824cA76e627E9CC8227Faa3B3993986ce9e48"),
    );
    m.insert(
        "BitBase_1",
        address!("6DCD15A0dbeFd0700063a4445382D3506391A41A"),
    );
    m.insert(
        "BitBlinx",
        address!("5D375281582791A38E0348915Fa9CBc6139E9C2a"),
    );
    m.insert(
        "BitForex",
        address!("2125d0d68a13b1E7Fe73641Ef2098f621486e457"),
    );
    m.insert(
        "BitForex_1",
        address!("3A723e58C4808DDE4591543282adC7D6b378715b"),
    );
    m.insert(
        "BitForex_2",
        address!("3C48f8457Dbfbcea63AAf936A71d24DE7D37cc99"),
    );
    m.insert(
        "BitForex_3",
        address!("704dDD09eF7A6D3034d76ae8Ca8c854eFA59B669"),
    );
    m.insert(
        "BitForex_4",
        address!("a546E1D9D3748E9F9fE784A221fB8A8081702514"),
    );
    m.insert(
        "BitForex_5",
        address!("eeC0Ed9E41C209c1c53a35900a06BF5DcA927405"),
    );
    m.insert(
        "BitGo",
        address!("294c6F1Ec18494abe9f608eCD97a307C80586775"),
    );
    m.insert(
        "BitGo_1",
        address!("6Fb3934EE371F5ea06c5F6a71cF7c7C6688fBd8D"),
    );
    m.insert(
        "BitGo_2",
        address!("758982386D532d977B462F5b80E16928ea6652dc"),
    );
    m.insert(
        "BitGo_3",
        address!("95EEaDDe20306a602cBa20AE8B4F29A95c5d6405"),
    );
    m.insert(
        "BitGo_4",
        address!("99126dAF078c693d200155E2dd7a668479120745"),
    );
    m.insert(
        "BitGo_5",
        address!("a5D29237a8F14FF25ab9683C38e64671E3bB5cc8"),
    );
    m.insert(
        "BitKeep",
        address!("34F1b0d87BB332d3D99A410376AC30499a9F97B9"),
    );
    m.insert(
        "BitKeep_1",
        address!("4E29fa717FB61753e26885421b84ff7E06Df585e"),
    );
    m.insert(
        "BitKeep_2",
        address!("603D022611BfE6A101DCdaB207D96C527F1d4d8e"),
    );
    m.insert(
        "BitKeep_3",
        address!("77E7c5CBeAaD915cf5462064B02984E16A902e67"),
    );
    m.insert(
        "BitKeep_4",
        address!("7d1288E5dbC91b9d2e7be736Cc789114e8D58f69"),
    );
    m.insert(
        "BitKeep_5",
        address!("8967711D157561656b236F36dB5F448bD63F7029"),
    );
    m.insert(
        "BitKeep_6",
        address!("a366Ac4542bedd0ab96b9e80687BbA2A0F1B7b19"),
    );
    m.insert(
        "BitKeep_7",
        address!("ee0ca9ca2deFF0F8be6A1229D89555689f8fe365"),
    );
    m.insert(
        "BitKeep_8",
        address!("F8C7e1e2f92cCBFb6911dbE62F38966Fe836eb9E"),
    );
    m.insert(
        "BitMEX",
        address!("EEA81C4416d71CeF071224611359F6F99A4c4294"),
    );
    m.insert(
        "BitMEX_1",
        address!("fB8131c260749c7835a08ccBdb64728De432858E"),
    );
    m.insert(
        "BitPay",
        address!("2730ef3C0c180E7f7bCFCA249c757421B208e333"),
    );
    m.insert(
        "BitPay_1",
        address!("5763A2A8194E9BD0b8140ABccB9171F005470324"),
    );
    m.insert(
        "BitPay_2",
        address!("F2a14015EaA3F9cC987f2c3b62FC93Eee41aA5d0"),
    );
    m.insert(
        "BitStorage",
        address!("1b8a38ea02cEDA9440E00C1Aeba26eE2DC570423"),
    );
    m.insert(
        "BitUN.io",
        address!("aa90b4aaE74CEE41e004BC45e45A427406C4dcAe"),
    );
    m.insert(
        "BitUN.io_1",
        address!("F8D04A720520d0bCbc722B1d21CA194AA22699f2"),
    );
    m.insert(
        "BitVenus",
        address!("25Ee4Ce905Da85df8620cB82884adDf96A14498A"),
    );
    m.insert(
        "BitVenus_1",
        address!("2B097741854EEdeB9e5c3ef9D221fb403d8d8609"),
    );
    m.insert(
        "BitVenus_2",
        address!("4785e47aE7061632C2782384DA28B9F68a5647a3"),
    );
    m.insert(
        "BitVenus_3",
        address!("5631aA1fc1868703a962e2fD713dc02cad07C1DB"),
    );
    m.insert(
        "BitVenus_4",
        address!("686b9202a36C09CE8aBa8b49Ae5F75707EDEc5fE"),
    );
    m.insert(
        "BitVenus_5",
        address!("E1E5F8caCc6B9Ace0894Fe7ba467328587e60bE7"),
    );
    m.insert(
        "BitVenus_6",
        address!("E43C53c466A282773F204df0b0A58fb6F6A88633"),
    );
    m.insert(
        "BitVenus_7",
        address!("ef7A2610a7C9cfB2537d68916B6A87FeA8Acfec3"),
    );
    m.insert(
        "Bitbank",
        address!("3727cfCBD85390Bb11B3fF421878123AdB866be8"),
    );
    m.insert(
        "Bitbank_1",
        address!("620a3E5cDdD2748E111A11810757f419d10B1AaC"),
    );
    m.insert(
        "Bitbank_2",
        address!("DBfC4549b325b6f00013A3861B87AAB6696FdBEe"),
    );
    m.insert(
        "Bitbank_3",
        address!("eB6c4bE4b92a52e969F4bF405025D997703D5383"),
    );
    m.insert(
        "Bitbank_4",
        address!("F9225f3288f6cb0d0f80A5561e73102565E8bD8C"),
    );
    m.insert(
        "Bitbee",
        address!("2b49cE21Ad2004CFb3d0b51B2E8eC0406d632513"),
    );
    m.insert(
        "Bitberry",
        address!("6B59210aDE46B62B25e82e95ab390A7CcAdd4c3a"),
    );
    m.insert(
        "Bitcasino",
        address!("094b4cf43908F0AdB3dBDb5025F52470AAc3B160"),
    );
    m.insert(
        "Bitcasino_1",
        address!("5BCbdfB6cc624b959c39A2D16110D1f2D9204F72"),
    );
    m.insert(
        "Bitcasino_2",
        address!("910c00A13F2AF11c1e35fE6f6C43B8Ae4c82cF8a"),
    );
    m.insert(
        "Bitcasino_3",
        address!("97180753F93E250D846d51034bd2bD62375Dc7b0"),
    );
    m.insert(
        "Bitcasino_4",
        address!("9e3ef83728a399d6f5f757E3FA04f4850BBfB5f0"),
    );
    m.insert(
        "Bitcasino_5",
        address!("a339aAEE0acC7A96fB34F3F65e600Fd5237dEe22"),
    );
    m.insert(
        "Bitcasino_6",
        address!("E94d9b695fF36AFa8Db1f764beDC604FB04eCb95"),
    );
    m.insert(
        "Bitci",
        address!("7A91a362d4f2c9C4627688D5B7090BBB12e5715f"),
    );
    m.insert(
        "Bitci_1",
        address!("E954B098b80D43FD66AF4a58400C05E62B087b72"),
    );
    m.insert(
        "Bitcoin Meester",
        address!("D57fe94225A8Fd8e1a1826de1c6d6b3AFc97C062"),
    );
    m.insert(
        "Bitcoin Suisse",
        address!("2a7077399B3e90F5392D55A1Dc7046ad8D152348"),
    );
    m.insert(
        "Bitcoin Suisse_1",
        address!("31dFf0cf605F9719b9171f6049150595CC1240F1"),
    );
    m.insert(
        "Bitcoin Suisse_2",
        address!("3f262579E4332e1Be2722684EAa1C1b111F7a8d8"),
    );
    m.insert(
        "Bitcoin Suisse_3",
        address!("7B4576d06D0Ce1F83F9a9B76BF8077bFFD34FcB1"),
    );
    m.insert(
        "Bitcoin Suisse_4",
        address!("c2288B408Dc872A1546F13E6eBFA9c94998316a2"),
    );
    m.insert(
        "Bitcoin Suisse_5",
        address!("FCB7Edb966d320c7f3AE1f751a8c86F30fA5ad37"),
    );
    m.insert(
        "BiteBTC",
        address!("28eBe764B8F9A853509840645216D3C2c0fd774b"),
    );
    m.insert(
        "BiteBTC_1",
        address!("53bA297c0FF8973B470436360e74aE92eA332399"),
    );
    m.insert(
        "BiteBTC_2",
        address!("76F1Cd864fc153Eda7A9f5c407380d9a80154A16"),
    );
    m.insert(
        "BiteBTC_3",
        address!("d8ee4A76B99292c1BB0F45d5c6Fa5F99919b7e64"),
    );
    m.insert(
        "Bitexlive",
        address!("57A47cFE647306A406118B6cF36459a1756823D0"),
    );
    m.insert(
        "Bitexlive_1",
        address!("7217d64f77041Ce320c356D1a2185Bcb89798A0A"),
    );
    m.insert(
        "Bitfex.trade",
        address!("dfc38911F6E0bfDD0472F6f68d83E8A0115768b2"),
    );
    m.insert(
        "Bitfex.trade_1",
        address!("f2e0e06771414a14d9d1bb70cD81030434421Cb3"),
    );
    m.insert(
        "Bitfront",
        address!("dF5021a4C1401F1125cD347e394d977630e17Cf7"),
    );
    m.insert(
        "Bitget",
        address!("0639556F03714A74a5fEEaF5736a4A64fF70D206"),
    );
    m.insert(
        "Bitget_1",
        address!("149Ded7438Caf5e5BFDc507a6c25436214d445E1"),
    );
    m.insert(
        "Bitget_10",
        address!("4dFc15890972eceA7A213bDA2b478DAbC382e7a1"),
    );
    m.insert(
        "Bitget_11",
        address!("5051e9860c1889Eb1bfa394365364B3dd61787F1"),
    );
    m.insert(
        "Bitget_12",
        address!("51971c86b04516062c1e708CDC048CB04fbe959f"),
    );
    m.insert(
        "Bitget_13",
        address!("5bdf85216ec1e38D6458C870992A69e38e03F7Ef"),
    );
    m.insert(
        "Bitget_14",
        address!("6a3F28C47542bD5811AE37ab358D5d7E3ab84127"),
    );
    m.insert(
        "Bitget_15",
        address!("731309E453972598eA05D706C6Ee6c3c21AB4D2a"),
    );
    m.insert(
        "Bitget_16",
        address!("7651fC1605a58Fe9a99F1fe0d6db05D4182a9a93"),
    );
    m.insert(
        "Bitget_17",
        address!("842Ea89f73ADD9e4fe963Ae7929fDc1e80AcdB52"),
    );
    m.insert(
        "Bitget_18",
        address!("97b9D2102A9a65A26E1EE82D59e42d1B73B68689"),
    );
    m.insert(
        "Bitget_19",
        address!("9E00816F61a709fa124D36664Cd7b6f14c13eE05"),
    );
    m.insert(
        "Bitget_2",
        address!("1a96E5dA1315efCF9b75100F5757d5E8B76abb0C"),
    );
    m.insert(
        "Bitget_20",
        address!("B8cda8d72DA558Ef8F76A0d928f9652D2b003e2e"),
    );
    m.insert(
        "Bitget_21",
        address!("bC942E2250Ec7AB83bFC4516BCa4e281Dbfbb393"),
    );
    m.insert(
        "Bitget_22",
        address!("dFE4B89cf009BFfa33D9BCA1f19694FC2d4d943d"),
    );
    m.insert(
        "Bitget_23",
        address!("E2B406EC9227143A8830229eEb3Eb6E24b5c60Be"),
    );
    m.insert(
        "Bitget_24",
        address!("e6a421f24d330967a3Af2F4cDB5c34067E7e4d75"),
    );
    m.insert(
        "Bitget_25",
        address!("e80623a9d41f2f05780D9cD9cea0F797Fd53062A"),
    );
    m.insert(
        "Bitget_26",
        address!("f646d9B7d20BABE204a89235774248BA18086dae"),
    );
    m.insert(
        "Bitget_3",
        address!("1AB4973a48dc892Cd9971ECE8e01DcC7688f8F23"),
    );
    m.insert(
        "Bitget_4",
        address!("1Ae3739E17d8500F2b2D80086ed092596A116E0b"),
    );
    m.insert(
        "Bitget_5",
        address!("1d5BA5414f2983212E03Bf7725adD9eB4CdB00dC"),
    );
    m.insert(
        "Bitget_6",
        address!("2bf7494111a59bD51f731DCd4873D7d71F8feEEC"),
    );
    m.insert(
        "Bitget_7",
        address!("31A36512D4903635b7dd6828a934C3915A5809Be"),
    );
    m.insert(
        "Bitget_8",
        address!("3A7d1A8C3A8dC9d48a68e628432198a2eAD4917c"),
    );
    m.insert(
        "Bitget_9",
        address!("461f6dCdd5Be42D41FE71611154279d87c06B406"),
    );
    m.insert(
        "Bitkan",
        address!("8c00dDA00F9E2AabDE88a4d17f9EF9fCe265592F"),
    );
    m.insert(
        "Bitkan_1",
        address!("Cfbbf8dc80bb324d2F8634cc73d6e8F6784D3230"),
    );
    m.insert(
        "Bitkub",
        address!("1579B5f6582C7a04f5fFEec683C13008C4b0A520"),
    );
    m.insert(
        "Bitkub_1",
        address!("1fd9393359b825156e8353e390E23664BB5ab4B8"),
    );
    m.insert(
        "Bitkub_10",
        address!("B9C764114C5619a95d7f232594e3B8dDDF95b9CF"),
    );
    m.insert(
        "Bitkub_11",
        address!("C488E4AfEC0414511d1518d8d6B8Dc5f820Fb92a"),
    );
    m.insert(
        "Bitkub_12",
        address!("Ca7404EED62a6976Afc335fe08044B04dBB7e97D"),
    );
    m.insert(
        "Bitkub_13",
        address!("DB044B8298E04D442FdBE5ce01B8cc8F77130e33"),
    );
    m.insert(
        "Bitkub_2",
        address!("326D9f47BA49BBAac279172634827483af70a601"),
    );
    m.insert(
        "Bitkub_3",
        address!("3d1D8A1d418220fd53C18744d44c182C46f47468"),
    );
    m.insert(
        "Bitkub_4",
        address!("49876520C866D138dd749d6C2C33e4DA5bfAeC66"),
    );
    m.insert(
        "Bitkub_5",
        address!("59E0cDA5922eFbA00a57794faF09BF6252d64126"),
    );
    m.insert(
        "Bitkub_6",
        address!("6254B927ecC25DDd233aAECD5296D746B1C006B4"),
    );
    m.insert(
        "Bitkub_7",
        address!("79169E7818968cD0C6DBd8929f24d797CC1Af9A1"),
    );
    m.insert(
        "Bitkub_8",
        address!("9be7B0f285d04701f27682F591a60417C47d095A"),
    );
    m.insert(
        "Bitkub_9",
        address!("Adf4c208d546E7F1Ec24cab1CcDA9B47B90B8540"),
    );
    m.insert(
        "BitoPro",
        address!("0B01450061e68c4f0f89167EFEAd245a3d393750"),
    );
    m.insert(
        "BitoPro_1",
        address!("16905697103Cd4AD6a22193B242941b62f660bBc"),
    );
    m.insert(
        "BitoPro_2",
        address!("1DfEFF70fbcC5949b29FaF52196f0711e4915B2c"),
    );
    m.insert(
        "BitoPro_3",
        address!("Aa65CC699F9B0258064d952aD987269f7C9DBFAD"),
    );
    m.insert(
        "Bitpanda",
        address!("3da452fd6947BEa2735C5EAea04F1C22587cb5B2"),
    );
    m.insert(
        "Bitpanda_1",
        address!("7071F121C038A98F8A7d485648a27FCD48891Ba8"),
    );
    m.insert(
        "Bitpanda_2",
        address!("74dEc05E5b894b0EfEc69Cdf6316971802A2F9a1"),
    );
    m.insert(
        "Bitpanda_3",
        address!("8cFa2d9047cA6573219C252E345506dE5e9da5d9"),
    );
    m.insert(
        "Bitpanda_4",
        address!("B10EdD6fa6067DbA8d4326F1c8f0d1C791594F13"),
    );
    m.insert(
        "Bitpanda_5",
        address!("b8Cbbf78c7Ad1cDF4cA0e111B35491f3bFE027AC"),
    );
    m.insert(
        "Bitpanda_6",
        address!("F197c6F2aC14d25eE2789A73e4847732C7F16bC9"),
    );
    m.insert(
        "Bitpanda_7",
        address!("F32682d5F99ba4143532618d6f516859a055Ea06"),
    );
    m.insert(
        "Bitpie",
        address!("19E460AC0a23706F156909b545De9D5072802065"),
    );
    m.insert(
        "Bitpie_1",
        address!("346B46826Be175c943a45cDBeC2E9D95dc52FB38"),
    );
    m.insert(
        "Bitpie_2",
        address!("48Fc0b223b818b55F2a6feb3ab329FF816edD2bc"),
    );
    m.insert(
        "Bitpie_3",
        address!("5c9F3ffF6ee846a83080F373F8ceA1451bB4a3D9"),
    );
    m.insert(
        "Bitpie_4",
        address!("7b138cc83ecCcD6331Cd7651c75ee3EB2ec6682E"),
    );
    m.insert(
        "Bitpie_5",
        address!("8979d1E0EcaB3cF5aE8A5a7b2d792aa119f34bE1"),
    );
    m.insert(
        "Bitpie_6",
        address!("ADEf6b2c5e848A4F04145ee88f20aC079f17bEab"),
    );
    m.insert(
        "Bitpie_7",
        address!("B685d0DAea607Fe75e02c1A7679261eAc661b9bc"),
    );
    m.insert(
        "Bitpie_8",
        address!("f64edD94558Ca8B3a0e3b362e20BB13ff52eA513"),
    );
    m.insert(
        "Bitrefill",
        address!("4945cE2d1B5BD904CAc839b7FDAbAfd19Cab982b"),
    );
    m.insert(
        "Bitrefill_1",
        address!("4d31f0089a27E7AFe56AC47C1551e09102d2834D"),
    );
    m.insert(
        "Bitrefill_10",
        address!("B5de51cb0BF61554e47a7a6B6c7f429feC925B4f"),
    );
    m.insert(
        "Bitrefill_11",
        address!("c5B4a8B2414C339a471CeF9663E67D57E33f41f9"),
    );
    m.insert(
        "Bitrefill_12",
        address!("CBD17EE0703B92f858744F658c53Ad26CE206dfA"),
    );
    m.insert(
        "Bitrefill_13",
        address!("EAdf413c2AB5B1488430469551241CcA29dF65b4"),
    );
    m.insert(
        "Bitrefill_14",
        address!("F5E410B97aabdFbd019f1428fDe4E12815CABBAE"),
    );
    m.insert(
        "Bitrefill_2",
        address!("6438beeB08cf4fABa533eCde748180DD56D43513"),
    );
    m.insert(
        "Bitrefill_3",
        address!("69B8F9E1f633fBB8De3e279aAF34D4EfF05e61AF"),
    );
    m.insert(
        "Bitrefill_4",
        address!("79267CD876AFe3E400917C1B26743630cC29f4e0"),
    );
    m.insert(
        "Bitrefill_5",
        address!("88C633D244ee11269B33a5d29133c5E66E8951A5"),
    );
    m.insert(
        "Bitrefill_6",
        address!("8f662404D92908324D25d7E9b814c89Ae2bFAD5E"),
    );
    m.insert(
        "Bitrefill_7",
        address!("94Aa0b75A10bf4e92A00dE8A115a6f61095cF663"),
    );
    m.insert(
        "Bitrefill_8",
        address!("95ba04c934d231C51435A266fFf2b2288e1D0AF3"),
    );
    m.insert(
        "Bitrefill_9",
        address!("A381c04ebFDf07F9A63Eb8DbE2eF55c0122620AB"),
    );
    m.insert(
        "Bitrue",
        address!("6cc8dCbCA746a6E4Fdefb98E1d0DF903b107fd21"),
    );
    m.insert(
        "Bitrue_1",
        address!("d205a958527f083F1b222061B4D60D147fEc5044"),
    );
    m.insert(
        "Bitrue_2",
        address!("F4C62B4f8b7b1b1c4BA88BFD3a8ea392641516e9"),
    );
    m.insert(
        "Bitso",
        address!("20bEea119E70255A8c36E4009C94AedB1F8B8Eea"),
    );
    m.insert(
        "Bitso_1",
        address!("29D5527CaA78f1946a409FA6aCaf14A0a4A0274b"),
    );
    m.insert(
        "Bitso_2",
        address!("58b704065B7aFF3ED351052f8560019E05925023"),
    );
    m.insert(
        "Bitso_3",
        address!("A01AA2196724a39290f465b3925E5dCaFe7F2256"),
    );
    m.insert(
        "Bitso_4",
        address!("f9D2D8D90B4b35aADA5AEd2F46FCD15DA0608B4e"),
    );
    m.insert(
        "Bitsten",
        address!("bDeF6e20692c302044B98284090922F683F3b523"),
    );
    m.insert(
        "Bitsten_1",
        address!("fe0cb30AFCb0EB1c27Ae33D11Be7ef749Ed25072"),
    );
    m.insert(
        "Bitvavo",
        address!("04A6b67FAC93e507d2642f129f5cB8aD474f9823"),
    );
    m.insert(
        "Bitvavo_1",
        address!("079A892628EBf28d0Ed8f00151cff225A093dc63"),
    );
    m.insert(
        "Bitvavo_10",
        address!("9d5DB4B86AfD9Fe80720cA0F5637d6B790CE5Bcb"),
    );
    m.insert(
        "Bitvavo_11",
        address!("aB782bc7D4a2b306825de5a7730034F8F63ee1bC"),
    );
    m.insert(
        "Bitvavo_12",
        address!("B2C9fFdbe4b4BBB6aA2951390506C39be6998751"),
    );
    m.insert(
        "Bitvavo_13",
        address!("c15dc0B4223b05588A257E48Ed129Fb59C6eB7E3"),
    );
    m.insert(
        "Bitvavo_14",
        address!("c8E0EF05B7f250fc63702fBdaDA6972aFDCae8C2"),
    );
    m.insert(
        "Bitvavo_15",
        address!("Cc9Febe81EaBC8e1B0dcC29dEc2bdc70DEF9180f"),
    );
    m.insert(
        "Bitvavo_16",
        address!("ccF8594f833FE5eBBF0e72FDa51F9fb291b39A57"),
    );
    m.insert(
        "Bitvavo_17",
        address!("d2674dA94285660c9b2353131bef2d8211369A4B"),
    );
    m.insert(
        "Bitvavo_18",
        address!("dd25cA203C060213Eaffe59E37101a01dAC463c1"),
    );
    m.insert(
        "Bitvavo_19",
        address!("dF07BCb647Ca70Bd8e78b51d898777426c6d121A"),
    );
    m.insert(
        "Bitvavo_2",
        address!("1A1c87d9A6F55D3BbB064bfF1059ad37B6Bdc097"),
    );
    m.insert(
        "Bitvavo_20",
        address!("edC6BacdC1e29D7c5FA6f6ECA6FDD447B9C487c9"),
    );
    m.insert(
        "Bitvavo_3",
        address!("20840B20f1eEE3B7b225866Dc2e0d669CC2553fb"),
    );
    m.insert(
        "Bitvavo_4",
        address!("2586Db422EC15d5999b50C43bE219756C46Bb0Bd"),
    );
    m.insert(
        "Bitvavo_5",
        address!("2f1752e8B49C6E7391Dc6e55934ceedC7AbF18F7"),
    );
    m.insert(
        "Bitvavo_6",
        address!("75f594083de65FF173Eb7CE505aa26332fe389f5"),
    );
    m.insert(
        "Bitvavo_7",
        address!("87eAB4F119113e0E5920288936aD94C307b4849a"),
    );
    m.insert(
        "Bitvavo_8",
        address!("897321a0Ee56b57ef56F284037FFE4B63287D52A"),
    );
    m.insert(
        "Bitvavo_9",
        address!("95B564F3B3BaE3f206aa418667bA000AFAFAcc8a"),
    );
    m.insert("Bity", address!("fb9F7F41319157ac5C5dccaE308A63a4337ad5d9"));
    m.insert(
        "Bitzlato",
        address!("00cdC153Aa8894D08207719Fe921FfF964f28Ba3"),
    );
    m.insert(
        "Bitzlato_1",
        address!("d1b7cCa582578bFF37faab547a4D50FF6342c6D0"),
    );
    m.insert(
        "BlockFi",
        address!("04046027549f739eDFd5b2a78EfDBAf0F0bf4514"),
    );
    m.insert(
        "BlockFi_1",
        address!("22FFDA6813f4F34C520bf36E5Ea01167bC9DF159"),
    );
    m.insert(
        "BlockFi_2",
        address!("2A549b4AF9Ec39B03142DA6dC32221fC390B5533"),
    );
    m.insert(
        "BlockFi_3",
        address!("3FDA25F27211a138ADF211F4C060f2149674Be6D"),
    );
    m.insert(
        "BlockFi_4",
        address!("530e0A6993eA99ffc96615aF43f327225a5fe536"),
    );
    m.insert(
        "BlockFi_5",
        address!("808b4dA0Be6c9512E948521452227EFc619BeA52"),
    );
    m.insert(
        "BlockFi_6",
        address!("A26A8e242A7470476DF1dc11dEd5DBc9FCa610fA"),
    );
    m.insert(
        "BlockTrades",
        address!("007174732705604bBbf77038332Dc52FD5A5000C"),
    );
    m.insert(
        "Blockchain.com",
        address!("23f4569002a5A07f0Ecf688142eEB6bcD883eeF8"),
    );
    m.insert(
        "Blockchain.com_1",
        address!("46E0813Dcb480517e3E73449761BDc596e424A79"),
    );
    m.insert(
        "Blockchain.com_2",
        address!("52749D8c2f70f7ea6343054b7a04FB16337e5F58"),
    );
    m.insert(
        "Blockchain.com_3",
        address!("9AA65464b4cFbe3Dc2BDB3dF412AeE2B3De86687"),
    );
    m.insert(
        "Blockchain.com_4",
        address!("A00E2A7652248AbEb209398227DAE413E9479e52"),
    );
    m.insert(
        "Blockchain.com_5",
        address!("C88F7666330b4b511358b7742dC2a3234710e7B1"),
    );
    m.insert(
        "Blockchain.com_6",
        address!("fC3E21f959551512D68b6b00F8931593D3106151"),
    );
    m.insert(
        "Bololex.com",
        address!("DF8752caa319668006580dDf48DB25A23728b926"),
    );
    m.insert(
        "BtcTurk",
        address!("16769E533352798deB664bA570230A758346Ca1A"),
    );
    m.insert(
        "BtcTurk_1",
        address!("1C17622cfa9B6fD2043A76DfC39A5B5a109aa708"),
    );
    m.insert(
        "BtcTurk_10",
        address!("D2589c4061bF45a9a5212846AA72C2eD46377145"),
    );
    m.insert(
        "BtcTurk_11",
        address!("dE0f5F79dffd1E8daA6051bd960AefB964D9845f"),
    );
    m.insert(
        "BtcTurk_12",
        address!("fc3bFe0ECb87c1eea44e5338828149480e8D2B74"),
    );
    m.insert(
        "BtcTurk_2",
        address!("2eE555C9006A9DC4674f01E0d4Dfc58e013708f0"),
    );
    m.insert(
        "BtcTurk_3",
        address!("40E832C3Df9562DfaE5A86A4849F27F687A9B46B"),
    );
    m.insert(
        "BtcTurk_4",
        address!("46f80018211D5cBBc988e853A8683501FCA4ee9b"),
    );
    m.insert(
        "BtcTurk_5",
        address!("628a6fe299589B4b5A67d4B43CcDCB4821b3D80C"),
    );
    m.insert(
        "BtcTurk_6",
        address!("832F166799A407275500430b61b622F0058f15d6"),
    );
    m.insert(
        "BtcTurk_7",
        address!("8C54EbDD960056d2CfF5998df5695dACA1FC0190"),
    );
    m.insert(
        "BtcTurk_8",
        address!("9FCaFcca8aec0367abB35fBd161c241f7b79891B"),
    );
    m.insert(
        "BtcTurk_9",
        address!("B02f1329d6a6AcEF07a763258f8509c2847A0a3E"),
    );
    m.insert(
        "Bullish",
        address!("6908C23476D3c8Aa59994662e95B40414B396bf0"),
    );
    m.insert(
        "Bullish_1",
        address!("756D64Dc5eDb56740fC617628dC832DDBCfd373c"),
    );
    m.insert(
        "Bullish_2",
        address!("a96853390C776ce8307B46B4D1a857Ab310Ab018"),
    );
    m.insert(
        "Bullish_3",
        address!("C08FB884576cc89957e9058eF11587C468c2952F"),
    );
    m.insert(
        "C-Patex",
        address!("4c8F52106D72dA5E090bc08Fb4d8063F3d591beb"),
    );
    m.insert("C2CX", address!("789ddCAC6A48A593CFcD957637bc1CEBD65eBAeb"));
    m.insert(
        "C2CX_1",
        address!("D7C866d0D536937bF9123E02F7C052446588189f"),
    );
    m.insert(
        "CEX.IO",
        address!("1f973B233f5Ebb1E5D7CFe51B9aE4A32415A3A08"),
    );
    m.insert(
        "CEX.IO_1",
        address!("c9f5296Eb3ac266c94568D790b6e91ebA7D76a11"),
    );
    m.insert(
        "COSS Exchange",
        address!("0D6B5A54F940BF3D52E438CaB785981aAeFDf40C"),
    );
    m.insert(
        "COSS Exchange_1",
        address!("38C939ECD144788a50aAE1dB7EB17D35Ffe16eb0"),
    );
    m.insert(
        "COSS Exchange_2",
        address!("43F07efe28E092A0fE4ec5B5662022B461fFac80"),
    );
    m.insert(
        "COSS Exchange_3",
        address!("6fa0a717c1073402a963E38ac8CB0d52C271b36E"),
    );
    m.insert(
        "COSS Exchange_4",
        address!("d1560b3984B7481CD9a8F40435a53C860187174d"),
    );
    m.insert(
        "CREX24",
        address!("521dB06bF657Ed1D6C98553A70319a8DdBAc75A3"),
    );
    m.insert(
        "Calypso Exchange",
        address!("a63fdc6684c9E454433Ceec20eFfcf9Fbc96BAfB"),
    );
    m.insert(
        "CamboChanger",
        address!("4dC98C79A52968a6c20cE9A7A08d5e8D1C2D5605"),
    );
    m.insert(
        "CamboChanger_1",
        address!("88988D6Ef12d7084e34814b9edafA01aE0D05082"),
    );
    m.insert(
        "Cashierest",
        address!("41A313bC923927A86a384c9128718300Fd75C34F"),
    );
    m.insert(
        "Cashierest_1",
        address!("6999603912eeB1B3f10A2058384DB5E109B4DF6F"),
    );
    m.insert(
        "Cashierest_2",
        address!("72BCFA6932FeACd91CB2Ea44b0731ed8Ae04d0d3"),
    );
    m.insert(
        "Catex Exchange",
        address!("7A56F645DCB513D0326CBAA048E9106fF6D4CD5f"),
    );
    m.insert(
        "Catex Exchange_1",
        address!("845437bD99dBE5595494077b6b0b6C6A16ec1878"),
    );
    m.insert(
        "Ceffu",
        address!("33e55B9bADc446f7FDDd8BDA1b5F33c8E3e82a2c"),
    );
    m.insert(
        "Ceffu_1",
        address!("3a3C006053a9B40286B9951A11bE4C5808c11dc8"),
    );
    m.insert(
        "Ceffu_2",
        address!("4Ed6Cf63bd9C009d247ee51224Fc1c7041f517F1"),
    );
    m.insert(
        "Ceffu_3",
        address!("756B86fD69914db92229C07c7393d7d9948fEd33"),
    );
    m.insert(
        "Ceffu_4",
        address!("84dBd7da7464B54d2F271b383106795d7dcd9069"),
    );
    m.insert(
        "Ceffu_5",
        address!("9f34A5fe4846d89dB84e30aF799459a691834cA0"),
    );
    m.insert(
        "Ceffu_6",
        address!("D3a22590f8243f8E83Ac230D1842C9Af0404C4A1"),
    );
    m.insert(
        "Ceffu_7",
        address!("d3Fdd98162902844342E7F8b59aB17CbD03BA36B"),
    );
    m.insert(
        "Ceffu_8",
        address!("e91268537B6dc2a43D8c81c4c4CAE0D5312B9C2c"),
    );
    m.insert(
        "Ceffu_9",
        address!("eA01C23b82F5B13A811ae49100E2b9008b1e6AEE"),
    );
    m.insert(
        "Celsuis",
        address!("06FC63b5C211aC29A9dA0cc24461581786163a67"),
    );
    m.insert(
        "Celsuis_1",
        address!("0A9872e72b86C682032d3cAbEE8ab70e38DE7f35"),
    );
    m.insert(
        "Celsuis_10",
        address!("44b68b196148e426402b3e9aB3bE974176Ad32C0"),
    );
    m.insert(
        "Celsuis_11",
        address!("4f6742bADB049791CD9A37ea913f2BAC38d01279"),
    );
    m.insert(
        "Celsuis_12",
        address!("5132d0a2fC15FBA4a9a64EA714854270BEc382FB"),
    );
    m.insert(
        "Celsuis_13",
        address!("5670b3E44AdE4BD9D04A73fB88994E37A0d48f27"),
    );
    m.insert(
        "Celsuis_14",
        address!("59A7F0779829Bf51DbB1EcD9dB873B6a2cF57F42"),
    );
    m.insert(
        "Celsuis_15",
        address!("6101B69C06EFC3c1Ae68EbEa61AA1c746FE628D7"),
    );
    m.insert(
        "Celsuis_16",
        address!("6A3528677e598B47952749b08469CE806C2524e7"),
    );
    m.insert(
        "Celsuis_17",
        address!("6d9db9275C144533071FeA75866993Ef2AfcaAB7"),
    );
    m.insert(
        "Celsuis_18",
        address!("6E216a1D8b19a505511a7f1B10084BAd57D31A48"),
    );
    m.insert(
        "Celsuis_19",
        address!("752C8191E6b1Db38B41A8c8921F7a703F2969d18"),
    );
    m.insert(
        "Celsuis_2",
        address!("0b6438F10FDae49E48815c4B5222B562789FB9F6"),
    );
    m.insert(
        "Celsuis_20",
        address!("8d0EfDeE1cf48710F2E02d81660E2A6654d65a8D"),
    );
    m.insert(
        "Celsuis_21",
        address!("99fa1561E63DDd3CB318ef6Cb306f04FA6E149b0"),
    );
    m.insert(
        "Celsuis_22",
        address!("a16a857292228228D34ad99255f4CbDd3fcbbBF9"),
    );
    m.insert(
        "Celsuis_23",
        address!("a8631Fa3B38C84C22B2e4B6E33997Bc7563A4baA"),
    );
    m.insert(
        "Celsuis_24",
        address!("c602dc3fb4a966cd6AED233db2ae4a5E596fcC27"),
    );
    m.insert(
        "Celsuis_25",
        address!("cb8BBFa45541a95C1de883eB3606708cAe9fd45C"),
    );
    m.insert(
        "Celsuis_26",
        address!("D44DB45fff057C2e08c49FF046A3CC3A5D199B12"),
    );
    m.insert(
        "Celsuis_27",
        address!("Db31651967684A40A05c4aB8Ec56FC32f060998d"),
    );
    m.insert(
        "Celsuis_28",
        address!("DffFb7bA70eaBd7138D9c89e02d9e5A15D7fE096"),
    );
    m.insert(
        "Celsuis_29",
        address!("Ef22c14F46858d5aC61326497b056974167F2eE1"),
    );
    m.insert(
        "Celsuis_3",
        address!("11889C10CA33FBAbdbEB0C5Ffc016c8eE56f87F4"),
    );
    m.insert(
        "Celsuis_30",
        address!("Ef8dc2b0005eeF85902C78f8E5040609d72Ea9C0"),
    );
    m.insert(
        "Celsuis_31",
        address!("F03E41A34e97737ecF424e48127a21aE47945AA8"),
    );
    m.insert(
        "Celsuis_32",
        address!("f0d54551a359D5b57B7035d847B5C8D8Eb374b73"),
    );
    m.insert(
        "Celsuis_33",
        address!("F9D89Dc506c55738379C44Dc27205fD6f68e1974"),
    );
    m.insert(
        "Celsuis_4",
        address!("1CeDC0f3Af8f9841B0a1F5c1a4DDc6e1a1629074"),
    );
    m.insert(
        "Celsuis_5",
        address!("2156A3d636Ee80f437A21f6C41df8F14c39aDb19"),
    );
    m.insert(
        "Celsuis_6",
        address!("2339a732DfA3dB16b3fFB550a42b0fbCbe2435D5"),
    );
    m.insert(
        "Celsuis_7",
        address!("3473779Fd4D366774fE7D2Ceb089B30d94D7F1d1"),
    );
    m.insert(
        "Celsuis_8",
        address!("41318419CFa25396b47A94896FfA2C77c6434040"),
    );
    m.insert(
        "Celsuis_9",
        address!("4135F67bE29CE9EF8Bc9b2f37f3c466304902F6C"),
    );
    m.insert(
        "ChainUp",
        address!("0e747EB2ff0F26fB77c3a1eA67EE07FAc2DbB783"),
    );
    m.insert(
        "ChainUp_1",
        address!("10349DaaE3D75bABBC00B4Ec70416590DEdaA7e8"),
    );
    m.insert(
        "ChainUp_10",
        address!("b563C72D3b5514Fa098F9a588523915469DCFD88"),
    );
    m.insert(
        "ChainUp_11",
        address!("E17338Bd00Bf71D16953D4A00f1e02c42823F25C"),
    );
    m.insert(
        "ChainUp_12",
        address!("E66b1c7EDE325133e51346b7ee4814c7831F2542"),
    );
    m.insert(
        "ChainUp_2",
        address!("10EE46C16BE0fcfD506F3eed301254e4bc434FF1"),
    );
    m.insert(
        "ChainUp_3",
        address!("1cD9eF04c7833f4e3182Bf49FB806F75bB674152"),
    );
    m.insert(
        "ChainUp_4",
        address!("1f8F16a29251fA399D89e1005E3f95427Bf5B1dE"),
    );
    m.insert(
        "ChainUp_5",
        address!("49EC29B54acc0F85595C2F9bDbe58fb856094665"),
    );
    m.insert(
        "ChainUp_6",
        address!("5f77a4C9F962349ac2AAEC4459594031d1829AF8"),
    );
    m.insert(
        "ChainUp_7",
        address!("6b45921E693AC503a03f5E46f793Aff877698Fdc"),
    );
    m.insert(
        "ChainUp_8",
        address!("7f92c0eB5765FD0a97B9F227ED183c7397234FaD"),
    );
    m.insert(
        "ChainUp_9",
        address!("ae7FDD687bc6C802B55FaA373b468F3Fb6620a06"),
    );
    m.insert(
        "ChainX",
        address!("fd648cC72F1b4E71CbDDa7A0a91Fe34D32abD656"),
    );
    m.insert(
        "ChangeNOW",
        address!("077D360f11D220E4d5D831430c81C26c9be7C4A4"),
    );
    m.insert(
        "ChangeNOW_1",
        address!("0a1cE4496471867Fac0Ad71b785E5258993C9b33"),
    );
    m.insert(
        "ChangeNOW_10",
        address!("754993C330E8aE2ce276a9c792B40047BA9f8b1f"),
    );
    m.insert(
        "ChangeNOW_11",
        address!("7a3BEa333246efcD74ebf5835987a5398Eac10fE"),
    );
    m.insert(
        "ChangeNOW_12",
        address!("8f54972F4Ca40bD3ffC8b085f6Ece1739C40c65f"),
    );
    m.insert(
        "ChangeNOW_13",
        address!("975d9Bd9928F398c7E01F6ba236816Fa558cD94B"),
    );
    m.insert(
        "ChangeNOW_14",
        address!("9bc2f223026c252c8ef5f7f33F00F4bEE21434B8"),
    );
    m.insert(
        "ChangeNOW_15",
        address!("a12e1462d0ceD572f396F58B6E2D03894cD7C8a4"),
    );
    m.insert(
        "ChangeNOW_16",
        address!("A4e5961B58DBE487639929643dCB1Dc3848dAF5E"),
    );
    m.insert(
        "ChangeNOW_17",
        address!("A6ba490e1aF9849B6220aB9F709A32f7A82afaD6"),
    );
    m.insert(
        "ChangeNOW_18",
        address!("A96Be652A08D9905F15B7FbE2255708709BeCD09"),
    );
    m.insert(
        "ChangeNOW_19",
        address!("Ba1955c198FcA20834340414F0Ea951452BDC7DE"),
    );
    m.insert(
        "ChangeNOW_2",
        address!("3421230289980b8EA81781B170Ef7d475673102B"),
    );
    m.insert(
        "ChangeNOW_20",
        address!("Bac051BBF79C5321c0f825ea9BCA71f992144029"),
    );
    m.insert(
        "ChangeNOW_21",
        address!("be6439B25E2C6560590407731bB9FE2908F30c94"),
    );
    m.insert(
        "ChangeNOW_22",
        address!("c275119660Fefe4519083Ea6e57CBd1B672bc020"),
    );
    m.insert(
        "ChangeNOW_23",
        address!("D421bB540B5f980a9f7D4206e1404C107AAD9ecc"),
    );
    m.insert(
        "ChangeNOW_24",
        address!("D5b73fC035d4d679234323e0d891cAB4A4f5a1Ab"),
    );
    m.insert(
        "ChangeNOW_25",
        address!("d98CfE4A2B9FCE9b884D2ec3698E775ee54753AF"),
    );
    m.insert(
        "ChangeNOW_26",
        address!("D9f2C88Ae7372E7E418E5304105b918D6CF2CE1f"),
    );
    m.insert(
        "ChangeNOW_3",
        address!("3525d3a883F743CA146288c146dE7CCD59d48BF5"),
    );
    m.insert(
        "ChangeNOW_4",
        address!("3A0d24d59Af3a3444dc6Ef12cdb0c6e38C985288"),
    );
    m.insert(
        "ChangeNOW_5",
        address!("4657F866a0D9B46f288893FFF466e8d87C556B1a"),
    );
    m.insert(
        "ChangeNOW_6",
        address!("48c04ed5691981C42154C6167398f95e8f38a7fF"),
    );
    m.insert(
        "ChangeNOW_7",
        address!("637c86B5F2C8399716964C702BcAEABC69B900dF"),
    );
    m.insert(
        "ChangeNOW_8",
        address!("6ba7Fe01EeC4C6A4308C8E2B35970e0488ce9a86"),
    );
    m.insert(
        "ChangeNOW_9",
        address!("6cb399D73cA29859d795c68A68F0DBF6a74F55D4"),
    );
    m.insert(
        "Changelly",
        address!("96fC4553a00C117C5b0bED950Dd625d1c16Dc894"),
    );
    m.insert(
        "Cobinhood",
        address!("0BB9Fc3Ba7BCF6e5d6F6fC15123ff8d5F96cEE00"),
    );
    m.insert(
        "Cobinhood_1",
        address!("18a2111eB97884caE46BEeb560b3e67fB0978773"),
    );
    m.insert(
        "Cobinhood_10",
        address!("87a42198419c2381fE19d9028D794608B8e19A93"),
    );
    m.insert(
        "Cobinhood_11",
        address!("8958618332dF62AF93053cb9c535e26462c959B0"),
    );
    m.insert(
        "Cobinhood_12",
        address!("9A1a3e23E9c781d9f4A549D390f120C76751C245"),
    );
    m.insert(
        "Cobinhood_13",
        address!("a2daa486Bb03c0eb966133345c8AbA89bf114E50"),
    );
    m.insert(
        "Cobinhood_14",
        address!("A7E53B8De2620a8d0072b06eCa271Fd4EbD8D262"),
    );
    m.insert(
        "Cobinhood_15",
        address!("b2d0B80dCeca962766C742630305066F700dc074"),
    );
    m.insert(
        "Cobinhood_16",
        address!("B726dA4fbdc3E4dBda97bb20998cF899b0e727E0"),
    );
    m.insert(
        "Cobinhood_17",
        address!("C5663294f16d3bEf912bA06a62AeC73C32d1bEe4"),
    );
    m.insert(
        "Cobinhood_2",
        address!("2B58521c4493210C172862D22E630537B3B63302"),
    );
    m.insert(
        "Cobinhood_3",
        address!("2e6B57F24B7c0b1d4De8bC514B89234432d60221"),
    );
    m.insert(
        "Cobinhood_4",
        address!("3FC4163546e64b5c98eb18c2E6A35d84a601590f"),
    );
    m.insert(
        "Cobinhood_5",
        address!("41Fced841a2c4dBAf9B31a2AAeeaEC2C5fb655B3"),
    );
    m.insert(
        "Cobinhood_6",
        address!("4886d2E96aBB131083B4072D9d102e77ffCFf74b"),
    );
    m.insert(
        "Cobinhood_7",
        address!("69e9Bf9b1e5A725F5910EaD276Dc6932f687dCD0"),
    );
    m.insert(
        "Cobinhood_8",
        address!("7917aF9E9D49504956108d86537Aa3dE61Fc9045"),
    );
    m.insert(
        "Cobinhood_9",
        address!("7a6fF2Ac9A80d2fA56587BBEc63B59BfAEA31Ae0"),
    );
    m.insert("Cobo", address!("2487cb1A359c942312259BBc64a01CEe32E9f539"));
    m.insert(
        "Cobo_1",
        address!("39A80b830a4b77a56EF952df33CaabA70F27Fd5D"),
    );
    m.insert(
        "Cobo_10",
        address!("B9711550ec6Dc977f26B73809A2D6791c0F0E9C8"),
    );
    m.insert(
        "Cobo_11",
        address!("BF957e1c121FA769580D29bF320Ee8BfF138Ad12"),
    );
    m.insert(
        "Cobo_12",
        address!("DAC967C10444267EE2e30De4D94AD997AdA96Be6"),
    );
    m.insert(
        "Cobo_13",
        address!("F1300cc9c2Cf347f7902742fEC4dF9dbA952fD7b"),
    );
    m.insert(
        "Cobo_2",
        address!("5733edE0F7109acf4565336b59E530272446f470"),
    );
    m.insert(
        "Cobo_3",
        address!("6B365AF8d060E7F7989985D62485357E34e2e8f5"),
    );
    m.insert(
        "Cobo_4",
        address!("7a843e6ff730CB48Ac94fD88235d393a41d23c5b"),
    );
    m.insert(
        "Cobo_5",
        address!("7EF2D7B88D43F1831241F0dD63E0bdeF048Ba8aC"),
    );
    m.insert(
        "Cobo_6",
        address!("A443320A40fdd0e8B3c21c94ed4363Efb7621679"),
    );
    m.insert(
        "Cobo_7",
        address!("A9C7d31BB1879BfF8BE25EaD2F59B310a52b7c5a"),
    );
    m.insert(
        "Cobo_8",
        address!("Af850683a03Af068d2870bFD1C7852980e754ee8"),
    );
    m.insert(
        "Cobo_9",
        address!("B8001C3eC9AA1985f6c747E25c28324E4A361ec1"),
    );
    m.insert(
        "CoinBene",
        address!("0a33Fa8D5037940C33BEA55dF21e3A101F16F784"),
    );
    m.insert(
        "CoinBene_1",
        address!("194A0d4139Bd1eA6f471dD6B7c3a241479292AC6"),
    );
    m.insert(
        "CoinBene_2",
        address!("2E771b31E7A2DA659F5eC05E3f5bFF2Eeb382aff"),
    );
    m.insert(
        "CoinBene_3",
        address!("33683b94334eeBc9BD3EA85DDBDA4a86Fb461405"),
    );
    m.insert(
        "CoinBene_4",
        address!("3788539703c1e469fE0EB408095E97B0C247042a"),
    );
    m.insert(
        "CoinBene_5",
        address!("6A3eB79E1C4023f1610FF046C5dc30f9790d326f"),
    );
    m.insert(
        "CoinBene_6",
        address!("9539e0b14021a43cDE41d9d45Dc34969bE9c7cb0"),
    );
    m.insert(
        "CoinBene_7",
        address!("Ee57141075EA3FFFC41F06AF2cEba23e521019a8"),
    );
    m.insert(
        "CoinDCX",
        address!("06051836ac6C5112b890f8b6Ec78e33D1AfeaE7c"),
    );
    m.insert(
        "CoinDCX_1",
        address!("07E114C06462D8892Ae4574A7502b8c1c0FBdFbb"),
    );
    m.insert(
        "CoinDCX_10",
        address!("3698cc7F524BAde1a05e02910538F436a3E94384"),
    );
    m.insert(
        "CoinDCX_11",
        address!("37b6bD5fECE5b88B6E8e825196bcc868a2FeEd51"),
    );
    m.insert(
        "CoinDCX_12",
        address!("38f76d1C8fcC854fb4d2416dDAeC8Df41Ab60867"),
    );
    m.insert(
        "CoinDCX_13",
        address!("44A69D8443521969FFDC8a0721AD1013582d7097"),
    );
    m.insert(
        "CoinDCX_14",
        address!("4928B328d4261120a493dfdb32B6d4Cc57D58850"),
    );
    m.insert(
        "CoinDCX_15",
        address!("4D24EecEcb86041F47bca41265319e9f06aE2Fcb"),
    );
    m.insert(
        "CoinDCX_16",
        address!("50B0063161e507bEc6c21cC23FD11EC2945b7b52"),
    );
    m.insert(
        "CoinDCX_17",
        address!("59548449926551ACb5480d610292c4266b872beF"),
    );
    m.insert(
        "CoinDCX_18",
        address!("5fED0d9bE6FD42e086E0D4F1bF6ceCd19635dEAd"),
    );
    m.insert(
        "CoinDCX_19",
        address!("660e3Bd3bcDa11538fa331282666F1d001b87A42"),
    );
    m.insert(
        "CoinDCX_2",
        address!("0B5ffAE844a4B21c7beeDed2595F22215288173C"),
    );
    m.insert(
        "CoinDCX_20",
        address!("6D92f2E52481ce219D201EaA1b0Cf6839270152F"),
    );
    m.insert(
        "CoinDCX_21",
        address!("7157Bb38613c5362e59dFd769C2Fcf4996B4cc8b"),
    );
    m.insert(
        "CoinDCX_22",
        address!("763104507945B6b7f21Ee68b92048A53F7debF18"),
    );
    m.insert(
        "CoinDCX_23",
        address!("78bba2389c2cEEb6f94C70eD133712E3B3e2C4D0"),
    );
    m.insert(
        "CoinDCX_24",
        address!("7cCCf66DB1A0d4069a52A0C26859EDbC34177065"),
    );
    m.insert(
        "CoinDCX_25",
        address!("881f982575a3EcBEA6fe133ddB0951303215d130"),
    );
    m.insert(
        "CoinDCX_26",
        address!("892787C947fdd1CF6C525C6107d80265D3D7EBb4"),
    );
    m.insert(
        "CoinDCX_27",
        address!("8c7Efd5B04331EFC618e8006f19019A3Dc88973e"),
    );
    m.insert(
        "CoinDCX_28",
        address!("90f76616d34Cb6A1F4423B33c0201B2A1980Fc81"),
    );
    m.insert(
        "CoinDCX_29",
        address!("978E746330870627CC353092eDd2dBA3Cc99461c"),
    );
    m.insert(
        "CoinDCX_3",
        address!("1CE0c2827e2eF14D5C4f29a091d735A204794041"),
    );
    m.insert(
        "CoinDCX_30",
        address!("A15B94629727152c952a6979d899F71426cE7976"),
    );
    m.insert(
        "CoinDCX_31",
        address!("A4FE2F90a8991A410c825C983CbB6A92d03607fc"),
    );
    m.insert(
        "CoinDCX_32",
        address!("A916a54af7553BAe6172e510D067826Bd204d0dD"),
    );
    m.insert(
        "CoinDCX_33",
        address!("AA8bC1fc0FCfdcA5b7E5D35e5AC13800850d90C7"),
    );
    m.insert(
        "CoinDCX_34",
        address!("Abd9193388C58D7e9f45fF5E4ca741212d1Ec827"),
    );
    m.insert(
        "CoinDCX_35",
        address!("Ada1fA671651D335998c6a0dE336a78F5b49Ad3F"),
    );
    m.insert(
        "CoinDCX_36",
        address!("b188a49Da0836c289dcB4Fa0E856647a33DE537F"),
    );
    m.insert(
        "CoinDCX_37",
        address!("b6DFCF39503dddDe140105954a819e944CE543A7"),
    );
    m.insert(
        "CoinDCX_38",
        address!("b79421720b92180487f71F13c5D5D8B9ecA27BF1"),
    );
    m.insert(
        "CoinDCX_39",
        address!("b85E9868a0E8492353Db5C3022e6F96fc62F2306"),
    );
    m.insert(
        "CoinDCX_4",
        address!("2407b9B9662d970ecE2224A0403D3B15c7e4D1FE"),
    );
    m.insert(
        "CoinDCX_40",
        address!("C1723Af0Dc5400A1cAAa47E76a45c39538A6AD49"),
    );
    m.insert(
        "CoinDCX_41",
        address!("C4e805208D8d25b71BDcfB558F29259544EC4fcC"),
    );
    m.insert(
        "CoinDCX_42",
        address!("CCFA6f3b01c7bf07B033A9d496Fdf22F0cdF5293"),
    );
    m.insert(
        "CoinDCX_43",
        address!("D4D7Aedb9AbeEC03101dB6f8426DeeE390E3cCF9"),
    );
    m.insert(
        "CoinDCX_44",
        address!("Df263Bd241B694b1Fbe30eE954757ddDEd7D95e6"),
    );
    m.insert(
        "CoinDCX_45",
        address!("e298dC1c377e4511f32Afd2362726c4F3A644356"),
    );
    m.insert(
        "CoinDCX_46",
        address!("EF0Fc6322b2b5b02f0Db68f8eA74819560124b2d"),
    );
    m.insert(
        "CoinDCX_47",
        address!("f031356Fa1bD3949420884db1a35A99422E36df1"),
    );
    m.insert(
        "CoinDCX_48",
        address!("f250EE103b8e4C5b9825a270e5c70Ea0C113c854"),
    );
    m.insert(
        "CoinDCX_49",
        address!("F25d1D2507ce1f956F5BAb45aD2341e3c0DB6d3C"),
    );
    m.insert(
        "CoinDCX_5",
        address!("274c427B1BF0bB4a137EDE688c6D621263CA7Ce8"),
    );
    m.insert(
        "CoinDCX_50",
        address!("F379FcD9C996d85de025985bA9B1C9C96DAa4a72"),
    );
    m.insert(
        "CoinDCX_51",
        address!("f809c975eFAD2Bc33E21B5972DB765A6230E956A"),
    );
    m.insert(
        "CoinDCX_6",
        address!("29A62a542b6EA441abB6F03C2bca54aD72fF750C"),
    );
    m.insert(
        "CoinDCX_7",
        address!("2DEFfdde0867B6EcA9a63fE53ef0FC3106d3DBa0"),
    );
    m.insert(
        "CoinDCX_8",
        address!("2e5129e77c928D96b5A70c0effB97Ee6e95D77b6"),
    );
    m.insert(
        "CoinDCX_9",
        address!("35efE40CeDdb8DfA27F0c3e4cd65B711C29CB5D8"),
    );
    m.insert(
        "CoinDhan",
        address!("bf1a97D8D4229d61B031214d5BbE9a5cB1e737f9"),
    );
    m.insert(
        "CoinEgg",
        address!("93f36930F94FBB5aFc5fB506D3f7ABB9179a4e4e"),
    );
    m.insert(
        "CoinEx",
        address!("187E3534f461d7C59a7d6899a983A5305b48f93F"),
    );
    m.insert(
        "CoinEx_1",
        address!("1e450c2A1870A52606eDd37ac0bF593dca9C1c3F"),
    );
    m.insert(
        "CoinEx_10",
        address!("89F2ab029DcD11bD5A00Ed6A77ccBE46315212e8"),
    );
    m.insert(
        "CoinEx_11",
        address!("8dA2931006453e0285682314af5bcd2377deaEB1"),
    );
    m.insert(
        "CoinEx_12",
        address!("90f86774e792e91cf81B2Ff9F341EfcA649343A6"),
    );
    m.insert(
        "CoinEx_13",
        address!("AFedF06777839D59eED3163cC3e0A5057b514399"),
    );
    m.insert(
        "CoinEx_14",
        address!("B55270a75D548de1f7b89eA571438E9745465644"),
    );
    m.insert(
        "CoinEx_15",
        address!("B85bd2A1D76586BB8F81494BFA061DfBF803b4db"),
    );
    m.insert(
        "CoinEx_16",
        address!("b9ee1e551f538A464E8F8C41E9904498505B49b0"),
    );
    m.insert(
        "CoinEx_17",
        address!("cF7105FDE573695D62666aeFa4e1691bf3Ab50d5"),
    );
    m.insert(
        "CoinEx_18",
        address!("D782E53A49D564F5fce4bA99555DD25d16d02a75"),
    );
    m.insert(
        "CoinEx_19",
        address!("DA07F1603a1c514b2F4362F3eae7224A9CDEfAF9"),
    );
    m.insert(
        "CoinEx_2",
        address!("3339Afcb8e237aA8AaF3e1040473173830EfD09C"),
    );
    m.insert(
        "CoinEx_20",
        address!("E70b8dc28E795738A772379E9D456E7d74f50aB5"),
    );
    m.insert(
        "CoinEx_21",
        address!("f54635836862aAD6e255E9B4FE49275fA5047E5d"),
    );
    m.insert(
        "CoinEx_3",
        address!("33Ddd548FE3a082d753E5fE721a26E1Ab43e3598"),
    );
    m.insert(
        "CoinEx_4",
        address!("53Eb3Ea47643E87e8f25dd997A37B3b5260e7336"),
    );
    m.insert(
        "CoinEx_5",
        address!("5Ad4D300FA795e9C2FE4221F0e64A983aCdBCaC9"),
    );
    m.insert(
        "CoinEx_6",
        address!("5Cf44f2cB65aF7D56B30719312eCD13151A0470b"),
    );
    m.insert(
        "CoinEx_7",
        address!("601A63C50448477310feDb826ED0295499bAf623"),
    );
    m.insert(
        "CoinEx_8",
        address!("6fdbe6bEC0B63334c0dC26623Ca9C58dFc158c4e"),
    );
    m.insert(
        "CoinEx_9",
        address!("85Cf05F35b6D542aC1d777d3F8cfde57578696fc"),
    );
    m.insert(
        "CoinExchange",
        address!("4B01721F0244E7c5B5F63c20942850E447f5a5Ee"),
    );
    m.insert(
        "CoinFLEX",
        address!("01E79FF04a66CE034758172f8B422B0f8115921B"),
    );
    m.insert(
        "CoinFLEX_1",
        address!("1FA2157797a4Ef47DCb5cD4d50835F0c69F70Af2"),
    );
    m.insert(
        "CoinFLEX_2",
        address!("544cd22157aBF2d06c683B71A97B33FAAE65a791"),
    );
    m.insert(
        "CoinFLEX_3",
        address!("D56E9Af0243FFbbE4c6a801ad8f20a097D247122"),
    );
    m.insert(
        "CoinField",
        address!("a1813f73448a7392e6f069299c23120aeEAb879A"),
    );
    m.insert(
        "CoinList",
        address!("187c0E0aa33282096b39a33457939f1dC3Ea8e0f"),
    );
    m.insert(
        "CoinList_1",
        address!("8D1f2eBFACCf1136dB76FDD1b86f1deDE2D23852"),
    );
    m.insert(
        "CoinList_2",
        address!("D1669Ac6044269b59Fa12c5822439F609Ca54F41"),
    );
    m.insert(
        "CoinList_3",
        address!("D2C82F2e5FA236E114A81173e375a73664610998"),
    );
    m.insert(
        "CoinPayments.net",
        address!("003E36550908907c2a2dA960FD19A419B9A774b7"),
    );
    m.insert(
        "CoinPayments.net_1",
        address!("005AD9b93955Bf76d180C66f33Fe84bD0f3310b5"),
    );
    m.insert(
        "CoinPayments.net_10",
        address!("8AE0b6bb5ea5b8AB7C9322FCB0Dfcd8f7eFfd169"),
    );
    m.insert(
        "CoinPayments.net_11",
        address!("a3fa59C716908290AB8611b33e33CD55876aEdDF"),
    );
    m.insert(
        "CoinPayments.net_12",
        address!("C49688C286FA5FDC9f4DAA0f09F35861115a16f4"),
    );
    m.insert(
        "CoinPayments.net_13",
        address!("cA15769b2b683F414bd4B95DFEF405101D883a4b"),
    );
    m.insert(
        "CoinPayments.net_14",
        address!("ce3DaEbEC9A5cB4904bEb01C8998bbca6e96Ed9F"),
    );
    m.insert(
        "CoinPayments.net_15",
        address!("e1B0F467BB4667BcAF77a147F784cFE1edffD6cb"),
    );
    m.insert(
        "CoinPayments.net_16",
        address!("ffCd95D059bA265194CEc9b1FB6609bf2aA6B436"),
    );
    m.insert(
        "CoinPayments.net_2",
        address!("0833B566dD4D3a2F55B8416E7Ca7e43921510885"),
    );
    m.insert(
        "CoinPayments.net_3",
        address!("1a459813Db7758B0309b7785FEc854966685Ce3b"),
    );
    m.insert(
        "CoinPayments.net_4",
        address!("278425B97f719bC8cFA1d8aA1bBf7520CA81610c"),
    );
    m.insert(
        "CoinPayments.net_5",
        address!("31d3a108FAc60b0D5584226d9D2aa2294C637d76"),
    );
    m.insert(
        "CoinPayments.net_6",
        address!("4685BDB11C75aDE469d6Aa294c332D4f60736EBB"),
    );
    m.insert(
        "CoinPayments.net_7",
        address!("5A060db7cAc8786818B752EE3cc802548D86a30C"),
    );
    m.insert(
        "CoinPayments.net_8",
        address!("632d17e6f450B0b89cf1D8C61E93b9e9a4b8608B"),
    );
    m.insert(
        "CoinPayments.net_9",
        address!("7DE68A47A861A549Dbe2baE37884FC6c84D12026"),
    );
    m.insert(
        "CoinSpot",
        address!("09363887A4096b142f3F6b58A7eeD2F1A0FF7343"),
    );
    m.insert(
        "CoinSpot_1",
        address!("20312e96b1a0568Ac31C6630844a962383cc66c2"),
    );
    m.insert(
        "CoinSpot_10",
        address!("91a0a3043f68986043D7083C4D85B558B21F0A7B"),
    );
    m.insert(
        "CoinSpot_11",
        address!("9239dF3E9996c776D539EB9f01A8aE8E7957b3c3"),
    );
    m.insert(
        "CoinSpot_12",
        address!("a89a1278Ac85367F38BDF6746658CE2B9875526E"),
    );
    m.insert(
        "CoinSpot_13",
        address!("a9Bd318A4Ca1747E6068D100e18711B529386e29"),
    );
    m.insert(
        "CoinSpot_14",
        address!("db6FDc30AB61C7cCA742D4c13D1b035F3F82019A"),
    );
    m.insert(
        "CoinSpot_15",
        address!("DF1553A2130cbAFA70a35e68eFC6cCF67F0A278C"),
    );
    m.insert(
        "CoinSpot_16",
        address!("E4b3dD9839ed1780351Dc5412925cf05F07A1939"),
    );
    m.insert(
        "CoinSpot_17",
        address!("e6f79f8B46b30f293cfBDe50eF787d2Fe0610782"),
    );
    m.insert(
        "CoinSpot_18",
        address!("eeD86B90448C371Eab47b7f16E294297C27E4F51"),
    );
    m.insert(
        "CoinSpot_19",
        address!("f1088841Ab08FC1Cb3835fd75207bCB3137F6EE3"),
    );
    m.insert(
        "CoinSpot_2",
        address!("32143A02Fb6484D18C79Fa0401c9bF760DD3DE68"),
    );
    m.insert(
        "CoinSpot_20",
        address!("f35A6bD6E0459A4B53A27862c51A2A7292b383d1"),
    );
    m.insert(
        "CoinSpot_3",
        address!("32E567E8B527d3194C60ea3C6a5c009d58A0B36d"),
    );
    m.insert(
        "CoinSpot_4",
        address!("33A64dcDfa041bEfebC9161a3e0c6180cd94Fa89"),
    );
    m.insert(
        "CoinSpot_5",
        address!("4207837D4Cd914467EB76bf88c4d6e7Ba11ccDf9"),
    );
    m.insert(
        "CoinSpot_6",
        address!("56de1961fDA5454E6F8e6D0e3124fF648FD69400"),
    );
    m.insert(
        "CoinSpot_7",
        address!("60F9e80D0D40b2958ac39006635dE782096866C3"),
    );
    m.insert(
        "CoinSpot_8",
        address!("867bfA133D64fAd734C89f886D2A169B6504Ab2b"),
    );
    m.insert(
        "CoinSpot_9",
        address!("916ED5586bB328E0eC1a428af060DC3D10919d84"),
    );
    m.insert(
        "CoinTiger",
        address!("0E58a7652143a0A275750E5ac57a4Fb14eb4A18D"),
    );
    m.insert(
        "CoinTiger_1",
        address!("5Dd697c77E80C7Fda3dA1cccd5214c3a75503727"),
    );
    m.insert(
        "CoinTiger_2",
        address!("707C6F7d35798780CEFCE81B747392198566a186"),
    );
    m.insert(
        "CoinTiger_3",
        address!("77f814F8ccba595B83c69E587b159E9887890639"),
    );
    m.insert(
        "CoinTiger_4",
        address!("8A023F23d0DC363A74dc5CB480FB393cfef0CCB9"),
    );
    m.insert(
        "CoinTiger_5",
        address!("97d31bF5271052888863791FeD990c17cE656B95"),
    );
    m.insert(
        "CoinTiger_6",
        address!("BCB301999bB1FA44A9181Fbe5E279218B02Aae32"),
    );
    m.insert(
        "CoinTiger_7",
        address!("e9d2AFFF18F08375D5B7a8A804e272b6c81Ceb9F"),
    );
    m.insert(
        "CoinTiger_8",
        address!("fBdEB87969F5610d006d1a4ed79308A5778E77E5"),
    );
    m.insert(
        "CoinW",
        address!("092738E25c5d132e235e8bd4451d180e86dA6736"),
    );
    m.insert(
        "CoinW_1",
        address!("204828c049AaBC79405510bd287E066A64c1f307"),
    );
    m.insert(
        "CoinW_10",
        address!("639d9BA0f11Ff73A25c0a26849D4d5A7175169b6"),
    );
    m.insert(
        "CoinW_11",
        address!("64919eB2823e0dcCb3bD29D24B7d4bc6ab992f0c"),
    );
    m.insert(
        "CoinW_12",
        address!("67EC7eA2dE759458B9A7f28956618bA841C9Bf9f"),
    );
    m.insert(
        "CoinW_13",
        address!("696CFd63F98DCd1FeA2d6BD27f3CF85C2007a2FD"),
    );
    m.insert(
        "CoinW_14",
        address!("6A9F6ab41A90D59fC5Ef0D373559F39002c22a0f"),
    );
    m.insert(
        "CoinW_15",
        address!("6f31D347457962c9811ff953742870EF5a755dE3"),
    );
    m.insert(
        "CoinW_16",
        address!("7009407B417759C34a35d63530D30A818661B857"),
    );
    m.insert(
        "CoinW_17",
        address!("80E29AcB842498fE6591F020bd82766DCe619D43"),
    );
    m.insert(
        "CoinW_18",
        address!("83a0B03E76c88052cd7989edeaB80aA4cACc66b5"),
    );
    m.insert(
        "CoinW_19",
        address!("8705CcFd8A6dF3785217C307cbEbf9b793310B94"),
    );
    m.insert(
        "CoinW_2",
        address!("2aE40EeAf27a28D31eAbB050dAC2e0568c7B1a7E"),
    );
    m.insert(
        "CoinW_20",
        address!("87B748aC0a8eAe308882F13522fa81f384588Cb1"),
    );
    m.insert(
        "CoinW_21",
        address!("940Bdc6844d9b801c60a775397f2945b97a6E1C6"),
    );
    m.insert(
        "CoinW_22",
        address!("9998d99daAE38B87F0f43c40974720A0B3D3D95C"),
    );
    m.insert(
        "CoinW_23",
        address!("9f8646A35db0f466aC9322e2D194cc18f209Fc75"),
    );
    m.insert(
        "CoinW_24",
        address!("a20f10289248717374e9B7776dC368aa526cb6F2"),
    );
    m.insert(
        "CoinW_25",
        address!("a75bae74E39DC2f38eE879eC3De9489ABE65280e"),
    );
    m.insert(
        "CoinW_26",
        address!("ab59487C43211da1d8B8e60479CAd6aADC52cd1c"),
    );
    m.insert(
        "CoinW_27",
        address!("B840fe2b3fd8F75275240c671d6EC659e4c9a500"),
    );
    m.insert(
        "CoinW_28",
        address!("BEf77b5f2B7434333b13f6441cB88866e07ECa2D"),
    );
    m.insert(
        "CoinW_29",
        address!("bf2d58698a8A215F868CF24BAba360c77266b466"),
    );
    m.insert(
        "CoinW_3",
        address!("2Ce80f8d81fa594c4dBc1374A974d277b1188598"),
    );
    m.insert(
        "CoinW_30",
        address!("C4e08dd8Dfc72cDBB61bc9Cfa82F8738F3ab6D05"),
    );
    m.insert(
        "CoinW_31",
        address!("cb243bf48FB443082FAE7db47eC96Cb120Cd6801"),
    );
    m.insert(
        "CoinW_32",
        address!("DC8f0f2Aef01EB3a11b7F73B087A2F6A2eE3067f"),
    );
    m.insert(
        "CoinW_33",
        address!("Dcc06BB2a55169a897E8b4148f151D7574EfF66C"),
    );
    m.insert(
        "CoinW_34",
        address!("e3Be8206E74773Dbc4902480344ae1F94323170b"),
    );
    m.insert(
        "CoinW_35",
        address!("E48A4E20BE4EA888748c56BDCb632D960Cbfb011"),
    );
    m.insert(
        "CoinW_36",
        address!("eCAB44B10b8faEE48d7eEe03F81a815078070624"),
    );
    m.insert(
        "CoinW_37",
        address!("F110c5f6e04Eea18e415C5753a43479851F27530"),
    );
    m.insert(
        "CoinW_38",
        address!("F27e7FA401B991A593A9E83AA6f23f2A212A9aC4"),
    );
    m.insert(
        "CoinW_39",
        address!("F6E9D98134194fcc78bBF5908491fDd81a6Ee13D"),
    );
    m.insert(
        "CoinW_4",
        address!("2D6323cc438B96F0aE942280762Cc507B5398563"),
    );
    m.insert(
        "CoinW_5",
        address!("3864D8F360bA98212A2edDF05A357599F25196c1"),
    );
    m.insert(
        "CoinW_6",
        address!("3974D816A969908A82428384C728fbd8fAFfbFe1"),
    );
    m.insert(
        "CoinW_7",
        address!("429Bf8EC3330E02401D72bEadE86000d9a2E19EB"),
    );
    m.insert(
        "CoinW_8",
        address!("46499f2dff9f551bC71646a0448396B1C4702343"),
    );
    m.insert(
        "CoinW_9",
        address!("611F32E5d7F6640ecAf3e66759318aBB9CbEce64"),
    );
    m.insert(
        "Coincheck",
        address!("189B9cBd4AfF470aF2C0102f365FC1823d857965"),
    );
    m.insert(
        "Coincheck_1",
        address!("8696e84aB5e78983f2456bCB5c199eEa9648C8C2"),
    );
    m.insert(
        "Coincheck_2",
        address!("9C19B0497997Fe9E75862688a295168070456951"),
    );
    m.insert(
        "Coincheck_3",
        address!("d52814615DC129e1eEa3520e7e7f8d44DBfc6c5b"),
    );
    m.insert(
        "Coindelta",
        address!("B6ba1931e4E74FD080587688F6DB10E830F810d5"),
    );
    m.insert(
        "Coinhako",
        address!("1562Bd4f90EB997611B5D7579ab38e5a23aCb33e"),
    );
    m.insert(
        "Coinhako_1",
        address!("19d97aa29cd33Bd966d52e2Bc9dFc719f2Bb9aE1"),
    );
    m.insert(
        "Coinhako_2",
        address!("1d1bD550197c7c0787b9ad0aEA9c1CCa66eE0E90"),
    );
    m.insert(
        "Coinhako_3",
        address!("a193C943980A9340F306b3d59Deb183Dc501B35f"),
    );
    m.insert(
        "Coinhako_4",
        address!("bb44E3349C23cC430CAe6EbBaf0256c9f2a1872f"),
    );
    m.insert(
        "Coinhako_5",
        address!("d4BDDf5E3D0435D7A6214A0B949C7BB58621F37C"),
    );
    m.insert(
        "Coinhako_6",
        address!("E66BAa0B612003AF308D78f066Bbdb9a5e00fF6c"),
    );
    m.insert(
        "Coinify",
        address!("86D3E894b5CDb6a80affFd35eD348868fb98DD3f"),
    );
    m.insert(
        "Coinjar",
        address!("0363bFC09B48616190d15ddEe5987Ae2D05Da6F4"),
    );
    m.insert(
        "Coinjar_1",
        address!("6e5bBE3f4Fe7b0CCd74d7dA1Cbd5CfE3c868BbdC"),
    );
    m.insert(
        "Coinjar_2",
        address!("C1C88785E9B5c9c85B3f6c99255C3bef9e3B14A7"),
    );
    m.insert(
        "Coinjar_3",
        address!("d6a062CAE6123C158768A5C444CA0896CC60D6B1"),
    );
    m.insert(
        "Coinjar_4",
        address!("ddC50252A3080d5028D1c25261f78f023E9117d5"),
    );
    m.insert(
        "Coinmetro",
        address!("0BC7b31BF8ffAc8213E496102167FE73Fc30937a"),
    );
    m.insert(
        "Coinmetro_1",
        address!("165Fe6a10812fAA49515522d685A27c6Bf12DbA9"),
    );
    m.insert(
        "Coinmetro_10",
        address!("Fad672DC92C2d2Db0aa093331bD1098e30249AB8"),
    );
    m.insert(
        "Coinmetro_2",
        address!("1E7b450322E42b53D55ABC795913537fD680fe0c"),
    );
    m.insert(
        "Coinmetro_3",
        address!("46a189C7F37953b91E5bd75D0E608efdAfefAF1C"),
    );
    m.insert(
        "Coinmetro_4",
        address!("4CF2220105995F006813923019f02BE1CCcA8132"),
    );
    m.insert(
        "Coinmetro_5",
        address!("706Ee6cF36a4bF95ec7Fe5d468571B9E9e63FBB6"),
    );
    m.insert(
        "Coinmetro_6",
        address!("a270F3ad1a7a82E6a3157F12a900f1E25BC4FbFD"),
    );
    m.insert(
        "Coinmetro_7",
        address!("bAC7c449689A2d3c51c386D8E657338C41ab3030"),
    );
    m.insert(
        "Coinmetro_8",
        address!("DD06B66c76D9c6fdC41935A7b32566C646325005"),
    );
    m.insert(
        "Coinmetro_9",
        address!("F3e35734B7413F87c2054A16cE04230d803E4dC3"),
    );
    m.insert(
        "Coinone",
        address!("167A9333BF582556f35Bd4d16A7E80E191aa6476"),
    );
    m.insert(
        "Coinone_1",
        address!("1e2FCfd26d36183f1A5d90f0e6296915b02BCb40"),
    );
    m.insert(
        "Coinone_2",
        address!("7A1BA53C0E2D218DF39E76e4eFBF0455978cc23E"),
    );
    m.insert(
        "Coins.ph",
        address!("17Ac1517c30d5Cd7f065cb28C4703431bb69D47D"),
    );
    m.insert(
        "Coins.ph_1",
        address!("4577F1A9b54492fA1B9EF3b58d7CDc1ea8b3225C"),
    );
    m.insert(
        "CoinsPaid",
        address!("292f04a44506c2fd49Bac032E1ca148C35A478c8"),
    );
    m.insert(
        "CoinsPaid_1",
        address!("bd0fCcdC19bC3b979e8E256b7B88AAe7C77A5BEC"),
    );
    m.insert(
        "CoinsPaid_2",
        address!("Dce92f40cAdDE2C4e3EA78b8892c540e6bFe2f81"),
    );
    m.insert(
        "Coinsbit",
        address!("21Dd5c13925407e5bCec3f27aB11a355a9Dafbe3"),
    );
    m.insert(
        "Coinsbit_1",
        address!("252C8bbeF63101eBcA22C27951175C6B33c86b07"),
    );
    m.insert(
        "Coinsbit_2",
        address!("75987b9edB5463CE1a3a857E11671424600927A4"),
    );
    m.insert(
        "Coinsquare",
        address!("02fdc44Bf226E49DCecA4775Afaef3360e9C4EE9"),
    );
    m.insert(
        "Coinsquare_1",
        address!("0fcFF154753e337983613889b69dd85Fe8a1a145"),
    );
    m.insert(
        "Coinsquare_10",
        address!("7061d86A274B398a1fB7Cdb74B3abBc7601e105f"),
    );
    m.insert(
        "Coinsquare_11",
        address!("7ee87dd5BB9924Cb85CA2916Bd4E04299D3A8EcC"),
    );
    m.insert(
        "Coinsquare_12",
        address!("82Be7cFeF05B70c4AF47F8fd70F636201121341b"),
    );
    m.insert(
        "Coinsquare_13",
        address!("8623c08A4B880799CF65E75137ec9759DB336637"),
    );
    m.insert(
        "Coinsquare_14",
        address!("89813b57AE92e74Fb808eb7639d3A0050c9b3D7D"),
    );
    m.insert(
        "Coinsquare_15",
        address!("8e080C5d233F2A14A37d024c0382bF0585146993"),
    );
    m.insert(
        "Coinsquare_16",
        address!("910695E5C7c14499B554fb132A9710988a42fC38"),
    );
    m.insert(
        "Coinsquare_17",
        address!("91ADf14f4C0782634E04Dfc6e9Be16d950AA4daA"),
    );
    m.insert(
        "Coinsquare_18",
        address!("9C6D4A1922Eed56Ee9de148c5BA9b1b477FEcBb6"),
    );
    m.insert(
        "Coinsquare_19",
        address!("C4d75abAb14Ef006d5Ac9fe901a8ed616C4e2627"),
    );
    m.insert(
        "Coinsquare_2",
        address!("14AA1AD09664c33679aE5689d93085B8F7c84bd3"),
    );
    m.insert(
        "Coinsquare_20",
        address!("d093F2Ee92cf32B4D3EBefd965447415074DD6c8"),
    );
    m.insert(
        "Coinsquare_21",
        address!("D381347EE757F53aE4B3b6822DAeC3E2A14B2005"),
    );
    m.insert(
        "Coinsquare_22",
        address!("D5B2C371808018ee131ad387877C4d58e08e7A06"),
    );
    m.insert(
        "Coinsquare_23",
        address!("f9c91937737cCaFE9bBb662b1917B54F9606Ca13"),
    );
    m.insert(
        "Coinsquare_24",
        address!("fac596Facd1901458C1C6347397a6e5D0769736c"),
    );
    m.insert(
        "Coinsquare_3",
        address!("2f671a39613861EC17ad35de44F4f84e3D4C69f4"),
    );
    m.insert(
        "Coinsquare_4",
        address!("3858A27eeCB5f1144473E35A293cb1B2bda6DfF4"),
    );
    m.insert(
        "Coinsquare_5",
        address!("476B067CbFF8ACB805038E9dAEF5D51c7612d593"),
    );
    m.insert(
        "Coinsquare_6",
        address!("48a0B5f7DE8789a3962918C6DF4A766c0c8857B0"),
    );
    m.insert(
        "Coinsquare_7",
        address!("56E89a4b2E3924c336d52CE0ad98fF23E1a51627"),
    );
    m.insert(
        "Coinsquare_8",
        address!("6A73f209d25CC9c089170cc5b54962e0c7614E0c"),
    );
    m.insert(
        "Coinsquare_9",
        address!("6d712f120bD65aD54a5F56670976788a044Cb987"),
    );
    m.insert(
        "Coinstore",
        address!("20664cacdcfeb318C8e145a03C75e34bc2CC4A3b"),
    );
    m.insert(
        "Coinstore_1",
        address!("65e1615eFC11c63E15c00aC4447C56aF294135a9"),
    );
    m.insert(
        "Coinstore_2",
        address!("86790abbaCcD1B21F5ecFDaA67EC6282AFbf3E83"),
    );
    m.insert(
        "Coinstore_3",
        address!("93639ba2c0138c9EF450C247D33c49aC9dbE97D5"),
    );
    m.insert(
        "Coinstore_4",
        address!("A33Ea0BACaCb74800f89762779AC03073b1182A3"),
    );
    m.insert(
        "Coinstore_5",
        address!("DF12A6f5C800eCE10D3A53636fF0a41a1AcA4850"),
    );
    m.insert(
        "Coinstore_6",
        address!("F2067aBFaB8BC621211935431519d41825d2f344"),
    );
    m.insert(
        "Coinstore_7",
        address!("f83F7ae1403fAd899108F99D9f427dC6981806C4"),
    );
    m.insert(
        "Coinswitch",
        address!("d0808Da05cc71a9F308D330bC9c5C81Bbc26FC59"),
    );
    m.insert(
        "Coinzix",
        address!("014C18ad92838E8bB62b0135a0Cf3f5CDD5Bd6F4"),
    );
    m.insert(
        "Coinzix_1",
        address!("48077400FAF11183c043Feb5184a13ea628Bb0DB"),
    );
    m.insert(
        "Coinzix_2",
        address!("4f50226BBc651EEB8E766591c4Dac762ad832de2"),
    );
    m.insert(
        "Coinzix_3",
        address!("9eE23657e6764f6DbAE17af8a8191ea3c29A6D1D"),
    );
    m.insert(
        "Copper",
        address!("0349923aE2B35FF4f0099869aeea99d1f3FD12a9"),
    );
    m.insert(
        "Copper_1",
        address!("7A1D38cacA09F408D58837E60E172ea3785f9ff0"),
    );
    m.insert(
        "Copper_2",
        address!("7C6782476e29fB26E1556FbA058eb7ECED93D327"),
    );
    m.insert(
        "Copper_3",
        address!("a205fD7344656c72FDC645b72fAF5a3DE0B3E825"),
    );
    m.insert(
        "Copper_4",
        address!("A5995359b9941E060e366B4Ee3ebB6A9f47649be"),
    );
    m.insert(
        "Copper_5",
        address!("Af64555DDD61FcF7D094824dd9B4eBea165aFc5b"),
    );
    m.insert(
        "Copper_6",
        address!("B7a2D83d94fce4A4cC8f92c961Af418D5C797565"),
    );
    m.insert(
        "Copper_7",
        address!("cFbba243c567738C2d3D958026b1Bf6Ad894A19e"),
    );
    m.insert(
        "Cryptal",
        address!("1A0324046933dDa97F0296fcCff033966278B532"),
    );
    m.insert(
        "Cryptal_1",
        address!("a046FDd5Dd224BA5498F7821A757c4b67622Db2b"),
    );
    m.insert(
        "Cryptal_2",
        address!("F3294A05Fe37673B96c6fF166aFe50cDd6B33391"),
    );
    m.insert(
        "Cryptonator",
        address!("0975CA9F986EeE35F5CbbA2d672ad9bc8D2a0844"),
    );
    m.insert(
        "Cryptonator_1",
        address!("4b407c966C2cacD502a73E46bD0B97aAf78929ee"),
    );
    m.insert(
        "Cryptonator_2",
        address!("4FED1fC4144c223aE3C1553be203cDFcbD38C581"),
    );
    m.insert(
        "Cryptonator_3",
        address!("687c8EcBc10ed1Ff6b1D700037FC4b873cf7e424"),
    );
    m.insert(
        "Cryptopia",
        address!("1B3d794bbEECD9240F46dBb3b79F4f71a972e00A"),
    );
    m.insert(
        "Cryptopia_1",
        address!("2984581eCE53A4390d1F568673cf693139C97049"),
    );
    m.insert(
        "Cryptopia_2",
        address!("5BaEac0a0417a05733884852aa068B706967e790"),
    );
    m.insert(
        "Cryptopia_3",
        address!("CEED7802EA80992aF9dA3811c455fD7BAA3f644C"),
    );
    m.insert("DDEX", address!("e269E891A2Ec8585a378882fFA531141205e92E9"));
    m.insert(
        "DEx.top",
        address!("7600977Eb9eFFA627D6BD0DA2E5be35E11566341"),
    );
    m.insert("DIFX", address!("276766330eA4447289df29648474d1BBDb3fee90"));
    m.insert(
        "DIFX_1",
        address!("58A5B842A629B9D134DFD348C714b7f1d8212253"),
    );
    m.insert(
        "DIFX_2",
        address!("bE774bB4A11c033C68FF0CC515B3316e93e94465"),
    );
    m.insert("DMEX", address!("2101e480e22C953b37b9D0FE6551C1354Fe705E6"));
    m.insert(
        "Deepcoin",
        address!("6c73b1cA08bBC3F44340603b1Fb9E331C2ABaCa7"),
    );
    m.insert(
        "Delta Exchange",
        address!("1a7574D48c4960278e89b7e7e069E5b9809D7b67"),
    );
    m.insert(
        "Delta Exchange_1",
        address!("50a3F3B8855c2DA88e56C7B0EF6E0e4A79F853f9"),
    );
    m.insert(
        "Delta Exchange_2",
        address!("c07b9DDC7F87e76E682a7a4F3859586eEF1c7efd"),
    );
    m.insert(
        "Deribit",
        address!("062448f804191128D71FC72e10a1D13Bd7308e7E"),
    );
    m.insert(
        "Deribit_1",
        address!("2EeD6A08Fb89a5CD111efA33F8DcA46CfdBE370F"),
    );
    m.insert(
        "Deribit_10",
        address!("A0F6121319a34f24653fB82aDdC8dD268Af5b9e1"),
    );
    m.insert(
        "Deribit_11",
        address!("A7e15eF7C01B58eBe5eF74Aa73625Ae4b11FE754"),
    );
    m.insert(
        "Deribit_12",
        address!("cFEe6efEc3471874022e205f4894733C42CbBF64"),
    );
    m.insert(
        "Deribit_2",
        address!("58F56615180A8eeA4c462235D9e215F72484B4A3"),
    );
    m.insert(
        "Deribit_3",
        address!("5f397B62502e255f68382791947D54C4B2d37F09"),
    );
    m.insert(
        "Deribit_4",
        address!("63F41034871535ceE49996Cc47719891Fe03dff9"),
    );
    m.insert(
        "Deribit_5",
        address!("6B378bE3c9642ccF25b1A27faCb8ace24aC34A12"),
    );
    m.insert(
        "Deribit_6",
        address!("77021d475E36b3ab1921a0e3A8380f069d3263de"),
    );
    m.insert(
        "Deribit_7",
        address!("904cC2B2694FFa78F04708D6F7dE205108213126"),
    );
    m.insert(
        "Deribit_8",
        address!("9cf8d36F4Aab14Ef4975aC6C98f896865F0900c5"),
    );
    m.insert(
        "Deribit_9",
        address!("9FaE72D291949Ed6fa8b74881328FDc123C645D3"),
    );
    m.insert(
        "Dex-Trade",
        address!("4081A867B3F3fB8761a054024d09d2575d81D381"),
    );
    m.insert(
        "Dex-Trade_1",
        address!("881368E08CC5353E0188b2cA0401b5de35F319F4"),
    );
    m.insert(
        "Dex-Trade_2",
        address!("a0f66086C32c692033b664A775Cb3484C62c2C9e"),
    );
    m.insert(
        "Dex-Trade_3",
        address!("B355104B0AE14265b69AfC31d6a117f982337280"),
    );
    m.insert(
        "Dex-Trade_4",
        address!("bde577B9582aA0305f623b51f1b70c5a118B922A"),
    );
    m.insert(
        "Dex-Trade_5",
        address!("D0ACB9C61cD72E0f57B19268D70c73b77DbDd553"),
    );
    m.insert(
        "DigiFinex",
        address!("1b930C43526B09191a74175eAA47F2A650aEB73d"),
    );
    m.insert(
        "DigiFinex_1",
        address!("3B73D7e1266e02a68185f5221a6718dB04dF6301"),
    );
    m.insert(
        "DigiFinex_2",
        address!("96C01fA1d6A732F7888b564453cED1cD6c1fa5b9"),
    );
    m.insert(
        "DigiFinex_3",
        address!("b2cA9495F3e1972801c12a964fA344a6630F427a"),
    );
    m.insert(
        "DigiFinex_4",
        address!("B37640f5F7ef7b0fDCce2c0C053DB4f976945647"),
    );
    m.insert(
        "DigiFinex_5",
        address!("e17ee7B3c676701c66B395A35f0DF4C2276a344E"),
    );
    m.insert(
        "Digital Surge",
        address!("Fb75B231C307738ce506c242bABaD2FD2e77B0bf"),
    );
    m.insert(
        "Duelbits",
        address!("3E97ae61Ceda35B857b05e6B579c54aBD5568C36"),
    );
    m.insert(
        "Duelbits_1",
        address!("4E80744fa23cEC76e1621ce0DfACeB4B1D532e12"),
    );
    m.insert("EXMO", address!("147a95FA1cCeCCc20B341824952106CB0EE1eded"));
    m.insert(
        "EXMO_1",
        address!("1Fd6267f0D86F62D88172B998390AfEE2a1F54B6"),
    );
    m.insert(
        "EXMO_2",
        address!("495a7e55dAf85d5C098a2aD5ba9a46312329D63c"),
    );
    m.insert(
        "EXMO_3",
        address!("b36CBe7f95A39984384E6AA4068B02C1697Ef80E"),
    );
    m.insert(
        "EXMO_4",
        address!("C179FBDDC946694d11185d4e15DbBa5Fd0aDac0a"),
    );
    m.insert(
        "EXMO_5",
        address!("d7B9A9b2F665849C4071Ad5af77d8c76aa30fb32"),
    );
    m.insert(
        "EXMO_6",
        address!("ffF52a282c215aeD0C4C71786D85519bbcC86464"),
    );
    m.insert(
        "Eidoo",
        address!("07ec30473eF6e1b9a434b1D48B97f79D46C13D5f"),
    );
    m.insert(
        "Eidoo_1",
        address!("aCF9e2469fBc57E6Ad391f97a41f8Aad4d68B097"),
    );
    m.insert(
        "Eidoo_2",
        address!("F1C525a488a848b58B95D79dA48C21Ce434290f7"),
    );
    m.insert(
        "Eigen Fx",
        address!("608F94DF1c1D89ea13e5984D7bF107DF137A6541"),
    );
    m.insert(
        "Eigen Fx_1",
        address!("eB9EbF2c624eBee42e0853da6443dDC6C8020de7"),
    );
    m.insert(
        "Emirex",
        address!("2F006050F172f3C2a85F5E72A5A2c6c67790d6B7"),
    );
    m.insert(
        "Emirex_1",
        address!("fA5ccf9FC5828B589aF6f321770a7260f31b4746"),
    );
    m.insert(
        "Exchange A",
        address!("d3808c5D48903be1490989F3fcE2a2b3890E8eB6"),
    );
    m.insert(
        "FCoin",
        address!("915d7915f2b469bb654A7D903A5d4417Cb8eA7Df"),
    );
    m.insert(
        "FINXFLO",
        address!("71D7cb7F3F4731Ec26E281E58d9FcE443E7B3f83"),
    );
    m.insert(
        "FINXFLO_1",
        address!("7C63a3f37d3d18e29a683b8769964a98b056b542"),
    );
    m.insert(
        "FINXFLO_2",
        address!("a31e062155B3387aeeF9A476e162b0b4AF298935"),
    );
    m.insert("FTX", address!("25eAff5B179f209Cf186B1cdCbFa463A69Df4C45"));
    m.insert(
        "FTX US",
        address!("3701F04E25b57e549A2a348C18e2d925C2805602"),
    );
    m.insert(
        "FTX US_1",
        address!("42d6Ce661bB2e5F5cc639E7BEFE74Ff9Fd649541"),
    );
    m.insert(
        "FTX US_2",
        address!("46cE2DA01290EA2D1BCED39BbA149d017DF34CB9"),
    );
    m.insert(
        "FTX US_3",
        address!("7abE0cE388281d2aCF297Cb089caef3819b13448"),
    );
    m.insert(
        "FTX US_4",
        address!("A182aAB7B51232FBFABC22d989f21d264B0B246f"),
    );
    m.insert(
        "FTX_1",
        address!("279f8940ca2a44C35ca3eDf7d28945254d0F0aE6"),
    );
    m.insert(
        "FTX_10",
        address!("C098B2a3Aa256D2140208C3de6543aAEf5cd3A94"),
    );
    m.insert(
        "FTX_11",
        address!("d8019a114e86ad41D71a3EeB6620b19Dd166A969"),
    );
    m.insert(
        "FTX_2",
        address!("2FAF487A4414Fe77e2327F0bf4AE2a264a776AD2"),
    );
    m.insert(
        "FTX_3",
        address!("41772eDd47D9DDF9ef848cDB34fE76143908c7Ad"),
    );
    m.insert(
        "FTX_4",
        address!("51bfacfcE67821EC05d3C9bC9a8BC8300fB29564"),
    );
    m.insert(
        "FTX_5",
        address!("6001CE416FF9801dba27c6eb217DfD7C258f6d27"),
    );
    m.insert(
        "FTX_6",
        address!("6E685A45Db4d97BA160FA067cB81b40Dfed47245"),
    );
    m.insert(
        "FTX_7",
        address!("772589e99bC9C54DD40acb7d73F88Ccbc9D9CF47"),
    );
    m.insert(
        "FTX_8",
        address!("9ade1c17d25246c405604344f89E8F23F8c1c632"),
    );
    m.insert(
        "FTX_9",
        address!("A60113f7d43130919802b0863abdCdb956664fD5"),
    );
    m.insert(
        "Faa.st",
        address!("94fe3AD91dACbA8eC4B82F56ff7C122181f1535d"),
    );
    m.insert(
        "Fairdesk",
        address!("0f69fd9Fee9D01034D612497fCf07e1f5AB9ED3C"),
    );
    m.insert(
        "Fairdesk_1",
        address!("63295D75B5A01eFF3D7F7Cd199fa48f9C3De6395"),
    );
    m.insert(
        "Fairdesk_2",
        address!("cc5F916C45E7a3506186ffF370fb985bA93a5167"),
    );
    m.insert(
        "Fairdesk_3",
        address!("FfD0eCEE885FcD2eBCe0FeC9add69Bf1Ed03194B"),
    );
    m.insert(
        "FalconX",
        address!("1157A2076b9bB22a85CC2C162f20fAB3898F4101"),
    );
    m.insert(
        "FalconX_1",
        address!("9FC6beF0702CF47dCD2e5a42b48e19aed8732499"),
    );
    m.insert(
        "FalconX_2",
        address!("e1eD4DA4284924dDAf69983B4D813FB1be58c380"),
    );
    m.insert(
        "FalconX_3",
        address!("F2eF6Ac0F00BC91B3CB5FFb621B64c69E87b4f77"),
    );
    m.insert(
        "FastEx",
        address!("85E1De87a7575C6581F7930F857a3813B66A14d8"),
    );
    m.insert(
        "FastEx_1",
        address!("c21A1D213f64FeDEA3415737CCe2BE37Eb59be81"),
    );
    m.insert("Firi", address!("66A0be112EFE2cc3bc2f09Fa2aCaaf9f593B0265"));
    m.insert(
        "Firi_1",
        address!("a6F617f873684ED062C9Df281145250b3E4EE2D2"),
    );
    m.insert(
        "FixedFloat",
        address!("0bC6E8ceAc156acBE85E2c46A505fF33407c45F5"),
    );
    m.insert(
        "FixedFloat_1",
        address!("440Efc740Ca49ED88051c05CF756cdE6c8Be3b27"),
    );
    m.insert(
        "FixedFloat_2",
        address!("4727250679294802377dD6cA6541B8E459077c95"),
    );
    m.insert(
        "FixedFloat_3",
        address!("4E5B2e1dc63F6b91cb6Cd759936495434C7e972F"),
    );
    m.insert(
        "FixedFloat_4",
        address!("5959dBA0123D7a60DF9E1409DE4d7b5604976060"),
    );
    m.insert(
        "FixedFloat_5",
        address!("6c344a0bBf8Ef7aF72C141c7EdA129AF4A71a9b5"),
    );
    m.insert(
        "FixedFloat_6",
        address!("71d4249079684479F2651745fA2fcD79c9b45f53"),
    );
    m.insert(
        "FixedFloat_7",
        address!("95d3B980D54CC9580DBFe88A7E3A6c48f27FfC47"),
    );
    m.insert(
        "FixedFloat_8",
        address!("f5CBD91EaBD71Dab7343F56aBe03250BD0C2fFf4"),
    );
    m.insert(
        "FixedFloat_9",
        address!("f6Ee635C28Dda95a74BA989807000cbe00317d27"),
    );
    m.insert(
        "Flata Exchange",
        address!("14301566b9669b672878d86fF0B1d18Dd58054e9"),
    );
    m.insert(
        "Flata Exchange_1",
        address!("C517dFAcBee5DFa82FeefB5cA29A2BB40b2d4fAA"),
    );
    m.insert(
        "Flybit",
        address!("91e18eE76483FA2eC5Cfe2959DF46673c2565BE0"),
    );
    m.insert(
        "Folgory Exchange",
        address!("0021845f4c2604c58F9ba5b7BFF58d16A2aB372c"),
    );
    m.insert(
        "Freewallet",
        address!("4834E61F91EC2304Cf51b590073f3c9ff8161446"),
    );
    m.insert(
        "Freewallet_1",
        address!("6025D96932D378BE7D0A46343b437678A126eCCa"),
    );
    m.insert(
        "Freewallet_2",
        address!("7eD1E469fCb3EE19C0366D829e291451bE638E59"),
    );
    m.insert(
        "Freewallet_3",
        address!("8Dd640228e1eb91e05d58206f3D9B0cCaf21bcF1"),
    );
    m.insert("GBX", address!("9F5ca0012B9B72E8F3Db57092a6f26bF4f13DC69"));
    m.insert("GDAC", address!("5735fBAC26BB21CA3C5228022cc382136038087c"));
    m.insert(
        "GDAC_1",
        address!("9f4745DF0d6713B08323e0d39Ab4CeF6891C11E1"),
    );
    m.insert(
        "GGBTC.com",
        address!("35c049D01b3782EFf7Fcf8C5770d891CC5A03561"),
    );
    m.insert(
        "GGBTC.com_1",
        address!("9fB01A2584Aac5aAE3faB1ed25F86c5269b32999"),
    );
    m.insert(
        "GMO Coin",
        address!("52b4567c37b48D51198B25CaA6e79e4fDa6D9734"),
    );
    m.insert(
        "GMO Coin_1",
        address!("85C0d1110C57a329ccD6Cc91e50fB4666F2287c3"),
    );
    m.insert(
        "GMO Coin_2",
        address!("86E284421664840Cb65C5b918Da59c01ED8fA666"),
    );
    m.insert(
        "GMO Coin_3",
        address!("976b251AB62371d6d154EF5989F5fA11452fB260"),
    );
    m.insert(
        "GMO Coin_4",
        address!("9C4fA4Ee466dfF080ADcCb12D39b99e83cdb1077"),
    );
    m.insert(
        "GMO Coin_5",
        address!("a8EBa6dA3801489093C0299529AC79fC16C59b6d"),
    );
    m.insert(
        "GMO Coin_6",
        address!("b77c64A1fd89d57e0f26e1a5c0a4d5934aD84350"),
    );
    m.insert(
        "GMO Coin_7",
        address!("c39BDF685F289B1F261EE9b0b1B2Bf9eae4C1980"),
    );
    m.insert(
        "GMO Coin_8",
        address!("e89943Ec20d856F064A87349a00Ef6aB00AED042"),
    );
    m.insert(
        "GMO Coin_9",
        address!("E978d95B437D75826ABa2ED1A0BDb534f173E28c"),
    );
    m.insert(
        "GOPAX",
        address!("2331f22f1701D79bf102a66672AB4293dB1bF72F"),
    );
    m.insert(
        "GOPAX_1",
        address!("6707A6763c6Dc64DE7C4048A27b6303292F88F50"),
    );
    m.insert(
        "GOPAX_2",
        address!("84c609976C73C38cDd36895cc998E1ac95734e4e"),
    );
    m.insert(
        "GOPAX_3",
        address!("e3031C1BfaA7825813c562CbDCC69d96FCad2087"),
    );
    m.insert(
        "Galaxy Digital",
        address!("15abb66bA754F05cBC0165A64A11cDed1543dE48"),
    );
    m.insert(
        "Galaxy Digital_1",
        address!("220eaDF10315dCB744Cb4C12d3d8d5F3d1028336"),
    );
    m.insert(
        "Galaxy Digital_2",
        address!("33566c9D8BE6Cf0B23795E0d380E112Be9d75836"),
    );
    m.insert(
        "Galaxy Digital_3",
        address!("46f34C24A7bA7a2Ac6DD76c3F09B32D41C144d08"),
    );
    m.insert(
        "Galaxy Digital_4",
        address!("5797F722b1FeE36e3D2c3481D938d1372bCD99A7"),
    );
    m.insert(
        "Galaxy Digital_5",
        address!("6AE55181F90c954993789546956A8453E63B0015"),
    );
    m.insert(
        "Galaxy Digital_6",
        address!("b6949a1A9C335cE1b73D490b1e134086Ec5718F5"),
    );
    m.insert(
        "Galaxy Digital_7",
        address!("b9C3832e688736ed8c8954d158cB6b00FCa6C8F2"),
    );
    m.insert(
        "Galaxy Digital_8",
        address!("caF66b3C3c343064F19c697E53B126cBb9E7Ad99"),
    );
    m.insert(
        "Galaxy Digital_9",
        address!("F4561C710ba450Aab302Ffc3557EE59Bbce94CA6"),
    );
    m.insert(
        "Gamdom",
        address!("d5FBDa4C79F38920159fE5f22DF9655FDe292d47"),
    );
    m.insert(
        "Graviex",
        address!("C1a012E58aD5a229E4a7051e07Bc5bdF2EcB91a2"),
    );
    m.insert(
        "HashKey Exchange",
        address!("ffe15FF598e719d29DFe5E1d60BE1A5521A779Ae"),
    );
    m.insert(
        "HitBTC",
        address!("0113a6b755fBaD36B4249Fd63002e2035E401143"),
    );
    m.insert(
        "HitBTC_1",
        address!("04EcC5ba3258840725FD808BE0fAE3430B8346F2"),
    );
    m.insert(
        "HitBTC_10",
        address!("1aD9461fda9C678ea768B9161E9EFe26b9965f64"),
    );
    m.insert(
        "HitBTC_11",
        address!("1af06e390EC67cf9B14cc092558279AED0Dd923A"),
    );
    m.insert(
        "HitBTC_12",
        address!("1F07D0C207bBba4B7745C3604215b0262e56c6A0"),
    );
    m.insert(
        "HitBTC_13",
        address!("24e54449c11aAe288033b12746CF552071d449e8"),
    );
    m.insert(
        "HitBTC_14",
        address!("265a9cb831EAa7FdD7cd2FF80aFEc4eA79E2Ac7A"),
    );
    m.insert(
        "HitBTC_15",
        address!("300615A4AFDBefDC79819D7434defD18a16463C7"),
    );
    m.insert(
        "HitBTC_16",
        address!("303C8F04773A7Ef6608D9E6c7dFb497D0A0B0A9D"),
    );
    m.insert(
        "HitBTC_17",
        address!("379E981f8E51628143bDbaF5464F9436b937D321"),
    );
    m.insert(
        "HitBTC_18",
        address!("37d3e950dc5cEb6309cbc5Bfa456cAAEDd179Dd4"),
    );
    m.insert(
        "HitBTC_19",
        address!("3a7d3565DE0038E0AEEe32c02D47d5d8b599E87A"),
    );
    m.insert(
        "HitBTC_2",
        address!("05EAC110625CB1715988b1757A94CB0f0548cF05"),
    );
    m.insert(
        "HitBTC_20",
        address!("3E3B18E821dB7B6d44ea445730847eb18fF8135F"),
    );
    m.insert(
        "HitBTC_21",
        address!("422026c9fE5DD65Dc259Ef1140735bDd952CC2eB"),
    );
    m.insert(
        "HitBTC_22",
        address!("469A0EFadF0A5404859f269A97459d6A48cB444a"),
    );
    m.insert(
        "HitBTC_23",
        address!("487628D81Ce77fA8CE5aebdeF6eFF7C940ADD15B"),
    );
    m.insert(
        "HitBTC_24",
        address!("4C34aE54Dc716808e94Af3d1d638b8EA3A23fA9B"),
    );
    m.insert(
        "HitBTC_25",
        address!("4FE6C1a64E7fBFf55a6517eF85489CA187A91BBF"),
    );
    m.insert(
        "HitBTC_26",
        address!("5255eCdC334c3D3f4D0DeB0cB5657b64881db46F"),
    );
    m.insert(
        "HitBTC_27",
        address!("5871FCC9af1016185F1f6732C518c73f4821C16F"),
    );
    m.insert(
        "HitBTC_28",
        address!("59a5208B32e627891C389EbafC644145224006E8"),
    );
    m.insert(
        "HitBTC_29",
        address!("5B7934cdBb5cD076bd486e0f017AeB777bf0D04c"),
    );
    m.insert(
        "HitBTC_3",
        address!("085489C3A85c51d5a443cFBDeaf9F13dda961CE0"),
    );
    m.insert(
        "HitBTC_30",
        address!("644bC076f4A098445971E75bE8fb887d7EC1C6Fb"),
    );
    m.insert(
        "HitBTC_31",
        address!("6543Ea3d1eEAEF316eF1d5EBef30a6a0B8f5dd76"),
    );
    m.insert(
        "HitBTC_32",
        address!("65E2C5175e2E618F48E70343b14C31B280E42d90"),
    );
    m.insert(
        "HitBTC_33",
        address!("688C09e6Bb9bE28ae7aE3bfEebC0A65d532B3305"),
    );
    m.insert(
        "HitBTC_34",
        address!("6C18d587D762A347d025A4Bd027d9f9cBa2E0736"),
    );
    m.insert(
        "HitBTC_35",
        address!("6E9470664abc929C1bb98f7860cEf82E2aB7b678"),
    );
    m.insert(
        "HitBTC_36",
        address!("6EB53062ef576dF1c4EB5CA326B866590571bcbd"),
    );
    m.insert(
        "HitBTC_37",
        address!("6F9E3de5F5cFc53a0ceC58D5CEc8C9abb99a4F78"),
    );
    m.insert(
        "HitBTC_38",
        address!("6Fd591EAEE4AAf90ACa728F3b02799b99b7fD18e"),
    );
    m.insert(
        "HitBTC_39",
        address!("7516952EA89721cF97990cF232AbbBd4C6A922f7"),
    );
    m.insert(
        "HitBTC_4",
        address!("09EB27139BbFe4B97458Cb85875E94A3a4D74a2e"),
    );
    m.insert(
        "HitBTC_40",
        address!("76d0f5dDB248F03d7d38444E15CEe5161Fa66aDF"),
    );
    m.insert(
        "HitBTC_41",
        address!("77D0c7F4eBCBB1e1D27036C7a9725391ab3cE8D7"),
    );
    m.insert(
        "HitBTC_42",
        address!("78C1B2fC2484cD112CC980c3CB2F12c8e30B4c94"),
    );
    m.insert(
        "HitBTC_43",
        address!("79bE4dCe73B64667BA06c05d011fABBc9FD240a8"),
    );
    m.insert(
        "HitBTC_44",
        address!("7b8067d5CA62A9BA370360F72C951e821669Ba0a"),
    );
    m.insert(
        "HitBTC_45",
        address!("7e02AB6a31580e0320266cD53e9E5C49dF0B2Fdd"),
    );
    m.insert(
        "HitBTC_46",
        address!("85cC44702763ADe958bffD6F3257Fa7cF2378d75"),
    );
    m.insert(
        "HitBTC_47",
        address!("8c2C0235c9838a48Be4585304D1aedDcc2eaBc85"),
    );
    m.insert(
        "HitBTC_48",
        address!("9089C54813a793AC6c911bf13e9909b6072790D4"),
    );
    m.insert(
        "HitBTC_49",
        address!("93bc5582bC721F30f8279d03346A9e708ddB0672"),
    );
    m.insert(
        "HitBTC_5",
        address!("0DC1DF88B06a437dD22C5CE0466ccBFe1a4C12A4"),
    );
    m.insert(
        "HitBTC_50",
        address!("9431003113aeC4372d4FdfbA08845f3939a1F8B2"),
    );
    m.insert(
        "HitBTC_51",
        address!("97952751dc220aa755CA1B7bDa9F0d3D8DeF660D"),
    );
    m.insert(
        "HitBTC_52",
        address!("989D7a06345F89662De142b6F9Eeb1f8c8022Af4"),
    );
    m.insert(
        "HitBTC_53",
        address!("9C67e141C0472115AA1b98BD0088418Be68fD249"),
    );
    m.insert(
        "HitBTC_54",
        address!("9CB95Ee6A4FbA2c8898F32254a26Cea74Ad7E26F"),
    );
    m.insert(
        "HitBTC_55",
        address!("A12431D0B9dB640034b0CDFcEEF9CCe161e62be4"),
    );
    m.insert(
        "HitBTC_56",
        address!("A8220Ce798eb9a75883189FBDc9a8cF633aD1795"),
    );
    m.insert(
        "HitBTC_57",
        address!("aD68942A95fDd56594aA5cF862B358790E37834C"),
    );
    m.insert(
        "HitBTC_58",
        address!("b02A5c8C7844bf6ce02ad55a95645B03e6bD5c9e"),
    );
    m.insert(
        "HitBTC_59",
        address!("BFCd86e36D947A9103A7D4a95d178A432723d6aD"),
    );
    m.insert(
        "HitBTC_6",
        address!("1366A2ca67594fFD5174d0216d60D9Ea8DEB511F"),
    );
    m.insert(
        "HitBTC_60",
        address!("cb4a847dB63faFA275e4f53114f4DdF09e600b65"),
    );
    m.insert(
        "HitBTC_61",
        address!("D690a9F0909acb7fBeE8F7915137000D2ff851eF"),
    );
    m.insert(
        "HitBTC_62",
        address!("DCA8B1E595708fB891009C541BAFFD376BAFbB92"),
    );
    m.insert(
        "HitBTC_63",
        address!("dee41DE15B86367d39A8BBd021bF6d527630fa74"),
    );
    m.insert(
        "HitBTC_64",
        address!("E18cE1Fce73e6b8343b616C9ca0Bd50727d9b357"),
    );
    m.insert(
        "HitBTC_65",
        address!("e5cd51c7523D78C14B004e50526d6D532F56Ba77"),
    );
    m.insert(
        "HitBTC_66",
        address!("e6e7b66daB1A20A548fA100f4773A5904Bb69158"),
    );
    m.insert(
        "HitBTC_67",
        address!("f07232Bc85D995c32C1EDf1C985C84A8B7b0DEd7"),
    );
    m.insert(
        "HitBTC_68",
        address!("F6Ab4b2EF674911C3cBf2197334C72C57e070C33"),
    );
    m.insert(
        "HitBTC_69",
        address!("F6b63e252ffB19B676410c966d23442e79709210"),
    );
    m.insert(
        "HitBTC_7",
        address!("1484cAdeE1A93E04D02cab154BB84Bc767270936"),
    );
    m.insert(
        "HitBTC_70",
        address!("FBA17B00768d9109045433AEE66c70E5215F0B75"),
    );
    m.insert(
        "HitBTC_71",
        address!("FDeda15e2922C5ed41fc1fdF36DA2FB2623666b3"),
    );
    m.insert(
        "HitBTC_8",
        address!("15c6A9291e296E5C91727D9C01c55337c6757a55"),
    );
    m.insert(
        "HitBTC_9",
        address!("17be334446EEef539BE497c297Eb902BE030D227"),
    );
    m.insert(
        "Hoo.com",
        address!("0000Bf7f4e4B7fb2315fc6D5d0F8854c91dfF1d8"),
    );
    m.insert(
        "Hoo.com_1",
        address!("008932bE50098089C6a075D35F4B5182EE549F8A"),
    );
    m.insert(
        "Hoo.com_10",
        address!("980a4732c8855Ffc8112E6746bd62095b4c2228F"),
    );
    m.insert(
        "Hoo.com_11",
        address!("993b7fcba51d8F75C2DfAeC0d17B6649ee0C9068"),
    );
    m.insert(
        "Hoo.com_12",
        address!("D0Ec209AD2134899148bEc8aEF905A6E9997456A"),
    );
    m.insert(
        "Hoo.com_13",
        address!("Ec293b9C56f06C8f71392269313D7e2Da681D9aC"),
    );
    m.insert(
        "Hoo.com_14",
        address!("eed85Ddd6828B8C714141d34A12D0c07F7553182"),
    );
    m.insert(
        "Hoo.com_2",
        address!("0093e5f2A850268c0ca3093c7EA53731296487eB"),
    );
    m.insert(
        "Hoo.com_3",
        address!("00D46709171b0796c90024f55CAcfc92acbacdA7"),
    );
    m.insert(
        "Hoo.com_4",
        address!("00DdB997C6b6015e187E9A242FF081df23C2DAC8"),
    );
    m.insert(
        "Hoo.com_5",
        address!("05f0fDD0E49A5225011fff92aD85cC68e1D1F08e"),
    );
    m.insert(
        "Hoo.com_6",
        address!("21169023f11B38EF9C95BA3aABd295be8e5a96Ec"),
    );
    m.insert(
        "Hoo.com_7",
        address!("4d4FfB448194504242267585F0Ea6f9de6A96DE3"),
    );
    m.insert(
        "Hoo.com_8",
        address!("74a73578e56b43aa9c088e013C5B7Bd91aA5221a"),
    );
    m.insert(
        "Hoo.com_9",
        address!("9760862d09B70433a91Ff27cBD069f51ef1cbd5c"),
    );
    m.insert(
        "Hotbit",
        address!("274F3c32C90517975e29Dfc209a23f315c1e5Fc7"),
    );
    m.insert(
        "Hotbit_1",
        address!("39aa02D6B499a76A70faB5F164e8Be587C366141"),
    );
    m.insert(
        "Hotbit_10",
        address!("8533A0bd9310Eb63E7CC8E1116c18a3D67B1976A"),
    );
    m.insert(
        "Hotbit_11",
        address!("A986266B60390E1d313Ecd3B2311A9aF74c57400"),
    );
    m.insert(
        "Hotbit_12",
        address!("B18fbFe3d34FdC227eB4508cdE437412B6233121"),
    );
    m.insert(
        "Hotbit_13",
        address!("B34eD85bc0B9DA2fA3C5e5d2f4B24f8EE96CE4E9"),
    );
    m.insert(
        "Hotbit_14",
        address!("b4E95e70015eeb04a23e0FCB6BAE06835310B356"),
    );
    m.insert(
        "Hotbit_15",
        address!("c62A0781934744E05927ceABB94a3043CdCfEA89"),
    );
    m.insert(
        "Hotbit_16",
        address!("C7029E939075F48fa2D5953381660c7d01570171"),
    );
    m.insert(
        "Hotbit_17",
        address!("d690a9DfD7e4B02898Cdd1a9E50eD1fd7D3d3442"),
    );
    m.insert(
        "Hotbit_18",
        address!("fa6cf22527d88270eEa37F45aF1808AdBF3C1B17"),
    );
    m.insert(
        "Hotbit_2",
        address!("4A41D76B0524a3989998380c033f12bfEb5f7201"),
    );
    m.insert(
        "Hotbit_3",
        address!("4b81c7Ff6912856AFBb40ACb32084A41F019B433"),
    );
    m.insert(
        "Hotbit_4",
        address!("4E568bffd661031993d530456d85Fb37DA2C30AE"),
    );
    m.insert(
        "Hotbit_5",
        address!("562680a4dC50ed2f14d75BF31f494cfE0b8D10a1"),
    );
    m.insert(
        "Hotbit_6",
        address!("5Ecddc1AC099074ae965D140A7c62bd71B7Fc80a"),
    );
    m.insert(
        "Hotbit_7",
        address!("66F7Bbf25c07e5D407A80010B0e9Ba96bF5A2A3E"),
    );
    m.insert(
        "Hotbit_8",
        address!("6C2e8d4F73f6A129843d1b3D2ACAFF1DB22E3366"),
    );
    m.insert(
        "Hotbit_9",
        address!("768F2A7CcdFDe9eBDFd5Cea8B635dd590Cb3A3F1"),
    );
    m.insert("IDAX", address!("3c11c3025ce387D76C2eDDf1493eC55a8cC2A0f7"));
    m.insert(
        "IDAX_1",
        address!("408af76966D49e57D79aAA6bD55f5162476B051B"),
    );
    m.insert(
        "IDAX_2",
        address!("6E743CdC08F22578f5FE99779aa2EdC019363483"),
    );
    m.insert(
        "IDAX_3",
        address!("BD6d79F3f02584cfcB754437Ac6776c4C6E0a0eC"),
    );
    m.insert("IDEX", address!("8af97264482B59c7AA11010907710DEe6d8D8c6C"));
    m.insert(
        "IDEX_1",
        address!("9A79994Bf5b887FF7e909498c14DD896c62FF891"),
    );
    m.insert(
        "IDEX_2",
        address!("A7a7899d944fE658c4B0a1803BAB2F490bd3849e"),
    );
    m.insert(
        "Iconomi",
        address!("154Af3E01eC56Bc55fD585622E33E3dfb8a248d8"),
    );
    m.insert(
        "Iconomi_1",
        address!("4F003663aB7C6CcAD2B14688B1d1f8332763F0a9"),
    );
    m.insert(
        "IndoEx LTD",
        address!("B081886Da8d77Fff190dCA39528ce1ceA6151096"),
    );
    m.insert(
        "IndoEx LTD_1",
        address!("B1A34309AF7f29B4195a6b589737f86E14597DdC"),
    );
    m.insert(
        "IndoEx LTD_2",
        address!("C43F9c2f2B61f3dEC87E6c3F0f33C150A6526Ace"),
    );
    m.insert(
        "IndoEx LTD_3",
        address!("Da793AC155a2aF190015F7393B2A36AFC94B7EcF"),
    );
    m.insert(
        "Indodax",
        address!("3C02290922a3618A4646E3BbCa65853eA45FE7C6"),
    );
    m.insert(
        "Indodax_1",
        address!("51836A753E344257B361519E948ffCAF5fb8d521"),
    );
    m.insert(
        "Indodax_2",
        address!("8Ab399CBB9FDB9a36518a7e7EddF89158E56c595"),
    );
    m.insert(
        "Indodax_3",
        address!("9554EFa1669014C25070BC23C2dF262825704228"),
    );
    m.insert(
        "Indodax_4",
        address!("9CbADD5Ce7E14742F70414A6DcbD4e7bB8712719"),
    );
    m.insert("JPEX", address!("0810Cc2d46fC76355E33E10f19D3F04d03A11ece"));
    m.insert(
        "JPEX_1",
        address!("50c85E5587d5611cf5cDFBa23640BC18b3571665"),
    );
    m.insert(
        "JPEX_2",
        address!("9528043B8Fc2a68380F1583C389a94dcd50d085e"),
    );
    m.insert(
        "JPEX_3",
        address!("a72Ad701807e5902F458e1844D560128F3F57750"),
    );
    m.insert(
        "JPEX_4",
        address!("D2f41167a391014Bc5df2234BE7f3a64a06a9fD7"),
    );
    m.insert("Juno", address!("88880809D6345119cCABe8A9015e4b1309456990"));
    m.insert(
        "Kanga Exchange",
        address!("42D17b7f3532Ec2f7C4E4e5E239BAA476846E2CD"),
    );
    m.insert(
        "KickEX",
        address!("352BDaBe484499e4c25c3536CC3edA1edbC5ad29"),
    );
    m.insert(
        "KickEX_1",
        address!("3bb14B49Da0c47a0B2a2a112BEde35655C39a032"),
    );
    m.insert(
        "KickEX_2",
        address!("aF4ff15C9809e246111802f04A6acC7160992feF"),
    );
    m.insert(
        "KickEX_3",
        address!("C153121042832Ac11587eBE361b8dc3cCd90e9E4"),
    );
    m.insert(
        "Klever",
        address!("4a5F98e2C2784d359FC0deCc8533Ae27AF0e5974"),
    );
    m.insert(
        "Klever_1",
        address!("5a57cfAFE8b9E94419Cc7d0Cb1F4a95C73f40110"),
    );
    m.insert(
        "Klever_2",
        address!("5AF8da2675DD31BEffa2619145957B15E8013f37"),
    );
    m.insert(
        "Klever_3",
        address!("91af50AdB57283283C8B442622e95c26D46D911c"),
    );
    m.insert(
        "Klever_4",
        address!("96c38EEeD002d3Df2e369DefFe6cc84688eAdb01"),
    );
    m.insert(
        "Korbit",
        address!("0c01089AEdc45Ab0F43467CCeCA6B4d3E4170bEa"),
    );
    m.insert(
        "Korbit_1",
        address!("223674Cc4433A50CAddF13c65F92151d75996E41"),
    );
    m.insert(
        "Korbit_10",
        address!("d17e26529E5fca53901f65F1A914317877CAB08a"),
    );
    m.insert(
        "Korbit_11",
        address!("d6e0F7dA4480b3AD7A2C8b31bc5a19325355CA15"),
    );
    m.insert(
        "Korbit_12",
        address!("D77f7f8868F20835FdFc8c7E851f6f23cE9F651D"),
    );
    m.insert(
        "Korbit_13",
        address!("e5d7CcC5fc3b3216C4DFF3a59442F1d83038468C"),
    );
    m.insert(
        "Korbit_14",
        address!("E83a48CaE4d7120e8bA1C2E0409568fFBA532E87"),
    );
    m.insert(
        "Korbit_15",
        address!("f0bc8FdDB1F358cEf470D63F96aE65B1D7914953"),
    );
    m.insert(
        "Korbit_16",
        address!("f6230e7E98D2BBeBF96d14888020E9c3E8c27d69"),
    );
    m.insert(
        "Korbit_2",
        address!("2864DE013415B6c2C7A96333183B20f0F9cC7532"),
    );
    m.insert(
        "Korbit_3",
        address!("33DE12b5Cf336692c6b7cfD3aAF779425067f21c"),
    );
    m.insert(
        "Korbit_4",
        address!("3A70c2528265F0624E7ac8495B212A367b5b61b2"),
    );
    m.insert(
        "Korbit_5",
        address!("455aE9F6d25fD49ED7bE9F3D8Dd1Cf58a79a5958"),
    );
    m.insert(
        "Korbit_6",
        address!("5BD811987Ee931Ac85ed0eDA8871282F9c5C88A4"),
    );
    m.insert(
        "Korbit_7",
        address!("8550E644D74536f1DF38B17D5F69aa1BFe28aE86"),
    );
    m.insert(
        "Korbit_8",
        address!("8E2040aB7A6af6BBA67e6d9b280c6feA7F930C87"),
    );
    m.insert(
        "Korbit_9",
        address!("D03be958e6b8da2D28aC8231a2291d6E4f0a7ea7"),
    );
    m.insert(
        "Kryptono",
        address!("30B71D015F60e2f959743038cE0aAeC9b4C1ea44"),
    );
    m.insert(
        "Kryptono_1",
        address!("41C00d79005a78C23D8CF73075F739f12f03bbAA"),
    );
    m.insert(
        "Kryptono_2",
        address!("629a7144235259336ea2694167F3C8b856EDD7dC"),
    );
    m.insert(
        "Kryptono_3",
        address!("E8A0E282e6a3E8023465acCd47FaE39dD5Db010b"),
    );
    m.insert("Kuna", address!("04196627190fF624492427317D853deaa270F9d2"));
    m.insert(
        "Kuna_1",
        address!("77aB999d1e9F152156B4411E1f3E2A42Dab8CD6D"),
    );
    m.insert(
        "Kuna_2",
        address!("9030a104a49141459F4B419BD6f56E4bA6fcd800"),
    );
    m.insert(
        "Kuna_3",
        address!("EA81CE54A0AfA10A027f65503bd52FBa83d745b8"),
    );
    m.insert(
        "Kuna_4",
        address!("FAc0f29459896ED8550e002344734989d958DE01"),
    );
    m.insert(
        "LAToken",
        address!("00343217B01188388C0E3242278231Ace35E1b61"),
    );
    m.insert(
        "LAToken_1",
        address!("0861Fca546225fbF8806986D211C8398f7457734"),
    );
    m.insert(
        "LAToken_10",
        address!("44C19a079B869A3E74942dF2CfE0152f834A458D"),
    );
    m.insert(
        "LAToken_11",
        address!("6fb194fc9806fE320E0CBD658e31F13B1bAa3925"),
    );
    m.insert(
        "LAToken_12",
        address!("7891b20C690605F4E370d6944C8A5DBfAc5a451c"),
    );
    m.insert(
        "LAToken_13",
        address!("7D8a212940933114DaC826EfBA2f673dc66310D0"),
    );
    m.insert(
        "LAToken_14",
        address!("8D056D457a52c4dAF71CEf45F540a040c143Ea05"),
    );
    m.insert(
        "LAToken_15",
        address!("9480D1cc3fd4cb7936D114f7d63124107870A7b8"),
    );
    m.insert(
        "LAToken_16",
        address!("9976c40e8186a5E0C2a9D50d55b51F905d10ce52"),
    );
    m.insert(
        "LAToken_17",
        address!("A1a0538D556B3E77f7E1340E3Ebd70C649c4bb84"),
    );
    m.insert(
        "LAToken_18",
        address!("A614180C69aBF82f3E7AAbB53AD9976EC90aeAC6"),
    );
    m.insert(
        "LAToken_19",
        address!("BA6C98f1cc6869ECCbeB892b7A603F8F02Db3b29"),
    );
    m.insert(
        "LAToken_2",
        address!("0F307b17d41acE555620DF5a55Dd5A01637e3b42"),
    );
    m.insert(
        "LAToken_20",
        address!("c00EEbe4E2bE29679781fc5fC350057eE8132BaB"),
    );
    m.insert(
        "LAToken_21",
        address!("CE55977E7B33E4e5534Bd370eE31504Fc7Ac9ADc"),
    );
    m.insert(
        "LAToken_22",
        address!("d76D939B455743e96adbCdf800627b11F3446780"),
    );
    m.insert(
        "LAToken_23",
        address!("E69963CE13ED742639C8287913682bC008B3e622"),
    );
    m.insert(
        "LAToken_24",
        address!("ecabeA0fB22f82F3A5a5D6043D7cCf65F3640c85"),
    );
    m.insert(
        "LAToken_25",
        address!("eD8D8f4Ff53915D80987BCD51C2DE582a05b2322"),
    );
    m.insert(
        "LAToken_26",
        address!("eE61F5fB0dB81d3A09392375Ee96f723C0620E07"),
    );
    m.insert(
        "LAToken_27",
        address!("EeC02a6D1a7F9f534b9609c8EE30B9cF9A7fe1B3"),
    );
    m.insert(
        "LAToken_28",
        address!("EFf6E17Fdc68d56812DA40f7d05FC8cDfd212440"),
    );
    m.insert(
        "LAToken_3",
        address!("1771C9c8d5AF830d322c2E1D2161D002844679EF"),
    );
    m.insert(
        "LAToken_4",
        address!("1B6C1A0e20aF81b922Cb454c3E52408496eE7201"),
    );
    m.insert(
        "LAToken_5",
        address!("235e8ceD6b42eE6E226837EB551E86D810d49f22"),
    );
    m.insert(
        "LAToken_6",
        address!("26b52C889FCf3B8f449aD1c0F07b8572E6ACE262"),
    );
    m.insert(
        "LAToken_7",
        address!("2790E66986d4f701443b3F053d7d9ebFa69c990e"),
    );
    m.insert(
        "LAToken_8",
        address!("3b28358e9CDde80A24f0f811daD13aB9fc2A0d2A"),
    );
    m.insert(
        "LAToken_9",
        address!("4114d8D509503592175A8E044594b29EC081dbe0"),
    );
    m.insert(
        "LBank",
        address!("0E80Abb31aCE45ae26F9A2943Acaab72E77DE9bE"),
    );
    m.insert(
        "LBank_1",
        address!("11B1e83818028a8F44EF84613D5c058734b2d5CF"),
    );
    m.insert(
        "LBank_10",
        address!("a690621b1D2F8cc06dE2De11e4cf69935F82655A"),
    );
    m.insert(
        "LBank_11",
        address!("b0E5Ec2A0BB8b8f3a727787f90b959611e4062b7"),
    );
    m.insert(
        "LBank_12",
        address!("C06e0513A150a021104FdCDD20Ce362fA593Ba1F"),
    );
    m.insert(
        "LBank_13",
        address!("e0F0aA98b4A4d305Ac4A04D830C96A158BdA9cd8"),
    );
    m.insert(
        "LBank_14",
        address!("E5a8B35eaa3c14E0a6514F800825e1e6687bF23E"),
    );
    m.insert(
        "LBank_15",
        address!("Ea48643446540fe29A909Ad4aa6Df182c8b7E997"),
    );
    m.insert(
        "LBank_16",
        address!("EC174c25bba7359b56Fd672c8899664121E7dBCF"),
    );
    m.insert(
        "LBank_17",
        address!("fa9f7a1cBfBCB688729c522b4F0905CcF4d26D25"),
    );
    m.insert(
        "LBank_2",
        address!("120051a72966950B8ce12eB5496B5D1eEEC1541B"),
    );
    m.insert(
        "LBank_3",
        address!("124D9BF2fecBc16b54eC4AcCdB14D44C2144f012"),
    );
    m.insert(
        "LBank_4",
        address!("22f83e4b9cB95CB99B88E8f4f15ea598C74c2788"),
    );
    m.insert(
        "LBank_5",
        address!("25b63C988310DE90031B84975aE13a6015ca6a16"),
    );
    m.insert(
        "LBank_6",
        address!("356dC48d74F107cfBfd61790B0808CdA6a0D364f"),
    );
    m.insert(
        "LBank_7",
        address!("43cBDc908A0860Ae23fA06AA5B20dFEe43c196a6"),
    );
    m.insert(
        "LBank_8",
        address!("7fF77C761e5669d1696059A3b46Cda0aE293aAE3"),
    );
    m.insert(
        "LBank_9",
        address!("9c4Fe1C3D5975E5C5E493F24352969aa280B7CFc"),
    );
    m.insert("LCX", address!("2957eA6D4f06bC2BadFB2958c65fc7d1bE5461B1"));
    m.insert(
        "LCX_1",
        address!("4631018F63d5E31680FB53C11C9e1B11F1503e6f"),
    );
    m.insert(
        "LCX_2",
        address!("c0C704BFc375b3FC657Bc378b1cfee009194e2b3"),
    );
    m.insert(
        "LCX_3",
        address!("c90970DD648415756681163F69Eaeb7EB9c28A9C"),
    );
    m.insert(
        "Lemon Cash",
        address!("20bB82F2Db6FF52b42c60cE79cDE4C7094Ce133F"),
    );
    m.insert(
        "Liqui",
        address!("1BEDb2AA5d389b13980d7B7cA7B7266a95020781"),
    );
    m.insert(
        "Liqui_1",
        address!("3AE7F3679D63077B4ab30dC96af2DF72239fAaff"),
    );
    m.insert(
        "Liqui_2",
        address!("5E575279bf9f4acf0A130c186861454247394C06"),
    );
    m.insert(
        "Liqui_3",
        address!("8271B2E8CBe29396e9563229030c89679B9470db"),
    );
    m.insert(
        "Liqui_4",
        address!("D9Bd20EFcA7b0e6606b969548b1516C08D37374b"),
    );
    m.insert(
        "Liqui_5",
        address!("E17AF102ba3c45301Cca05953892e2eD357486e6"),
    );
    m.insert(
        "Liqui_6",
        address!("ea133a07880B99Ceb7A973e64DD3AC18f7A20888"),
    );
    m.insert(
        "Liquid",
        address!("07445065963c2D563Cd70Ddf2AA49fc771e59a98"),
    );
    m.insert(
        "Liquid_1",
        address!("3c0a69CBb7e0830f8fAB41F11F75100f886998a6"),
    );
    m.insert(
        "Liquid_10",
        address!("dF4B6Fb700C428476Bd3C02E6FA83e110741145b"),
    );
    m.insert(
        "Liquid_11",
        address!("E11eda4F7ee51bF6EFf7Cb3CA0F1Dc10809e01B1"),
    );
    m.insert(
        "Liquid_12",
        address!("edBB72E6b3Cf66a792bFF7FaaC5Ea769fe810517"),
    );
    m.insert(
        "Liquid_13",
        address!("eE0fB34631f0e6503c5fFB4A30F6FA345Cf1BA99"),
    );
    m.insert(
        "Liquid_2",
        address!("41D5233f434d98b73F22Ce664D48bE06F4eb073F"),
    );
    m.insert(
        "Liquid_3",
        address!("8e01eF6A4D864698Ff80B419fe9ec2e3C85e9E9e"),
    );
    m.insert(
        "Liquid_4",
        address!("9cC2dCe817093CEEa82bb67A4Cf43131fA354c06"),
    );
    m.insert(
        "Liquid_5",
        address!("9F571BB918e98B8DEb462F14C54a3E36Ad43627A"),
    );
    m.insert(
        "Liquid_6",
        address!("a80EE1D4E41dc43eE5BCc54BD2867B44A8e6A385"),
    );
    m.insert(
        "Liquid_7",
        address!("ccDfdc87341605200a3582aB52350fb4FB261AB0"),
    );
    m.insert(
        "Liquid_8",
        address!("Db2caD4f306B47C9b35541988c7656F1BB092e15"),
    );
    m.insert(
        "Liquid_9",
        address!("Db2E63058A01A3F304c9E337003f425889BA135F"),
    );
    m.insert(
        "LiteBit",
        address!("67655E8FCd903f75aceB82bB3f782687CE985d35"),
    );
    m.insert(
        "LiteBit_1",
        address!("9Afa066884cE723200438B672C0B8D5769E03A6A"),
    );
    m.insert(
        "Livecoin.net",
        address!("243BEc9256C9A3469DA22103891465B47583d9F1"),
    );
    m.insert(
        "LordToken",
        address!("76c674F9bcb5Eb01Ad64629D3f68895B60029305"),
    );
    m.insert("Luno", address!("05CdB1526F6e224e02919a4C018D9784Ea25eb3d"));
    m.insert(
        "Luno_1",
        address!("3a5cc8689D1b0cEf2c317bC5C0aD6Ce88B27D597"),
    );
    m.insert(
        "Luno_2",
        address!("416299AAde6443e6F6e8ab67126e65a7F606eeF5"),
    );
    m.insert(
        "Luno_3",
        address!("Af1931c20ee0c11BEA17A41BfBbAd299B2763bc0"),
    );
    m.insert(
        "MAX Exchange",
        address!("2E0279b98000182d1C286da4102AAcdbEe4c2d85"),
    );
    m.insert(
        "MAX Exchange_1",
        address!("A9BfF538A906154c80A8dBccd229F3DEddFa52D6"),
    );
    m.insert(
        "MAX Exchange_2",
        address!("c4EB040289e0d8A8F38184c52757E691C1D1d112"),
    );
    m.insert(
        "MAX Exchange_3",
        address!("cD32602db028cB3827f66cedc2c6d7c0B97A8b34"),
    );
    m.insert(
        "MaiCoin",
        address!("477b8D5eF7C2C42DB84deB555419cd817c336b6F"),
    );
    m.insert(
        "MaiCoin_1",
        address!("986AAC49D38CEA193e56DF1A114894f8f25333Ee"),
    );
    m.insert(
        "MaskEX",
        address!("09b1806Df13062B5f653BeDA6998972cabCF7009"),
    );
    m.insert(
        "MaskEX_1",
        address!("0B3c7bcE764E6f1B52443e30fcb4f34997A0674c"),
    );
    m.insert(
        "MaskEX_10",
        address!("3Dd878A95DCAEF2800cD57BB065B5e8f2F438131"),
    );
    m.insert(
        "MaskEX_11",
        address!("46c75Fc52E0263946f8F1a75A95C23a767D2f26e"),
    );
    m.insert(
        "MaskEX_12",
        address!("5f23b26C6D76F836aa99F174E71BCD89bdEe3226"),
    );
    m.insert(
        "MaskEX_13",
        address!("6DB133E840376555A5aD5c1D7616872EF57e7F13"),
    );
    m.insert(
        "MaskEX_14",
        address!("6e2673095545280F6F10E22eB861a555C6E94bEc"),
    );
    m.insert(
        "MaskEX_15",
        address!("6f531cf07F2D659DcfB371B1A7f4c0157A168332"),
    );
    m.insert(
        "MaskEX_16",
        address!("71467Da4c0b0db4e889DA703E6fF1cd740F1F74A"),
    );
    m.insert(
        "MaskEX_17",
        address!("7Ac724cAC6E4dDc24C102b1006f41bc8A6A5C1C5"),
    );
    m.insert(
        "MaskEX_18",
        address!("7E0616656934a09373b1E1114DE2c20A77513D16"),
    );
    m.insert(
        "MaskEX_19",
        address!("80B62F0ea7a89bbc4dF4C95e2ad363e5C153b80e"),
    );
    m.insert(
        "MaskEX_2",
        address!("0c78fD926A8fC9CFc682bDc6b411942D9C7EDb7a"),
    );
    m.insert(
        "MaskEX_20",
        address!("823C8E533657B0004B5Ab8553d84502ba2E571f7"),
    );
    m.insert(
        "MaskEX_21",
        address!("833F3B6fAa717079fb3A1030f6207C57B1c591Bd"),
    );
    m.insert(
        "MaskEX_22",
        address!("84457412efE8b3A05583cb496E1D2c03E6F36155"),
    );
    m.insert(
        "MaskEX_23",
        address!("8458c828D602230e92Eb0aAC5a6aed5580011B6A"),
    );
    m.insert(
        "MaskEX_24",
        address!("95ad8841376058a000F489196F05ecf176bEB8ac"),
    );
    m.insert(
        "MaskEX_25",
        address!("9F1bB5349d481065561A84CBD7F84982fD533359"),
    );
    m.insert(
        "MaskEX_26",
        address!("A310b3eecA53B9C115af529faF92Bb5ca4B41494"),
    );
    m.insert(
        "MaskEX_27",
        address!("a4E71851A8c8eaeFeb20A994159F4A443E46059b"),
    );
    m.insert(
        "MaskEX_28",
        address!("BE921EA3bd0c879a8688B7fabE6b3c8A471df90d"),
    );
    m.insert(
        "MaskEX_29",
        address!("c3eDBB9C181016Cef5d76491F835930e9C8c4D2C"),
    );
    m.insert(
        "MaskEX_3",
        address!("0cE7EeFB9f862aa0374EE7bbC4D8A0Fc2C651517"),
    );
    m.insert(
        "MaskEX_30",
        address!("C6aCB77BEFebfF0359CC581973859eeE8CbAEDa1"),
    );
    m.insert(
        "MaskEX_31",
        address!("c7570e308464c7838A3eEA1e5788D0d901cc6A80"),
    );
    m.insert(
        "MaskEX_32",
        address!("d666aD8D95903BcE9B4dcD2cacdE5145E36405c2"),
    );
    m.insert(
        "MaskEX_33",
        address!("D7aEd730A7c4cf8dFE313b16712AF3406f6Dca5b"),
    );
    m.insert(
        "MaskEX_34",
        address!("DCa6951B82e82AF6AAB4bB9e90CA00F5760370e1"),
    );
    m.insert(
        "MaskEX_35",
        address!("dD9c649Edb7fF80c6C9D238344260184A4f94b88"),
    );
    m.insert(
        "MaskEX_36",
        address!("FB65377800a7282CF81bAf0f335FBC6f8FF36776"),
    );
    m.insert(
        "MaskEX_4",
        address!("0cE92D3a15908b53371Ff1afCaE800F28142250c"),
    );
    m.insert(
        "MaskEX_5",
        address!("0FEABB61f67E859811aAfce83a5aB780f8C53c0a"),
    );
    m.insert(
        "MaskEX_6",
        address!("1349907C197731c5Ed98D8442309a15107cB6baD"),
    );
    m.insert(
        "MaskEX_7",
        address!("2161217d22FAC0188775432F8bA32F1D4272dD19"),
    );
    m.insert(
        "MaskEX_8",
        address!("32FFFc894503eBa55Af4c371fd50198e7356D780"),
    );
    m.insert(
        "MaskEX_9",
        address!("33fe5557E90a872A065f2acfD973847e33fC4532"),
    );
    m.insert(
        "Matrixport",
        address!("0090E10302a3dDeff920BA96a023423F306dc0a8"),
    );
    m.insert(
        "Matrixport_1",
        address!("0525eB58F9d226ADb659777791ddD994dEF56E1c"),
    );
    m.insert(
        "Matrixport_2",
        address!("Ab5A9FCb27e4F97E87a536E768b9cb49dC8B1A4F"),
    );
    m.insert(
        "Matrixport_3",
        address!("C5b8A59fdaAb89bdbf22Dbd906A03C1F48bc9eC8"),
    );
    m.insert(
        "Matrixport_4",
        address!("D00C56cd2cA9fFe295BfF960a14c65783dd87162"),
    );
    m.insert(
        "Mbcbit",
        address!("5e6aD578fe3a2DA7bBD0255f04179e1E77317D1a"),
    );
    m.insert(
        "Mercado Bitcoin",
        address!("8CE13C17B9C9caE9193538DC2a64ca7be07E2C00"),
    );
    m.insert(
        "Mercado Bitcoin_1",
        address!("b8bA36E591FAceE901FfD3d5D82dF491551AD7eF"),
    );
    m.insert(
        "Mercatox",
        address!("e03c23519e18D64F144d2800E30E81B0065C48B5"),
    );
    m.insert(
        "Mercuryo",
        address!("8C8D7C46219D9205f056f28fee5950aD564d7465"),
    );
    m.insert(
        "MinedTrade.com",
        address!("ac338d9fAaC562Df26d702880c796e1024E2698A"),
    );
    m.insert(
        "MoonPay",
        address!("0b5c4a7FcDA49e0a8661419Bb55B86161a86db2a"),
    );
    m.insert(
        "MoonPay_1",
        address!("1440ec793aE50fA046B95bFeCa5aF475b6003f9e"),
    );
    m.insert(
        "MoonPay_2",
        address!("151B381058f91cF871E7eA1eE83c45326F61e96D"),
    );
    m.insert(
        "MoonPay_3",
        address!("22F6CC8738308a8c92a6a71ea67832463d1Fec0d"),
    );
    m.insert(
        "MoonPay_4",
        address!("7AFC12C8DD2e6591581D95586eB2c2A4905a12a9"),
    );
    m.insert(
        "MoonPay_5",
        address!("8216874887415e2650D12D53Ff53516F04a74FD7"),
    );
    m.insert(
        "MoonPay_6",
        address!("B287eaC48aB21c5FB1d3723830d60b4c797555B0"),
    );
    m.insert(
        "MoonPay_7",
        address!("d108FD0E8c8E71552a167E7a44FF1d345D233BA6"),
    );
    m.insert(
        "MoonPay_8",
        address!("D42f958E1C3e2a10e5d66343c4c9a57726E5b4b6"),
    );
    m.insert(
        "MultiBank Group",
        address!("C21aE7Af41c4dfeEb7eCa07Df573288523076b60"),
    );
    m.insert("NDAX", address!("14c4017f81675F7aC1321218d4a385a2D3117328"));
    m.insert(
        "NDAX_1",
        address!("3B56258cFfD6ae5604A4906E55e79B9BFd3cdcfE"),
    );
    m.insert(
        "NEXBIT Pro",
        address!("ae7006588d03bd15d6954e3084A7e644596bC251"),
    );
    m.insert(
        "Netcoins",
        address!("404460039499c774c48248552F802CE5dd482e32"),
    );
    m.insert(
        "Netcoins_1",
        address!("ba20996529D2722c4D9D800A7b20E96a0A235336"),
    );
    m.insert(
        "Netcoins_2",
        address!("e3743D1bf4bdc86BF1FaAc0fFe325a23cF495Ad7"),
    );
    m.insert("Nexo", address!("0031e147A79c45f24319dc02ca860cB6142FCBA1"));
    m.insert(
        "Nexo_1",
        address!("00EE047A66d5cff27587A61559138c26b62F7CEb"),
    );
    m.insert(
        "Nexo_10",
        address!("57793E249825492212de2aA4306379017301e1Da"),
    );
    m.insert(
        "Nexo_11",
        address!("65b0BF8Ee4947edD2A500D74E50a3d757DC79de0"),
    );
    m.insert(
        "Nexo_12",
        address!("6914FC70fAC4caB20a8922E900C4BA57fEECf8E1"),
    );
    m.insert(
        "Nexo_13",
        address!("7344E478574aCBe6DaC9dE1077430139E17EEc3D"),
    );
    m.insert(
        "Nexo_14",
        address!("7AB6c736baf1DAc266aAb43884d82974a9ADCcCF"),
    );
    m.insert(
        "Nexo_15",
        address!("8Fd589AA8bfA402156a6D1ad323FEC0ECee50D9D"),
    );
    m.insert(
        "Nexo_16",
        address!("9bdB521a97E95177BF252C253E256A60C3e14447"),
    );
    m.insert(
        "Nexo_17",
        address!("A75EDE99F376Dd47f3993Bc77037F61b5737C6EA"),
    );
    m.insert(
        "Nexo_18",
        address!("B60C61DBb7456f024f9338c739B02Be68e3F545C"),
    );
    m.insert(
        "Nexo_19",
        address!("Ba90b5bc12DAAb8d06582967a22c86AE7eed0469"),
    );
    m.insert(
        "Nexo_2",
        address!("121EFFb8160f7206444f5a57d13c7A4424a237A4"),
    );
    m.insert(
        "Nexo_20",
        address!("e498E7a77a2ffBd33E4a14253C3d11F97AeBa18B"),
    );
    m.insert(
        "Nexo_21",
        address!("E6Fa688c09E196a8f9D08911dF84e3b5f350e507"),
    );
    m.insert(
        "Nexo_22",
        address!("eD212a4A2E82d5ee0D62F70B5deE2f5Ee0f10c5D"),
    );
    m.insert(
        "Nexo_23",
        address!("f36A47300F002c0C9F8c131962F077C3543B2fC6"),
    );
    m.insert(
        "Nexo_24",
        address!("Ffec0067F5a79CFf07527f63D83dD5462cCf8BA4"),
    );
    m.insert(
        "Nexo_3",
        address!("1D85f929EE6AEDc3b4981d8FE408Ae43942b2e53"),
    );
    m.insert(
        "Nexo_4",
        address!("31E9b3373F2AD5d964CAd0fd01332d6550cBBdE6"),
    );
    m.insert(
        "Nexo_5",
        address!("354e9Fa5c6Ee7e6092158a8c1B203CcAc932D66d"),
    );
    m.insert(
        "Nexo_6",
        address!("463E5b673d1029989c9B059d36393c539beF9094"),
    );
    m.insert(
        "Nexo_7",
        address!("4bb7f4c3d47C4b431cb0658F44287d52006fb506"),
    );
    m.insert(
        "Nexo_8",
        address!("4d7F1790644Af787933c9fF0e2cff9a9B4299Abb"),
    );
    m.insert(
        "Nexo_9",
        address!("55e4d16f9c3041EfF17Ca32850662f3e9Dddbce7"),
    );
    m.insert(
        "Nobitex",
        address!("09672F26Cc257F7B216710864c98aB1841453E4a"),
    );
    m.insert(
        "Nobitex_1",
        address!("598C60Bb7E929F02008A098461Fd2FAec3f74771"),
    );
    m.insert(
        "Nobitex_2",
        address!("641FB555527B9108a1F58eA24E0C04fF86C4Ed0d"),
    );
    m.insert(
        "Nobitex_3",
        address!("7eB6a79587DFd5Da426DFF27ff11da1F09c90A2B"),
    );
    m.insert(
        "Nobitex_4",
        address!("8D56f551b44a6dA6072a9608d63d664ce67681a5"),
    );
    m.insert(
        "Nobitex_5",
        address!("D16E4cdb153B2DCc617061174223a6D4BFaE53f5"),
    );
    m.insert(
        "Nobitex_6",
        address!("D5BcF75c0573B14818C42F0118067BE859131acE"),
    );
    m.insert(
        "Nobitex_7",
        address!("F639d88a89384A4D97f2bA9159567Ddb3890Ea07"),
    );
    m.insert(
        "Nominex",
        address!("10298Be5Abf74D111D133dc3493Dc4C6a9FD924b"),
    );
    m.insert(
        "Nominex_1",
        address!("26804231a528c894AB6790530b237449a817da6A"),
    );
    m.insert(
        "Nominex_10",
        address!("A937Eddfd12930F758788BcC936B4762BDE9d54C"),
    );
    m.insert(
        "Nominex_11",
        address!("ab2f4297E7e31638eBE8362471b3038018A106D8"),
    );
    m.insert(
        "Nominex_12",
        address!("DbF1B10FE3e05397Cd454163F6F1eD0c1181C3B3"),
    );
    m.insert(
        "Nominex_2",
        address!("2D8b192eAd2f402867323B072D143d44435EDd74"),
    );
    m.insert(
        "Nominex_3",
        address!("5cd67d65Ff07D5BE2488E51F1a8C69273D258338"),
    );
    m.insert(
        "Nominex_4",
        address!("63A81d936cb14fA3649A4D071608758cFFb3Bd94"),
    );
    m.insert(
        "Nominex_5",
        address!("8326E22a36486ae7D4B85e8DFA732527b962805c"),
    );
    m.insert(
        "Nominex_6",
        address!("857083580AeD7b5726860937EF030ED8072BC9aB"),
    );
    m.insert(
        "Nominex_7",
        address!("99b674Ba03E896D952983908DbA8D7b560FB10d5"),
    );
    m.insert(
        "Nominex_8",
        address!("9Cd2D1A3214c12BB6dbfA7DBc3B0641C26a2f9a6"),
    );
    m.insert(
        "Nominex_9",
        address!("A0F2C13e20A11e00acF4e7B47604b24ca8908797"),
    );
    m.insert(
        "Norwegian Block Exchange",
        address!("052Ed0aD68Ffc470386FDAb82F7046E0b55FD663"),
    );
    m.insert(
        "Norwegian Block Exchange_1",
        address!("0d019414aC7DD7E8262aE7Dc9EFCC6bDe050b0DD"),
    );
    m.insert(
        "Norwegian Block Exchange_2",
        address!("0d075CCD8FE8C5C8e28A308f66422Db3456Eb9cc"),
    );
    m.insert(
        "Norwegian Block Exchange_3",
        address!("29af949c3D218C1133bD16257ed029E92deFb168"),
    );
    m.insert(
        "Norwegian Block Exchange_4",
        address!("8Cad96fB23924Ebc37b8CdAFa8400AD856fE4a2C"),
    );
    m.insert(
        "Norwegian Block Exchange_5",
        address!("AeB81c391Ac427B6443310fF1cB73a21E071e5ad"),
    );
    m.insert(
        "Norwegian Block Exchange_6",
        address!("fACCB74832546a745aaB8Dbd2d155Dc67a222048"),
    );
    m.insert("OPNX", address!("134530E94d8c603Ca627c114D2d5B206D652e895"));
    m.insert(
        "OPNX_1",
        address!("2335919E01Fa45744815290Ee255dC0C066C47D3"),
    );
    m.insert(
        "OPNX_2",
        address!("610887f0AE329557d1aE067F6b7697fF07032116"),
    );
    m.insert(
        "OPNX_3",
        address!("E5241AC645cad844e94A2E7486283ED6398Fb3Aa"),
    );
    m.insert(
        "OTCBTC",
        address!("AeEc6f5aCA72F3A005af1B3420ab8c8c7009BaC8"),
    );
    m.insert(
        "OceanEx",
        address!("7454a609EA877f37FA6D42FEcA76a8f18DE841C7"),
    );
    m.insert(
        "OceanEx_1",
        address!("904a0600582eD1003016D143756A4F427D5BA344"),
    );
    m.insert(
        "OceanEx_2",
        address!("B385d810CC3Bf0f4A4629528967e18Cd0196E077"),
    );
    m.insert(
        "Omgfin",
        address!("03E3fF995863828554282e80870B489cc31dC8bc"),
    );
    m.insert(
        "Oobit",
        address!("0BC99404E0fCa2028D7665e3B473869C6C6FF002"),
    );
    m.insert(
        "Oobit_1",
        address!("65FB180C8bfAA811e9b2E3d2fa60EF27c15F8732"),
    );
    m.insert(
        "Oobit_2",
        address!("a8EE70F68fffAAE3cb8475112409ADB9282f3246"),
    );
    m.insert(
        "Oobit_3",
        address!("bF2EbF6C701068CCf046644315Ff7417C879bd8A"),
    );
    m.insert(
        "Oobit_4",
        address!("f6BE265fa72148DFb64106247d21BB15CE650E5e"),
    );
    m.insert(
        "Orionx",
        address!("aFDa8eBA0aC933661F45b41a438840dc07DF1761"),
    );
    m.insert(
        "Panda Exchange",
        address!("b709D82f0706476457ae6baD7C3534fBf424382c"),
    );
    m.insert(
        "Panda Exchange_1",
        address!("caCc694840eCeBaDD9B4c419E5B7f1D73FEdf999"),
    );
    m.insert(
        "Paribu",
        address!("04D9199D397Ed9b0C497ad9bbc10F0E0047DD3B7"),
    );
    m.insert(
        "Paribu_1",
        address!("2bB97B6CF6FfE53576032c11711D59Bd056830eE"),
    );
    m.insert(
        "Paribu_10",
        address!("EB91A3754E48Fcd8d3e6F89528D67Ac5386Cd0D0"),
    );
    m.insert(
        "Paribu_11",
        address!("FB90501083a3b6AF766c8dA35d3Dde01eB0d2a68"),
    );
    m.insert(
        "Paribu_2",
        address!("595063172C85B1e8AC2fe74Fcb6b7dC26844CC2D"),
    );
    m.insert(
        "Paribu_3",
        address!("64440d8A6E6F949536646C363A4b734819EeDbfd"),
    );
    m.insert(
        "Paribu_4",
        address!("9acbB72Cf67103A30333A32CD203459c6a9c3311"),
    );
    m.insert(
        "Paribu_5",
        address!("9bC5A1d65cb56288C0d110Ce2Da3D0aFB3F573cd"),
    );
    m.insert(
        "Paribu_6",
        address!("abc74170f3Cb8Ab352820C39cC1d1e05cE9e41D3"),
    );
    m.insert(
        "Paribu_7",
        address!("Bd8ef191Caa1571e8aD4619ae894e07A75De0C35"),
    );
    m.insert(
        "Paribu_8",
        address!("c6da94cA4CdBE74B777B53e42470F676c44B9ab6"),
    );
    m.insert(
        "Paribu_9",
        address!("C86562C1E08cac700F0dDb33EB5A148EC1227d62"),
    );
    m.insert(
        "Paxful",
        address!("17F1a51dA68d27c94D2a51d92B27B5Bd4718b986"),
    );
    m.insert(
        "Paxful_1",
        address!("777d4627E31863b2a49e2985AF46525F21a9846C"),
    );
    m.insert(
        "Paxful_2",
        address!("7A20527ba5a749b3b054a821950Bfcc2C01b959f"),
    );
    m.insert(
        "Paxos",
        address!("0C23fc0Ef06716D2f8ba19bC4bEd56D045581F2d"),
    );
    m.insert(
        "Paxos_1",
        address!("264bd8291fAE1D75DB2c5F573b07faA6715997B5"),
    );
    m.insert(
        "Paxos_2",
        address!("286AF5CF60aE834199949bBc815485f07CC9C644"),
    );
    m.insert(
        "Paxos_3",
        address!("41b309236C87b1bc6FA8Eb865833E44158Fa991a"),
    );
    m.insert(
        "Paxos_4",
        address!("5195427ca88DF768c298721dA791B93AD11ECa65"),
    );
    m.insert(
        "Paxos_5",
        address!("7d766B06e7164Be4196EE62E6036c9FCFF68107d"),
    );
    m.insert(
        "Paxos_6",
        address!("8a89E016750479Dc1d7Ad32ecfFCeCd76E118697"),
    );
    m.insert(
        "Paxos_7",
        address!("E25a329d385f77df5D4eD56265babe2b99A5436e"),
    );
    m.insert(
        "PayKassa.pro",
        address!("5D9fE07813a260857Cf60639daC710EBb9531a20"),
    );
    m.insert(
        "PayKassa.pro_1",
        address!("8b8a4abc707F16dA24B795e3e46ed22975A9D329"),
    );
    m.insert(
        "PayKassa.pro_2",
        address!("eCceFaB82bb383afC90b94C8d378DE314e62AC5D"),
    );
    m.insert(
        "Paybis",
        address!("D65E0Cbd31977B2E0e23c8330C8B5f020818Fc91"),
    );
    m.insert(
        "Peatio",
        address!("203304a42132928Fa77e2285cC05111693795328"),
    );
    m.insert(
        "Peatio_1",
        address!("468B858c964f297Fd8Fce058032BF4B4911A8ad8"),
    );
    m.insert(
        "Peatio_2",
        address!("4BCBe67E72F9324F5E9B3F3dB45Ab47768401227"),
    );
    m.insert(
        "Peatio_3",
        address!("7B8ce58fAb63B225f776417c903A1191f497D211"),
    );
    m.insert(
        "Peatio_4",
        address!("88e343F4599292C2CfFe683C1bb93cD3480BdbAb"),
    );
    m.insert(
        "Peatio_5",
        address!("A6c33332A253E0927b320401626999F6E534722a"),
    );
    m.insert(
        "Peatio_6",
        address!("D4Dcd2459BB78d7a645Aa7E196857D421b10D93F"),
    );
    m.insert(
        "Peatio_7",
        address!("f5EF8590b55dB151D559c3954e994A6c4DAF12Bb"),
    );
    m.insert(
        "Phemex",
        address!("2710EB8B7f29def9Fc856c87Aeb64d05d068DA3A"),
    );
    m.insert(
        "Phemex_1",
        address!("35D2d03607b9155b42CF673102FE58251AC4F644"),
    );
    m.insert(
        "Phemex_2",
        address!("50BE13b54f3EeBBe415d20250598D81280e56772"),
    );
    m.insert(
        "Phemex_3",
        address!("576318Ab7A36A40EBaE463a1604396167eC04B35"),
    );
    m.insert(
        "Phemex_4",
        address!("c7eC1A730f718Fb6931D10caEBd85742184ee359"),
    );
    m.insert(
        "Phemex_5",
        address!("f7D13C7dBec85ff86Ee815f6dCbb3DEDAc78ca49"),
    );
    m.insert(
        "Phemex_6",
        address!("fe065653cFf3154F44eD30EE876570E49051A6E8"),
    );
    m.insert(
        "Prime Trust",
        address!("33FE7Ad77394281e43Cc82D86ad0cbb5b9e9575D"),
    );
    m.insert(
        "Prime Trust_1",
        address!("352e0242a58c4F43dc40F3Ee9a2eA14CcC6Bb2ea"),
    );
    m.insert(
        "Prime Trust_2",
        address!("9416fd2bc773C85A65d699cA9fC9525F1424Df94"),
    );
    m.insert(
        "Prime Trust_3",
        address!("d8b81f6849dFbBe7B8f3c32bbB3A15aD2AdB6898"),
    );
    m.insert(
        "Prime Trust_4",
        address!("DDBB8d8B5Da3dFAf65D4F8FA846127ACD6A844B1"),
    );
    m.insert(
        "ProBit",
        address!("72E5263FF33D2494692D7F94A758aA9F82062F73"),
    );
    m.insert(
        "ProBit_1",
        address!("aD285fDEDFC0D5f944A33e478356524293c7eC68"),
    );
    m.insert(
        "ProBit_2",
        address!("dBA24f19Bce0F32ea4273FaeA7C01D7f9D4F91D6"),
    );
    m.insert(
        "ProBit_3",
        address!("F71AfE21Cd32959113Fc47aE2EF886B43A9413d5"),
    );
    m.insert(
        "QMall",
        address!("07e551E31A793E20dc18494ff6b03095A8F8Ee36"),
    );
    m.insert(
        "QMall_1",
        address!("0C0511d1eE844A516B6bDa54db3bcA01E2cE2A19"),
    );
    m.insert(
        "QMall_2",
        address!("5d636F90B48c9f14BD0BF9D8016d4cB0DD9e1D9f"),
    );
    m.insert(
        "QMall_3",
        address!("d3e5b815843C31f621f2253c836B34f84debFE29"),
    );
    m.insert(
        "QuadrigaCX",
        address!("027BEEFcBaD782faF69FAD12DeE97Ed894c68549"),
    );
    m.insert(
        "QuadrigaCX_1",
        address!("0EE4E2d09AEC35Bdf08083b649033Ac0A41aa75E"),
    );
    m.insert(
        "QuadrigaCX_2",
        address!("5B5B69f4E0add2Df5d2176D7dBd20B4897bc7eC4"),
    );
    m.insert(
        "QuadrigaCX_3",
        address!("B6AaC3b56FF818496B747EA57fCBe42A9aae6218"),
    );
    m.insert(
        "QuantaEx",
        address!("2a048d9A8fFDd239F063B09854976c3049AE659C"),
    );
    m.insert(
        "QuantaEx_1",
        address!("5cA39c42F4dEE3A5Ba8FEc3Ad4902157D48700bf"),
    );
    m.insert(
        "QuantaEx_2",
        address!("d344539efe31f8b6DE983A0Cab4Fb721fC69C547"),
    );
    m.insert(
        "Ramp Network",
        address!("8a37F0290AE85D08522d2A605617e76128Fd0712"),
    );
    m.insert(
        "Ramp Network_1",
        address!("98DB3a41bF8bF4DeD2C92A84ec0705689DdEEF8B"),
    );
    m.insert(
        "Remitano",
        address!("2819c144D5946404C0516B6f817a960dB37D4929"),
    );
    m.insert(
        "Remitano_1",
        address!("7982789Dd8b4D4a76783d5d43b94c55B75bdEe0B"),
    );
    m.insert(
        "Remitano_2",
        address!("7Ae17a0f6f8F02b5B6e76b327DB15F91306194E6"),
    );
    m.insert(
        "Remitano_3",
        address!("8365EFb25D0822AaF15Ee1D314147B6a7831C403"),
    );
    m.insert(
        "Remitano_4",
        address!("8b2f57d12AE055f26Fb643f9C4F64FfFe9F4c6a1"),
    );
    m.insert(
        "Remitano_5",
        address!("ac180b9Be764Fb542Cac26D6A2D227fa8E7792EA"),
    );
    m.insert(
        "Remitano_6",
        address!("B8CF411b956B3f9013C1d0Ac8C909b086218207c"),
    );
    m.insert(
        "RenrenBit",
        address!("28c9386eBab8D52Ead4A327e6423316435B2d4fc"),
    );
    m.insert(
        "Revolut",
        address!("2b3FeD49557bd88f78b898684F82FBb355305DbB"),
    );
    m.insert(
        "Revolut_1",
        address!("9b0c45d46D386cEdD98873168C36efd0DcBa8d46"),
    );
    m.insert(
        "Revolut_2",
        address!("b23360CCDd9Ed1b15D45E5d3824Bb409C8D7c460"),
    );
    m.insert(
        "Revolut_3",
        address!("C44b7316936E2F004E688fD53a95e060Df1811C3"),
    );
    m.insert(
        "Robinhood",
        address!("2eFB50e952580f4ff32D8d2122853432bbF2E204"),
    );
    m.insert(
        "Robinhood_1",
        address!("40B38765696e3d5d8d9d834D8AaD4bB6e418E489"),
    );
    m.insert(
        "Robinhood_2",
        address!("4A5b84fb4c7666692C49F2E11664710AA4D0d2a0"),
    );
    m.insert(
        "Robinhood_3",
        address!("6081258689a75d253d87cE902A8de3887239Fe80"),
    );
    m.insert(
        "Robinhood_4",
        address!("7222dE11e132C6F315789eEb5C0182caBD4a9530"),
    );
    m.insert(
        "Robinhood_5",
        address!("73AF3bcf944a6559933396c1577B257e2054D935"),
    );
    m.insert(
        "Robinhood_6",
        address!("97972fA6D980aA9B93D1b584541055840302dE05"),
    );
    m.insert(
        "Robinhood_7",
        address!("A0116A92A032D17a9Ce431EaBE75C5B5F29E2d5E"),
    );
    m.insert(
        "Robinhood_8",
        address!("a26e73C8E9507D50bF808B7A2CA9D5dE4fcC4A04"),
    );
    m.insert(
        "Rollbit",
        address!("8aE57A027c63fcA8070D1Bf38622321dE8004c67"),
    );
    m.insert(
        "Rollbit_1",
        address!("CBD6832Ebc203e49E2B771897067fce3c58575ac"),
    );
    m.insert(
        "Rollbit_2",
        address!("Ef8801eaf234ff82801821FFe2d78D60a0237F97"),
    );
    m.insert(
        "Roobet",
        address!("9eBe47c83C996E5cBBe44c423d7F20DB19dEfc39"),
    );
    m.insert(
        "Roobet_1",
        address!("C94eBB328aC25b95DB0E0AA968371885Fa516215"),
    );
    m.insert(
        "Shakepay",
        address!("000F422887eA7d370FF31173FD3B46c8F66A5B1c"),
    );
    m.insert(
        "Shakepay_1",
        address!("3B794929566e3Ba0f25e4263e1987828b5c87161"),
    );
    m.insert(
        "Shakepay_2",
        address!("4d846dA8257BB0Ebd164EFf513DfF0F0c2C3c0ba"),
    );
    m.insert(
        "Shakepay_3",
        address!("5EaE73d4D24B2922FE614D4F58018b34A7E20a83"),
    );
    m.insert(
        "Shakepay_4",
        address!("88DCdd4A0A58b7e2208805D547043c37dca2b6Dc"),
    );
    m.insert(
        "Shakepay_5",
        address!("912fD21d7a69678227fE6d08C64222Db41477bA0"),
    );
    m.insert(
        "ShapeShift",
        address!("114806Fb56456a525199B957a3a04F726d67b847"),
    );
    m.insert(
        "ShapeShift_1",
        address!("120A270bbC009644e35F0bB6ab13f95b8199c4ad"),
    );
    m.insert(
        "ShapeShift_10",
        address!("52B26F14627e2BF706D257BCfA32D71Eb1BFA70e"),
    );
    m.insert(
        "ShapeShift_11",
        address!("563b377A956c80d77A7c613a9343699Ad6123911"),
    );
    m.insert(
        "ShapeShift_12",
        address!("5aa107C71A314CADA39db8fe7f9b591f67521C14"),
    );
    m.insert(
        "ShapeShift_13",
        address!("5D089C8141e58773Bc88fBA973198B3EC95aa501"),
    );
    m.insert(
        "ShapeShift_14",
        address!("5E44c3E467a49C9Ca0296a9F130fc433041aAa28"),
    );
    m.insert(
        "ShapeShift_15",
        address!("5F06fc2A5aFe54e926a21FdfeF056F93A7F1E6CB"),
    );
    m.insert(
        "ShapeShift_16",
        address!("5F42801ac21677008433d70F1060831ff2a04602"),
    );
    m.insert(
        "ShapeShift_17",
        address!("650C7eA0Ecd6a59b427Ea74c7Aec83f55B47BD02"),
    );
    m.insert(
        "ShapeShift_18",
        address!("6665B4A947a1E4864E8Ef64a2006Add1899e6EBf"),
    );
    m.insert(
        "ShapeShift_19",
        address!("70faa28A6B8d6829a4b1E649d26eC9a2a39ba413"),
    );
    m.insert(
        "ShapeShift_2",
        address!("16A160826b9A2ea7B328b9624ce8971688fC8D4b"),
    );
    m.insert(
        "ShapeShift_20",
        address!("714D7321fe17aCbcf1D64FD3A48aC22d3d214204"),
    );
    m.insert(
        "ShapeShift_21",
        address!("75cDd6bEDACfb4841378C21b076d3E1852a8bB64"),
    );
    m.insert(
        "ShapeShift_22",
        address!("7B9Bc474667Db2fFE5b08d000F1Acc285B2Ae47D"),
    );
    m.insert(
        "ShapeShift_23",
        address!("7Fa926A56D376925d253E10bde289a749A51BE24"),
    );
    m.insert(
        "ShapeShift_24",
        address!("8a65ac0E23F31979db06Ec62Af62b132a6dF4741"),
    );
    m.insert(
        "ShapeShift_25",
        address!("915EC7DE9C792bF9b530178162D5E87E3d22c632"),
    );
    m.insert(
        "ShapeShift_26",
        address!("923ff2b262f4BBE0bcCE7f477De6652905F31A2B"),
    );
    m.insert(
        "ShapeShift_27",
        address!("9BcB0733C56B1D8F0c7c4310949E00485cAe4E9d"),
    );
    m.insert(
        "ShapeShift_28",
        address!("9e6316f44BaEeeE5d41A1070516cc5fA47BAF227"),
    );
    m.insert(
        "ShapeShift_29",
        address!("a345341B99B36C2eC355333199999c73B17bdA9b"),
    );
    m.insert(
        "ShapeShift_3",
        address!("2492D1C00953AD258E1ce6363eb474595f5279F2"),
    );
    m.insert(
        "ShapeShift_30",
        address!("A620958b0F7D59ed764e77423960a3cb9321680b"),
    );
    m.insert(
        "ShapeShift_31",
        address!("b36eFd48c9912Bd9fd58b67b65f7438F6364a256"),
    );
    m.insert(
        "ShapeShift_32",
        address!("Ba991DBF44893D237829fE23D3973971B88f8F5e"),
    );
    m.insert(
        "ShapeShift_33",
        address!("bfc1fC4d2546AF6420D2Fc14819a1Da2A8E9dD6b"),
    );
    m.insert(
        "ShapeShift_34",
        address!("c7BA53854Ea347EDCe88c37bB99a699EdbB77096"),
    );
    m.insert(
        "ShapeShift_35",
        address!("caE39061F41686e1Aaf9cf10145e5d4a4265635C"),
    );
    m.insert(
        "ShapeShift_36",
        address!("D063435D7caB1A792e1D56F7Aab04313B3D87179"),
    );
    m.insert(
        "ShapeShift_37",
        address!("D3273EBa07248020bf98A8B560ec1576a612102F"),
    );
    m.insert(
        "ShapeShift_38",
        address!("Da1E5D4Cc9873963f788562354b55A772253b92f"),
    );
    m.insert(
        "ShapeShift_39",
        address!("DA8b9075f0F3094EC6F614C36E92317813C80957"),
    );
    m.insert(
        "ShapeShift_4",
        address!("2624eDaFCE546781883c26Fc9C461D4c8E782Ef9"),
    );
    m.insert(
        "ShapeShift_40",
        address!("df69de4a2a58866afeBb7713e3dd10C2153fF27C"),
    );
    m.insert(
        "ShapeShift_41",
        address!("e65A88f487F5d26469Cfd37ce7Ef763D6d9BE454"),
    );
    m.insert(
        "ShapeShift_42",
        address!("e8ed915E208B28c617d20F3F8Ca8e11455933aDf"),
    );
    m.insert(
        "ShapeShift_43",
        address!("E9319eBA87Af7C2fc1F55ccDe9d10eA8efbd592d"),
    );
    m.insert(
        "ShapeShift_44",
        address!("E93E588821A00a9F2ff3f9E40E224cAA5118f275"),
    );
    m.insert(
        "ShapeShift_45",
        address!("eed16856D551569D134530ee3967Ec79995E2051"),
    );
    m.insert(
        "ShapeShift_46",
        address!("F08BDf21373A09aB7eDD7769A402D3a22826D317"),
    );
    m.insert(
        "ShapeShift_47",
        address!("f2038B4368FC6F4eEcbeaE93F37262F1De3a23e6"),
    );
    m.insert(
        "ShapeShift_48",
        address!("F316e9af231C1c5004540c220c78627F8A9f5419"),
    );
    m.insert(
        "ShapeShift_49",
        address!("F610Fae41259f6FE66dC32fAc20ba6D2d72A506b"),
    );
    m.insert(
        "ShapeShift_5",
        address!("2e0714166a5095D0730B97110A11028158F1F3b7"),
    );
    m.insert(
        "ShapeShift_6",
        address!("2f155ddeFC29c414C94b801B91F55B257231825E"),
    );
    m.insert(
        "ShapeShift_7",
        address!("39D3b15006e580077a2E8B51B93BE90cCF1EC0e0"),
    );
    m.insert(
        "ShapeShift_8",
        address!("3AEf01dB231c3C9fF844f7E611c63b8c36bc6A02"),
    );
    m.insert(
        "ShapeShift_9",
        address!("3b0BC51Ab9De1e5B7B6E34E5b960285805C41736"),
    );
    m.insert(
        "Sideshift",
        address!("3ee1fac3d8cC67A0676830622D3AFc55cF6ffF27"),
    );
    m.insert(
        "Sideshift_1",
        address!("6cf6a8488D70b1743134d6D69950cDa60325A42F"),
    );
    m.insert(
        "Sideshift_2",
        address!("722b33b843BAca81aa70cEf29C9512de7B3f8767"),
    );
    m.insert(
        "Sideshift_3",
        address!("cDd37Ada79F589c15bD4f8fD2083dc88E34A2af2"),
    );
    m.insert(
        "Sideshift_4",
        address!("F7E00e8Df9d41B891bbA8263f74c7ec23C12Acac"),
    );
    m.insert(
        "SimpleSwap",
        address!("09fe30D5B6e19B38F04a01A217519cECa15B5388"),
    );
    m.insert(
        "SimpleSwap_1",
        address!("0FdD8454CdA144b88955e8bb7931456989f853CC"),
    );
    m.insert(
        "SimpleSwap_10",
        address!("Bb3fd383d1C5540E52EF0A7bcb9433375793aEAF"),
    );
    m.insert(
        "SimpleSwap_11",
        address!("BBE4B05aAEF7526153888d0cdd054b78c72A7E85"),
    );
    m.insert(
        "SimpleSwap_12",
        address!("Ca604a3e8B6277492EbC558a4457B6e60e611096"),
    );
    m.insert(
        "SimpleSwap_13",
        address!("d8F9Ced745e429Ea0723aA72693EFf03B5182DC7"),
    );
    m.insert(
        "SimpleSwap_14",
        address!("eE7f2D8257Aa658c5895796f070e4046bA8Fb37e"),
    );
    m.insert(
        "SimpleSwap_2",
        address!("1d05ACf4e760b1E06C735B67818fdc91558Df17d"),
    );
    m.insert(
        "SimpleSwap_3",
        address!("32E9dc9968Fab4C4528165cd37B613dD5d229650"),
    );
    m.insert(
        "SimpleSwap_4",
        address!("40Bbfa70b338efD6E81D93eD0a25A2cB67bB7Bb9"),
    );
    m.insert(
        "SimpleSwap_5",
        address!("4B0401Fe6B84C52d4F4310c371c731a2B6D0964D"),
    );
    m.insert(
        "SimpleSwap_6",
        address!("59B36B4b1B25bc61C8A81eaf70aD923D149F3d95"),
    );
    m.insert(
        "SimpleSwap_7",
        address!("7BaCd3E83522F484Bc5128EA93Bf7290f1F1B9E5"),
    );
    m.insert(
        "SimpleSwap_8",
        address!("876470570C01806261A981D653C4A601CD6875c0"),
    );
    m.insert(
        "SimpleSwap_9",
        address!("afd99a1a7e2195a8E0fdB6e8bD45EFDff15FEadD"),
    );
    m.insert(
        "Simplex",
        address!("6ec88a2Cb932eb46dfda0280c0eadB93b6eCa13B"),
    );
    m.insert(
        "Simplex_1",
        address!("77300C71071eCa35Cb673a0b7571B2907dEB77C7"),
    );
    m.insert(
        "SouthXchange",
        address!("324cC2c9fb379EA7a0D1C0862C3b48cA28D174A4"),
    );
    m.insert(
        "Sparrow",
        address!("91F6d99b232153CB655Ad3E0d05e13EF505F6cd5"),
    );
    m.insert(
        "Sparrow_1",
        address!("e855283086FbEe485aECF2084345A91424c23954"),
    );
    m.insert(
        "Stake.com",
        address!("019D0706D65c4768ec8081eD7CE41F59Eef9b86c"),
    );
    m.insert(
        "Stake.com_1",
        address!("0392b64B8BfDA184F0A72cE37D73dC7dF978C4f7"),
    );
    m.insert(
        "Stake.com_10",
        address!("bBc43C282B2f829176F4Fc3802436D8fAD3413F3"),
    );
    m.insert(
        "Stake.com_11",
        address!("DebfBE80C8aebA98A32968278463ccB639C6C4e3"),
    );
    m.insert(
        "Stake.com_12",
        address!("F598b81Ef8c7b52a7F2a89253436e72ec6DC871f"),
    );
    m.insert(
        "Stake.com_13",
        address!("Fa500178de024BF43CFA69B7e636A28AB68F2741"),
    );
    m.insert(
        "Stake.com_2",
        address!("6e29f75b0350fd0e85EE34a21eF94767b0186996"),
    );
    m.insert(
        "Stake.com_3",
        address!("6F419642AD147853A91E1CB50D4B909dde19Cece"),
    );
    m.insert(
        "Stake.com_4",
        address!("758BE77a3eE14e7193730560daA07dd3fcBFD200"),
    );
    m.insert(
        "Stake.com_5",
        address!("787B8840100d9BaAdD7463f4a73b5BA73B00C6cA"),
    );
    m.insert(
        "Stake.com_6",
        address!("974CaA59e49682CdA0AD2bbe82983419A2ECC400"),
    );
    m.insert(
        "Stake.com_7",
        address!("A29148c2A656E5Ddc68acB95626D6B64A1131c06"),
    );
    m.insert(
        "Stake.com_8",
        address!("b04c0EB29C72cEBC467b9d4944D29116fa02C44a"),
    );
    m.insert(
        "Stake.com_9",
        address!("B2723BEacce4BC54F23544343927f048CeF6bD5A"),
    );
    m.insert(
        "Steam Exchange",
        address!("0542Df7daCc8716653Df3fd9F991520AA2f2D0bc"),
    );
    m.insert(
        "Steam Exchange_1",
        address!("c0924EDEFB2C0C303de2d0c21BfF07ab763163B5"),
    );
    m.insert("Stex", address!("7D2d2bC5FB453673c3E31c6b002ef78613165CDC"));
    m.insert(
        "Stex_1",
        address!("97E12BD75bdee72d4975D6df410D2d145b3d8457"),
    );
    m.insert(
        "Streamity",
        address!("9BF25700727d10a857099D1033Ce2cC493c3B61A"),
    );
    m.insert(
        "SwissBorg",
        address!("0A52368D5a7E70D8c927f75Ea6618C2c468031D4"),
    );
    m.insert(
        "SwissBorg_1",
        address!("11444C6389A26C8E41d7FD5CafBfCC511303b7d3"),
    );
    m.insert(
        "SwissBorg_10",
        address!("691e3Cbb2a8F504fC650F21C9af6226051340559"),
    );
    m.insert(
        "SwissBorg_11",
        address!("6Cf9AA65EBaD7028536E353393630e2340ca6049"),
    );
    m.insert(
        "SwissBorg_12",
        address!("7153D2ef9F14a6b1Bb2Ed822745f65E58d836C3F"),
    );
    m.insert(
        "SwissBorg_13",
        address!("87cbc48075d7aa1760Ac71C41e8Bc289b6A31F56"),
    );
    m.insert(
        "SwissBorg_14",
        address!("94596096320A6B4EaB43556AD1Ed8c4c3d51C9aA"),
    );
    m.insert(
        "SwissBorg_15",
        address!("A03D3611B34C3c49DBcb8206eD08fe6467f684a5"),
    );
    m.insert(
        "SwissBorg_16",
        address!("a5546c4bC006D23B60D690D3033b8dF40Cecc230"),
    );
    m.insert(
        "SwissBorg_17",
        address!("cDE4c1b984F3F02f997ECfF9980B06316de2577d"),
    );
    m.insert(
        "SwissBorg_18",
        address!("cDf2bFd0ff20811c98471b331db19AAbf3D3b972"),
    );
    m.insert(
        "SwissBorg_19",
        address!("D0c3647CB16460A230B4Aa93f3723823Aa17c943"),
    );
    m.insert(
        "SwissBorg_2",
        address!("22bF0A4C4eff418b3306AbFeE20813D0b6E8Dc74"),
    );
    m.insert(
        "SwissBorg_20",
        address!("FF4606bd3884554CDbDabd9B6e25E2faD4f6fc54"),
    );
    m.insert(
        "SwissBorg_3",
        address!("2e8C1131BE1A839B375ac3ED1BA061dF3d87CBFd"),
    );
    m.insert(
        "SwissBorg_4",
        address!("42b86A269fb3d5368D880c519BadABa77eC00130"),
    );
    m.insert(
        "SwissBorg_5",
        address!("43fdA7708C97C4C40d5402c6392f0457f23c0b8e"),
    );
    m.insert(
        "SwissBorg_6",
        address!("5770815B0c2a09A43C9E5AEcb7e2f3886075B605"),
    );
    m.insert(
        "SwissBorg_7",
        address!("5cEDc1923c33B253aedf24bF038eeE6Cbbb68A6A"),
    );
    m.insert(
        "SwissBorg_8",
        address!("61488B940E5796b0D4FF096a441ebc48B8eaC6DE"),
    );
    m.insert(
        "SwissBorg_9",
        address!("67FE3293FC4e877F3CDc3F0ed93721a600f72BdE"),
    );
    m.insert(
        "Switchain",
        address!("A96b536eEf496e21F5432FD258b6F78CF3673F74"),
    );
    m.insert("TAGZ", address!("ea3a46BD1dbd0620d80037f70d0bF7c7dc5a837C"));
    m.insert(
        "TAGZ_1",
        address!("ED8204345a0Cf4639D2dB61a4877128FE5Cf7599"),
    );
    m.insert(
        "TBCC Global",
        address!("D9D307698e03Db8BE472E92E1c42b0d66245eeAc"),
    );
    m.insert(
        "Thodex",
        address!("214989c36c5fD378bcBb27F70315049E3D8Aa74c"),
    );
    m.insert(
        "Thodex_1",
        address!("68859697fdC8c3069303Fa87947ADB622Ce990DC"),
    );
    m.insert(
        "Thodex_2",
        address!("B6B9bAD197225DEda72f452A2660F813B557cCc2"),
    );
    m.insert(
        "Tidex",
        address!("0a73573Cf2903d2D8305b1eCb9e9730186a312aE"),
    );
    m.insert(
        "Tidex_1",
        address!("3613ef1125A078EF96Ffc898c4eC28D73C5b8C52"),
    );
    m.insert(
        "Tidex_2",
        address!("9acAfC3FE25E7DA2B8bF49c836619d40E395A859"),
    );
    m.insert(
        "TigerGaming",
        address!("2F19E5C3C66C44E6405D4c200fE064ECe9bC253a"),
    );
    m.insert(
        "TigerGaming_1",
        address!("41292153E7F5e78C3b7382D59E742b92461CBC70"),
    );
    m.insert(
        "TigerGaming_2",
        address!("bd5CdD1ca9aE5F1443AeC2642D43a538c32a473F"),
    );
    m.insert(
        "TigerGaming_3",
        address!("D86e3A116Bc98A354253dea47ffd36E86b1A1BC2"),
    );
    m.insert(
        "TigerGaming_4",
        address!("E21D837cd1437305632ac1660A94c64b1ECd3151"),
    );
    m.insert(
        "TokenMarket",
        address!("c330C1A3c7Db9c75f60AeD0A9B7C0Fc5FA22D5A2"),
    );
    m.insert(
        "Tokenize Xchange",
        address!("26637e1362A0C9F57D317CB417A9dEDdFe137F2a"),
    );
    m.insert(
        "Tokenize Xchange_1",
        address!("27D7f6147A8748454b88bE685c2804F96bF69dB1"),
    );
    m.insert(
        "Tokenize Xchange_10",
        address!("Edc53939315e2EBe28ebc771E99aD17463D28102"),
    );
    m.insert(
        "Tokenize Xchange_11",
        address!("F12Db6a6B8ECc2EA6245F8590135ca372ABB36E1"),
    );
    m.insert(
        "Tokenize Xchange_12",
        address!("f4FcaBded10b2d3D18d5040EcAE3a6D0FBBa10BC"),
    );
    m.insert(
        "Tokenize Xchange_2",
        address!("2c2F95DC8d3558F490CC0ca431a3BEF0B1E13aC8"),
    );
    m.insert(
        "Tokenize Xchange_3",
        address!("5f1F90B762baFA7F964050A347228B3b36425A55"),
    );
    m.insert(
        "Tokenize Xchange_4",
        address!("6cdc7C73345410dB99945433278df0bcbEEf4716"),
    );
    m.insert(
        "Tokenize Xchange_5",
        address!("b5E8C25f34A84613229BaBf4D0899157D74568F9"),
    );
    m.insert(
        "Tokenize Xchange_6",
        address!("B64d9784E8516983243434ce3BadF967Fd5cc71e"),
    );
    m.insert(
        "Tokenize Xchange_7",
        address!("B911c9ab63600B84b17Ef37720B332c73231E904"),
    );
    m.insert(
        "Tokenize Xchange_8",
        address!("BDB2aD8B5e5606013506c160B75264f9B1b48794"),
    );
    m.insert(
        "Tokenize Xchange_9",
        address!("CEf15405edCB31942c29792C113a818789259c18"),
    );
    m.insert(
        "Tokocrypto",
        address!("0068eB681EC52DBd9944517d785727310B494575"),
    );
    m.insert(
        "Tokocrypto_1",
        address!("7D8Dd9A8Be1c3eeE9101Ffb77C3BF0E89CBe74bA"),
    );
    m.insert(
        "Tokocrypto_2",
        address!("9A2f5556e9A637e8fBcE886d8e3cf8b316a1D8a2"),
    );
    m.insert(
        "Tokocrypto_3",
        address!("9f589e3eabe42ebC94A44727b3f3531C0c877809"),
    );
    m.insert(
        "Tokocrypto_4",
        address!("B6c1a7cCd41530A35Cd3d8ae5F1eaF40b588FaEf"),
    );
    m.insert(
        "Tokocrypto_5",
        address!("ef136c9beA3e397Ffed3cd8aD12511A7421116A0"),
    );
    m.insert(
        "TopBTC",
        address!("B2cc3cDd53fC9A1AEAf3A68Edeba2736238DDC5D"),
    );
    m.insert(
        "Trade.io",
        address!("0C38C14188CF42c2ab4bDc55084085059F0a507B"),
    );
    m.insert(
        "Trade.io_1",
        address!("1119AaEfB02bF12b84d28A5D8ea48ec3C90Ef1Db"),
    );
    m.insert(
        "Trade.io_2",
        address!("5652555D47430E948Ca15660c5652A23410d5072"),
    );
    m.insert(
        "Trade.io_3",
        address!("58f75dDACFFB183a30F69fE58a67a0d0985fce0F"),
    );
    m.insert(
        "Trade.io_4",
        address!("5A2FAd810f990C4535ADa938400B6b67eF7646af"),
    );
    m.insert(
        "TradeOgre",
        address!("4648451b5F87FF8F0F7D622bD40574bb97E25980"),
    );
    m.insert(
        "TradeOgre_1",
        address!("5E38AD84A902078D61Ca8D3BEbd378bC0e32C422"),
    );
    m.insert(
        "Transak",
        address!("27899ffaCe558bdE9F284Ba5C8c91ec79EE60FD6"),
    );
    m.insert(
        "Txbit",
        address!("339Fe932809E39A95B621A7f88BbF6C08eb6C978"),
    );
    m.insert(
        "Txbit_1",
        address!("53EdC98CB6C21dFCDdAF7F91Ebf39789B93E2Ac6"),
    );
    m.insert(
        "Txbit_2",
        address!("E4FEb3e94B4128d973A366dc4814167a90629A08"),
    );
    m.insert("UEX", address!("2f1233Ec3a4930Fd95874291DB7da9E90dfB2F03"));
    m.insert(
        "UEX_1",
        address!("a78976995AE1a5670F6faC64a8a3E802fdcF1208"),
    );
    m.insert(
        "Ultimate Champions",
        address!("18B7517cf34a3277f3DB3381c3B2679Cc3dc1116"),
    );
    m.insert(
        "Uphold",
        address!("1C727a55eA3c11B0ab7D3a361Fe0F3C47cE6de5d"),
    );
    m.insert(
        "Uphold_1",
        address!("340d693ED55d7bA167D184ea76Ea2Fd092a35BDc"),
    );
    m.insert(
        "Uphold_2",
        address!("352E504813B9E0b30F9cA70eFc27A52D298F6697"),
    );
    m.insert(
        "Uphold_3",
        address!("3D8FC1CFfAa110F7A7F9f8BC237B73d54C4aBf61"),
    );
    m.insert(
        "Uphold_4",
        address!("6E5d4a29833e51a83539a57461E803BCff409050"),
    );
    m.insert(
        "Uphold_5",
        address!("a95350d70B18FA29f6B5EB8D627cEEEEE499340d"),
    );
    m.insert(
        "VinDAX",
        address!("07ac908cCa9c69AF022541D8fc0Bb29485FEB4bf"),
    );
    m.insert(
        "VinDAX_1",
        address!("c5a7C9D185A47DA11878F46932D23c0fdc56F275"),
    );
    m.insert(
        "Vinex",
        address!("b436c96c6DE1f50A160eD307317C275424DBe4F2"),
    );
    m.insert(
        "Voyager",
        address!("05a5E62CEBFB3FC3790F6C85fA620E82b5C58BD1"),
    );
    m.insert(
        "Voyager_1",
        address!("149090aefD763c3348dbf862bd9D7A2B53C54F97"),
    );
    m.insert(
        "Voyager_10",
        address!("663fD1D7658cD428b256057C3a85d13b710804b6"),
    );
    m.insert(
        "Voyager_11",
        address!("73dd3e09A0A75012480F9753efC37d6ccC9A4058"),
    );
    m.insert(
        "Voyager_12",
        address!("746350bFC022f90cACa573124F7396D46837874e"),
    );
    m.insert(
        "Voyager_13",
        address!("764735E89a15EE53aA9C353f042e1CB637Aa4AC6"),
    );
    m.insert(
        "Voyager_14",
        address!("7ccEf9ed17824214D60403171d889Bd4cE878B27"),
    );
    m.insert(
        "Voyager_15",
        address!("91962711a4D2E4a830b366ce7276D99001e8564b"),
    );
    m.insert(
        "Voyager_16",
        address!("A6aac20c2F51101A92a01E28c8da87927677f9Cc"),
    );
    m.insert(
        "Voyager_17",
        address!("ac7E222B95A1e764186Ebe7C10fC32F6C969D076"),
    );
    m.insert(
        "Voyager_18",
        address!("e120Ed33cAffdcF269EfD822fa0b77CD8c31BFdF"),
    );
    m.insert(
        "Voyager_19",
        address!("E8724f21Aa13f86BcEb8c9c86e3EbE5da643c730"),
    );
    m.insert(
        "Voyager_2",
        address!("203520F4ec42Ea39b03F62B20e20Cf17DB5fdfA7"),
    );
    m.insert(
        "Voyager_20",
        address!("Ee977dC5F20D8e4771f2AF5711C9F75E795bFCbA"),
    );
    m.insert(
        "Voyager_21",
        address!("F27C5989B39b0BFa537C5aa9617d78235eccD7F7"),
    );
    m.insert(
        "Voyager_22",
        address!("F91A11B31ECd9a93Aed7060680b5f7899D7CC98d"),
    );
    m.insert(
        "Voyager_23",
        address!("fAc87e892800F73feA9bD4a81B4E0269f4363fE3"),
    );
    m.insert(
        "Voyager_3",
        address!("30D02D3351a43220a249C6e87426D1250c976e91"),
    );
    m.insert(
        "Voyager_4",
        address!("31C741dF303F8F3506c70E69B1893d93236a360B"),
    );
    m.insert(
        "Voyager_5",
        address!("3F31DbDC099761156244a5Bd5e06CDADbd2F30c0"),
    );
    m.insert(
        "Voyager_6",
        address!("43E9A1Dc8a6d2f0DD9F1903b82c43AAc69eADCB4"),
    );
    m.insert(
        "Voyager_7",
        address!("500A746c9a44f68Fe6AA86a92e7B3AF4F322Ae66"),
    );
    m.insert(
        "Voyager_8",
        address!("5157740482B8AcAD392D696f1ED1e2C09D028ac2"),
    );
    m.insert(
        "Voyager_9",
        address!("5Bc4599621485566ceF235A8a01D3985c03D7b62"),
    );
    m.insert("WEX", address!("38Bb8A02eF1EDeBccA370f178F2974417Ed95F12"));
    m.insert(
        "WEX_1",
        address!("b3AAAae47070264f3595c5032eE94b620A583a39"),
    );
    m.insert(
        "WazirX",
        address!("0E293a9E57D22F6ec575b376e3E3Bd8e642Fd5fc"),
    );
    m.insert(
        "WazirX_1",
        address!("1b9509aeb1FBb605868D21cAf2EEB6aC8351264D"),
    );
    m.insert(
        "WazirX_2",
        address!("618fFD1cDAbeE36CE5992a857Cc7463f21272bD7"),
    );
    m.insert(
        "WazirX_3",
        address!("7aeB3314E041153c4F6bbea19AbECBCe20946fD4"),
    );
    m.insert(
        "WazirX_4",
        address!("7db77E4c967C95A5a9E2ec57Ec21788daB481893"),
    );
    m.insert(
        "WazirX_5",
        address!("cDeF28FE85aB08a6632C97fd90534666c1ae96a3"),
    );
    m.insert(
        "WazirX_6",
        address!("fA54B4085811aef6ACf47D51B05FdA188DEAe28b"),
    );
    m.insert(
        "Wealthsimple",
        address!("a21A16EC22a940990922220E4ab5bF4C2310F556"),
    );
    m.insert(
        "WhiteBIT",
        address!("1689a089AA12d6CbBd88bC2755E4c192f8702000"),
    );
    m.insert(
        "WhiteBIT_1",
        address!("33Eac50b7fAf4B8842A621d0475335693F5D21fe"),
    );
    m.insert(
        "WhiteBIT_2",
        address!("39F6a6C85d39d5ABAd8A398310c52E7c374F2bA3"),
    );
    m.insert(
        "WhiteBIT_3",
        address!("515281812Fdf5b0D7bE5Fb25b823a2aB79E0A621"),
    );
    m.insert(
        "WhiteBIT_4",
        address!("5c3Be6fA73CaB89f27744e886dB983e64B689Bf9"),
    );
    m.insert(
        "WhiteBIT_5",
        address!("98cEA98BE2a37A8bB52451Bd46259b2FBeE1bDc0"),
    );
    m.insert(
        "WhiteBIT_6",
        address!("aB928E30bEdE5919D4Bd9ec244711495769d2d85"),
    );
    m.insert(
        "WhiteBIT_7",
        address!("e3dB465646EA2aD39ff5672bB5FF0E83dcC91F0E"),
    );
    m.insert(
        "WhiteBIT_8",
        address!("eeFBd9626704DCd9C672c1031fC81e7f346ff3B8"),
    );
    m.insert(
        "Wirex",
        address!("0A1820f0ff7Dc9FCE0A4F0B589ee14DdAe88233C"),
    );
    m.insert(
        "Wirex_1",
        address!("2f13d388b85e0eCd32e7C3D7F36D1053354EF104"),
    );
    m.insert(
        "Wirex_2",
        address!("4afDBa85a0E24A0D3F3245C8d91f5A0e2914E3C3"),
    );
    m.insert(
        "Wirex_3",
        address!("4B8bd5ad437D43Babd20bBD618F73e3f50D9475f"),
    );
    m.insert(
        "Wirex_4",
        address!("6966cE74E593df08e33F9eF0D8a4c0c9E0336bE5"),
    );
    m.insert(
        "Wirex_5",
        address!("935f64B44B5C48A1539C4AdA5161D27ace4205b5"),
    );
    m.insert(
        "Wirex_6",
        address!("b57decCD4B0Aa811FE1eC947c66Ee65C08617A76"),
    );
    m.insert(
        "Wirex_7",
        address!("E3f277382419535245a345e923898c2d43f7CBE5"),
    );
    m.insert(
        "Woo X",
        address!("03Dd167D62E1DFC223Ffd7b37Fc8bF45fB973478"),
    );
    m.insert(
        "Woo X_1",
        address!("15271E572267dEf474366bB683719Cc59489eFBe"),
    );
    m.insert(
        "Woo X_10",
        address!("EeF97691d3307b4E61522170F648Ee2df1312fEE"),
    );
    m.insert(
        "Woo X_11",
        address!("fA2d1f15557170F6c4A4C5249e77f534184cdb79"),
    );
    m.insert(
        "Woo X_2",
        address!("1E6DCe7cE381774286abb8c9AAc461Bb7B1C4b05"),
    );
    m.insert(
        "Woo X_3",
        address!("594203E46e0B41B1eDB54a551E7784c194d1335b"),
    );
    m.insert(
        "Woo X_4",
        address!("607e062E3986a16283047BEAeD1A7dC3E220ff0E"),
    );
    m.insert(
        "Woo X_5",
        address!("63DFE4e34A3bFC00eB0220786238a7C6cEF8Ffc4"),
    );
    m.insert(
        "Woo X_6",
        address!("D7d8bCaE65537CB5079a4fB249b9fbB4526e4084"),
    );
    m.insert(
        "Woo X_7",
        address!("e2933566f172D08f8C90144fEd5Ae28E9d54B1ec"),
    );
    m.insert(
        "Woo X_8",
        address!("E505Bf08C03cc0FA4e0FDFa2487E2c11085b3FD9"),
    );
    m.insert(
        "Woo X_9",
        address!("E64eB20471491956338eEdC0F98242Bc3aD0C91b"),
    );
    m.insert(
        "XT.com",
        address!("104703893f56243c0e56441a99eb3F32E1ed6AD2"),
    );
    m.insert(
        "XT.com_1",
        address!("24Ca039E1F71EF468d2946045c1F3e90D31FDB71"),
    );
    m.insert(
        "XT.com_10",
        address!("B98486f2dF13ba242F173fFCBfF5C122B4d95A16"),
    );
    m.insert(
        "XT.com_11",
        address!("cAB044b301D94875aC1502B73e54d63b0162C70f"),
    );
    m.insert(
        "XT.com_12",
        address!("E0a616C3659bE29567E08819772e6905307AdF21"),
    );
    m.insert(
        "XT.com_13",
        address!("E5B8ff1ca1c3Ef2ac704783d6473Ee5a9BE7e02d"),
    );
    m.insert(
        "XT.com_14",
        address!("eFDA0cB780A8564903285ED25df3CC024f3b2982"),
    );
    m.insert(
        "XT.com_2",
        address!("2Bc0DdE194d722FE98Ed3912cad464380F2225aC"),
    );
    m.insert(
        "XT.com_3",
        address!("3Fcad6c6fB6BFfBb3585F2A401fbC87A6719D28d"),
    );
    m.insert(
        "XT.com_4",
        address!("4Fd7c9BDb194f3Cf0589803d0A664A80e59ebFa6"),
    );
    m.insert(
        "XT.com_5",
        address!("716198E735e1DF5F5423E74Fb98d493a1F789c1e"),
    );
    m.insert(
        "XT.com_6",
        address!("724fD0870996faAcf4F6C169F484c678cf2Aa209"),
    );
    m.insert(
        "XT.com_7",
        address!("80309282231e9e01c74C4E0EFeD35b063E6c5D27"),
    );
    m.insert(
        "XT.com_8",
        address!("8c395E3fB26939438ACD5ca4bE7684752829e269"),
    );
    m.insert(
        "XT.com_9",
        address!("98a79dbf216DaeF423bab001E23795f606bF7c2E"),
    );
    m.insert(
        "XeggeX",
        address!("20FfE0D07D7f7c2C21A24537538b4cDE06c9048a"),
    );
    m.insert(
        "XeggeX_1",
        address!("5fB29283c2cF472B86DB4f74A2B32F9CaF5578a4"),
    );
    m.insert(
        "XeggeX_2",
        address!("a0387AdBA7636722ABE119cbF9220Ce0B9938b0b"),
    );
    m.insert(
        "YOOBTC",
        address!("8F3AB2c3B651382b07A76653D2be9EB4b87E1630"),
    );
    m.insert(
        "YoBit",
        address!("8c240D98E179A9e283A2394e5969a2EEA95CA810"),
    );
    m.insert(
        "YoBit_1",
        address!("F5bEC430576fF1b82e44DDB5a1C93F6F9d0884f3"),
    );
    m.insert(
        "YouBank",
        address!("3BF5a62CCD5ea7a0f56EF5E42616A0D5fEc3Fa95"),
    );
    m.insert(
        "YouBank_1",
        address!("683b8615d6099a71941CD693c151ef7C0573Fb44"),
    );
    m.insert(
        "YouBank_10",
        address!("D894A19CF72e2Ef2EF6A0B51A2fCb36ECB733A01"),
    );
    m.insert(
        "YouBank_11",
        address!("F509AcCd096A82Ef2562D316669d0AA4b60f3796"),
    );
    m.insert(
        "YouBank_2",
        address!("82817C5528F8593e9E88a6A583BF5E77326Bdb55"),
    );
    m.insert(
        "YouBank_3",
        address!("8Fa7E42E4ac6C1d876c99018Bb4A210e7Bb7564c"),
    );
    m.insert(
        "YouBank_4",
        address!("938534B724e7ea82Da66f22eed82Dd75bB486194"),
    );
    m.insert(
        "YouBank_5",
        address!("aD2Ef235E673a23DAB2CDd1EA834509AA6EA8bEB"),
    );
    m.insert(
        "YouBank_6",
        address!("c5821feD609BeB4943AEd9CCf7fD20f93c8743E2"),
    );
    m.insert(
        "YouBank_7",
        address!("c8f1df1b67F5E151d7DeE0738D50c6152c36BfFB"),
    );
    m.insert(
        "YouBank_8",
        address!("d2349d84187580c73f1c77669F6f409DC425C3A3"),
    );
    m.insert(
        "YouBank_9",
        address!("D2762EFEa5B803b8E87Aa62049794355BB5F74F3"),
    );
    m.insert(
        "YouHodler",
        address!("260Ee8F2B0C167e0cd6119b2DF923FD061dc1093"),
    );
    m.insert(
        "YouHodler_1",
        address!("2F1f7FC76e6CF5ab3e186A2A2FD4Fc31952a77Cc"),
    );
    m.insert(
        "Yunbi",
        address!("42dA8a05CB7eD9A43572b5BA1B8F82A0a6E263DC"),
    );
    m.insert(
        "Yunbi_1",
        address!("700f6912e5753e91ea3Fae877A2374A2db1245D7"),
    );
    m.insert(
        "Yunbi_2",
        address!("A32eeab263c7542958258BBeB52F8d4039b76511"),
    );
    m.insert(
        "Yunbi_3",
        address!("d94c9ff168dc6aEbf9b6CC86dEfF54f3fb0AFC33"),
    );
    m.insert(
        "ZB.com",
        address!("0e394D3fAcF0Ce3BD5fCcE584E16E0cBAc164346"),
    );
    m.insert(
        "ZB.com_1",
        address!("5E91E8CcAe2Dd2c6db87F677e161Ed1e07D6cCC8"),
    );
    m.insert(
        "ZB.com_10",
        address!("F0D9FcB4FefdBd3e7929374b4632f8AD511BD7e3"),
    );
    m.insert(
        "ZB.com_11",
        address!("FD6724B4b3e8eca764F0DD07ccd903aD348D70F8"),
    );
    m.insert(
        "ZB.com_2",
        address!("60d0cC2aE15859f69bF74DADb8AE3Bd58434976b"),
    );
    m.insert(
        "ZB.com_3",
        address!("6ba3FFBd026a4ec164aD477092000B9CF1e4C351"),
    );
    m.insert(
        "ZB.com_4",
        address!("734Ac651Dd95a339c633cdEd410228515F97fAfF"),
    );
    m.insert(
        "ZB.com_5",
        address!("b793FE6745fD79a42d7491B0861ef438c45e8A0f"),
    );
    m.insert(
        "ZB.com_6",
        address!("c97A4ed29F03FD549c4ae79086673523122d2Bc5"),
    );
    m.insert(
        "ZB.com_7",
        address!("cB45822DFA8A65b97bc854A1A61B674153a42967"),
    );
    m.insert(
        "ZB.com_8",
        address!("d371fBa58AB4C209f35001B30B39539f05091148"),
    );
    m.insert(
        "ZB.com_9",
        address!("Db7248d26Ea60170a4Fdc2ea44dc839C9E8C9ee4"),
    );
    m.insert(
        "Zero Hash",
        address!("85D9aef1Ec67356A0D60c3ABE5aDb2E1dB9DE963"),
    );
    m.insert(
        "Zero Hash_1",
        address!("a1271A8A80748abd3F0DaFD4914aD2F481264447"),
    );
    m.insert(
        "Zero Hash_2",
        address!("b02a9b7400545925746d8B9B985BC74A0601fB8D"),
    );
    m.insert(
        "Zero Hash_3",
        address!("C1712944C5A544eF1287D6959068EAe6090b89Aa"),
    );
    m.insert(
        "Zero Hash_4",
        address!("CfC0F98f30742B6d880f90155d4EbB885e55aB33"),
    );
    m.insert(
        "Zipmex",
        address!("1FD5dAA9BB707afb8A3197dEf72005757F8d1E36"),
    );
    m.insert(
        "Zipmex_1",
        address!("4a87135693A5b7dd0653C151c16EDFA3c524403F"),
    );
    m.insert(
        "Zipmex_2",
        address!("B487562715aC79C81C44830F964c8c65a4b76CBD"),
    );
    m.insert(
        "Zipmex_3",
        address!("b94Db18c60A429CB5080FE36c9333c07Cd158598"),
    );
    m.insert(
        "Zipmex_4",
        address!("cBe3f2e3E5da7E19F973BD07db4D22c28fC71c68"),
    );
    m.insert(
        "Zonda",
        address!("0FF24158220A14398F047a80a513617Ddc4f5289"),
    );
    m.insert(
        "Zonda_1",
        address!("2b645268E2fbb384B423e50089657395F749763a"),
    );
    m.insert(
        "Zonda_2",
        address!("5BfF49EeC8F76C066F979A818187b9732AC69503"),
    );
    m.insert(
        "Zonda_3",
        address!("6EDF968DA408a9640b8865826429a977a11C5048"),
    );
    m.insert(
        "Zonda_4",
        address!("781229c7a798c33EC788520a6bBe12a79eD657FC"),
    );
    m.insert(
        "Zonda_5",
        address!("818ab3c61f66e975b8E6290c20999d6749F60d8D"),
    );
    m.insert(
        "Zonda_6",
        address!("d388009f01bbE5e6D2Cb6bA8525ca50B56308046"),
    );
    m.insert(
        "Zonda_7",
        address!("f646CBe3B030fb6c2569215F0117dbA58baDB95E"),
    );
    m.insert(
        "bitFlyer",
        address!("111cFf45948819988857BBF1966A0399e0D1141e"),
    );
    m.insert(
        "bitFlyer_1",
        address!("89460424c14378c2407518cEEA6D427830084822"),
    );
    m.insert(
        "bitFlyer_2",
        address!("8abe3ac564098adcaD1Cb0f812358E5E4555e2bA"),
    );
    m.insert(
        "bitFlyer_3",
        address!("B01cb49fe0D6D6E47EDf3A072d15dfe73155331C"),
    );
    m.insert(
        "eToro",
        address!("2953452dF5D7285b9a3a8a1E876A4bAcb09a976E"),
    );
    m.insert(
        "eXch.sc",
        address!("1681D536B8A47C01d7ea07f0D80A2eB10E7Ce842"),
    );
    m.insert(
        "eXch.sc_1",
        address!("f1dA173228fcf015F43f3eA15aBBB51f0d8f1123"),
    );
    m.insert(
        "xs2.exchange",
        address!("15C5312E24482547FF35899AFeDCAEB572ECB029"),
    );
    m.insert(
        "BlackRock ETHA",
        address!("0171F896002665C3ea3Ed0c55f21026cA0A734A0"),
    );
    m.insert(
        "BlackRock ETHA_1",
        address!("0195Bd0E9Dc6F98BD9bB3d0bFF69749a52065FA5"),
    );
    m.insert(
        "BlackRock ETHA_10",
        address!("14f175ba60C6be3087ac5ebe322396c99c2a2f54"),
    );
    m.insert(
        "BlackRock ETHA_100",
        address!("bD96dB1676C9B9136030C69A08eC93507917fCdB"),
    );
    m.insert(
        "BlackRock ETHA_101",
        address!("bE344dEcD5dE7798f54aa0e0159d494413585c51"),
    );
    m.insert(
        "BlackRock ETHA_102",
        address!("C074DD1D43342E797cdB680bb48aeF5E8f1b4B27"),
    );
    m.insert(
        "BlackRock ETHA_103",
        address!("c0b13bBe75717Db4218Ce8031276B7859c2114A1"),
    );
    m.insert(
        "BlackRock ETHA_104",
        address!("c1aC6dF64f397C324a8bf570fD12E59892b18cA7"),
    );
    m.insert(
        "BlackRock ETHA_105",
        address!("C2b40e86178d7611d4bF906f52B0174778a8aAA9"),
    );
    m.insert(
        "BlackRock ETHA_106",
        address!("c3Fe7c7084eA95730Dbc67DF66E509346D7e691a"),
    );
    m.insert(
        "BlackRock ETHA_107",
        address!("c4C0D26A53A1d9646055623447Dc66fE1aA3d2F6"),
    );
    m.insert(
        "BlackRock ETHA_108",
        address!("C5685D5dD92f9003cb5087fcDeE27e7401da6d71"),
    );
    m.insert(
        "BlackRock ETHA_109",
        address!("c89D09B6F6Cf1783D4baF0bb6eAf7E4EDF6b0e2C"),
    );
    m.insert(
        "BlackRock ETHA_11",
        address!("17939de7446C7cB703ab2ebDA1f63167f1b90693"),
    );
    m.insert(
        "BlackRock ETHA_110",
        address!("ca060AEad23c6ef6dBc8ca281D7275985d979AfB"),
    );
    m.insert(
        "BlackRock ETHA_111",
        address!("Cb9F54Ec8Cd14A31BFF10C9C843C843ed67eFa60"),
    );
    m.insert(
        "BlackRock ETHA_112",
        address!("Cf0C293a6da23A7006Be793B3320aaCB274dEe6E"),
    );
    m.insert(
        "BlackRock ETHA_113",
        address!("d1A599C4903eb195aCd0BB45f2AB400718AF0907"),
    );
    m.insert(
        "BlackRock ETHA_114",
        address!("d1dD34E87aEB9DC900c766d98b5376E41938bF03"),
    );
    m.insert(
        "BlackRock ETHA_115",
        address!("D546f97125f7969e4CA3BA44c6Af33a85a22B00c"),
    );
    m.insert(
        "BlackRock ETHA_116",
        address!("d56c29eb4F4B3468902dB986FCbeCEbe6FB7ADB4"),
    );
    m.insert(
        "BlackRock ETHA_117",
        address!("D6CcDACc837f00ec5125fc173ffD01BA7849cFB7"),
    );
    m.insert(
        "BlackRock ETHA_118",
        address!("D7ac899F6f76bE7278C70cc71B37e21350669743"),
    );
    m.insert(
        "BlackRock ETHA_119",
        address!("daeDD4f2150f547ebFd49459E7B6468CC90065a2"),
    );
    m.insert(
        "BlackRock ETHA_12",
        address!("1bFc9c727d03fEc405347E9ECc6A71AAfcED0c66"),
    );
    m.insert(
        "BlackRock ETHA_120",
        address!("DBFBAdBDFC83aD032E343A4dd3515F5e1241837e"),
    );
    m.insert(
        "BlackRock ETHA_121",
        address!("DC6205cD2A4BccF39f0f2C9106dE6Cb8f46Ffa03"),
    );
    m.insert(
        "BlackRock ETHA_122",
        address!("DCD47cF2DF8811272F43e3Eea2412F175cBb6A33"),
    );
    m.insert(
        "BlackRock ETHA_123",
        address!("dff29e9bCCa88222d2e32Ea5F72aCdd69B0BBD10"),
    );
    m.insert(
        "BlackRock ETHA_124",
        address!("e3a0E6ACFeb43F4E1Bf5be753088dDe379161273"),
    );
    m.insert(
        "BlackRock ETHA_125",
        address!("E3ad36592F502eF926e265E4252868F5FbA7aE51"),
    );
    m.insert(
        "BlackRock ETHA_126",
        address!("E740606845c3FB7F527195dceF4523C0b1966d0D"),
    );
    m.insert(
        "BlackRock ETHA_127",
        address!("e815C44b7Cab8A5f559B4080a5d0536830B6cf5b"),
    );
    m.insert(
        "BlackRock ETHA_128",
        address!("eb47E5353256E196687Eb0649b48EB14bc4C6c92"),
    );
    m.insert(
        "BlackRock ETHA_129",
        address!("eD75D85ca83D10Fd05F7640574b56cc1dAd5B5F0"),
    );
    m.insert(
        "BlackRock ETHA_13",
        address!("1Cad5f1359224003fFf72096e4d02EC0587e66a0"),
    );
    m.insert(
        "BlackRock ETHA_130",
        address!("ED9666354fEa95a24195EC40f27eF861Cf08B981"),
    );
    m.insert(
        "BlackRock ETHA_131",
        address!("eDbcd2E00bADCC3d3D09a8a2823A3f08c1879fe3"),
    );
    m.insert(
        "BlackRock ETHA_132",
        address!("EdF56687fd7304dD91b3bd8661F95497E2F9e9BC"),
    );
    m.insert(
        "BlackRock ETHA_133",
        address!("F08BF6Dc5406C7c3Ae579894DEefd5c13ea8933D"),
    );
    m.insert(
        "BlackRock ETHA_134",
        address!("f0b3e9c849fFE71EF66aCE92b3b4A4b6E20d9f73"),
    );
    m.insert(
        "BlackRock ETHA_135",
        address!("F3bd8F3381cDD795E9727789d0B7972EF8F5666E"),
    );
    m.insert(
        "BlackRock ETHA_136",
        address!("F4c008e48150b83Df538bc5B96b4EbFe77Fc8f96"),
    );
    m.insert(
        "BlackRock ETHA_137",
        address!("f55d07aad3864B34016e31674Aa7C5B3b2598A4D"),
    );
    m.insert(
        "BlackRock ETHA_138",
        address!("FD2001Ab71878C33C462e32FAD3b5Cbd7602f7ed"),
    );
    m.insert(
        "BlackRock ETHA_139",
        address!("fdFBE80e1a3b1E7df769054DB3aC89Bc3FCC8958"),
    );
    m.insert(
        "BlackRock ETHA_14",
        address!("1EbA5f237543CC08d67195649E9E06ACE6AA46B3"),
    );
    m.insert(
        "BlackRock ETHA_140",
        address!("ff99FaD518f0733765F28181252Dc1F479E7fA9a"),
    );
    m.insert(
        "BlackRock ETHA_15",
        address!("200d0D4ED39da71BBF2eAA86d7c0923041a8292e"),
    );
    m.insert(
        "BlackRock ETHA_16",
        address!("24efa0f9AA3C020e980Ab9CF5E396327e9D54CE5"),
    );
    m.insert(
        "BlackRock ETHA_17",
        address!("28c4f4082748A61Aa959f2f9FA8BE4e27E7Bb2a5"),
    );
    m.insert(
        "BlackRock ETHA_18",
        address!("296C93E66bCAEBCFFf0c873e54806964c0F63aED"),
    );
    m.insert(
        "BlackRock ETHA_19",
        address!("2a3e882057769d304c0F43e91aBD2c81e2470c41"),
    );
    m.insert(
        "BlackRock ETHA_2",
        address!("02d05BA91b77f664122E86cb42CAaE5eb4107144"),
    );
    m.insert(
        "BlackRock ETHA_20",
        address!("2C110C2a1eCDA251441B2CCea49Ca27910e92e5e"),
    );
    m.insert(
        "BlackRock ETHA_21",
        address!("2F1d3dA11eAbb07D33d185314B198abd4dF656f0"),
    );
    m.insert(
        "BlackRock ETHA_22",
        address!("2fd84202979DbAF4B835656b72ABde24C5652436"),
    );
    m.insert(
        "BlackRock ETHA_23",
        address!("3007C5F232A61206418a1d487ac86699eb71217F"),
    );
    m.insert(
        "BlackRock ETHA_24",
        address!("304426f628C96Ac4517738d6857ECFd6a0D00D72"),
    );
    m.insert(
        "BlackRock ETHA_25",
        address!("324be15Bcf22Ef6A0090e04d4164959d60E40EE1"),
    );
    m.insert(
        "BlackRock ETHA_26",
        address!("36d5868e7f1012eB45F6702B801AD6736edf6e26"),
    );
    m.insert(
        "BlackRock ETHA_27",
        address!("3877D4faDabDB513A370A4381bBB1509E60290bb"),
    );
    m.insert(
        "BlackRock ETHA_28",
        address!("3A8EFdA4f6bbE967fC4FDd6ff4742080Bc1DE3bF"),
    );
    m.insert(
        "BlackRock ETHA_29",
        address!("3EBBaD53F6d00210069BDedd7d9b5C9486b51ece"),
    );
    m.insert(
        "BlackRock ETHA_3",
        address!("059488Aa193371C47E5cc6e5Be7FCF0ed3404267"),
    );
    m.insert(
        "BlackRock ETHA_30",
        address!("4003399e77aF1f556455516dC983102351cdeb01"),
    );
    m.insert(
        "BlackRock ETHA_31",
        address!("40457f403f76fDA0213790EF3aB7a7AC23cC7F1D"),
    );
    m.insert(
        "BlackRock ETHA_32",
        address!("4244a400c55958de14D47064E89DC039105EF20f"),
    );
    m.insert(
        "BlackRock ETHA_33",
        address!("435449B156Cbb6473166d599Bb925BbAb6E26F96"),
    );
    m.insert(
        "BlackRock ETHA_34",
        address!("4dC9b37e794D0ec6c2faBf59b64A421BAdE92b82"),
    );
    m.insert(
        "BlackRock ETHA_35",
        address!("50Dd722DC8b2d735Fc36C992164612E2834Cf1E3"),
    );
    m.insert(
        "BlackRock ETHA_36",
        address!("52B19A80695A09D553D805877F437DDa5a18aC1e"),
    );
    m.insert(
        "BlackRock ETHA_37",
        address!("53071380278512C05A8DcbF253B43C69dB425f04"),
    );
    m.insert(
        "BlackRock ETHA_38",
        address!("54471c6357A90664f5Fda663619FF12C4942DB50"),
    );
    m.insert(
        "BlackRock ETHA_39",
        address!("5552Be2668E180621A916C81d382f746b3019094"),
    );
    m.insert(
        "BlackRock ETHA_4",
        address!("059551481976475ed8B62890F570457d61682d71"),
    );
    m.insert(
        "BlackRock ETHA_40",
        address!("5557c8c45B4f55016707cf22B6e54E9189A3FBd5"),
    );
    m.insert(
        "BlackRock ETHA_41",
        address!("56a6e187660B69ec36ed4b421742a74a442d6021"),
    );
    m.insert(
        "BlackRock ETHA_42",
        address!("59dF1ce7A32f89e58EF56090D7D81b163e6cCddb"),
    );
    m.insert(
        "BlackRock ETHA_43",
        address!("59e40b07fCB23E45335CFd367f516a69c081d179"),
    );
    m.insert(
        "BlackRock ETHA_44",
        address!("5AFB8D40C6577D4810214e4B80BD5317E53A33a0"),
    );
    m.insert(
        "BlackRock ETHA_45",
        address!("5C106A0Ea5f5c0a8D81Cf330e53F058fd3b770B7"),
    );
    m.insert(
        "BlackRock ETHA_46",
        address!("604Ce9c9236A6252b9D3b935423CF61b335CEcee"),
    );
    m.insert(
        "BlackRock ETHA_47",
        address!("624f8d573BFfDEbB3e54c3C9571308dc1b944179"),
    );
    m.insert(
        "BlackRock ETHA_48",
        address!("6277F7B2eb4ec35630b3b76917AB7C142dDF4445"),
    );
    m.insert(
        "BlackRock ETHA_49",
        address!("639d6101E9c65D352b85b2252D82A4AE02F701a9"),
    );
    m.insert(
        "BlackRock ETHA_5",
        address!("09AAB9E78A539e8a26cB693cA208A3c99F14Ef21"),
    );
    m.insert(
        "BlackRock ETHA_50",
        address!("669C35897857BA63cfF1cde9618Ee40bE3C04eA6"),
    );
    m.insert(
        "BlackRock ETHA_51",
        address!("679C31256DFf9FD6385AeA736012dF5A5F783Bde"),
    );
    m.insert(
        "BlackRock ETHA_52",
        address!("67e49F57DD2B69288BA7A898466824BE3C433137"),
    );
    m.insert(
        "BlackRock ETHA_53",
        address!("68d60195E97A75E85D13Dec327ffEFACc4cDB9fE"),
    );
    m.insert(
        "BlackRock ETHA_54",
        address!("716Dd77Bc1D1f89115E1FB8a7Cb6568a0561E941"),
    );
    m.insert(
        "BlackRock ETHA_55",
        address!("73800c8E828637a7A4319aeF28c6cf09e408af66"),
    );
    m.insert(
        "BlackRock ETHA_56",
        address!("79B86B98B1aA714E7CB834330980b8A0f270b37e"),
    );
    m.insert(
        "BlackRock ETHA_57",
        address!("7C07d3BEd88e23DA29A406Eb41bDA7Ae3B64747b"),
    );
    m.insert(
        "BlackRock ETHA_58",
        address!("7E2a9D3B486CaeEd30312f5A20b02bA7b77E8e8F"),
    );
    m.insert(
        "BlackRock ETHA_59",
        address!("7e8f035b613D8073E37FEAC146CDB87cCeff7198"),
    );
    m.insert(
        "BlackRock ETHA_6",
        address!("09c9f203E6E1BCC4EDE5c594771E9dA6937699Ee"),
    );
    m.insert(
        "BlackRock ETHA_60",
        address!("7fAE237f63992d32407cc74B4Ad5C495f5709996"),
    );
    m.insert(
        "BlackRock ETHA_61",
        address!("81127954046a1aBa305F2b562ba1ad7129D73c1D"),
    );
    m.insert(
        "BlackRock ETHA_62",
        address!("85152a72AA24A3D0b667aDF9e81624925043bFbe"),
    );
    m.insert(
        "BlackRock ETHA_63",
        address!("862A495cE4487C56F40A338dE0b36b4c89594d6B"),
    );
    m.insert(
        "BlackRock ETHA_64",
        address!("878D0d68D260E265Eb051688375b50b5DAf7e2b0"),
    );
    m.insert(
        "BlackRock ETHA_65",
        address!("88c0484300e9BDba36A1F82C15421Bf2c86e0A65"),
    );
    m.insert(
        "BlackRock ETHA_66",
        address!("89F7DA9cD853dD055C1f7BF4CF15327bf537469b"),
    );
    m.insert(
        "BlackRock ETHA_67",
        address!("8A4536dc0719896838C5a203b1e1b45359151339"),
    );
    m.insert(
        "BlackRock ETHA_68",
        address!("8A5Cf0c48A35A1035d03fE2DF53d80062f631ae4"),
    );
    m.insert(
        "BlackRock ETHA_69",
        address!("8C140be941D4888C035F2316185A214016F6DE2f"),
    );
    m.insert(
        "BlackRock ETHA_7",
        address!("0Ab743140Fb2F788DA56f4569a012D049cdca41e"),
    );
    m.insert(
        "BlackRock ETHA_70",
        address!("8D1197Ba508d95D8d1921aAf525F1BF1eA983e25"),
    );
    m.insert(
        "BlackRock ETHA_71",
        address!("8d1faAbaF2A5d0FF33E6A74c6bE290a60f8550E0"),
    );
    m.insert(
        "BlackRock ETHA_72",
        address!("8d3aF64C5919735069f46Af6518Dc8654D5Fad8d"),
    );
    m.insert(
        "BlackRock ETHA_73",
        address!("8fAc44E97E90312fCe3254cff74FD61F0Fe1e7E8"),
    );
    m.insert(
        "BlackRock ETHA_74",
        address!("906DdD6005407a1252999726676dcd71b3E76e7f"),
    );
    m.insert(
        "BlackRock ETHA_75",
        address!("944B076bb5DC1F2aC3921b6Df035432d1A05B707"),
    );
    m.insert(
        "BlackRock ETHA_76",
        address!("95f120A2aD5448041c1910Fc36d60380c24CD4Fd"),
    );
    m.insert(
        "BlackRock ETHA_77",
        address!("9645edD5BD30b6fB9447A17FAaA029056e6AD329"),
    );
    m.insert(
        "BlackRock ETHA_78",
        address!("97505Eb589b9Cb68C9f993f20081009d05cb0E8C"),
    );
    m.insert(
        "BlackRock ETHA_79",
        address!("9C2e2D46234C1C3B690202cFA6098F85d71E84Bb"),
    );
    m.insert(
        "BlackRock ETHA_8",
        address!("1353B50091b6A5E3643d3b08255F794eC8579955"),
    );
    m.insert(
        "BlackRock ETHA_80",
        address!("a0D402f74923793b0e9CC42Af0928323bfad55b1"),
    );
    m.insert(
        "BlackRock ETHA_81",
        address!("A1B84DbADee6E535e38c9Aef545C037A5f60e84e"),
    );
    m.insert(
        "BlackRock ETHA_82",
        address!("A2858077cddfC3b2FA16ea17B4121717915651C1"),
    );
    m.insert(
        "BlackRock ETHA_83",
        address!("A29EdF3c4d7349709F57fb5Affb1Ea7CeCEE1B7f"),
    );
    m.insert(
        "BlackRock ETHA_84",
        address!("a2cbd7bA4b5767E7EdA25ECCA1ce5b65e2C9409b"),
    );
    m.insert(
        "BlackRock ETHA_85",
        address!("a95a23115B0aB94182C5893E1A45140F0F3B6D53"),
    );
    m.insert(
        "BlackRock ETHA_86",
        address!("aB9566c24dF471eaF433FFBAF179f464A557D94E"),
    );
    m.insert(
        "BlackRock ETHA_87",
        address!("ad53B2177d517772797aD502fC436216Ffe0426F"),
    );
    m.insert(
        "BlackRock ETHA_88",
        address!("aEBFf1fF9d5b0c3E646d790c1d5F0C16c8afaa6D"),
    );
    m.insert(
        "BlackRock ETHA_89",
        address!("B05cB0758F1E3b59583dd19d5Cc147D082794BB7"),
    );
    m.insert(
        "BlackRock ETHA_9",
        address!("135427379848c023DE50Ed551E4FA4f192F2D376"),
    );
    m.insert(
        "BlackRock ETHA_90",
        address!("B0781d1b70386B4aa5206c3565ade5dF128bE161"),
    );
    m.insert(
        "BlackRock ETHA_91",
        address!("b0F78b3e230e22EF85eB759BB73A04614F5Bb1eb"),
    );
    m.insert(
        "BlackRock ETHA_92",
        address!("b3569Eb2b6Bf220814308fBB407f24144F45368E"),
    );
    m.insert(
        "BlackRock ETHA_93",
        address!("B402F0b9D277B6C74f2d8503641B86E19Eb42321"),
    );
    m.insert(
        "BlackRock ETHA_94",
        address!("b437F43Ad845F966fE18950C4C540BEf26802fD3"),
    );
    m.insert(
        "BlackRock ETHA_95",
        address!("b4C0C83645411a2d40f51735B33371D018e2BFe5"),
    );
    m.insert(
        "BlackRock ETHA_96",
        address!("b79E9b4e6E280Fb913Aa03F8D11ed11e30D9640C"),
    );
    m.insert(
        "BlackRock ETHA_97",
        address!("b85BCb14f0F305106b9B9B61E33bFD4b14169DED"),
    );
    m.insert(
        "BlackRock ETHA_98",
        address!("BAE3828911e90331325B7D07b07290cEB4Ec4b3e"),
    );
    m.insert(
        "BlackRock ETHA_99",
        address!("BcbC56D24595a231fC8DFD2a2aA1cdC3683A5138"),
    );
    m.insert(
        "Grayscale ETHE",
        address!("0007EaB78C7C2cCa63D70a3A0E3658D9FAF4506F"),
    );
    m.insert(
        "Grayscale ETHE_1",
        address!("004cd91081DC9dfF0B9B75274beCcc97B5db3c36"),
    );
    m.insert(
        "Grayscale ETHE_10",
        address!("0380EA7994E22dc7503BAa40BA5ab206F6A55Dab"),
    );
    m.insert(
        "Grayscale ETHE_100",
        address!("1aAAE4c7ff3fc9b3b5c007802B4A1229c1ecf41E"),
    );
    m.insert(
        "Grayscale ETHE_101",
        address!("1B3cC8C6ED708C7cB4dd5AfB97543538E176BBe3"),
    );
    m.insert(
        "Grayscale ETHE_102",
        address!("1b3e055112ce117156a72cAE967A2e5b7C4c9bbF"),
    );
    m.insert(
        "Grayscale ETHE_103",
        address!("1b964d30C2Da31024274e79cbE32aE6CbE7ad198"),
    );
    m.insert(
        "Grayscale ETHE_104",
        address!("1cB95602b86c90731A9145082BF0a55f2dca9124"),
    );
    m.insert(
        "Grayscale ETHE_105",
        address!("1E15580cB4993D473939dAD2B9Caa35dD25E09C7"),
    );
    m.insert(
        "Grayscale ETHE_106",
        address!("1EFEE223012896fe0C66359d30908Efd964cDfAC"),
    );
    m.insert(
        "Grayscale ETHE_107",
        address!("1F4a0A7c953397F6d486AcdB7271890D1e5426B7"),
    );
    m.insert(
        "Grayscale ETHE_108",
        address!("1F51cA435E893FD90e4B4Ceb559C7D97C63ddF10"),
    );
    m.insert(
        "Grayscale ETHE_109",
        address!("1F5bA07332a800BeEd1F90931b7ea378692C5aa0"),
    );
    m.insert(
        "Grayscale ETHE_11",
        address!("0391A9EBE1Fe3d350bed011f76dC12A246569E9D"),
    );
    m.insert(
        "Grayscale ETHE_110",
        address!("1f7E68EB933d18180445ef374C1286232f4ad73F"),
    );
    m.insert(
        "Grayscale ETHE_111",
        address!("1f7f3F97570e2ab1334816Ec0989c2b3a559FA97"),
    );
    m.insert(
        "Grayscale ETHE_112",
        address!("1f9A59E61C7AA9cac90d2B38Cf5AD4964F6F9423"),
    );
    m.insert(
        "Grayscale ETHE_113",
        address!("1Fb0B7f567d4bFF19b3b3ba22538eD7B0934Cbe2"),
    );
    m.insert(
        "Grayscale ETHE_114",
        address!("1fC2bE992CB1CDd714df3Ac9e4bdEF80ED6F0F69"),
    );
    m.insert(
        "Grayscale ETHE_115",
        address!("1Fcb40ff0a6A0bF8e9e473101C56af9a2CAcc187"),
    );
    m.insert(
        "Grayscale ETHE_116",
        address!("1FdD50659FD48c748DF039Dc32420Cd0e8C79CdB"),
    );
    m.insert(
        "Grayscale ETHE_117",
        address!("1Fe729426Df3d04c033cD59771d2Ac4e1e5E046A"),
    );
    m.insert(
        "Grayscale ETHE_118",
        address!("2060A3d40fe510A38cd2861273C5C02262542803"),
    );
    m.insert(
        "Grayscale ETHE_119",
        address!("20A29F464Eed22b688D9F33B43Be99052Dac13fa"),
    );
    m.insert(
        "Grayscale ETHE_12",
        address!("03a39109B2bF39D5Da499dDCF6774d9FE1490924"),
    );
    m.insert(
        "Grayscale ETHE_120",
        address!("20C985e854AD71A9E22829E34c25C83CF992FdDc"),
    );
    m.insert(
        "Grayscale ETHE_121",
        address!("2116ec8b0ba52F1e40c63ba10e40a823bE68D91D"),
    );
    m.insert(
        "Grayscale ETHE_122",
        address!("212370f0a209C0E76d278A1ED7528CA18107487E"),
    );
    m.insert(
        "Grayscale ETHE_123",
        address!("21692f8E636C3a79f7eBc8fb9df787f1c5A21a1a"),
    );
    m.insert(
        "Grayscale ETHE_124",
        address!("2194294C661f901BD0375098eb345cE16BA7889F"),
    );
    m.insert(
        "Grayscale ETHE_125",
        address!("21f2f89FDA0c9081c80679aaec61FCA118410d95"),
    );
    m.insert(
        "Grayscale ETHE_126",
        address!("220c575a00A9F62B6Aae2Ea2A484971C5Fa3Cdb0"),
    );
    m.insert(
        "Grayscale ETHE_127",
        address!("225a75e90b76fb1825D82e7Bb67691f0e45aF026"),
    );
    m.insert(
        "Grayscale ETHE_128",
        address!("2298937De1d04b818e74Fc13273caA6476186204"),
    );
    m.insert(
        "Grayscale ETHE_129",
        address!("22a456f05857acd2A78e6cB1067bBd62c68BF9c1"),
    );
    m.insert(
        "Grayscale ETHE_13",
        address!("03AcB8C8F020CC9be44ef7639C1e909262381d93"),
    );
    m.insert(
        "Grayscale ETHE_130",
        address!("23042bC51e20d5d376A9de40b47117751B043B5e"),
    );
    m.insert(
        "Grayscale ETHE_131",
        address!("23a6f70Cf131F17c9b587A8F4a0B19C81189B29e"),
    );
    m.insert(
        "Grayscale ETHE_132",
        address!("23A84cA03bB0e7Fd7897426250eEcFE68493a09f"),
    );
    m.insert(
        "Grayscale ETHE_133",
        address!("23Aa93e55bbbb81cEA3ad02FCb54aFD5304e99Cf"),
    );
    m.insert(
        "Grayscale ETHE_134",
        address!("23c45a0456B25DAa2eBC25A15c2b604bA4Ff7e4D"),
    );
    m.insert(
        "Grayscale ETHE_135",
        address!("23C912DcFBFaAf9c502fB478428cB221BEB64ED9"),
    );
    m.insert(
        "Grayscale ETHE_136",
        address!("2426457F8d67DF33AcaB7905C3d13f3F30c17593"),
    );
    m.insert(
        "Grayscale ETHE_137",
        address!("242B7b0cdacF427C92ECdceC292084de7A0993Ba"),
    );
    m.insert(
        "Grayscale ETHE_138",
        address!("2452A774CD4B7c3FEd1b630A9b4b5f85a9194918"),
    );
    m.insert(
        "Grayscale ETHE_139",
        address!("245a01D2f879bc75909F0e86032F9236cf3b1DbA"),
    );
    m.insert(
        "Grayscale ETHE_14",
        address!("042d21E41044af8C7a8643F01aecBc7Eb7908463"),
    );
    m.insert(
        "Grayscale ETHE_140",
        address!("247cb643F8828Dcb8F5742023A608579667d3eD7"),
    );
    m.insert(
        "Grayscale ETHE_141",
        address!("2486Fcc85CB7D96dc80114341E9151c81d15084E"),
    );
    m.insert(
        "Grayscale ETHE_142",
        address!("24a52ECE417147f1a7AD7F11ad572900ab8047B6"),
    );
    m.insert(
        "Grayscale ETHE_143",
        address!("24A9a53931193bcA84e1683474a5180fa8ee71Bd"),
    );
    m.insert(
        "Grayscale ETHE_144",
        address!("24DEE6705A9E7368522F35119f128de1e178F2cC"),
    );
    m.insert(
        "Grayscale ETHE_145",
        address!("2555EF4fE1005af6d870B4aF4cB39521326FF67E"),
    );
    m.insert(
        "Grayscale ETHE_146",
        address!("2589329Ef7FBB1776d56E3FdbCAd19797FeB8a81"),
    );
    m.insert(
        "Grayscale ETHE_147",
        address!("25E50cAb2185963446dfBe85CBbBdcE47083bCFB"),
    );
    m.insert(
        "Grayscale ETHE_148",
        address!("268e811fA91b55FFb6E92907C6515909ff23EEa0"),
    );
    m.insert(
        "Grayscale ETHE_149",
        address!("26a18533DFa961Bc2FD550184Fe7e256925Cc3D4"),
    );
    m.insert(
        "Grayscale ETHE_15",
        address!("049373187fb5783CFDdC6c7cEA8Fb6E3D426Df92"),
    );
    m.insert(
        "Grayscale ETHE_150",
        address!("26aA2057D926137f9a23Fe2EA53f49a349D07a6c"),
    );
    m.insert(
        "Grayscale ETHE_151",
        address!("26D9ff75372192E206afCdF8737784508311E322"),
    );
    m.insert(
        "Grayscale ETHE_152",
        address!("278Ad2617A315a184Dec06b7d57d9fa84034cadb"),
    );
    m.insert(
        "Grayscale ETHE_153",
        address!("27C9Fa9009bea9aD283147D778eb306C8e670759"),
    );
    m.insert(
        "Grayscale ETHE_154",
        address!("280E4582269826E0432E2bD545113C610Fb590C0"),
    );
    m.insert(
        "Grayscale ETHE_155",
        address!("2825410c632851d9a8590BCf57C799c56DD4BaD1"),
    );
    m.insert(
        "Grayscale ETHE_156",
        address!("2831a789f373506F93Ee4B424a6d5C07510891B7"),
    );
    m.insert(
        "Grayscale ETHE_157",
        address!("2832dDe931aa43385d12A4b542433d5cEA605268"),
    );
    m.insert(
        "Grayscale ETHE_158",
        address!("288a51ea448d4505178410c1Fbf768d7F5C1E1Cf"),
    );
    m.insert(
        "Grayscale ETHE_159",
        address!("28ba2AcEc9F711FB89a2B16c76AE842891d212c3"),
    );
    m.insert(
        "Grayscale ETHE_16",
        address!("04C1c1125338A392251a51bcdFfeCe74669766DF"),
    );
    m.insert(
        "Grayscale ETHE_160",
        address!("28BB7d15E9cc8b1c07Cc0fb16C7b9C4896A68C1C"),
    );
    m.insert(
        "Grayscale ETHE_161",
        address!("291b1F8A326A4dBD3c89f142cBeDCDc914cE57ea"),
    );
    m.insert(
        "Grayscale ETHE_162",
        address!("2975bd4909ac4bB879c05712B2774098Af3eAc0A"),
    );
    m.insert(
        "Grayscale ETHE_163",
        address!("297627e1DDD26b53601C416DBf98bE27DAF89d58"),
    );
    m.insert(
        "Grayscale ETHE_164",
        address!("2A53f7cFF1a204698013412EC5a1b98A9D3deAb1"),
    );
    m.insert(
        "Grayscale ETHE_165",
        address!("2aCadb3aDd5D074017EB03a705156c329eB51b99"),
    );
    m.insert(
        "Grayscale ETHE_166",
        address!("2b892d1A18a1881bEC419F5478FE8c565Ab50928"),
    );
    m.insert(
        "Grayscale ETHE_167",
        address!("2B93c057F1Ec01926FA4feB75310510f807c415D"),
    );
    m.insert(
        "Grayscale ETHE_168",
        address!("2B9a19334c5027717A18C31B8A53038dEc6Bab25"),
    );
    m.insert(
        "Grayscale ETHE_169",
        address!("2bac1984fEa7E034F3AC67dC385Ca2Fa0fCe406A"),
    );
    m.insert(
        "Grayscale ETHE_17",
        address!("04D9cc35d5bf408A7d442fB45d235667144E4D92"),
    );
    m.insert(
        "Grayscale ETHE_170",
        address!("2bce681Cb725bB848EC82c28F848AB1454073471"),
    );
    m.insert(
        "Grayscale ETHE_171",
        address!("2bED9992D06da80d19d2256759d58873fE5f8006"),
    );
    m.insert(
        "Grayscale ETHE_172",
        address!("2C2D62573a4D304Af961400e57F4959Ac14Cc367"),
    );
    m.insert(
        "Grayscale ETHE_173",
        address!("2D14297b7ebc1c16CE9D09247C2287aFa454046F"),
    );
    m.insert(
        "Grayscale ETHE_174",
        address!("2D609485B0Dbe2721ae07981f9AF3A51fEA02aB9"),
    );
    m.insert(
        "Grayscale ETHE_175",
        address!("2D9A1D603FdF2C5Ed5674fac9Cd05231EAC6136E"),
    );
    m.insert(
        "Grayscale ETHE_176",
        address!("2E0697ef55Ec6727AD0157FB8aF5214D14b660F9"),
    );
    m.insert(
        "Grayscale ETHE_177",
        address!("2e2474172B94BcdC5bADB69bC66A2D80D01930f8"),
    );
    m.insert(
        "Grayscale ETHE_178",
        address!("2ec777B09DFB9B3aECCdEF1ecEf8904cAAf7E04A"),
    );
    m.insert(
        "Grayscale ETHE_179",
        address!("2f4F99ed52a663439fe23cFe1CbA1E81d13609dA"),
    );
    m.insert(
        "Grayscale ETHE_18",
        address!("050f910aE80022f5B1dF75B6907584090DAf8e94"),
    );
    m.insert(
        "Grayscale ETHE_180",
        address!("3049B8Da81E2e0D45daD156fD17672c96C23c4c5"),
    );
    m.insert(
        "Grayscale ETHE_181",
        address!("305f81C7724c3A9B32833E55ac8A11407BCfB5E4"),
    );
    m.insert(
        "Grayscale ETHE_182",
        address!("3088c51b1AaA6D647bEef74C5e2f7E0DF7078E80"),
    );
    m.insert(
        "Grayscale ETHE_183",
        address!("30A02Abf6421957dB3c25aB4baC048cBf4807b1a"),
    );
    m.insert(
        "Grayscale ETHE_184",
        address!("30c1e8a2767eD5589C70FA3647A82e4390b2ef6A"),
    );
    m.insert(
        "Grayscale ETHE_185",
        address!("3114ABdDd2025156519fB8DcFD06137767DA8E94"),
    );
    m.insert(
        "Grayscale ETHE_186",
        address!("31435AeD74efe468918932019550245a971A76A7"),
    );
    m.insert(
        "Grayscale ETHE_187",
        address!("31e102Be69a7400afb6076c73159Ac86a1F32079"),
    );
    m.insert(
        "Grayscale ETHE_188",
        address!("3208A54432413C3D8f55566Ca9C8C79B4f73aFdD"),
    );
    m.insert(
        "Grayscale ETHE_189",
        address!("327122cf542F9CcCA02792154F50A46fb430eC6A"),
    );
    m.insert(
        "Grayscale ETHE_19",
        address!("05434aD86F21df497f857405Fe6714F2894aa66C"),
    );
    m.insert(
        "Grayscale ETHE_190",
        address!("3290dCd945a66b0fe1c21f2252c9F9e229208F05"),
    );
    m.insert(
        "Grayscale ETHE_191",
        address!("33003EF1d6EF205e280c9529096c28D56958F715"),
    );
    m.insert(
        "Grayscale ETHE_192",
        address!("334196e498a17aCDDFC0D74e43f6f53088E66e9d"),
    );
    m.insert(
        "Grayscale ETHE_193",
        address!("33d7bae275F7F92F55bA569113220520C930BfBf"),
    );
    m.insert(
        "Grayscale ETHE_194",
        address!("33fEf238DC09BDDfdc733B42451F98276E5E5985"),
    );
    m.insert(
        "Grayscale ETHE_195",
        address!("34138Ddf9aD45B73EBeDe5D4a95dFa5b03AE045e"),
    );
    m.insert(
        "Grayscale ETHE_196",
        address!("341A069071cde88b3491f7C4934f34Cd51100406"),
    );
    m.insert(
        "Grayscale ETHE_197",
        address!("3442D55E7a7b76d38Dd4f2F599744A208aDe587e"),
    );
    m.insert(
        "Grayscale ETHE_198",
        address!("347C4C0b7cDFEAbeed74B0d3Ac81A107008e9579"),
    );
    m.insert(
        "Grayscale ETHE_199",
        address!("34B9A6d48F527538E26Ca68aef8e868150A69d79"),
    );
    m.insert(
        "Grayscale ETHE_2",
        address!("00c79701f58C8bD9d31Ab54d8ce7344063372049"),
    );
    m.insert(
        "Grayscale ETHE_20",
        address!("055C8EBC495BC1363260544312CFAa9615f7Fc14"),
    );
    m.insert(
        "Grayscale ETHE_200",
        address!("34C8Dc4bB3DC940f7b126c4417634E4dA7218383"),
    );
    m.insert(
        "Grayscale ETHE_201",
        address!("353A2D0453D88F8b88a1F14aFB97d6165f244D42"),
    );
    m.insert(
        "Grayscale ETHE_202",
        address!("35BdcA281f47f76bDc83d321a611EaBe5648284c"),
    );
    m.insert(
        "Grayscale ETHE_203",
        address!("362319Ca486b30551Df8d9F4bB763ED179c74B39"),
    );
    m.insert(
        "Grayscale ETHE_204",
        address!("363E8c2b87368c0dffB5B08c99057a07fB375A9F"),
    );
    m.insert(
        "Grayscale ETHE_205",
        address!("3666f19bc40a5b513DF91DE8461F5f3a29E26d95"),
    );
    m.insert(
        "Grayscale ETHE_206",
        address!("36e5c4B77138F0C6386eF969225A005C28BCdA63"),
    );
    m.insert(
        "Grayscale ETHE_207",
        address!("375ab28414204fcefb6E1d9ad4f2197FB29D9374"),
    );
    m.insert(
        "Grayscale ETHE_208",
        address!("3775234F636d6b2F56549FE60adEeB58eCCF504c"),
    );
    m.insert(
        "Grayscale ETHE_209",
        address!("37F21816c8F1770b6a208530375CB864fB70FDC8"),
    );
    m.insert(
        "Grayscale ETHE_21",
        address!("0569FfdE3ed2f3802cBec6106014532a5DE8b191"),
    );
    m.insert(
        "Grayscale ETHE_210",
        address!("3819bD05a08A486EA3B85C0C82B9B0eEe6cB8465"),
    );
    m.insert(
        "Grayscale ETHE_211",
        address!("3846AEd50a2Ed5959A4ea897C32C97FC70881561"),
    );
    m.insert(
        "Grayscale ETHE_212",
        address!("38FBA0ab545C09E9B5E55F0eDE29887dEE8e0014"),
    );
    m.insert(
        "Grayscale ETHE_213",
        address!("3903c22A77219539561285CB317ad592083f8a98"),
    );
    m.insert(
        "Grayscale ETHE_214",
        address!("3922Af37DF33aBf27782dF75Da64CC618E6CccF0"),
    );
    m.insert(
        "Grayscale ETHE_215",
        address!("393930790d3b7202E1BEAdC458A48Ff7Ee4504BD"),
    );
    m.insert(
        "Grayscale ETHE_216",
        address!("3a113a33daB7996937b91FF986280eAF1708Cd36"),
    );
    m.insert(
        "Grayscale ETHE_217",
        address!("3a2E57560dA206AE0535C5fE98F5bF233b304b02"),
    );
    m.insert(
        "Grayscale ETHE_218",
        address!("3A2F53cc8c2A0013CdDB0b95f7676FDc214EF372"),
    );
    m.insert(
        "Grayscale ETHE_219",
        address!("3AD96760650139e59EC67f46C763A808fFB9b3BB"),
    );
    m.insert(
        "Grayscale ETHE_22",
        address!("059B1b6A0a60412c015CE60Bf4bC63681F96Bf78"),
    );
    m.insert(
        "Grayscale ETHE_220",
        address!("3B102E02Ce180f2f3435d44c6Be597Ca0094DB1d"),
    );
    m.insert(
        "Grayscale ETHE_221",
        address!("3b333b621B2b94e217CFA63d046698a4d99cB399"),
    );
    m.insert(
        "Grayscale ETHE_222",
        address!("3B613F088ED67B239df4ea5468F00ce66B2B65bb"),
    );
    m.insert(
        "Grayscale ETHE_223",
        address!("3B8D07fCb7c236c150177E2791b457270E7C85bF"),
    );
    m.insert(
        "Grayscale ETHE_224",
        address!("3c0C3a1df6EF1320006AaCf1792489503467a238"),
    );
    m.insert(
        "Grayscale ETHE_225",
        address!("3D458cB4144Dab393Ad1D9f90A94Bc00cd49dEA3"),
    );
    m.insert(
        "Grayscale ETHE_226",
        address!("3db3769d4f39A6a7A0948CF8b5D872E29353A9EA"),
    );
    m.insert(
        "Grayscale ETHE_227",
        address!("3DC12115d9D30Be692a169aE75deada74EEAcD69"),
    );
    m.insert(
        "Grayscale ETHE_228",
        address!("3DcD969EE88d1a5B3F5bD90eBC29253128647740"),
    );
    m.insert(
        "Grayscale ETHE_229",
        address!("3E234c9B04ec6B1Aff78B7E9016577C27347DeDe"),
    );
    m.insert(
        "Grayscale ETHE_23",
        address!("0650E9308850A44300De044474dBaDd8fc38026c"),
    );
    m.insert(
        "Grayscale ETHE_230",
        address!("3E639A3240aa81AE91b8eDb391c66745CCd89573"),
    );
    m.insert(
        "Grayscale ETHE_231",
        address!("3Ef2411dE739fFeF2faDbFe33E10C6D13E67Ad6B"),
    );
    m.insert(
        "Grayscale ETHE_232",
        address!("3f3CFC8aB9189C0871e32Aa59388BcbCD5958e59"),
    );
    m.insert(
        "Grayscale ETHE_233",
        address!("3F41d650A3Ce99BAa2832f120BF5f07123Ed5D69"),
    );
    m.insert(
        "Grayscale ETHE_234",
        address!("3F45923b8a5d2c61Ce78aD8BE656532C5948A135"),
    );
    m.insert(
        "Grayscale ETHE_235",
        address!("3Fd4d26F176D1d9F9b6b4330b7F67C02D3a377E7"),
    );
    m.insert(
        "Grayscale ETHE_236",
        address!("404F7C0F8c11bF2d797918D8E867cD7254c6C0B2"),
    );
    m.insert(
        "Grayscale ETHE_237",
        address!("4069D5D58Ac8A6C8610094bEF5F165Acd92E70DB"),
    );
    m.insert(
        "Grayscale ETHE_238",
        address!("40A293f290aF3B8985fe0Dc0a0041323c220BE81"),
    );
    m.insert(
        "Grayscale ETHE_239",
        address!("40D398E2D1471a986A635de2e3C4099b8D368CE3"),
    );
    m.insert(
        "Grayscale ETHE_24",
        address!("06aB3A15E0E2012d5fDB2A5714f965c511a8aF32"),
    );
    m.insert(
        "Grayscale ETHE_240",
        address!("40E4cF00f021B5dCeE278f6619BFe966ba8A798d"),
    );
    m.insert(
        "Grayscale ETHE_241",
        address!("41144f86957101935C62E786b7Fea3a1D71bf111"),
    );
    m.insert(
        "Grayscale ETHE_242",
        address!("41958b04337651C73350415B1950B2049d69b40B"),
    );
    m.insert(
        "Grayscale ETHE_243",
        address!("41Ba4da465F2c9de9AdF70FB7A91E510778E9Af7"),
    );
    m.insert(
        "Grayscale ETHE_244",
        address!("421d841b17A3408A58178EF26e12bC41723E3766"),
    );
    m.insert(
        "Grayscale ETHE_245",
        address!("427644A3B26406fcf0693a7eD5377c9cD75AE6eb"),
    );
    m.insert(
        "Grayscale ETHE_246",
        address!("42917f0799F89366C54f5E7F50Cf1BF1046387e8"),
    );
    m.insert(
        "Grayscale ETHE_247",
        address!("430e4CdD77b8124B82188ebAC0359F5577aaB4E9"),
    );
    m.insert(
        "Grayscale ETHE_248",
        address!("43435522Fa02173d9d49D6495C236AAb95Db1d3D"),
    );
    m.insert(
        "Grayscale ETHE_249",
        address!("4368931747571cb8c28b4DD1183ef5db5a51A2C8"),
    );
    m.insert(
        "Grayscale ETHE_25",
        address!("06b8Ad078becC1B07BfD46aF32eff499446801f6"),
    );
    m.insert(
        "Grayscale ETHE_250",
        address!("44226D4869500dD3b01E3F20b77F7e725Fc81c03"),
    );
    m.insert(
        "Grayscale ETHE_251",
        address!("44AfAd2e378b2F37c4bA244621cCD74eFFF8f826"),
    );
    m.insert(
        "Grayscale ETHE_252",
        address!("44De51742Cd194040AEb156A13E40f81383ea92E"),
    );
    m.insert(
        "Grayscale ETHE_253",
        address!("44e1EaA71B565BeD26ffF68AdCae17D5b5daAA96"),
    );
    m.insert(
        "Grayscale ETHE_254",
        address!("44e8039CDDFf2C5aE124D47212999ce32D4e4430"),
    );
    m.insert(
        "Grayscale ETHE_255",
        address!("4597306b1ccFA70aF7C2B479c3d7C6246566D5f7"),
    );
    m.insert(
        "Grayscale ETHE_256",
        address!("45c9a1C0CB059Ef87D1d9A935f9d980951e29d47"),
    );
    m.insert(
        "Grayscale ETHE_257",
        address!("45d02905C31a6E9F6D26Ef3d998ed7BAFC91192D"),
    );
    m.insert(
        "Grayscale ETHE_258",
        address!("45E18f9E117F2bCB0eFAD51ae80fC2B7B6FB8344"),
    );
    m.insert(
        "Grayscale ETHE_259",
        address!("46556c2151bD4602830D48697d7FB964cDBc11b1"),
    );
    m.insert(
        "Grayscale ETHE_26",
        address!("06f87b41d828FA6AAE07C1aC85F46E827f456977"),
    );
    m.insert(
        "Grayscale ETHE_260",
        address!("4675bE7bFd711281Ba0074D373115FF2DDe31241"),
    );
    m.insert(
        "Grayscale ETHE_261",
        address!("47895E5009Fd609F0027eA896bb5C38869082223"),
    );
    m.insert(
        "Grayscale ETHE_262",
        address!("47f4D5c338A6853c89DE9dcE11ba78361b3de18E"),
    );
    m.insert(
        "Grayscale ETHE_263",
        address!("481C9CefeCA4d50fB31FB32D852b0838B44C2449"),
    );
    m.insert(
        "Grayscale ETHE_264",
        address!("482D65A58E8722025022AB68dE8F91C6136Dd861"),
    );
    m.insert(
        "Grayscale ETHE_265",
        address!("48dA64CE8dfDc9300E5d91c4C32115c2BcA6b505"),
    );
    m.insert(
        "Grayscale ETHE_266",
        address!("48E2Dd5c9bDe84916BC1DbAA976AC1cf2a8D7033"),
    );
    m.insert(
        "Grayscale ETHE_267",
        address!("48e8EA216e2cc0aC022EAcdED7C0fF7cC981C31c"),
    );
    m.insert(
        "Grayscale ETHE_268",
        address!("48FAd849Cec1aE0E945D4E0E9abE6253cB83cc40"),
    );
    m.insert(
        "Grayscale ETHE_269",
        address!("491d2F947369A24915FF2b05324CDFAA1b892e7c"),
    );
    m.insert(
        "Grayscale ETHE_27",
        address!("072c9C1c0660abA4d08C13e29A97F7Bb175918F6"),
    );
    m.insert(
        "Grayscale ETHE_270",
        address!("4976e9Df99b55D2334A920794d27bE00D9ac8423"),
    );
    m.insert(
        "Grayscale ETHE_271",
        address!("498441a9ccd6cDcB8374D0Af6024D99311167cD2"),
    );
    m.insert(
        "Grayscale ETHE_272",
        address!("4989d7983057fE34E5Ff0E33DFda49629c9D17f6"),
    );
    m.insert(
        "Grayscale ETHE_273",
        address!("49B7548Fb65eb35166475CdB61a5C4e893ceCa1F"),
    );
    m.insert(
        "Grayscale ETHE_274",
        address!("49F2702a89933Dfb14E9A09852CADc291688FE38"),
    );
    m.insert(
        "Grayscale ETHE_275",
        address!("4a063332cCc4C57EbBe62439b11E389Ee83Ccdff"),
    );
    m.insert(
        "Grayscale ETHE_276",
        address!("4A6a321BD349FfFd9f377bE00D3b4122423bAc87"),
    );
    m.insert(
        "Grayscale ETHE_277",
        address!("4A80c83a138fb21f54cb1B55d0FfcC6254bb53dd"),
    );
    m.insert(
        "Grayscale ETHE_278",
        address!("4aaa7c8D86Df45c2b4c2d4487857f203d86A1b0d"),
    );
    m.insert(
        "Grayscale ETHE_279",
        address!("4b1e34C92DA600Ce8da4B9FC5202d17dcA343671"),
    );
    m.insert(
        "Grayscale ETHE_28",
        address!("075b440D4D92AAf2B66f1365B81a508eCd25a40D"),
    );
    m.insert(
        "Grayscale ETHE_280",
        address!("4b3Ab8E184Ca2B02034A379f1ad8ee0CB85FBBd2"),
    );
    m.insert(
        "Grayscale ETHE_281",
        address!("4B6bC43C0BC9B671227988f8c25422bfF7306D2D"),
    );
    m.insert(
        "Grayscale ETHE_282",
        address!("4BBD78A6DBEA8dC5F5B0cA5117c82E3cC0Eb5875"),
    );
    m.insert(
        "Grayscale ETHE_283",
        address!("4c0179d931E8F23C75bE25912E891e15630aAB12"),
    );
    m.insert(
        "Grayscale ETHE_284",
        address!("4c1128880d330F5E0BB7D702b9b024B958a61D4d"),
    );
    m.insert(
        "Grayscale ETHE_285",
        address!("4C912A73Af93a6D99a456966eA27952426E2109B"),
    );
    m.insert(
        "Grayscale ETHE_286",
        address!("4cAd7E8d1bE7930b51DccD2F9654c78f2e326453"),
    );
    m.insert(
        "Grayscale ETHE_287",
        address!("4Cb5E0034c893f9fbbcc16e660A26a6F7cA4F3ED"),
    );
    m.insert(
        "Grayscale ETHE_288",
        address!("4CD28eb1A30742089f104Ccc4F7FE113BF75cbCC"),
    );
    m.insert(
        "Grayscale ETHE_289",
        address!("4d22E4F9f2BC8a0697279ce83Dd1D23F1136b707"),
    );
    m.insert(
        "Grayscale ETHE_29",
        address!("0778e57edA067231489382E83b968f86880FD1db"),
    );
    m.insert(
        "Grayscale ETHE_290",
        address!("4d31998C8a4d00648d4dB75982E8dfbBEB0Af313"),
    );
    m.insert(
        "Grayscale ETHE_291",
        address!("4d371ca3FFB6e6800aA3216283Dd21672eB647d7"),
    );
    m.insert(
        "Grayscale ETHE_292",
        address!("4D6E89BdF8374DC2E97Aed249b05EE1210e4F9CF"),
    );
    m.insert(
        "Grayscale ETHE_293",
        address!("4daC9B0a16e8C81D692A30c621763df7248DB6f0"),
    );
    m.insert(
        "Grayscale ETHE_294",
        address!("4DAf24CB5ac80F56DF1C342F77fE3F5A364645Ed"),
    );
    m.insert(
        "Grayscale ETHE_295",
        address!("4df294F41736c4767338D4164fDe345fA375d284"),
    );
    m.insert(
        "Grayscale ETHE_296",
        address!("4dF55D7f6795c218DD00430032e630814ca6eBcc"),
    );
    m.insert(
        "Grayscale ETHE_297",
        address!("4E3440271F59De4b072B3D5D9e0b408C4262594A"),
    );
    m.insert(
        "Grayscale ETHE_298",
        address!("4e873Ae18f40B679A2F7d49e495EfA882E8aAA15"),
    );
    m.insert(
        "Grayscale ETHE_299",
        address!("4Ef80540d1195BD17Fc86E42A334d5957A7c3E16"),
    );
    m.insert(
        "Grayscale ETHE_3",
        address!("012746EE19ad1D0a1648fF1B00d43273877a0ab8"),
    );
    m.insert(
        "Grayscale ETHE_30",
        address!("07D7a1D64E35e69241a1519Ea6bC1e09F66c99E9"),
    );
    m.insert(
        "Grayscale ETHE_300",
        address!("4F4b4Acc3907511d66442B3EDA559af03cBBED95"),
    );
    m.insert(
        "Grayscale ETHE_301",
        address!("4Fc8846E1d9122e9a58EBb0746da5fBb8DA67024"),
    );
    m.insert(
        "Grayscale ETHE_302",
        address!("4fe407507956aF73847d703c13F3b33504b8134e"),
    );
    m.insert(
        "Grayscale ETHE_303",
        address!("500140CE79eF3B75fC2DbD9690AA32E9dA4296D6"),
    );
    m.insert(
        "Grayscale ETHE_304",
        address!("503a526B4cE2565edB55DD581De2F4Dfe5eF65F0"),
    );
    m.insert(
        "Grayscale ETHE_305",
        address!("50a37d86b7F5C11A310E280B87f184138BcAaFC5"),
    );
    m.insert(
        "Grayscale ETHE_306",
        address!("50cdE33Eb1dbd3624529F2968E4443dd8fbC1811"),
    );
    m.insert(
        "Grayscale ETHE_307",
        address!("51AC57B623555909a6Ef97B88a1880Cb4547e1D3"),
    );
    m.insert(
        "Grayscale ETHE_308",
        address!("51D164829b4e2938E1ece6D5111290E8b41875Ff"),
    );
    m.insert(
        "Grayscale ETHE_309",
        address!("51fcA5cD4546e0b2745bBE45C1654DA75928C9bD"),
    );
    m.insert(
        "Grayscale ETHE_31",
        address!("07e06bA1150946cdF00b9f20e2e52568150E62a2"),
    );
    m.insert(
        "Grayscale ETHE_310",
        address!("52376658D27cd024Dd18De54912cf355719F0F3E"),
    );
    m.insert(
        "Grayscale ETHE_311",
        address!("526201784Fb54111a72ABDC50BB4950d75cA561a"),
    );
    m.insert(
        "Grayscale ETHE_312",
        address!("52676741724fB7E9700135164affbeC7A20C1AA0"),
    );
    m.insert(
        "Grayscale ETHE_313",
        address!("526bc3216c5419491f52C6c99BCE8406121B5C4c"),
    );
    m.insert(
        "Grayscale ETHE_314",
        address!("5296e103e9072d450970fc18d8736E12A1BC6f9b"),
    );
    m.insert(
        "Grayscale ETHE_315",
        address!("52B210970d7F054Ac20c346bC56973c35C8EFbfD"),
    );
    m.insert(
        "Grayscale ETHE_316",
        address!("52EC584ded2958E31A2ECafb76724fd2532Cd9c7"),
    );
    m.insert(
        "Grayscale ETHE_317",
        address!("532047Bd23F556a66a0B465307258332FC85E7cE"),
    );
    m.insert(
        "Grayscale ETHE_318",
        address!("534FFEaF23a8672B0a54c245AA1f80300670e246"),
    );
    m.insert(
        "Grayscale ETHE_319",
        address!("53986753C1dEF5956E17f8dF52cA0709e10356fC"),
    );
    m.insert(
        "Grayscale ETHE_32",
        address!("081BaAA7BB3F13F55aa1EC2F0911daf532035d5E"),
    );
    m.insert(
        "Grayscale ETHE_320",
        address!("53ed5F71A8164d5C75E2704Db4ea07189f45AEb8"),
    );
    m.insert(
        "Grayscale ETHE_321",
        address!("540Ccbf4E9BA934463984Bcb3D28A33b0647f798"),
    );
    m.insert(
        "Grayscale ETHE_322",
        address!("542E0946e97eA6934b27fb5825C342c477683f5c"),
    );
    m.insert(
        "Grayscale ETHE_323",
        address!("546b44D5C69C331A5f98D21f0318f762b420af84"),
    );
    m.insert(
        "Grayscale ETHE_324",
        address!("54b91CA7542Cb7387ec721A6f73DBd9434b3B0D9"),
    );
    m.insert(
        "Grayscale ETHE_325",
        address!("556D8aBf8e9DA36470Fe6f53e5D84fa62654B4FA"),
    );
    m.insert(
        "Grayscale ETHE_326",
        address!("55A834B42b33b737b1099b488967BF33a835185d"),
    );
    m.insert(
        "Grayscale ETHE_327",
        address!("55DE16080D11EEA7dc4bB503884c4d1446480342"),
    );
    m.insert(
        "Grayscale ETHE_328",
        address!("5642f1F205f440899b32c5E0625888Df9F3c3f53"),
    );
    m.insert(
        "Grayscale ETHE_329",
        address!("569FF84894dD10f944572A5375F09f326127Af1F"),
    );
    m.insert(
        "Grayscale ETHE_33",
        address!("0844E137f4e87Ad3f3738D5B39514024146097E0"),
    );
    m.insert(
        "Grayscale ETHE_330",
        address!("56AAEeFD0dEb98aB082659B592DCE108755AD0c6"),
    );
    m.insert(
        "Grayscale ETHE_331",
        address!("56EE86eF8298CD7D69b2184AF6b36322b467592d"),
    );
    m.insert(
        "Grayscale ETHE_332",
        address!("5705F2486Fc28BE7a1eA040bf7571Dd2E774F9F9"),
    );
    m.insert(
        "Grayscale ETHE_333",
        address!("575C493af071E84e9c383573158F3F53740fa900"),
    );
    m.insert(
        "Grayscale ETHE_334",
        address!("57b7904E9BE6D1C353022f7262b04F96bE14DffA"),
    );
    m.insert(
        "Grayscale ETHE_335",
        address!("586cA3c590988B73484C823eef9BdeC23ad1Cae8"),
    );
    m.insert(
        "Grayscale ETHE_336",
        address!("58baE5c59F73F764148B127873A211eae1111A1b"),
    );
    m.insert(
        "Grayscale ETHE_337",
        address!("59D2bDf33369266d0b5C0D1C31516e83d571c9dE"),
    );
    m.insert(
        "Grayscale ETHE_338",
        address!("5a4F0A702F1f864bfC3FeF7d312212BFd922EC5b"),
    );
    m.insert(
        "Grayscale ETHE_339",
        address!("5A66112882827010BA6b308062c3F7a722E40d37"),
    );
    m.insert(
        "Grayscale ETHE_34",
        address!("08582aB2F406fCb715dBDAdAbc134a889d53d50c"),
    );
    m.insert(
        "Grayscale ETHE_340",
        address!("5aCF811f58ae94c8A049fC977CfA4949e43D5264"),
    );
    m.insert(
        "Grayscale ETHE_341",
        address!("5B2c70D606244611396Ed6e6ab8860dbF2A88430"),
    );
    m.insert(
        "Grayscale ETHE_342",
        address!("5B82Eec97845749F29514521F5d3A49b2d733276"),
    );
    m.insert(
        "Grayscale ETHE_343",
        address!("5B9d696B46454e6A9450D0281866A6A6c0740788"),
    );
    m.insert(
        "Grayscale ETHE_344",
        address!("5ceCDa7Ab52609B4bD5459aB3f12B082A0fb21df"),
    );
    m.insert(
        "Grayscale ETHE_345",
        address!("5D48ae24E96ca893E7dE05a277d6A90c35588E5F"),
    );
    m.insert(
        "Grayscale ETHE_346",
        address!("5D81D3226F47536df852428CF2b66DdFB307a2b3"),
    );
    m.insert(
        "Grayscale ETHE_347",
        address!("5d84E5273E5d0260b39De2114F3b4dC81e4BEe2B"),
    );
    m.insert(
        "Grayscale ETHE_348",
        address!("5d8a8f2d93948b11009Bd81828A1452079983A4C"),
    );
    m.insert(
        "Grayscale ETHE_349",
        address!("5dDaeA1D0C93e6489D2C1fF4e28b7e44e2a0F81D"),
    );
    m.insert(
        "Grayscale ETHE_35",
        address!("085e1B2190Ad385A878d77e134e31274E9833A2F"),
    );
    m.insert(
        "Grayscale ETHE_350",
        address!("5ddE71d006b04272e7850B6A8FFFa567D5d785dD"),
    );
    m.insert(
        "Grayscale ETHE_351",
        address!("5E0C61bd1ad044d3a54d41C43310d9Ac22484868"),
    );
    m.insert(
        "Grayscale ETHE_352",
        address!("5e3a4c31EaDD4554AE901A4491cc62d6c03B8F64"),
    );
    m.insert(
        "Grayscale ETHE_353",
        address!("5E438815628819b1f1618016317bAb36A82c767E"),
    );
    m.insert(
        "Grayscale ETHE_354",
        address!("5ec074901908091Dd04779C120EFf22406497D8F"),
    );
    m.insert(
        "Grayscale ETHE_355",
        address!("5EC2Beb42F59b45E7540f350CD57eB6D6466e083"),
    );
    m.insert(
        "Grayscale ETHE_356",
        address!("5EfCFA606c56a2Def2afCe5B54D607423468997E"),
    );
    m.insert(
        "Grayscale ETHE_357",
        address!("5fB9966db64830a2C10FB298d9A4eF5c98550FDd"),
    );
    m.insert(
        "Grayscale ETHE_358",
        address!("5fc9917Fa8b221DB26F3EEe2A7484d587BDEFB50"),
    );
    m.insert(
        "Grayscale ETHE_359",
        address!("5FCf28A33928beD22fDe2680367f008E89F00FD3"),
    );
    m.insert(
        "Grayscale ETHE_36",
        address!("086617A910E1DDC1291a26cf65F0ee5746F848d8"),
    );
    m.insert(
        "Grayscale ETHE_360",
        address!("5FcFC77882BdB84905350355fEE16D6a10E282aC"),
    );
    m.insert(
        "Grayscale ETHE_361",
        address!("5ff792Cee953fa098036909a9d95E1F3198586E5"),
    );
    m.insert(
        "Grayscale ETHE_362",
        address!("600109d669EcC3110A83f72086cA188fDe39795F"),
    );
    m.insert(
        "Grayscale ETHE_363",
        address!("60184FAe7fE853dA30524B0AD364e3D5d7A53597"),
    );
    m.insert(
        "Grayscale ETHE_364",
        address!("6021DD7F879e0223955F8c68F8EF3194dd1cB527"),
    );
    m.insert(
        "Grayscale ETHE_365",
        address!("606df8d8b1A5F8fEbe72e68caE9401365Ee0d0b9"),
    );
    m.insert(
        "Grayscale ETHE_366",
        address!("60f8Cdca7a3164707958C3526bbE4b6C0CE8992e"),
    );
    m.insert(
        "Grayscale ETHE_367",
        address!("60f97534C05d432D18def0d9920C03Ac4b215E2C"),
    );
    m.insert(
        "Grayscale ETHE_368",
        address!("6121278E5d3bE46E93cFa6ab2a35134a7279cC5a"),
    );
    m.insert(
        "Grayscale ETHE_369",
        address!("6138c242D5b905C70976fD67DF07Bdc58a05B9b0"),
    );
    m.insert(
        "Grayscale ETHE_37",
        address!("087015b6d9d62c7346f25d01c5e03Acb44EA61d4"),
    );
    m.insert(
        "Grayscale ETHE_370",
        address!("614c0880A7735597c949e973cF40532860a7149c"),
    );
    m.insert(
        "Grayscale ETHE_371",
        address!("6166de40e3681cAeF80743B3d2aEF36e9f1876aE"),
    );
    m.insert(
        "Grayscale ETHE_372",
        address!("61AF113c645C0fd0214357eC1C08d0A52E5bfD8f"),
    );
    m.insert(
        "Grayscale ETHE_373",
        address!("61Bd22068505658F82904E334B3ec303245126B6"),
    );
    m.insert(
        "Grayscale ETHE_374",
        address!("61d6f8fF3e8c11d89af3BfFDd880bF7333a89078"),
    );
    m.insert(
        "Grayscale ETHE_375",
        address!("622a687Cfb2C552d4fAa12b984dFF48847DA8FBE"),
    );
    m.insert(
        "Grayscale ETHE_376",
        address!("625096293C013037F8e951b6CAA469840b27E0e0"),
    );
    m.insert(
        "Grayscale ETHE_377",
        address!("62FCc6Ba6e187eB39D435098f995cbf0239d3944"),
    );
    m.insert(
        "Grayscale ETHE_378",
        address!("630FA12c6fB12eeD6f0c6e3030161DB6872e6fB0"),
    );
    m.insert(
        "Grayscale ETHE_379",
        address!("6333a7192ca0dDbAcea55F586092A8A75AB4f2FB"),
    );
    m.insert(
        "Grayscale ETHE_38",
        address!("089FE46c34c17EF05CE6138ece644404da2039d4"),
    );
    m.insert(
        "Grayscale ETHE_380",
        address!("6349529C8E85408E0C5d939AE5C5A092641DD141"),
    );
    m.insert(
        "Grayscale ETHE_381",
        address!("63B8C30CbB4E63C9239E25F9387d37E6d987dE63"),
    );
    m.insert(
        "Grayscale ETHE_382",
        address!("63eD6c4AaFE066789b1c91afAD0DD9917990B79b"),
    );
    m.insert(
        "Grayscale ETHE_383",
        address!("645471133f8f7aF362a2878796Ed516452d5Cb24"),
    );
    m.insert(
        "Grayscale ETHE_384",
        address!("64a9305ee172b61e4Ed60cc293B818c47F773498"),
    );
    m.insert(
        "Grayscale ETHE_385",
        address!("64B101ecE3dC13cf23C44e5496976f14D5821F67"),
    );
    m.insert(
        "Grayscale ETHE_386",
        address!("65C1c7Df115124D7e268221907B7AAfc2437F75d"),
    );
    m.insert(
        "Grayscale ETHE_387",
        address!("65CdA63f1D330AB2c43DF76640a9080f0c18d702"),
    );
    m.insert(
        "Grayscale ETHE_388",
        address!("65fF904B040aa6790bC57E04450e2ae68c2ceD56"),
    );
    m.insert(
        "Grayscale ETHE_389",
        address!("6603C9f466373dCFFE3360e4f054D8F1BB23Af3b"),
    );
    m.insert(
        "Grayscale ETHE_39",
        address!("08C491e232A8E304e9d59883DBdbe8192B3cF027"),
    );
    m.insert(
        "Grayscale ETHE_390",
        address!("662c1D33923f1b48b5cdc1F4570423Ec1Ced1228"),
    );
    m.insert(
        "Grayscale ETHE_391",
        address!("6709842C12e713789f5706e0DC207921e0835d50"),
    );
    m.insert(
        "Grayscale ETHE_392",
        address!("67e2c03a29Bf611BbeF6E09D519945170486ff40"),
    );
    m.insert(
        "Grayscale ETHE_393",
        address!("68084Eac915E3429320eD6574811a0eA2873427C"),
    );
    m.insert(
        "Grayscale ETHE_394",
        address!("684bF2573A3fBB92F656787C6EC5b5E7844f29B2"),
    );
    m.insert(
        "Grayscale ETHE_395",
        address!("6864159A8022776c4F61F7c455c86bb9C92b0a2e"),
    );
    m.insert(
        "Grayscale ETHE_396",
        address!("687E8322e100318c890dfEB21123DC6FCbf42666"),
    );
    m.insert(
        "Grayscale ETHE_397",
        address!("6887dB27b113D72e6158DFD3fE0e69acd7b1FBcf"),
    );
    m.insert(
        "Grayscale ETHE_398",
        address!("688dD446b595D5702E9dcB545C89261B1C0C7CC7"),
    );
    m.insert(
        "Grayscale ETHE_399",
        address!("68e4b4a6D98ae3cBc969331BDB11257229165C83"),
    );
    m.insert(
        "Grayscale ETHE_4",
        address!("02215C9655FA9C14d993Dc6d75B06Aa863C53ec2"),
    );
    m.insert(
        "Grayscale ETHE_40",
        address!("08CC6C946Cda9D27Dbd2B0a03eCB3011514F8B03"),
    );
    m.insert(
        "Grayscale ETHE_400",
        address!("692fd667D7C7d38b99397f344d31fA6182f58bdf"),
    );
    m.insert(
        "Grayscale ETHE_401",
        address!("695483E6f68A939fEDdA945499C9e3eE557C1c3A"),
    );
    m.insert(
        "Grayscale ETHE_402",
        address!("6971f9330d938Bb2AcC64937301e6A472CC2aE56"),
    );
    m.insert(
        "Grayscale ETHE_403",
        address!("697e3972fc5E51cE91313E159ce19ebffA87dd75"),
    );
    m.insert(
        "Grayscale ETHE_404",
        address!("698aD35Fd27e5eA9A57C119C7B1770aCa092BBfC"),
    );
    m.insert(
        "Grayscale ETHE_405",
        address!("699152F1AcE5504Cbee3a962FB83eBdA907D1941"),
    );
    m.insert(
        "Grayscale ETHE_406",
        address!("69b88cC44CA70008237017A5C8D7161beF0EC043"),
    );
    m.insert(
        "Grayscale ETHE_407",
        address!("6a211b0FDd9eCa4339f02E8da47C348808CA7fC6"),
    );
    m.insert(
        "Grayscale ETHE_408",
        address!("6a2777630E021093A88d431D20dB8FB634E4D88F"),
    );
    m.insert(
        "Grayscale ETHE_409",
        address!("6a455E003D1b8Da4f036eBA4c9B7c913907E7F62"),
    );
    m.insert(
        "Grayscale ETHE_41",
        address!("0936BaB7E471efbd4eFe1d2BE2E37d1Ea70444FB"),
    );
    m.insert(
        "Grayscale ETHE_410",
        address!("6A4e04255371891D03daA2A928b553EAea883DdD"),
    );
    m.insert(
        "Grayscale ETHE_411",
        address!("6A9869dBc3497662b406DaF81a4E030866988319"),
    );
    m.insert(
        "Grayscale ETHE_412",
        address!("6a9A05415F78EeF19DE272e0677f42c954DDECB4"),
    );
    m.insert(
        "Grayscale ETHE_413",
        address!("6af5A0155E102889cEb4A38116194fa9404CC45B"),
    );
    m.insert(
        "Grayscale ETHE_414",
        address!("6B26487e2Ab4Cbe34d88EB5B57311f82Ed4F0e47"),
    );
    m.insert(
        "Grayscale ETHE_415",
        address!("6B7Da8F6844B90f90bfC646816DE6Cd5fD741fD0"),
    );
    m.insert(
        "Grayscale ETHE_416",
        address!("6bEC1049e676D6e71F7578bDC05D8A03d7A1931F"),
    );
    m.insert(
        "Grayscale ETHE_417",
        address!("6c3200e8B067Ec14e737dA32FaE6D902116aB50D"),
    );
    m.insert(
        "Grayscale ETHE_418",
        address!("6c65fDb3B8A07c0eE373eD4968291d5194b1b29D"),
    );
    m.insert(
        "Grayscale ETHE_419",
        address!("6c8d223C95E527BcE5B03CE3741D80d64912B6ac"),
    );
    m.insert(
        "Grayscale ETHE_42",
        address!("098d384670D9178136bBf89CB2f76CB3eE4DbaC5"),
    );
    m.insert(
        "Grayscale ETHE_420",
        address!("6D1338aE291fa7CbBF74F1B18815005b5d060e0c"),
    );
    m.insert(
        "Grayscale ETHE_421",
        address!("6Da0264F1c8a5eA780AD360fe1264cc6132aBe56"),
    );
    m.insert(
        "Grayscale ETHE_422",
        address!("6Dc0efE64CB54E87f137465fb9E94eC96B5E56c5"),
    );
    m.insert(
        "Grayscale ETHE_423",
        address!("6df177b73AdCD1fa24892ACd367673a19af00E9B"),
    );
    m.insert(
        "Grayscale ETHE_424",
        address!("6e02A9cBcA1C9678a58541256AD532Ee888Ae3aa"),
    );
    m.insert(
        "Grayscale ETHE_425",
        address!("6e473032E85E92A3dc5b57E99b9431918e61BE22"),
    );
    m.insert(
        "Grayscale ETHE_426",
        address!("6Ed11c6B4b64db34300565fB87778638b9114f81"),
    );
    m.insert(
        "Grayscale ETHE_427",
        address!("6f00d284995d3Ad8Ed08B0C9e046b6fF8B5FdFDE"),
    );
    m.insert(
        "Grayscale ETHE_428",
        address!("6F2b800370DBD6F45eCEB8d10d0E89067bf425D3"),
    );
    m.insert(
        "Grayscale ETHE_429",
        address!("6F64660cE9967D7B9285818C884EC602984b0591"),
    );
    m.insert(
        "Grayscale ETHE_43",
        address!("099206266A54d4D7a41Fb5eB5B687EBf7ac153B6"),
    );
    m.insert(
        "Grayscale ETHE_430",
        address!("7010aec5Af99FecAA716020CE52752eF07fEEDb9"),
    );
    m.insert(
        "Grayscale ETHE_431",
        address!("70154EdcD2278cA9acbD30Fc253b8B272e0A76a6"),
    );
    m.insert(
        "Grayscale ETHE_432",
        address!("7047E704228cEF3cFf0295Ca09dA1B5bAB3D7402"),
    );
    m.insert(
        "Grayscale ETHE_433",
        address!("70A3506B3ab2cA44CE461D702ec91D6B1ADE643c"),
    );
    m.insert(
        "Grayscale ETHE_434",
        address!("7157fd5747a219Ede2180b15865913365A80805F"),
    );
    m.insert(
        "Grayscale ETHE_435",
        address!("71a3DDF2A2614695A4e85E13BA15afb754a72Fe9"),
    );
    m.insert(
        "Grayscale ETHE_436",
        address!("71B77Eb874c2E8B530133244E2a28771040762DD"),
    );
    m.insert(
        "Grayscale ETHE_437",
        address!("720d7dDBDE53962a97f072D069eC11aBBF842B12"),
    );
    m.insert(
        "Grayscale ETHE_438",
        address!("72412CEC19b8215F48e34Be86D72eEC979AD10F3"),
    );
    m.insert(
        "Grayscale ETHE_439",
        address!("72848D06C79a85d5632e0B04c9f59578794F1beE"),
    );
    m.insert(
        "Grayscale ETHE_44",
        address!("09Be86C2f440E333bbB165F9eE810572d88df0b3"),
    );
    m.insert(
        "Grayscale ETHE_440",
        address!("72a7ddDBe54Ef33f55373B312810553518159BB1"),
    );
    m.insert(
        "Grayscale ETHE_441",
        address!("72F2702c9baaaa8a24319717B3068c514db87305"),
    );
    m.insert(
        "Grayscale ETHE_442",
        address!("7326972AE7eBa13794Ee2A8b57FaD7057331cE2c"),
    );
    m.insert(
        "Grayscale ETHE_443",
        address!("733639079F43CA1Ca1F05daAeABF015e0FAF5166"),
    );
    m.insert(
        "Grayscale ETHE_444",
        address!("736De6Dbd3E15a7BA020202F95582E0062f013A3"),
    );
    m.insert(
        "Grayscale ETHE_445",
        address!("73eA72aC2059e71Dbaa3A8bb30144Fb2D7A684F9"),
    );
    m.insert(
        "Grayscale ETHE_446",
        address!("7401eA6D95e77894b13775fE3e15b48D053D8E0f"),
    );
    m.insert(
        "Grayscale ETHE_447",
        address!("745a45AEe9b15EFcf2961617e5E107F7106841bA"),
    );
    m.insert(
        "Grayscale ETHE_448",
        address!("749bAD48A876B2b9BBA1F5d0DAc7b646D10283f4"),
    );
    m.insert(
        "Grayscale ETHE_449",
        address!("74CFB89a4b0b0C774f39AADdF80F882518Acf047"),
    );
    m.insert(
        "Grayscale ETHE_45",
        address!("09EA755F8026604D3F3Cc1f99a0EeA5c343657D6"),
    );
    m.insert(
        "Grayscale ETHE_450",
        address!("75BA1cE57cE55ae3aB8A83F333449aD3B63De8b3"),
    );
    m.insert(
        "Grayscale ETHE_451",
        address!("764B0ADfa038721cF27b15e2eB94C22341Fe5A2d"),
    );
    m.insert(
        "Grayscale ETHE_452",
        address!("767a26Cac94fe8B3C78ae75ad1fE53FC084C82b2"),
    );
    m.insert(
        "Grayscale ETHE_453",
        address!("77591ae37E1Be16611c330EEbed3A6498a2C4E76"),
    );
    m.insert(
        "Grayscale ETHE_454",
        address!("77982C9f1B231bcE3F03E712A09462863E73Ee29"),
    );
    m.insert(
        "Grayscale ETHE_455",
        address!("7879a9a2e67BecD33eA97c8597CA249BBeE54fAc"),
    );
    m.insert(
        "Grayscale ETHE_456",
        address!("78d13179A60c4f452ded10c92a70efd5A547a653"),
    );
    m.insert(
        "Grayscale ETHE_457",
        address!("793d8C2db11065b359Cc4196c945A39Fe449F1c0"),
    );
    m.insert(
        "Grayscale ETHE_458",
        address!("798565E80cB60314099C0E0CB1f11baf5a0Ea58C"),
    );
    m.insert(
        "Grayscale ETHE_459",
        address!("79Ae5DCd42A1Fe24E63EF15296a73Dd6c5e5DD16"),
    );
    m.insert(
        "Grayscale ETHE_46",
        address!("0A172C6c79c87babcA56115A43443aE6c087Cc83"),
    );
    m.insert(
        "Grayscale ETHE_460",
        address!("79F8c44A5EEa39528F6e03fbD256C0e8D414fcBb"),
    );
    m.insert(
        "Grayscale ETHE_461",
        address!("7A590D53a490663b592de429De498ce105976054"),
    );
    m.insert(
        "Grayscale ETHE_462",
        address!("7AA8cD0e94F81731da1DcEDf437FCbde21bd07b6"),
    );
    m.insert(
        "Grayscale ETHE_463",
        address!("7Ad8DF872E3d0933D47575c70FE22aEDC7aE6eA9"),
    );
    m.insert(
        "Grayscale ETHE_464",
        address!("7b250aE733d29cFeF657cfB881c34c4ee4AE08aC"),
    );
    m.insert(
        "Grayscale ETHE_465",
        address!("7b2E9f8678093ceC16eBfC9e050f0c5696630c7E"),
    );
    m.insert(
        "Grayscale ETHE_466",
        address!("7Ba4A54a91B316018998563145C9C172B32a9190"),
    );
    m.insert(
        "Grayscale ETHE_467",
        address!("7bddAa28Ac15ac13edFfE089B94839B9F9A267aC"),
    );
    m.insert(
        "Grayscale ETHE_468",
        address!("7c11edf59ed7Ddfa72Cdad9Ea413415Df5fd84e0"),
    );
    m.insert(
        "Grayscale ETHE_469",
        address!("7c23c86FE486C3995C842eE6Ae562D276f1AA4ed"),
    );
    m.insert(
        "Grayscale ETHE_47",
        address!("0a24Bc9b2725e94591ED360335903c852807f890"),
    );
    m.insert(
        "Grayscale ETHE_470",
        address!("7C9F709A65C0eE9Ed3f271Be0D20ab9f34607834"),
    );
    m.insert(
        "Grayscale ETHE_471",
        address!("7CE5d5cfec1482A2470682809d7095A90D2c768e"),
    );
    m.insert(
        "Grayscale ETHE_472",
        address!("7d0523795e35c67B14B5EDce151266A5C0Ff2d65"),
    );
    m.insert(
        "Grayscale ETHE_473",
        address!("7d8cBa4e867F85C4ea19E60eDDb40a04dE8F6b7e"),
    );
    m.insert(
        "Grayscale ETHE_474",
        address!("7dd9737a63B7DC02A7719d8b85EFBA52942f35e4"),
    );
    m.insert(
        "Grayscale ETHE_475",
        address!("7E87C3Ea18097BB096F9D7a1A5cf4521207C0fA6"),
    );
    m.insert(
        "Grayscale ETHE_476",
        address!("7EbB43fedb9544F565e0181bdD09De865408f86D"),
    );
    m.insert(
        "Grayscale ETHE_477",
        address!("7f44a10b27d00542aa2428deb3C1f0d5C715c3F5"),
    );
    m.insert(
        "Grayscale ETHE_478",
        address!("7f74AAFd223f2462e52De66f36e98498fd141c6e"),
    );
    m.insert(
        "Grayscale ETHE_479",
        address!("7FA888ba66819244DEDe8fafA1d2670b8b906204"),
    );
    m.insert(
        "Grayscale ETHE_48",
        address!("0a6700fd5c4B8d5928f7F31Baf3abdD0d53FC940"),
    );
    m.insert(
        "Grayscale ETHE_480",
        address!("801FAD4e481d610C167c1C45bf5C7A4606c935a1"),
    );
    m.insert(
        "Grayscale ETHE_481",
        address!("8076De48B130d169B1bFd9080E7f83932b2eF24F"),
    );
    m.insert(
        "Grayscale ETHE_482",
        address!("814f948Dc68e246D28aFDE2077CDF305BfA1dEa6"),
    );
    m.insert(
        "Grayscale ETHE_483",
        address!("816D8Cec003b00aa87a10D40EdAACadE27619ae3"),
    );
    m.insert(
        "Grayscale ETHE_484",
        address!("81D41De414eA4a781840D7c45f197506736db104"),
    );
    m.insert(
        "Grayscale ETHE_485",
        address!("821447965e1A0277393707A4C2a5F5F9B4d6F68e"),
    );
    m.insert(
        "Grayscale ETHE_486",
        address!("825650C11D8984CFbAb8d131074D93BE36E12F7A"),
    );
    m.insert(
        "Grayscale ETHE_487",
        address!("8307F2728756fE74Fe881bB6555aD7754B3270b0"),
    );
    m.insert(
        "Grayscale ETHE_488",
        address!("837F4A04694F54B48ff79779a1067d0A61cb0aa6"),
    );
    m.insert(
        "Grayscale ETHE_489",
        address!("83b4C6E8a68c6b108F9893c141F5671Ea0B32C87"),
    );
    m.insert(
        "Grayscale ETHE_49",
        address!("0a99D8394A28A19D90CDdb8894894db81693a9A1"),
    );
    m.insert(
        "Grayscale ETHE_490",
        address!("8431b405e21DcA8cC947B0073108f0C6922E872b"),
    );
    m.insert(
        "Grayscale ETHE_491",
        address!("8432b7ca18308E1ec9e4eF72619fE0F7De13A69E"),
    );
    m.insert(
        "Grayscale ETHE_492",
        address!("84528A9c8a5fB6205cEe8068DF033eb7F46b74eD"),
    );
    m.insert(
        "Grayscale ETHE_493",
        address!("8470590Edf758197368b3BC0327cEF0048c2fB1B"),
    );
    m.insert(
        "Grayscale ETHE_494",
        address!("85A8E39FeD50EFdE8310FaB6cDDc0B68d767F018"),
    );
    m.insert(
        "Grayscale ETHE_495",
        address!("85E22E411E0Ad55c66BF5718089cD77C5C2b2425"),
    );
    m.insert(
        "Grayscale ETHE_496",
        address!("861900B0276E2B681791fB0422371b570227927C"),
    );
    m.insert(
        "Grayscale ETHE_497",
        address!("86815B753bAd06cfEDE42de38B07d27350390472"),
    );
    m.insert(
        "Grayscale ETHE_498",
        address!("86De4FF1fc04bC47D941aa2721684b86d1066bf3"),
    );
    m.insert(
        "Grayscale ETHE_499",
        address!("86EF3eD11ad5ed0C9c5D2d1973FeE2c5089427B8"),
    );
    m.insert(
        "Grayscale ETHE_5",
        address!("02367833F3b379e66F4faF0ef015785b81B187fE"),
    );
    m.insert(
        "Grayscale ETHE_50",
        address!("0B1A9A55D574CeBBdB0E08c90A5CF4Ae3B6E273A"),
    );
    m.insert(
        "Grayscale ETHE_500",
        address!("8740eBf35c08627daC6a8CCB0310Aa920e0E66A1"),
    );
    m.insert(
        "Grayscale ETHE_501",
        address!("87556707D7f3EDfaBC3564D04c4F6DEE7a2bc2C8"),
    );
    m.insert(
        "Grayscale ETHE_502",
        address!("88baCDa2Ea0fCc99875244B9a0741e715dA4C18A"),
    );
    m.insert(
        "Grayscale ETHE_503",
        address!("88ee87F40C77ea2cA43294b4f65Dc4391223b1D8"),
    );
    m.insert(
        "Grayscale ETHE_504",
        address!("8936424b558a95FBDec5938b1dD95A11e40a4532"),
    );
    m.insert(
        "Grayscale ETHE_505",
        address!("8A091A0Ba5521dbb6C017A0461f9Ed069d97Ad5d"),
    );
    m.insert(
        "Grayscale ETHE_506",
        address!("8A6F02B8BA5C575fC668822fAEaDa690D7fD1eFC"),
    );
    m.insert(
        "Grayscale ETHE_507",
        address!("8A9af7e888ef1bB12820480322b042F52754ce5e"),
    );
    m.insert(
        "Grayscale ETHE_508",
        address!("8bdc9f9fb9e2399cfF176b1b8a1230E7606855e5"),
    );
    m.insert(
        "Grayscale ETHE_509",
        address!("8C24Fd5ec2FA8B54a927E98a44B3f420e73Eef23"),
    );
    m.insert(
        "Grayscale ETHE_51",
        address!("0B3FB306E59bf5A2740B452c27717F79756B48e5"),
    );
    m.insert(
        "Grayscale ETHE_510",
        address!("8C7b0BeDEddCf071Ba1C4FA0ADd63c472759c5c4"),
    );
    m.insert(
        "Grayscale ETHE_511",
        address!("8cB040DD5B26AC5d677b5206e92afAA8B3529734"),
    );
    m.insert(
        "Grayscale ETHE_512",
        address!("8d4878c44F18FC8B4C77727C4F3BC9CF3E657057"),
    );
    m.insert(
        "Grayscale ETHE_513",
        address!("8D5cd5cDc222a45CE2bBa319E3C037A38C2D92C0"),
    );
    m.insert(
        "Grayscale ETHE_514",
        address!("8D6897bea342692E4BB57eE3115378c98Da07932"),
    );
    m.insert(
        "Grayscale ETHE_515",
        address!("8d7fb46B650705D3B1DcfeF22FE284EC65Ac2dE1"),
    );
    m.insert(
        "Grayscale ETHE_516",
        address!("8dED4b4Ab7E4eB79eE929D955Bd4652500BaC64E"),
    );
    m.insert(
        "Grayscale ETHE_517",
        address!("8e26A0D4d724CC77C0A988C375B5cEBc0fF5ae62"),
    );
    m.insert(
        "Grayscale ETHE_518",
        address!("8e2B4BCEadc867Eee37281e8793D18996Bdea044"),
    );
    m.insert(
        "Grayscale ETHE_519",
        address!("8E88cb696F99f8b047B0BE56A6Ce743Fc4619F66"),
    );
    m.insert(
        "Grayscale ETHE_52",
        address!("0b8428AbD130eb4c6Ffa9Bc2D89bbbB1F3D09AA4"),
    );
    m.insert(
        "Grayscale ETHE_520",
        address!("8e8f382F5aEFA99C665AE5223685a8b6c308B4e3"),
    );
    m.insert(
        "Grayscale ETHE_521",
        address!("8f1eF2c70A61CAE8Dd54D25BE7B44c2E581C6b33"),
    );
    m.insert(
        "Grayscale ETHE_522",
        address!("8f728cA3561EDf7B015A0b972f8B6Fb04db89e2e"),
    );
    m.insert(
        "Grayscale ETHE_523",
        address!("8feFB2ac91c4184b79caC44095dED9cEE81d56c4"),
    );
    m.insert(
        "Grayscale ETHE_524",
        address!("90006E8948045C49B6e6aA50897D8f4f9aD3b091"),
    );
    m.insert(
        "Grayscale ETHE_525",
        address!("90875d559Db224225aC6d63507fa7b370b70Dd26"),
    );
    m.insert(
        "Grayscale ETHE_526",
        address!("90d6747a7e78467DE08Beb30FDB8De8f9E4F5C27"),
    );
    m.insert(
        "Grayscale ETHE_527",
        address!("90dEb2D65335e84b68f79a0706963C5aB9e26c86"),
    );
    m.insert(
        "Grayscale ETHE_528",
        address!("90eD6b0Cc2AE881A278d55C4831C4d4e708aF586"),
    );
    m.insert(
        "Grayscale ETHE_529",
        address!("910ca28755b776a7acD02503B3B405F7A48874Ba"),
    );
    m.insert(
        "Grayscale ETHE_53",
        address!("0b94bB7b93E0363Abb06fa8011105C56Ec0AF99c"),
    );
    m.insert(
        "Grayscale ETHE_530",
        address!("912B06feC618bDc57952839E0f5dc535f051c8E2"),
    );
    m.insert(
        "Grayscale ETHE_531",
        address!("922a33e294fC10c2bf893AF46706257d30165849"),
    );
    m.insert(
        "Grayscale ETHE_532",
        address!("9239D6A502F305a19a106847f31aE19b6E16B723"),
    );
    m.insert(
        "Grayscale ETHE_533",
        address!("92462C26Ac4033E896CdeE397E6195cF6653765f"),
    );
    m.insert(
        "Grayscale ETHE_534",
        address!("92469eD213817dCeB000A2454D2E3D9Eee7C47e2"),
    );
    m.insert(
        "Grayscale ETHE_535",
        address!("92c7bF3e1546C3B1Ad83F87439Ae7F827AaAb1a8"),
    );
    m.insert(
        "Grayscale ETHE_536",
        address!("933c0cF95e7Be11D06c3BafAD128C1993B1dF476"),
    );
    m.insert(
        "Grayscale ETHE_537",
        address!("9341078f6eF542aD70ab4E93BE78eeEf02fCF1e6"),
    );
    m.insert(
        "Grayscale ETHE_538",
        address!("93a654a2813F3aD052929752124B51D68bB9fa8f"),
    );
    m.insert(
        "Grayscale ETHE_539",
        address!("93E3cDd07B04d3045eec99AB14Ec1e556DfFC9e4"),
    );
    m.insert(
        "Grayscale ETHE_54",
        address!("0b96f4Ea295dE96D7bCf8d29884E62cD0D88522b"),
    );
    m.insert(
        "Grayscale ETHE_540",
        address!("940D49bEb1E89ddaEFf286e22c7cBed39190cb24"),
    );
    m.insert(
        "Grayscale ETHE_541",
        address!("943fF4d28614365bcb2A79a58F032F123f37503D"),
    );
    m.insert(
        "Grayscale ETHE_542",
        address!("95631d64b8825cEC13956b7Afeb8D797f103C372"),
    );
    m.insert(
        "Grayscale ETHE_543",
        address!("95690beAdCf5Ba9A93F61FD8678785e4948144DF"),
    );
    m.insert(
        "Grayscale ETHE_544",
        address!("95Bfd1BA0404e1f382448D0D5370B3dD775dB364"),
    );
    m.insert(
        "Grayscale ETHE_545",
        address!("95cAE6eC900D019265127f03C941BbA668038834"),
    );
    m.insert(
        "Grayscale ETHE_546",
        address!("961761B37606e9134EbCD14B56d82143377de52F"),
    );
    m.insert(
        "Grayscale ETHE_547",
        address!("96650ac26678d60c4c299c65017d52D95E96B166"),
    );
    m.insert(
        "Grayscale ETHE_548",
        address!("96956669C2e9691584e328395c726d9f73730917"),
    );
    m.insert(
        "Grayscale ETHE_549",
        address!("96DD0d4E10b1e8FaE64771EC14793f716B12E889"),
    );
    m.insert(
        "Grayscale ETHE_55",
        address!("0BecaA21eb09551cA77C90E0E5D7049bf0D96cb0"),
    );
    m.insert(
        "Grayscale ETHE_550",
        address!("971C785796FedB5Aa2bAe0395D055eFDb8A38058"),
    );
    m.insert(
        "Grayscale ETHE_551",
        address!("97902B5A8212f11C0bAE8f31665620D05bFdabB1"),
    );
    m.insert(
        "Grayscale ETHE_552",
        address!("97dD985D6F73AF02Cd0220b5D855CfD4C5A7a067"),
    );
    m.insert(
        "Grayscale ETHE_553",
        address!("97E935DDf3f5A2C4291E38f106194Bb507DD28f9"),
    );
    m.insert(
        "Grayscale ETHE_554",
        address!("9850a63DA6715A79c08a07E506C5115362128397"),
    );
    m.insert(
        "Grayscale ETHE_555",
        address!("98692A67c5B8eacce4dfc55c8aaD3879104f6D2f"),
    );
    m.insert(
        "Grayscale ETHE_556",
        address!("9886fb3f45819087fE1ec2173fe308692948Ca55"),
    );
    m.insert(
        "Grayscale ETHE_557",
        address!("98bed4c85FA10FA1cEe0fB4d7406C6cA8e93b6fA"),
    );
    m.insert(
        "Grayscale ETHE_558",
        address!("98C879587733db42b32A8f4EB2Db213A28ff3623"),
    );
    m.insert(
        "Grayscale ETHE_559",
        address!("98Ea569De7C4949c588BF2447DC6b89Cb47ED32A"),
    );
    m.insert(
        "Grayscale ETHE_56",
        address!("0d1ec2169de51e05cFEb9F7bC6e303cb7B55a585"),
    );
    m.insert(
        "Grayscale ETHE_560",
        address!("992271B5015274cD7cc934860695F7599CD930f8"),
    );
    m.insert(
        "Grayscale ETHE_561",
        address!("99532EF4c6DC41b32b9759b79780B7dE6D2eBfe0"),
    );
    m.insert(
        "Grayscale ETHE_562",
        address!("995Ae367358d35E0787e3A2E509D1692DE982307"),
    );
    m.insert(
        "Grayscale ETHE_563",
        address!("99f85310dA212afB643C934F37ED512e66bf6dE5"),
    );
    m.insert(
        "Grayscale ETHE_564",
        address!("9a345cA67Ee70248aCfb4A64843E7A1e9BeFee96"),
    );
    m.insert(
        "Grayscale ETHE_565",
        address!("9b46991F9AEC29Fb0e93a1f19055F081Bf9C4ed3"),
    );
    m.insert(
        "Grayscale ETHE_566",
        address!("9B6C31B83c2B4AC502443585037bC01d33955F5B"),
    );
    m.insert(
        "Grayscale ETHE_567",
        address!("9C7E1966808A95A3bDa73619f76c35faBc46fE6d"),
    );
    m.insert(
        "Grayscale ETHE_568",
        address!("9cC64AabAa3D3859742c7CB22B0CE4B9e4e77875"),
    );
    m.insert(
        "Grayscale ETHE_569",
        address!("9CDBd99E66966Ec0015F016a19D31c5159E1ec81"),
    );
    m.insert(
        "Grayscale ETHE_57",
        address!("0D2CD9F9573E803DC2C424b35c928297D00b2293"),
    );
    m.insert(
        "Grayscale ETHE_570",
        address!("9Cf6b66279cc4a0D490c26F4Df9E748248ACFCE2"),
    );
    m.insert(
        "Grayscale ETHE_571",
        address!("9d0588846a9616c3744c4A833859859fdb694ff4"),
    );
    m.insert(
        "Grayscale ETHE_572",
        address!("9D9Fb58780FDaDB1f2c211FC72e0Ba2E099811fb"),
    );
    m.insert(
        "Grayscale ETHE_573",
        address!("9DA3cB66538F09dFE14dAD5Bd89009D8CBA8Afbf"),
    );
    m.insert(
        "Grayscale ETHE_574",
        address!("9DBc36696640B023B372Fc6753758F2eb923eA95"),
    );
    m.insert(
        "Grayscale ETHE_575",
        address!("9dcA64C99cb71aB2eFde929bD732DF930b4E81a7"),
    );
    m.insert(
        "Grayscale ETHE_576",
        address!("9DdcD8501e1db8Dae8dcEe323b1537D99d5EF14C"),
    );
    m.insert(
        "Grayscale ETHE_577",
        address!("9e631E208b2B711F42fb43004897e28891Ec7c49"),
    );
    m.insert(
        "Grayscale ETHE_578",
        address!("9e68027f552079a19BFdcf39a0821Bad64020Ae7"),
    );
    m.insert(
        "Grayscale ETHE_579",
        address!("9F38da065041E797E45Fa1C38a04c256D70C5Cc1"),
    );
    m.insert(
        "Grayscale ETHE_58",
        address!("0DB341236151200Ea3D684576F56Ec2F5d23bfCa"),
    );
    m.insert(
        "Grayscale ETHE_580",
        address!("9f8aba64BeAcB170CEbfA0d1366d040051E51215"),
    );
    m.insert(
        "Grayscale ETHE_581",
        address!("A032437337637eD85bEc0fE2E1EE7a63967B20F8"),
    );
    m.insert(
        "Grayscale ETHE_582",
        address!("a04D81f5c75cc159A72548CAeD8bB77192715bc8"),
    );
    m.insert(
        "Grayscale ETHE_583",
        address!("A09ad0194Bd492C6F26B7b4Fa7C5a1B1e7D39B20"),
    );
    m.insert(
        "Grayscale ETHE_584",
        address!("a0D4175D637Ce2647C91486DaC78153c49F35C13"),
    );
    m.insert(
        "Grayscale ETHE_585",
        address!("a209ee65A27D7fA73269d959E13d1b44BbE29574"),
    );
    m.insert(
        "Grayscale ETHE_586",
        address!("A2Ce66F2706c8eCc3B1b8A80BB050B156FFF9b08"),
    );
    m.insert(
        "Grayscale ETHE_587",
        address!("a309fDd59A0071Cb0cFB0965b75D8b30C8D8cAFc"),
    );
    m.insert(
        "Grayscale ETHE_588",
        address!("A31Ddf9715132a2F78e3B52a7eE841327d7495C7"),
    );
    m.insert(
        "Grayscale ETHE_589",
        address!("a36D12f79169cb1e5677eB3815346CD1de20cd4e"),
    );
    m.insert(
        "Grayscale ETHE_59",
        address!("0Df04749e82E64cF8A7Ba69E1047e64649f841a9"),
    );
    m.insert(
        "Grayscale ETHE_590",
        address!("a43680f780C176216ec4d374Dd1EBE229EcB07aA"),
    );
    m.insert(
        "Grayscale ETHE_591",
        address!("A43b3a7FC3be6fEa9c605F7cEDC73D4D8f928e4b"),
    );
    m.insert(
        "Grayscale ETHE_592",
        address!("A4B09BA44C0B69e9b24fb367EF83fbe0BE98D9D9"),
    );
    m.insert(
        "Grayscale ETHE_593",
        address!("a4eb45a4bEe9dE616741919cc166d11C5612c113"),
    );
    m.insert(
        "Grayscale ETHE_594",
        address!("a56B62b30f7f48974dD6899c14a005a7A0203DEF"),
    );
    m.insert(
        "Grayscale ETHE_595",
        address!("a57D20A6C32DD62909fb53b682F39697d2b260Bb"),
    );
    m.insert(
        "Grayscale ETHE_596",
        address!("A6694389FC8Fd1161AbEeBaB4171BC92BF57D15E"),
    );
    m.insert(
        "Grayscale ETHE_597",
        address!("a6CB61a30b1A2071D1d6f08cE26189b51415Cf9e"),
    );
    m.insert(
        "Grayscale ETHE_598",
        address!("a6DC7bC50EeD59fc7EB15089DFc7d163809D1bad"),
    );
    m.insert(
        "Grayscale ETHE_599",
        address!("A6E4173EC39b9C7468a23e71a0Da11E6fB23e058"),
    );
    m.insert(
        "Grayscale ETHE_6",
        address!("02465b4B1eFc21a8eAd4b8E7FD33ac01ba2E224A"),
    );
    m.insert(
        "Grayscale ETHE_60",
        address!("0F3132E5E01240306f9FB0D7e5c4DD2E2ddee224"),
    );
    m.insert(
        "Grayscale ETHE_600",
        address!("a6f3ab8B87793A68957A1C676fc1bdEDd5121481"),
    );
    m.insert(
        "Grayscale ETHE_601",
        address!("A70D39965feCEA5b9ffaa945C7DC638B1aE5E205"),
    );
    m.insert(
        "Grayscale ETHE_602",
        address!("a7165B9ea2AB66A9b3A5A8A176d715C9A463fBD5"),
    );
    m.insert(
        "Grayscale ETHE_603",
        address!("a71d30BA23EE8CC1894E02ddD733034E1FCECe61"),
    );
    m.insert(
        "Grayscale ETHE_604",
        address!("A7Df254f08884883f845fb44281e0a914700bd81"),
    );
    m.insert(
        "Grayscale ETHE_605",
        address!("a81B8a4f5809646e4A8d8d7558bbCA0c065fa0c3"),
    );
    m.insert(
        "Grayscale ETHE_606",
        address!("A85B6DE24F64b0F95F2DEAB7AD897e68765391fa"),
    );
    m.insert(
        "Grayscale ETHE_607",
        address!("a993C1526Ad7b0d3BBF721d353d509EDAe948A73"),
    );
    m.insert(
        "Grayscale ETHE_608",
        address!("aA0e58aDb8d1ea1964D0b65c9e83EA84C80A3459"),
    );
    m.insert(
        "Grayscale ETHE_609",
        address!("Aa158387E6281AE605586D0ad26546Ab67eCE10a"),
    );
    m.insert(
        "Grayscale ETHE_61",
        address!("0fdE25AB1C9d7731CEb303a3f24B2f4df176F977"),
    );
    m.insert(
        "Grayscale ETHE_610",
        address!("AAe65937bD6C9f5eEE350e10C3f0c1e189488488"),
    );
    m.insert(
        "Grayscale ETHE_611",
        address!("AB552f41228C5375Ba6b5B791E6d1cb03827c04e"),
    );
    m.insert(
        "Grayscale ETHE_612",
        address!("aB6a364A21F1d4EbE58bcAe301bcF892b45E36c2"),
    );
    m.insert(
        "Grayscale ETHE_613",
        address!("Ab6D4480014bB6968671e9Da152e512fF8C51e8f"),
    );
    m.insert(
        "Grayscale ETHE_614",
        address!("aB6FE7E0d49316c9e93538AA223599Db06911145"),
    );
    m.insert(
        "Grayscale ETHE_615",
        address!("AC4E05dD5E7Cf0d68a70E575aA5c58feFd424145"),
    );
    m.insert(
        "Grayscale ETHE_616",
        address!("Ac4fb4951A82a7220f803768aA0bf7f7AE588289"),
    );
    m.insert(
        "Grayscale ETHE_617",
        address!("AD44Aaa5f232b7AC4eC5B9c6B853af1d3B360C90"),
    );
    m.insert(
        "Grayscale ETHE_618",
        address!("adCece4B9ad1fb062C1F115A9Ab2A50cBF98D96f"),
    );
    m.insert(
        "Grayscale ETHE_619",
        address!("Af2AA93D0cab26c490419BC2E7DAE96cf4e278A7"),
    );
    m.insert(
        "Grayscale ETHE_62",
        address!("104b1e8e4fDC032e258a655616C481e2207b5474"),
    );
    m.insert(
        "Grayscale ETHE_620",
        address!("af64B119b0031611B8d8e1b824a11B9CDa36aa4D"),
    );
    m.insert(
        "Grayscale ETHE_621",
        address!("AfDF3CCA2d9C02E6c9869bBCc687B249b930253D"),
    );
    m.insert(
        "Grayscale ETHE_622",
        address!("B08A93741A2F4f3897F2d9Db6Caf0F6212c7A5D4"),
    );
    m.insert(
        "Grayscale ETHE_623",
        address!("B0d9d83304152EAf4179109dFEe0Cd88fE4A9ef3"),
    );
    m.insert(
        "Grayscale ETHE_624",
        address!("b15eF72F94174c978cBB03E6A1642c6d583B0105"),
    );
    m.insert(
        "Grayscale ETHE_625",
        address!("B15FcB3D829D8dca8B96cA3217B7DB0e8a8A6c8A"),
    );
    m.insert(
        "Grayscale ETHE_626",
        address!("B1744815CF2decb53d4A38080A5Eb5FE9cC9E77C"),
    );
    m.insert(
        "Grayscale ETHE_627",
        address!("b1a01A43418904D67BD8e951373cd25553678459"),
    );
    m.insert(
        "Grayscale ETHE_628",
        address!("b1cAc0Fa8F99fc6fce1827A110112f51D3370730"),
    );
    m.insert(
        "Grayscale ETHE_629",
        address!("B222C09153F04b51bC968B36F35628F3d4841397"),
    );
    m.insert(
        "Grayscale ETHE_63",
        address!("1078876A403922813e4f4ADD2889A5b6e4132fCe"),
    );
    m.insert(
        "Grayscale ETHE_630",
        address!("b2BdF5b0039Ce3F4fe630bcA64717C754797f53D"),
    );
    m.insert(
        "Grayscale ETHE_631",
        address!("B39B01537a2cb3A32677C6465892f1c6637A1ddB"),
    );
    m.insert(
        "Grayscale ETHE_632",
        address!("b4192c96F5597AA59FfEbB7fe2534224C3cA2dB7"),
    );
    m.insert(
        "Grayscale ETHE_633",
        address!("B427C740992A643717Ab2E861e92255c3222587a"),
    );
    m.insert(
        "Grayscale ETHE_634",
        address!("B450652bBF2fFf454cd449C1Aa4F470cC9f35A9d"),
    );
    m.insert(
        "Grayscale ETHE_635",
        address!("B463aAbF19527a7908C0B11C3D39191BdbF70E32"),
    );
    m.insert(
        "Grayscale ETHE_636",
        address!("B4844CBDb1338076b0683606d3de729eA4F7514A"),
    );
    m.insert(
        "Grayscale ETHE_637",
        address!("b48B6674b674a79e8A6D77eD7A273F551221Fa97"),
    );
    m.insert(
        "Grayscale ETHE_638",
        address!("B5227E59ea073DfEdac2289173379241d8de8B1C"),
    );
    m.insert(
        "Grayscale ETHE_639",
        address!("B5761AA4D4f88EEAB67A90955a15017181975565"),
    );
    m.insert(
        "Grayscale ETHE_64",
        address!("1130039C36e63079Ac232bdcCa5011F1abef0763"),
    );
    m.insert(
        "Grayscale ETHE_640",
        address!("B5C4D5016c9f03E9F90f04D7e815DBE47deC5486"),
    );
    m.insert(
        "Grayscale ETHE_641",
        address!("B5EB74CC4Dc944F0dA9566fa44b7412eC312F3fE"),
    );
    m.insert(
        "Grayscale ETHE_642",
        address!("b670243723fe354b2F8782Cfb9ECFFaf95c0F90A"),
    );
    m.insert(
        "Grayscale ETHE_643",
        address!("B68b631E3D734A58bF3a8FcE36886664c7Dc99Ab"),
    );
    m.insert(
        "Grayscale ETHE_644",
        address!("b6A5351E8ed77181d585E97FcCd56FcBaAd9f5B6"),
    );
    m.insert(
        "Grayscale ETHE_645",
        address!("b6c7C2DbB376187f9cF6c1EA19A3C0fcF4428495"),
    );
    m.insert(
        "Grayscale ETHE_646",
        address!("B702775182932110b614Df8dd75c5A0c36DA0274"),
    );
    m.insert(
        "Grayscale ETHE_647",
        address!("b70DA0395f0eDa1D32a739163FeF9dC0E93CE187"),
    );
    m.insert(
        "Grayscale ETHE_648",
        address!("b73C73a993E8a827859fF403aFb4B861Aa4431CD"),
    );
    m.insert(
        "Grayscale ETHE_649",
        address!("B745C5b044843179ea0f3252851bB23eb328710D"),
    );
    m.insert(
        "Grayscale ETHE_65",
        address!("11fB06E3ed9a12eBbB1345326448aA01D78731D6"),
    );
    m.insert(
        "Grayscale ETHE_650",
        address!("B7D8D7E5F9098e16c2eE15649Fbb95bdfBDEB22D"),
    );
    m.insert(
        "Grayscale ETHE_651",
        address!("b84EBE9526C0F91cB82b99acb1703791078B8028"),
    );
    m.insert(
        "Grayscale ETHE_652",
        address!("B87c8b82Aa9F48E9F4F85C826028901AE69b775E"),
    );
    m.insert(
        "Grayscale ETHE_653",
        address!("B88aF39DA312323Bc3bA10c445BA1aFCBC9397b3"),
    );
    m.insert(
        "Grayscale ETHE_654",
        address!("b977d3Fa2C2f878e4e230b27CBdc35c7a5c0ca5D"),
    );
    m.insert(
        "Grayscale ETHE_655",
        address!("B9786DC15B3E2E4994328834c801266479900328"),
    );
    m.insert(
        "Grayscale ETHE_656",
        address!("b9abCb0CF26A98C2e37823CE78742B6E5A1dcAD8"),
    );
    m.insert(
        "Grayscale ETHE_657",
        address!("B9bCC7214568633611e91Db91b80EF72a02f8777"),
    );
    m.insert(
        "Grayscale ETHE_658",
        address!("b9f1A414cd3E820968Fd51840B26cC149a4f24Bd"),
    );
    m.insert(
        "Grayscale ETHE_659",
        address!("b9f69EC82415F366879f97B840E805e8Dfdce3d0"),
    );
    m.insert(
        "Grayscale ETHE_66",
        address!("124A443D6A09c1Cf5a5238AF6a49E2D03a7D5DaD"),
    );
    m.insert(
        "Grayscale ETHE_660",
        address!("BA141724e6C5F43CfBf050e0694FE3065F976AAB"),
    );
    m.insert(
        "Grayscale ETHE_661",
        address!("ba4bd83131fcB7D0B30cA9E9C3742200958635A4"),
    );
    m.insert(
        "Grayscale ETHE_662",
        address!("BA7886228ADDf36F8e9d477d0bA61366D6e4DDA6"),
    );
    m.insert(
        "Grayscale ETHE_663",
        address!("Ba7cf8f7328863d4d429938cE4dab34a2D5b6982"),
    );
    m.insert(
        "Grayscale ETHE_664",
        address!("Ba9603360DC9Cc17C94FF72De4914ff6ff450269"),
    );
    m.insert(
        "Grayscale ETHE_665",
        address!("BAc101A6C81533E718a285a02280d4952DDD5B06"),
    );
    m.insert(
        "Grayscale ETHE_666",
        address!("baf3d4E644DDD847c28C9c4aea6584448d56bFd6"),
    );
    m.insert(
        "Grayscale ETHE_667",
        address!("BaF78734222642ec50B6F464ba1D4e41B2Af5b79"),
    );
    m.insert(
        "Grayscale ETHE_668",
        address!("bB077EBe177c8f8259914f95aC1512842EEa2023"),
    );
    m.insert(
        "Grayscale ETHE_669",
        address!("bb10b35480EDaA5DD648253F584F15f1185b6C76"),
    );
    m.insert(
        "Grayscale ETHE_67",
        address!("1271892c3724CfF1416dc14C2B4e27a368cFFeb4"),
    );
    m.insert(
        "Grayscale ETHE_670",
        address!("BbaBC0c69055AC0FcDD92b6b168C04D532D71E0d"),
    );
    m.insert(
        "Grayscale ETHE_671",
        address!("bBf63e51527F3177e19BaA859cB2eD69C031AEB7"),
    );
    m.insert(
        "Grayscale ETHE_672",
        address!("Bc67bE0A18cb9aa53b4cE821baF35C2F43eeBEd2"),
    );
    m.insert(
        "Grayscale ETHE_673",
        address!("BCf28d8132f48Aec6e21019be77159Dda5886c96"),
    );
    m.insert(
        "Grayscale ETHE_674",
        address!("Bd0F47deAE5795A26521D1d014630Cb2B35a87e8"),
    );
    m.insert(
        "Grayscale ETHE_675",
        address!("BDC6384EF4EAd1Ea9BF2A82390f2eDe46bAf1aeF"),
    );
    m.insert(
        "Grayscale ETHE_676",
        address!("bE3d84dfaCaee119eF5685D82c06BFCdEE876dA2"),
    );
    m.insert(
        "Grayscale ETHE_677",
        address!("bE9723cDeC785bfa1046A128f3D383b835294cE0"),
    );
    m.insert(
        "Grayscale ETHE_678",
        address!("bEC4e43fc46e17bB8B543C150B7460b0753f20D9"),
    );
    m.insert(
        "Grayscale ETHE_679",
        address!("bEFB5BaC4aeFA84C59BD08479aa1926f521E254E"),
    );
    m.insert(
        "Grayscale ETHE_68",
        address!("12b42BCbf7D017e6Fc811896AC35546F7c00a3bd"),
    );
    m.insert(
        "Grayscale ETHE_680",
        address!("bf044a0D4cFA289628a4450f4E36E4A5fBeA9d0c"),
    );
    m.insert(
        "Grayscale ETHE_681",
        address!("bf13709a606f2b77fCe2D3219a586EBa12bD1e76"),
    );
    m.insert(
        "Grayscale ETHE_682",
        address!("Bf43Cbd9B7784A744d5705a2e8365ee96a3120Fc"),
    );
    m.insert(
        "Grayscale ETHE_683",
        address!("bfAeDe6eB73A94B55860E29a71edC12Fa46891d5"),
    );
    m.insert(
        "Grayscale ETHE_684",
        address!("bfead5796997751A1D201ad7553aA103FA641E95"),
    );
    m.insert(
        "Grayscale ETHE_685",
        address!("bFF89978302266e5DFaFCF7A20b2a733721Aa09D"),
    );
    m.insert(
        "Grayscale ETHE_686",
        address!("C0a462d9e3552D16283Cb4f4d0EAdf40a94aA73f"),
    );
    m.insert(
        "Grayscale ETHE_687",
        address!("C0B137404E4ACd6D35840e1a367636ad141ed1A6"),
    );
    m.insert(
        "Grayscale ETHE_688",
        address!("C0ce047837d421bc026d0F43764AAD659E27336C"),
    );
    m.insert(
        "Grayscale ETHE_689",
        address!("c12386613Bcbf62Be78ee80077A721E8CBa3Bc12"),
    );
    m.insert(
        "Grayscale ETHE_69",
        address!("12EEDD04b57FFB7c0d3DE58D1ea0d16995D5F748"),
    );
    m.insert(
        "Grayscale ETHE_690",
        address!("c19aEbC165869496B4Af8D86D2494dDc8927231a"),
    );
    m.insert(
        "Grayscale ETHE_691",
        address!("C1aE372f35adEDdC8B6F557206a11c725a0f38ef"),
    );
    m.insert(
        "Grayscale ETHE_692",
        address!("C203dC03Ff949B4E2111897850d8D481ae8fCE7E"),
    );
    m.insert(
        "Grayscale ETHE_693",
        address!("C259129b5270D6fedD4EFA7d8736f2F103d97F13"),
    );
    m.insert(
        "Grayscale ETHE_694",
        address!("c2b6AABFEC71Ff4a578b873EB2718a31bc4C4212"),
    );
    m.insert(
        "Grayscale ETHE_695",
        address!("c2bAC1cC7d684918b2eC09A3F7E77AF858aE9B29"),
    );
    m.insert(
        "Grayscale ETHE_696",
        address!("C384dd01cEFccCA8c0E4f3A26158e332c4cC3f34"),
    );
    m.insert(
        "Grayscale ETHE_697",
        address!("C39Af5291b2d9BAa38E49726832bc221BC9eb2DF"),
    );
    m.insert(
        "Grayscale ETHE_698",
        address!("c40464f629449E5B43a3772d4C0FE929Bade8ab9"),
    );
    m.insert(
        "Grayscale ETHE_699",
        address!("c40Cc765114393aEEaae82B74d021B61A0c11409"),
    );
    m.insert(
        "Grayscale ETHE_7",
        address!("024685DdED24F244e610d6023c3c4cbfB2b0e32D"),
    );
    m.insert(
        "Grayscale ETHE_70",
        address!("13424CE5cEA47d90Ef9b80d576DC4571C04345c7"),
    );
    m.insert(
        "Grayscale ETHE_700",
        address!("C438BC455ee34FD0469Ce709FA702DB1F0AF476A"),
    );
    m.insert(
        "Grayscale ETHE_701",
        address!("c439E34bF68d0bC4E116D8FE1bAeE89EefBD42D2"),
    );
    m.insert(
        "Grayscale ETHE_702",
        address!("C451f2F0A9348A6bC1C483aD0Ca46D7BC872d628"),
    );
    m.insert(
        "Grayscale ETHE_703",
        address!("c4a99353382616531ED7CEA9cfAFAF7b816F1AF7"),
    );
    m.insert(
        "Grayscale ETHE_704",
        address!("c515c281996118C3eb7e9985D079280c8347c8fe"),
    );
    m.insert(
        "Grayscale ETHE_705",
        address!("C53217a9C7eadbBF52b1515B9eB8f6D1B6FB6860"),
    );
    m.insert(
        "Grayscale ETHE_706",
        address!("C5f33B0751cbCCD6ea28a31A239b9DAb09dE8a1b"),
    );
    m.insert(
        "Grayscale ETHE_707",
        address!("c606c9c0F138A69b7b80491902f3d456e431ca04"),
    );
    m.insert(
        "Grayscale ETHE_708",
        address!("C61Fe4124F587A545B8DEe0072B4289b37e9F478"),
    );
    m.insert(
        "Grayscale ETHE_709",
        address!("C646a017997B450337423EA3b39e4482BBcb57e4"),
    );
    m.insert(
        "Grayscale ETHE_71",
        address!("134AC9752134362C82256981eb1c4dFfCcd0DF2e"),
    );
    m.insert(
        "Grayscale ETHE_710",
        address!("c685BAD74E7b3cad4eBa1Be9403F7e115295bd42"),
    );
    m.insert(
        "Grayscale ETHE_711",
        address!("c6911AA6C584B82aAa775a9DE9BC3054B4f2ba42"),
    );
    m.insert(
        "Grayscale ETHE_712",
        address!("C698BbD66a41adBce31F514DBFE278B7c43E4bB4"),
    );
    m.insert(
        "Grayscale ETHE_713",
        address!("c71a3d3A5a1eCc158384b59ad5e971D2C0a9Dbcd"),
    );
    m.insert(
        "Grayscale ETHE_714",
        address!("C72687db237D6055A9C013b09d822B563ec8691A"),
    );
    m.insert(
        "Grayscale ETHE_715",
        address!("c741d00DE2479C7216C33682E622C15Be0d94f8c"),
    );
    m.insert(
        "Grayscale ETHE_716",
        address!("c7a7fd11c1575d5A965e4Ebe670481d8B58f27C0"),
    );
    m.insert(
        "Grayscale ETHE_717",
        address!("c7b2b478AcaA7aCebD4245a9A824Af21221cD3F0"),
    );
    m.insert(
        "Grayscale ETHE_718",
        address!("C8395C066a8457D646D991f31ad6d8951d7162B4"),
    );
    m.insert(
        "Grayscale ETHE_719",
        address!("c8CcFBf8BE0164ef82743892d203e89dA86eC5bE"),
    );
    m.insert(
        "Grayscale ETHE_72",
        address!("139057617F9f88C82647E09424bfC0c745782Bef"),
    );
    m.insert(
        "Grayscale ETHE_720",
        address!("C8D56677d41F87aD7c4DC86023edB6C02dc7a29C"),
    );
    m.insert(
        "Grayscale ETHE_721",
        address!("C8F7A3F4791CA446CbBd4cB75f03299FE0602f99"),
    );
    m.insert(
        "Grayscale ETHE_722",
        address!("C92FCa40dAbcdEC6ba56932d87540B68A34FdFf9"),
    );
    m.insert(
        "Grayscale ETHE_723",
        address!("C958025075f9892360e9FbA962fc864766dAD1BC"),
    );
    m.insert(
        "Grayscale ETHE_724",
        address!("c9d83679aDFE23D07e2cc91C5946861f1f24b346"),
    );
    m.insert(
        "Grayscale ETHE_725",
        address!("c9FE147123bB185a8d37536294D69f1c403F9894"),
    );
    m.insert(
        "Grayscale ETHE_726",
        address!("Ca19401610E2222f19Cb9D47E53e9e29535e87E3"),
    );
    m.insert(
        "Grayscale ETHE_727",
        address!("Ca2C2B40c555a74CD444C1250Cc4a2dAd6CFBd6a"),
    );
    m.insert(
        "Grayscale ETHE_728",
        address!("ca2DD8f07407EE31201E03876F65F1f0e69eF103"),
    );
    m.insert(
        "Grayscale ETHE_729",
        address!("CA2E89110644D61361e188e322115267142eA3E9"),
    );
    m.insert(
        "Grayscale ETHE_73",
        address!("13C1FEBd46072f874fA54616b48fd1b1c2E67d56"),
    );
    m.insert(
        "Grayscale ETHE_730",
        address!("cA86E7ea0cebcCF00f89B4eC1FcB70655f528A81"),
    );
    m.insert(
        "Grayscale ETHE_731",
        address!("Ca95b7F44c35DBedf586Ae05f165779eeA23Db36"),
    );
    m.insert(
        "Grayscale ETHE_732",
        address!("cafe0a8846dF104Da9b760e484A98f63447971Eb"),
    );
    m.insert(
        "Grayscale ETHE_733",
        address!("CB54c301ab3fF4aa1bc38C979427360F47369633"),
    );
    m.insert(
        "Grayscale ETHE_734",
        address!("cc2C15A7fEC4958c71a5555807dFf588D8517DD8"),
    );
    m.insert(
        "Grayscale ETHE_735",
        address!("Cc3B4Ca51cA1A372c560136fbA16DD7D32A0967b"),
    );
    m.insert(
        "Grayscale ETHE_736",
        address!("cc87FF23c187A9b320D52D284C7A105f781e4B57"),
    );
    m.insert(
        "Grayscale ETHE_737",
        address!("cC8e4606Db0983Ca93b45c1b1436758EEcD2bD37"),
    );
    m.insert(
        "Grayscale ETHE_738",
        address!("cc979D2D40CCfaA353Fd2E892d3f1AAEfb6975CA"),
    );
    m.insert(
        "Grayscale ETHE_739",
        address!("cC98164e4B9C8EfdEB20C1A2625eF941cd02dC08"),
    );
    m.insert(
        "Grayscale ETHE_74",
        address!("143221bD51dbA017bCd33D5b65B8C576B7797355"),
    );
    m.insert(
        "Grayscale ETHE_740",
        address!("CCF4246e36E158258ED4Cb34FdBac6863DA7B526"),
    );
    m.insert(
        "Grayscale ETHE_741",
        address!("Cd10bF12FEC45Af1b2EA93337FDA543fA923f760"),
    );
    m.insert(
        "Grayscale ETHE_742",
        address!("cd2926E14c805d3391BF67223F2B7a2fa48BE175"),
    );
    m.insert(
        "Grayscale ETHE_743",
        address!("CD93e382eEaBd806023183D2De564459053Bfe3e"),
    );
    m.insert(
        "Grayscale ETHE_744",
        address!("cDa68773011fF8d36CE165091be4959898b06f1c"),
    );
    m.insert(
        "Grayscale ETHE_745",
        address!("CE4F9695EE844E079Cb102Ff965F6f18Dd5b37B7"),
    );
    m.insert(
        "Grayscale ETHE_746",
        address!("cE6cA7Fc8eceB2D26A874F2f7CE54B21C0C2e59A"),
    );
    m.insert(
        "Grayscale ETHE_747",
        address!("Ce79A744E97B379422FED0D1D9bCDE6dBE01e304"),
    );
    m.insert(
        "Grayscale ETHE_748",
        address!("cEB7d894b3C7B1d5327fC41BC18dcB2B4fa2e764"),
    );
    m.insert(
        "Grayscale ETHE_749",
        address!("cec19c7a3Cf460162eC33918c9256436192C5a88"),
    );
    m.insert(
        "Grayscale ETHE_75",
        address!("146871534fbc39Fd25328daeDD9225D6Ff0F2535"),
    );
    m.insert(
        "Grayscale ETHE_750",
        address!("CECF531f239BF2d44FD3B0137cEfC1832E993d80"),
    );
    m.insert(
        "Grayscale ETHE_751",
        address!("CF4a936F951D56fd38E457B7f3b2eb8f4f092727"),
    );
    m.insert(
        "Grayscale ETHE_752",
        address!("cF86E114D4f6706702266c35F25a9F1C9922d3b3"),
    );
    m.insert(
        "Grayscale ETHE_753",
        address!("CFb5dAf7f7B3cE2c4f8C22bCd1D7C764acA63B41"),
    );
    m.insert(
        "Grayscale ETHE_754",
        address!("CFc15DDeE4F921A0917D35Bb70174E14f6D45269"),
    );
    m.insert(
        "Grayscale ETHE_755",
        address!("cfce3Ab710e5EC5aD8434b7DAa50E4EbCAD44299"),
    );
    m.insert(
        "Grayscale ETHE_756",
        address!("d05fB26F19C64D0fb0942bA2939e1b5977b4177f"),
    );
    m.insert(
        "Grayscale ETHE_757",
        address!("d07e0c45e63d638aDFe2725C54206895bdBADd14"),
    );
    m.insert(
        "Grayscale ETHE_758",
        address!("D0D50bd6933668D962F8b601b155a5A6a2D2d178"),
    );
    m.insert(
        "Grayscale ETHE_759",
        address!("D1A150ccba297f1E160442aC3DEC8849f8eA5Afc"),
    );
    m.insert(
        "Grayscale ETHE_76",
        address!("14C728C9aEeAfCe01f1A7b87d02255dD4326f180"),
    );
    m.insert(
        "Grayscale ETHE_760",
        address!("D24f91b642699Bf73FbAe191F3fe3748a2b2e70c"),
    );
    m.insert(
        "Grayscale ETHE_761",
        address!("d2f7164935435E5423193c3d10337A2CdcfA154D"),
    );
    m.insert(
        "Grayscale ETHE_762",
        address!("d3171f648d8972c6CDF364446fD100F863E9e6d6"),
    );
    m.insert(
        "Grayscale ETHE_763",
        address!("D324D5634a6bf87BaA7b25027fbC6101ACDfFA87"),
    );
    m.insert(
        "Grayscale ETHE_764",
        address!("d32c52849aD7241306546113b2073310aD42322E"),
    );
    m.insert(
        "Grayscale ETHE_765",
        address!("D3317f334dB5106feb8d5E13433D4C6AB906A304"),
    );
    m.insert(
        "Grayscale ETHE_766",
        address!("d33fCB93542B761c92f1a0f33E5cEE0ba9655C86"),
    );
    m.insert(
        "Grayscale ETHE_767",
        address!("D35297EdF178d2fa55374F578C381ec379217c86"),
    );
    m.insert(
        "Grayscale ETHE_768",
        address!("D367eE265D151b3A06b51fa21d73671C1123da0a"),
    );
    m.insert(
        "Grayscale ETHE_769",
        address!("d3738b7c4Fc14FdF4D79a7563A71D17BBd2d6326"),
    );
    m.insert(
        "Grayscale ETHE_77",
        address!("151F202173147bC2c27B92E2341474E08E169714"),
    );
    m.insert(
        "Grayscale ETHE_770",
        address!("D382fD5c8A47866C79b296E3914c7c8AebA40994"),
    );
    m.insert(
        "Grayscale ETHE_771",
        address!("d404316d5BB0641853edaAE62A92CCb6F2d820bc"),
    );
    m.insert(
        "Grayscale ETHE_772",
        address!("d42a5BA156f6F2747652740620ed642B0Aa02fe9"),
    );
    m.insert(
        "Grayscale ETHE_773",
        address!("d49Df8e16AbcAc8846A7e23431320EbeBD80fE0a"),
    );
    m.insert(
        "Grayscale ETHE_774",
        address!("d4bE641AE3e926cf5F754bB3ac18264Cc65086DE"),
    );
    m.insert(
        "Grayscale ETHE_775",
        address!("d510Be34915EF01A74c2Cf86B1A8D7ae47243784"),
    );
    m.insert(
        "Grayscale ETHE_776",
        address!("d53000273d5eb404c8B5A57Ac7985648768a1384"),
    );
    m.insert(
        "Grayscale ETHE_777",
        address!("D5c150b0C0983b5396889eBA8489C7Fc642dbEAd"),
    );
    m.insert(
        "Grayscale ETHE_778",
        address!("d5F1c56e71b89fEe48dFCb6872E1eA422723eCB1"),
    );
    m.insert(
        "Grayscale ETHE_779",
        address!("d68fB33E14bc9bFd95854226262148C85C32dB1d"),
    );
    m.insert(
        "Grayscale ETHE_78",
        address!("1531B0467896ADe3b46A111999da55A83922C7FC"),
    );
    m.insert(
        "Grayscale ETHE_780",
        address!("d6d07c8f0308832551a17D0B6bC049949DD1504a"),
    );
    m.insert(
        "Grayscale ETHE_781",
        address!("D6e1326914d5c332FfAA7a3F7bBa4f8f60AcaACD"),
    );
    m.insert(
        "Grayscale ETHE_782",
        address!("D6ee647f7990E308cB99a767848d7F49cc4629cf"),
    );
    m.insert(
        "Grayscale ETHE_783",
        address!("D737d3Ba9A8735fC050a7c48fb3C10a12d9DE14F"),
    );
    m.insert(
        "Grayscale ETHE_784",
        address!("D7eFd7a45B2affaAb9DEe3713321eEB1e0a9DFBB"),
    );
    m.insert(
        "Grayscale ETHE_785",
        address!("d81E55288737eC99263dAd3a26f6308E39c01CA6"),
    );
    m.insert(
        "Grayscale ETHE_786",
        address!("D83754BDf786687e5e0Cfcc0CB88fadAd764A8b4"),
    );
    m.insert(
        "Grayscale ETHE_787",
        address!("d85Cb52760558dF986E6F594d5d8059bA439556E"),
    );
    m.insert(
        "Grayscale ETHE_788",
        address!("D883fF5eD913dbC5ec523dD11E1D607879373Eb4"),
    );
    m.insert(
        "Grayscale ETHE_789",
        address!("d88f7c2748C3C07C1a58167a2c26dD3fE4F8ffe9"),
    );
    m.insert(
        "Grayscale ETHE_79",
        address!("15551907810F1f1bEFD97f35bfFC41dC348b433a"),
    );
    m.insert(
        "Grayscale ETHE_790",
        address!("D91BA9fA219Cc9bF05624A472dE472Fe526E74F4"),
    );
    m.insert(
        "Grayscale ETHE_791",
        address!("d91E7cc8C48411f81972c1503a1600ea4e627410"),
    );
    m.insert(
        "Grayscale ETHE_792",
        address!("D939002bC74aF734649555E10654B5B5f5A13FEC"),
    );
    m.insert(
        "Grayscale ETHE_793",
        address!("d9d99bc4A95ceB64F5B9Dc709d790dd5C4c7516f"),
    );
    m.insert(
        "Grayscale ETHE_794",
        address!("dA3E74e10f6c789443366A06537c29f7df2105fb"),
    );
    m.insert(
        "Grayscale ETHE_795",
        address!("DA5967A3E6E4f67E00fd5A75D67FD58CbEFCde9e"),
    );
    m.insert(
        "Grayscale ETHE_796",
        address!("dA6Faeb52659DB77dfFc80A1cF3980f4CdcC5bD7"),
    );
    m.insert(
        "Grayscale ETHE_797",
        address!("DBf9B5f4b097f992b18B8203a62d9F3e74997d22"),
    );
    m.insert(
        "Grayscale ETHE_798",
        address!("dc990C399128275f0C16Eb57b6f13cB28e98297A"),
    );
    m.insert(
        "Grayscale ETHE_799",
        address!("dcF0e258c3A627d61e9Ec947bdd145661B4d11D7"),
    );
    m.insert(
        "Grayscale ETHE_8",
        address!("028ad651f143158581Fccb9793B08A58246dA693"),
    );
    m.insert(
        "Grayscale ETHE_80",
        address!("158d733EFd96495AE265b9C85DDc7B4d4966eAcd"),
    );
    m.insert(
        "Grayscale ETHE_800",
        address!("DCf57A5b6a8a41231858b242C163a77D1579EdEC"),
    );
    m.insert(
        "Grayscale ETHE_801",
        address!("DD0611037364EAa4621cC9576843fAdE310C49E1"),
    );
    m.insert(
        "Grayscale ETHE_802",
        address!("dd3aE84880e8D07f0151fC9010360007Bf93cD3b"),
    );
    m.insert(
        "Grayscale ETHE_803",
        address!("DD460BFc5274243D92FeA89d19e3f1afE1476c25"),
    );
    m.insert(
        "Grayscale ETHE_804",
        address!("dD504D37A1c420bA148202500515Cccb8360c7B7"),
    );
    m.insert(
        "Grayscale ETHE_805",
        address!("dd524A2a0d6914Cb2c04C2A16bf8716aCa51312B"),
    );
    m.insert(
        "Grayscale ETHE_806",
        address!("DD6eaf823131dF00749a0694C46EC51D8346E94e"),
    );
    m.insert(
        "Grayscale ETHE_807",
        address!("DdD3eE8ADc6fbA23b5323E667a5a817d33beD8B7"),
    );
    m.insert(
        "Grayscale ETHE_808",
        address!("DDDf06375DaF0546DE4f7529d3BfBB12804A88c5"),
    );
    m.insert(
        "Grayscale ETHE_809",
        address!("dDEb7F30944B4EFB72576100fE4aC493e97e3af7"),
    );
    m.insert(
        "Grayscale ETHE_81",
        address!("15ACA85DbAF8E2b98822A269156aD1D1459F499E"),
    );
    m.insert(
        "Grayscale ETHE_810",
        address!("DE2aFfA03d4020A44336685E893d82eB8F59BE2e"),
    );
    m.insert(
        "Grayscale ETHE_811",
        address!("DE34b24bC0f0Ebb4fbB4617de26d9b94f88eCF2d"),
    );
    m.insert(
        "Grayscale ETHE_812",
        address!("de8CbF72b73fe2409CC107970f4D9Ee189efEe2D"),
    );
    m.insert(
        "Grayscale ETHE_813",
        address!("deB294797D166D580675Dc5aAEF1A25378Ccb626"),
    );
    m.insert(
        "Grayscale ETHE_814",
        address!("DEb9Cc22cd17136CCE26f9341b81d2A5c83beCA5"),
    );
    m.insert(
        "Grayscale ETHE_815",
        address!("DEEeCdB8bD3B451d854459eAb41116Faa81b2fcC"),
    );
    m.insert(
        "Grayscale ETHE_816",
        address!("Df4c374a2a761d2fd6d074fCcBc76967C92a6dBE"),
    );
    m.insert(
        "Grayscale ETHE_817",
        address!("dF70039aEFcF66e94378f306f97C0f3765891E12"),
    );
    m.insert(
        "Grayscale ETHE_818",
        address!("DFc3653588183F0d5C79a42776830106486B8a34"),
    );
    m.insert(
        "Grayscale ETHE_819",
        address!("dfF48a525411d37355d7292C2F5706055507e601"),
    );
    m.insert(
        "Grayscale ETHE_82",
        address!("15aDE2264508cd4FE63698665a8823a77346BB26"),
    );
    m.insert(
        "Grayscale ETHE_820",
        address!("E06e9f3bDED52930107B34bD326f89e00d68CDD5"),
    );
    m.insert(
        "Grayscale ETHE_821",
        address!("E1024372E8335Fc3601eb6996cf3dae92eDbb904"),
    );
    m.insert(
        "Grayscale ETHE_822",
        address!("e156e01502fDa3A467E713F3B49Ce72c726d06AF"),
    );
    m.insert(
        "Grayscale ETHE_823",
        address!("E16913333978a0f756cAEdD1294307922B760CC5"),
    );
    m.insert(
        "Grayscale ETHE_824",
        address!("E1A9C2f6FC229f6e7094c49DF210761d90ACCd51"),
    );
    m.insert(
        "Grayscale ETHE_825",
        address!("E2467DBd5ef32a03f4650Af1DD665e734CaB58Ae"),
    );
    m.insert(
        "Grayscale ETHE_826",
        address!("E263aE2285b1e697A9e5C76fEa6657693F7b6d49"),
    );
    m.insert(
        "Grayscale ETHE_827",
        address!("e28498437F2818355A9C1bff93Cb3DD590Ff4e2a"),
    );
    m.insert(
        "Grayscale ETHE_828",
        address!("e298Bdc568F348fc7F0AE26f7C6Cd2039359e0B5"),
    );
    m.insert(
        "Grayscale ETHE_829",
        address!("E2bb4c6272D87A7F7E138C8B92A7D0a66fD47BCE"),
    );
    m.insert(
        "Grayscale ETHE_83",
        address!("16c59455eD84328e787E1b84f04Dc56063AAAEc0"),
    );
    m.insert(
        "Grayscale ETHE_830",
        address!("E2D3E2f61be5Bd2Ff65d931B94B0cf25aC63cE49"),
    );
    m.insert(
        "Grayscale ETHE_831",
        address!("E2defbbD57b9a81ff9dEE266e45492348FC8B2f8"),
    );
    m.insert(
        "Grayscale ETHE_832",
        address!("E355f1d3bAC35fcA6789570dfeA7648ac6a403d5"),
    );
    m.insert(
        "Grayscale ETHE_833",
        address!("e39152e4D1C9FF4F7Ae1E25dd7f0b8283999Bf19"),
    );
    m.insert(
        "Grayscale ETHE_834",
        address!("e4021CE8204fB271C09652f1842fcc5ADD47a9CB"),
    );
    m.insert(
        "Grayscale ETHE_835",
        address!("e467cDaB8ed6d14BaF2742352891FB16A9f76736"),
    );
    m.insert(
        "Grayscale ETHE_836",
        address!("E48B9FB8A44A6eaD33EEC616f98b922bfC8bc270"),
    );
    m.insert(
        "Grayscale ETHE_837",
        address!("e53E7FEe9D6BB139997aCEAc12aA5515768448F4"),
    );
    m.insert(
        "Grayscale ETHE_838",
        address!("E53F97a19f06c10De79862b66026e3aCDfF5607e"),
    );
    m.insert(
        "Grayscale ETHE_839",
        address!("E561CE56084bD11c0033B00209E8A4064B9a0159"),
    );
    m.insert(
        "Grayscale ETHE_84",
        address!("16E73D37Ff1FEaCED4baC01ebeF8879300Ea2b65"),
    );
    m.insert(
        "Grayscale ETHE_840",
        address!("E693AA3eB806A1D67dA66CA42DfF61010347D956"),
    );
    m.insert(
        "Grayscale ETHE_841",
        address!("e6c43632B0D657eBb7B9352BD18C7Ce79cE1221c"),
    );
    m.insert(
        "Grayscale ETHE_842",
        address!("E6c61D62411a5EDE0213646596B9264b5926d0AF"),
    );
    m.insert(
        "Grayscale ETHE_843",
        address!("e6dD0608F6f9bF589470A7Be991e25E3d76E62EE"),
    );
    m.insert(
        "Grayscale ETHE_844",
        address!("e6E90d59CB21F34a4268c9e18897c5bafb692c38"),
    );
    m.insert(
        "Grayscale ETHE_845",
        address!("E71cB75836f7F91276a1480C7E5ACf8378781bCb"),
    );
    m.insert(
        "Grayscale ETHE_846",
        address!("E71e91c4C151F8421033Ec2627D86fB749C58981"),
    );
    m.insert(
        "Grayscale ETHE_847",
        address!("e7948d408cb475ECb1a09999eE227a69EeD1F566"),
    );
    m.insert(
        "Grayscale ETHE_848",
        address!("E79a28F0089f94977251aaD279c834bd31DeaF30"),
    );
    m.insert(
        "Grayscale ETHE_849",
        address!("E7DF07f59156209ACb4e752a9f6f11844f93b722"),
    );
    m.insert(
        "Grayscale ETHE_85",
        address!("1714c52a6a6Dd686e8757CC28AAC150Ab746699E"),
    );
    m.insert(
        "Grayscale ETHE_850",
        address!("e7Ed02024a70680432bf4Bb2Aa9D2F5CA69B7C7b"),
    );
    m.insert(
        "Grayscale ETHE_851",
        address!("e80F2ddDcA556F9c9986a6F1E4F811B91948ae5b"),
    );
    m.insert(
        "Grayscale ETHE_852",
        address!("e8494fcB661EF9eDe4C0EFF5b49972D5eE48BB43"),
    );
    m.insert(
        "Grayscale ETHE_853",
        address!("e849ad35dA9560A67CeC7236dfa3C3246b939396"),
    );
    m.insert(
        "Grayscale ETHE_854",
        address!("E86838c5CCEefFD2A05bfB0B58aC87B153efF140"),
    );
    m.insert(
        "Grayscale ETHE_855",
        address!("e88b2F250E5719D015d40a5fD9b636DECa4E6180"),
    );
    m.insert(
        "Grayscale ETHE_856",
        address!("e8Dc22F57BC0Be62b76F21d1d41Bc0b53e9bde64"),
    );
    m.insert(
        "Grayscale ETHE_857",
        address!("ea039cf1857Bd0e14919c4EF1A8B332A83110BFF"),
    );
    m.insert(
        "Grayscale ETHE_858",
        address!("Ea28D8a2711f39D9F10B50878177360791500Cb6"),
    );
    m.insert(
        "Grayscale ETHE_859",
        address!("Ea481D5f0015d67F32854E45fe8F3af7b54c5575"),
    );
    m.insert(
        "Grayscale ETHE_86",
        address!("174A07844d29CA07638C6c5bEa2E88d3cb011c4e"),
    );
    m.insert(
        "Grayscale ETHE_860",
        address!("eb28e97A5Af30c62D55d8A81B193408d01FDD9ea"),
    );
    m.insert(
        "Grayscale ETHE_861",
        address!("eb3d5ddcdf38b898879c8A76A2e8cC3FD12a52Dd"),
    );
    m.insert(
        "Grayscale ETHE_862",
        address!("EBBBE02E2b41C17870152a27E8BEE518211854d9"),
    );
    m.insert(
        "Grayscale ETHE_863",
        address!("ebc89c80e20cE10194ff7f4B25124824CDBD5056"),
    );
    m.insert(
        "Grayscale ETHE_864",
        address!("ebDa0F76B6D941f2339a4A5B502aA8820B99DB34"),
    );
    m.insert(
        "Grayscale ETHE_865",
        address!("ec50949B71aABbD8A883eE12143F3cA9D0688870"),
    );
    m.insert(
        "Grayscale ETHE_866",
        address!("eD058B444dFEc7aDabDEEEd0fE7f2E67d8dFf478"),
    );
    m.insert(
        "Grayscale ETHE_867",
        address!("ede60080bD1407a65823C1001fd56aA9a3a3c4c4"),
    );
    m.insert(
        "Grayscale ETHE_868",
        address!("Ee141c878A47B9d8e136d03343dfeEF02985661A"),
    );
    m.insert(
        "Grayscale ETHE_869",
        address!("Ee675f59bA9A34Ca382A0067d50998028a1e1293"),
    );
    m.insert(
        "Grayscale ETHE_87",
        address!("1775642e42576c68A8919E4e35C05cAF8c94adEE"),
    );
    m.insert(
        "Grayscale ETHE_870",
        address!("eE731cF1b54A0E21aB0bD8465e3e396C1C2BcA40"),
    );
    m.insert(
        "Grayscale ETHE_871",
        address!("EeB4c136D37dEF3050DC41d0021a82F922E1f7E9"),
    );
    m.insert(
        "Grayscale ETHE_872",
        address!("eeD94941e5b6d503ab6e5d43B9599BAbae798801"),
    );
    m.insert(
        "Grayscale ETHE_873",
        address!("EF143D0ce31268676b4962E940BDe1B24fFE3BDD"),
    );
    m.insert(
        "Grayscale ETHE_874",
        address!("Ef24B7471613d9E74b0CA53D15f65e1a12738643"),
    );
    m.insert(
        "Grayscale ETHE_875",
        address!("ef50cfeAD7e67d49053c1698C27C9a3b0eB5A24A"),
    );
    m.insert(
        "Grayscale ETHE_876",
        address!("ef6C7e7409007b3a88d246F4af6aBA7D4264E336"),
    );
    m.insert(
        "Grayscale ETHE_877",
        address!("eF89827ec0e4Cce06fDD89E2d7252Bb4cEe7A1A9"),
    );
    m.insert(
        "Grayscale ETHE_878",
        address!("f0282099d251b292Fa64DfBc0d5fbDAcab91d9B7"),
    );
    m.insert(
        "Grayscale ETHE_879",
        address!("f0297345C515d7BCE09C7ec29E3454dB91E936Da"),
    );
    m.insert(
        "Grayscale ETHE_88",
        address!("177D756d34D1763962Cb61446cfC427Ff3E6ee44"),
    );
    m.insert(
        "Grayscale ETHE_880",
        address!("f030AB48A11787DD08222565864271e6Cf206C84"),
    );
    m.insert(
        "Grayscale ETHE_881",
        address!("f0B9DB9A9EED7B55B2e96Aa512F5D99527Af687e"),
    );
    m.insert(
        "Grayscale ETHE_882",
        address!("f1757AD7FB125701B52dD5514cf1F5edD1Db199D"),
    );
    m.insert(
        "Grayscale ETHE_883",
        address!("f1DCFa0837faA2a0ceD8849cB3bF312e163Ce412"),
    );
    m.insert(
        "Grayscale ETHE_884",
        address!("F1fD8009C90a7313B755924502F7AD08bb94DFDB"),
    );
    m.insert(
        "Grayscale ETHE_885",
        address!("F250b4ACd5855362DF708771DDF7280e8e4167b7"),
    );
    m.insert(
        "Grayscale ETHE_886",
        address!("f2573181DDE7d17AF446061e2bB4c8972E8D0171"),
    );
    m.insert(
        "Grayscale ETHE_887",
        address!("f260d8875a8261BA8c211b2857b7c69B4253A53E"),
    );
    m.insert(
        "Grayscale ETHE_888",
        address!("F28ad8F9Cb568D1B932573A76EAAd983A29904Ad"),
    );
    m.insert(
        "Grayscale ETHE_889",
        address!("f28eDd61b07f2B8874b8aBFE96F280ba77A42149"),
    );
    m.insert(
        "Grayscale ETHE_89",
        address!("17872CBE1c30D707B40b0e5Ab87393B53649082A"),
    );
    m.insert(
        "Grayscale ETHE_890",
        address!("F3348D6994C1D25Ca95a79B996E378F8e4eD22aC"),
    );
    m.insert(
        "Grayscale ETHE_891",
        address!("F35547c48Adc629D61FeA127e419DcF149Bb54d9"),
    );
    m.insert(
        "Grayscale ETHE_892",
        address!("F35F3B032C8bEB33a2Ec88057630d9a269eEc735"),
    );
    m.insert(
        "Grayscale ETHE_893",
        address!("f38356f7c5e47926c0417bf2763b5C44a00336b7"),
    );
    m.insert(
        "Grayscale ETHE_894",
        address!("F39c4573fA757fbc94151776Caaa0AfeE45950D0"),
    );
    m.insert(
        "Grayscale ETHE_895",
        address!("f3C4c3793bDF2655A536Ee0b76eC4AC4B6541B88"),
    );
    m.insert(
        "Grayscale ETHE_896",
        address!("f3C966A09C119ddB5389bb0A2671236c1823a363"),
    );
    m.insert(
        "Grayscale ETHE_897",
        address!("F3De466383196dDc78771F7D112a78EA33D75Ff5"),
    );
    m.insert(
        "Grayscale ETHE_898",
        address!("f3F2184fA4F29FF2a1B53Ce2B0939871183F135c"),
    );
    m.insert(
        "Grayscale ETHE_899",
        address!("F404806324deC457a0C5a8fB49302C8707b48386"),
    );
    m.insert(
        "Grayscale ETHE_9",
        address!("02EE13fFd5CF5E6731d4CeAB28394074Bb185F61"),
    );
    m.insert(
        "Grayscale ETHE_90",
        address!("17b153aa3Abe80655B558E52E478F0B3968bFf16"),
    );
    m.insert(
        "Grayscale ETHE_900",
        address!("F47d9B586F9948c7B3fC533eDFB25bf3fBeA3aF8"),
    );
    m.insert(
        "Grayscale ETHE_901",
        address!("F49Bbaec56f80f938700cD07F214be8442957755"),
    );
    m.insert(
        "Grayscale ETHE_902",
        address!("f4A07548a9D9e79D8d8D56E4C6f9297E502d690A"),
    );
    m.insert(
        "Grayscale ETHE_903",
        address!("f4e4B05b504ffCdeddA1F07d07255890a92a643E"),
    );
    m.insert(
        "Grayscale ETHE_904",
        address!("F505b552eAbb108ecde8C96a558c12C4dc4A15FB"),
    );
    m.insert(
        "Grayscale ETHE_905",
        address!("f515c80c77aD5245A6bE51aB5C89526bBbaF9855"),
    );
    m.insert(
        "Grayscale ETHE_906",
        address!("F53ef57b0F32Ed0d151e4e6eeE8C66c919dbd202"),
    );
    m.insert(
        "Grayscale ETHE_907",
        address!("f5552Bc2Cdaf997DDB2d08962ADdEB5C27bABf62"),
    );
    m.insert(
        "Grayscale ETHE_908",
        address!("f5B4386394cB52999d43517bF89AcB4fe902Fa09"),
    );
    m.insert(
        "Grayscale ETHE_909",
        address!("F5e122B93A6e2cF2B5D09B525780eb71B2A1b3D0"),
    );
    m.insert(
        "Grayscale ETHE_91",
        address!("17e8C689c143e373A9c595Fd02c8Dd641Db9A6D4"),
    );
    m.insert(
        "Grayscale ETHE_910",
        address!("F6e38aAC274EdbB4173643eA718d7002978E755a"),
    );
    m.insert(
        "Grayscale ETHE_911",
        address!("F72f8d7a0337EBdBa0FB85162d3a12e007b51F0D"),
    );
    m.insert(
        "Grayscale ETHE_912",
        address!("F732CCc723213d4bFC346bd346B51F150361cd79"),
    );
    m.insert(
        "Grayscale ETHE_913",
        address!("f77f5b8cAffb6Fdf9013809dF607b7aEd05767eA"),
    );
    m.insert(
        "Grayscale ETHE_914",
        address!("f7ECE7b64e6276924ffcd822ABA139f35afB99a6"),
    );
    m.insert(
        "Grayscale ETHE_915",
        address!("F820502be44549169a4bC136AfafD714d1d25708"),
    );
    m.insert(
        "Grayscale ETHE_916",
        address!("F8626e68A1f544E6fdA851fE089c4B8435223F37"),
    );
    m.insert(
        "Grayscale ETHE_917",
        address!("F8D1E3f35820c6BF262592dFaAe5DbEa24cA404A"),
    );
    m.insert(
        "Grayscale ETHE_918",
        address!("F939Ab194400Da7d12b5Def3f753b1423E6a45e3"),
    );
    m.insert(
        "Grayscale ETHE_919",
        address!("f9Ad8f6CDE350a9731ef7c77FC588Ce88Def56Db"),
    );
    m.insert(
        "Grayscale ETHE_92",
        address!("180c0c920843a35dDb6BFEEA5dd6436F6Ba7CC61"),
    );
    m.insert(
        "Grayscale ETHE_920",
        address!("f9b993a7136733aD019d5B2497E90a0887fb1049"),
    );
    m.insert(
        "Grayscale ETHE_921",
        address!("f9BadC1EaFfBB01BB132872CFE928AD1121e5438"),
    );
    m.insert(
        "Grayscale ETHE_922",
        address!("F9Ce8350D3A3E132E9Fd75660fD1F01541d31289"),
    );
    m.insert(
        "Grayscale ETHE_923",
        address!("f9F160B50C4B54E9FE639C7438251ffff57749Bd"),
    );
    m.insert(
        "Grayscale ETHE_924",
        address!("Fa57EaA79213Ab1bE91F4C52e5a267a5Ca49b244"),
    );
    m.insert(
        "Grayscale ETHE_925",
        address!("FA923a35AC776c1c4f807fA30E0fe62643F26134"),
    );
    m.insert(
        "Grayscale ETHE_926",
        address!("FaF7B76670F4265d9d8D57022998c105898064e1"),
    );
    m.insert(
        "Grayscale ETHE_927",
        address!("fb9cACD26D1C249757A7bFf78A514bca734BEE8B"),
    );
    m.insert(
        "Grayscale ETHE_928",
        address!("fc0eeB73aE90BD6dB3CcCD036ca4dDfd32020118"),
    );
    m.insert(
        "Grayscale ETHE_929",
        address!("FC48a70F1121196371020aA61B4f03327Be60301"),
    );
    m.insert(
        "Grayscale ETHE_93",
        address!("18172CB661F0AC82E1acAAdD3c83Cb1c61736d9c"),
    );
    m.insert(
        "Grayscale ETHE_930",
        address!("fCa5d74e9faC2aA672D305F5Eab40c0639C1ad51"),
    );
    m.insert(
        "Grayscale ETHE_931",
        address!("fCaBf30a584ED10b4EB58877AE173452a5De12c9"),
    );
    m.insert(
        "Grayscale ETHE_932",
        address!("FCB8e0FB3bb60Cb7E63Cb3203Aee3F039f48119A"),
    );
    m.insert(
        "Grayscale ETHE_933",
        address!("Fd0Bcb76f5Fce547c7e783C85205442891f4b743"),
    );
    m.insert(
        "Grayscale ETHE_934",
        address!("FD3e8b8B6e4DDFdD25d0a79Bc99E7cdbF287199d"),
    );
    m.insert(
        "Grayscale ETHE_935",
        address!("fD58F0BD175d6695C4faaEA04A5DB85A149bb479"),
    );
    m.insert(
        "Grayscale ETHE_936",
        address!("fD71c87DA768207dB2B8532cab368AaD8CDE9Cc3"),
    );
    m.insert(
        "Grayscale ETHE_937",
        address!("fdFb48A407d436530E2732dF52d39c9c63995e42"),
    );
    m.insert(
        "Grayscale ETHE_938",
        address!("fE1448C64198126eAcbe3E27375a529327bE3D3A"),
    );
    m.insert(
        "Grayscale ETHE_939",
        address!("Fe96B5239D60fe339542c3f0f3d389b93710aDE5"),
    );
    m.insert(
        "Grayscale ETHE_94",
        address!("188690650Ef51C16FE8959BE63Ddb4dA3c25d7C7"),
    );
    m.insert(
        "Grayscale ETHE_940",
        address!("FEFaD2b50F3bFc0fbe899f3B6bA489EAF9E7B650"),
    );
    m.insert(
        "Grayscale ETHE_941",
        address!("fF42E2A81A5aB429eEad7654344a9DEec215Dabf"),
    );
    m.insert(
        "Grayscale ETHE_942",
        address!("Ff8A34f749B97e19f9821615731Be346E65aeDD3"),
    );
    m.insert(
        "Grayscale ETHE_943",
        address!("fF9eE960D4a89d19762E6bCdaE96491cEAAE2b80"),
    );
    m.insert(
        "Grayscale ETHE_944",
        address!("FFd39d27E7cdc53c1a9c74013E6E1C2dF1F27bF1"),
    );
    m.insert(
        "Grayscale ETHE_945",
        address!("FfF4Fb977fC2F15e7C29527a9f398d6e287D3e03"),
    );
    m.insert(
        "Grayscale ETHE_946",
        address!("FffAa9F1d590B2A7879279c312d3B4A5f0BF679A"),
    );
    m.insert(
        "Grayscale ETHE_95",
        address!("18fC5f64d3a758c62Ce03c036705eA887154E550"),
    );
    m.insert(
        "Grayscale ETHE_96",
        address!("19a1cC5301589Eb80a8D07c0f5475997BbEf1F80"),
    );
    m.insert(
        "Grayscale ETHE_97",
        address!("1A46Ca4Be161E5D4CAD714602a22C9b7FBbA7FB2"),
    );
    m.insert(
        "Grayscale ETHE_98",
        address!("1a8F8a8714f11c3A35B13A34A3eaE15A5f850f47"),
    );
    m.insert(
        "Grayscale ETHE_99",
        address!("1A9635A16A9b2435985399eB64b025Dd8052EC75"),
    );
    m.insert(
        "Grayscale Mini ETH",
        address!("03058Fa830E90dE326009EB1b6B793B60076d7Dd"),
    );
    m.insert(
        "Grayscale Mini ETH_1",
        address!("03211dAae0b65cf396945868D022bB77dE22Efa2"),
    );
    m.insert(
        "Grayscale Mini ETH_10",
        address!("3a2410E77A3Ea976C7DCC9880527762fCEEE6FF3"),
    );
    m.insert(
        "Grayscale Mini ETH_11",
        address!("452962d1d9F7f0DEd2e73E79c859D0140181A9F7"),
    );
    m.insert(
        "Grayscale Mini ETH_12",
        address!("5B4ccB047b982Dc0Eba47c5cF35f80A1AAa25544"),
    );
    m.insert(
        "Grayscale Mini ETH_13",
        address!("5f07D515B5897af3BEDbE42d74d350A41508973e"),
    );
    m.insert(
        "Grayscale Mini ETH_14",
        address!("5F2c071093F4F03C718240F2Ac5DF3222909aCb8"),
    );
    m.insert(
        "Grayscale Mini ETH_15",
        address!("660B4f8b8Ac68fD6F6E2caeb19Bc5529d4c05Bd4"),
    );
    m.insert(
        "Grayscale Mini ETH_16",
        address!("78c42af600F67483474B1FEc5681e9B6938B9b4B"),
    );
    m.insert(
        "Grayscale Mini ETH_17",
        address!("88a709A920AeBC96706735C0b27B5460e2D61a1c"),
    );
    m.insert(
        "Grayscale Mini ETH_18",
        address!("9919098d320FA16EBe00E74aFEc41C054b3995e7"),
    );
    m.insert(
        "Grayscale Mini ETH_19",
        address!("9d8B6e761FDead614Bd1FBBB9F03D558a526C676"),
    );
    m.insert(
        "Grayscale Mini ETH_2",
        address!("037CA5ca8b5acFEb335B4aA389F08C325a62CD2d"),
    );
    m.insert(
        "Grayscale Mini ETH_20",
        address!("ab3E9f1133c597F03768510E7e65004A04c9d427"),
    );
    m.insert(
        "Grayscale Mini ETH_21",
        address!("BD46378faB9C17f50fD45C14980155dba9c814c5"),
    );
    m.insert(
        "Grayscale Mini ETH_22",
        address!("BE8628F850838E8683E53734aED211C8Ad7be95b"),
    );
    m.insert(
        "Grayscale Mini ETH_23",
        address!("BFD45E3030A9Cb9801954a7adFF074164A71605a"),
    );
    m.insert(
        "Grayscale Mini ETH_24",
        address!("C073f502e033185D211B2FD339706CE44E6F1054"),
    );
    m.insert(
        "Grayscale Mini ETH_25",
        address!("C7E9079f03a07D93E101Cd4079B69283D9f47d29"),
    );
    m.insert(
        "Grayscale Mini ETH_26",
        address!("CA8a031cDb5bbd4559d526a723093582F80aA3C2"),
    );
    m.insert(
        "Grayscale Mini ETH_27",
        address!("cACe62287B7794a8fa7B4dAF45f0D037434c54db"),
    );
    m.insert(
        "Grayscale Mini ETH_28",
        address!("cf3dC64a2F99Cd77148F0485A91933dadc4FaB6c"),
    );
    m.insert(
        "Grayscale Mini ETH_29",
        address!("D616e186C4DB1b46a47B5CBF368C331dD2fB709e"),
    );
    m.insert(
        "Grayscale Mini ETH_3",
        address!("09F928cB05359507866b97451E71d55Fbdb43A3C"),
    );
    m.insert(
        "Grayscale Mini ETH_30",
        address!("eaA76C2c18161a31487C6205Eb85671D87d7a0cC"),
    );
    m.insert(
        "Grayscale Mini ETH_4",
        address!("1Ad1eb01Ac0409dDd3B3c17ce1D4C83F3236A3CB"),
    );
    m.insert(
        "Grayscale Mini ETH_5",
        address!("21316bBeCD9Ac31dabBFF2A6a885Dd232807dfe8"),
    );
    m.insert(
        "Grayscale Mini ETH_6",
        address!("29AA2a311c184DF32d1D73d30EB3b54FF31583C8"),
    );
    m.insert(
        "Grayscale Mini ETH_7",
        address!("327eb8aa24e09BE3fB7aF5844657870D513687C5"),
    );
    m.insert(
        "Grayscale Mini ETH_8",
        address!("3707487518d9485A98f44D7B4b678DaAdB8360Da"),
    );
    m.insert(
        "Grayscale Mini ETH_9",
        address!("38eC364A13b6AF2bC3faA002D2A7Ad005743243d"),
    );
    m.insert(
        "Fidelity FETH",
        address!("0e21a608c7939D0F8005a8DCEA034FBcF5f6f194"),
    );
    m.insert(
        "Fidelity FETH_1",
        address!("6279fb0FE59125c290E0a9BA7A5c98e8f0c5Ab23"),
    );
    m.insert(
        "Fidelity FETH_2",
        address!("8aFA3d0A70b2f3A92F5b8D63afFafD31d3d46eB7"),
    );
    m.insert(
        "Fidelity FETH_3",
        address!("8b4b2B268766E28224CC03384d80518A91a6fD37"),
    );
    m.insert(
        "Fidelity FETH_4",
        address!("9371be03662333331D1C5D7d9Ee2e1c25A574743"),
    );
    m.insert(
        "Fidelity FETH_5",
        address!("9D18F81dE45a2ed1EF4B269A8d4C6E390A8C1C68"),
    );
    m.insert(
        "Fidelity FETH_6",
        address!("b3E392BEDD6956187F590D50C5Aa071C08DCfd6E"),
    );
    m.insert(
        "Fidelity FETH_7",
        address!("bE775c8a98e33DD6Fd5a139827F2c9339D1F9fdd"),
    );
    m.insert(
        "Fidelity FETH_8",
        address!("CB4460C60F9Ea6fB64F758f7E8dECFA847f7F71A"),
    );
    m.insert(
        "Fidelity FETH_9",
        address!("eF54c7bf79f089f868150D3c0684213237c184D7"),
    );
    m.insert(
        "Franklin Templeton EZET",
        address!("21818666eA7218Fa2579146Ccffb05C113fcD132"),
    );
    m.insert(
        "Franklin Templeton EZET_1",
        address!("f892c7b77d50B8ed19A33fD28e24151600478731"),
    );
    m.insert(
        "VanEck ETHV",
        address!("57F0566BDca5e8094285DEfB817C4E598F6d51f2"),
    );
    m.insert(
        "VanEck ETHV_1",
        address!("AD10A0Ec7A7FdD54B9d13fa8e2Ee1d5f4E94627A"),
    );
    m.insert(
        "Invesco QETH",
        address!("00c2c0fE37cA5B3b7A0BF0179AC5505AeDb6cfd2"),
    );
    m.insert(
        "Invesco QETH_1",
        address!("400D68de9f7769C106d1108471d1d6C0CfF78548"),
    );
    m.insert(
        "Invesco QETH_2",
        address!("aa16Ff2000885E9A2E725E84179BBB62427C6067"),
    );
    m.insert(
        "Invesco QETH_3",
        address!("c47b4a69aA1B7689983420443011f112B064A727"),
    );
    m.insert(
        "Bitwise ETHW",
        address!("3339AAD5f1a0CC95d6Dee6DDF10b444F080c7cB5"),
    );
    m.insert(
        "Bitwise ETHW_1",
        address!("6F28ebf3170AA00Bfdf7131A4635a04976c657Ed"),
    );
    m.insert(
        "Bitwise ETHW_2",
        address!("7716d9e70779ee3d2580DccA818521A85E59eec8"),
    );
    m.insert(
        "Bitwise ETHW_3",
        address!("a15c9d4aF12d42c612D5a7445d76f5cC3aC92A69"),
    );
    m.insert(
        "Bitwise ETHW_4",
        address!("Ed9258097cC80e1E6eBF2c9B132Eb135C81bb4eF"),
    );
    m.insert(
        "Bitwise ETHW_5",
        address!("FBaC831C5A71BF8f517B3a33cBFfFdD448eBB5C6"),
    );
    m.insert(
        "21Shares CETH",
        address!("1ae3adaB1c43f97D53Ee3619CD8220C294059Dec"),
    );
    m.insert(
        "21Shares CETH_1",
        address!("25e1a34e480443433cC7D16664c62c5A4d9dd43E"),
    );
    m.insert(
        "21Shares CETH_2",
        address!("405cBa46e0cBa39961fe2a813B4E403b841A0EE5"),
    );
    m.insert(
        "21Shares CETH_3",
        address!("7846e4F966C708799A81927cF34Bcf2544142428"),
    );
    m.insert(
        "21Shares CETH_4",
        address!("7b9B5cd00ebb0d4854a73dAfa0609003cF97ea31"),
    );
    m.insert(
        "21Shares CETH_5",
        address!("Cdc74Cb68695fD2da6dA8c717e22600cC624744d"),
    );
    m.insert(
        "21Shares CETH_6",
        address!("d28F68Feed9a47cb3CF63B058581761abff45FB3"),
    );
    m.insert(
        "21Shares CETH_7",
        address!("ff1dBB9e1D2e15B70869ab3BcBe7c1ac09048882"),
    );
    for (label, address) in WALLET_ADDRESSES.iter() {
        m.insert(*label, *address);
    }
    for (symbol, address) in SYMBOL_TO_ADDRESS.iter() {
        m.entry(*symbol).or_insert(*address);
    }
    m
});

/// Get token symbol by address
pub fn get_token_symbol(address: Address) -> Option<&'static str> {
    DENOM_ADDRESSES.get(&address).copied()
}

/// Get decimals for a token symbol
pub fn get_token_decimals(symbol: &str) -> Option<u8> {
    ERC20_TOKEN_DECIMALS.get(symbol).copied()
}

/// Check if an address is a known DENOM token
pub fn is_denom_token(address: Address) -> bool {
    DENOM_ADDRESSES.contains_key(&address)
}

/// Get address by name (for routers, factories, etc.)
pub fn get_address_by_name(name: &str) -> Option<Address> {
    ADDRESSES_BY_NAME.get(name).copied()
}
