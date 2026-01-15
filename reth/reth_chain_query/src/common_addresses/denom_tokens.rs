//! DENOM token addresses (major tokens and currencies)
//!
//! This file is auto-generated from Python address files.
//! Do not edit manually - regenerate using scripts/convert_addresses_to_rust.py
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
        address!("431d5dff03120afa4bdf332c61a6e1766ef37bdb"),
        "JPYCv2",
    );
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
    m.insert(address!("59D9356E565AB3A36DD77763FC0D87FEAF85508C"), "USDM");
    m.insert(address!("8E870D67F660D95D5be530380D0eC0bd388289E1"), "USDP");
    m.insert(address!("A4BDB11dC0a2beC88d24A3AA1e6bb17201112EBE"), "USDS");
    m.insert(address!("0C10BF8FCB7BF5412187A595aB97A3609160B5C6"), "USDD");
    m.insert(address!("4C9EDD5852cD905F086c759e8383E09BFF1E68B3"), "USDE");
    m.insert(address!("20B3B07E9C0E37815E2892AB09496559F57C3603"), "USDV");
    m.insert(address!("96F6EF951840721ADBF46AC996B59E0235CB985C"), "USDY");
    m.insert(address!("68749665FF8D2d112Fa859AA293F07A622782F38"), "XAUt");
    m.insert(address!("ebF2096E01455108bAdCbAF86cE30b6e5A72aa52"), "XIDR");
    m.insert(address!("70e8dE73cE538DA2bEEd35d14187F6959a8ecA96"), "XSGD");
    m.insert(address!("C08e7E23C235073C6807C2eFe7021304cB7C2815"), "XUSD");
    m.insert(address!("c56c2b7e71B54d38Aab6d52E94a04Cbfa8F604fA"), "ZUSD");
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
