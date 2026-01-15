//! Stablecoin addresses and metadata
//!
//! This file is auto-generated from Python address files.
//! Do not edit manually - regenerate using scripts/convert_addresses_to_rust.py

use alloy_primitives::{address, Address};
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Information about a stablecoin
#[derive(Debug, Clone)]
pub struct StablecoinInfo {
    pub address: Address,
    pub symbol: &'static str,
    pub decimals: u8,
    pub unit: &'static str,
}

/// All stablecoin information
pub const STABLECOINS: &[StablecoinInfo] = &[
    StablecoinInfo {
        address: address!("01D33Fd36ec67C6adA32Cf36B31E88Ee190b1839"),
        symbol: "BRZ",
        decimals: 18,
        unit: "Brazilian Real",
    },
    StablecoinInfo {
        address: address!("4fabb145d64652a948d72533023f6e7a623c7c53"),
        symbol: "BUSD",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("caDC0aCD4B445166f12D2C07EaC6E2544FbE2Eef"),
        symbol: "CADC",
        decimals: 18,
        unit: "Canadian Dollar",
    },
    StablecoinInfo {
        address: address!("6B175474E89094C44Da98b954EedeAC495271d0F"),
        symbol: "DAI",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("865377367054516e17014ccded1e7d814edc9ce4"),
        symbol: "DOLA",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("1aBaEA1f7C830bd89Acc67EC4af516284b1bC33c"),
        symbol: "EUROC",
        decimals: 6,
        unit: "Euro",
    },
    StablecoinInfo {
        address: address!("5F7827FDeb7c20b443265Fc2F40845B715385Ff2"),
        symbol: "EURCV",
        decimals: 18,
        unit: "Euro",
    },
    StablecoinInfo {
        address: address!("db25f211ab05b1c97d595516f45794528a807ad8"),
        symbol: "EURS",
        decimals: 2,
        unit: "Euro",
    },
    StablecoinInfo {
        address: address!("C581b735A1688071A1746c968e0798D642EDE491"),
        symbol: "EURT",
        decimals: 6,
        unit: "Euro",
    },
    StablecoinInfo {
        address: address!("c5f0F7B66764F6EC8c8dFF7Ba683102295E16409"),
        symbol: "FDUSD",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("956F47F50A910163D8BF957Cf5846D573E7f87CA"),
        symbol: "FEI",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("853d955aCEf822Db058eb8505911ED77F175b99e"),
        symbol: "FRAX",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("40D16FC0246aD3160Ccc09B8D0D3A2cD28aE6C2f"),
        symbol: "GHO",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("056FD409E1d7A124BD7017459dFEa2F387B6d5Cd"),
        symbol: "GUSD",
        decimals: 2,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("C08512927D12348F6620a698105e1BAac6EcD911"),
        symbol: "GYEN",
        decimals: 6,
        unit: "Japanese Yen",
    },
    StablecoinInfo {
        address: address!("998FFE1E43fAcffb941dc337dD0468d52BA5B48A"),
        symbol: "IDRT",
        decimals: 2,
        unit: "Indonesian Rupiah",
    },
    StablecoinInfo {
        address: address!("2370f9d504C7A6E775bf6E14B3F12846b594cD53"),
        symbol: "JPYC",
        decimals: 18,
        unit: "Japanese Yen",
    },
    StablecoinInfo {
        address: address!("431d5dff03120afa4bdf332c61a6e1766ef37bdb"),
        symbol: "JPYCv2",
        decimals: 18,
        unit: "Japanese Yen",
    },
    StablecoinInfo {
        address: address!("5f98805A4E8be255a32880FDeC7F6728C6568bA0"),
        symbol: "LUSD",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("4591DBfF62656E7859Afe5e45f6f47D3669fBB28"),
        symbol: "MKUSD",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("2A8e1E676Ec238d8A992307B495b45B3fEAa5e86"),
        symbol: "OUSD",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("45804880De22913dAFE09f4980848ECE6EcbAf78"),
        symbol: "PAXG",
        decimals: 18,
        unit: "Gold (troy ounce)",
    },
    StablecoinInfo {
        address: address!("6c3EA9036406852006290770BEdFcAbA0e23A0E8"),
        symbol: "PYUSD",
        decimals: 6,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("03ab458634910Aad20Ef5f1C8eE96f1d6Ac54919"),
        symbol: "RAI",
        decimals: 18,
        unit: "RAI Protocol",
    },
    StablecoinInfo {
        address: address!("57ab1eC28D129707052DF4DF418D58A2D46d5f51"),
        symbol: "sUSD",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("0000000000085d4780B73119b644AE5ecd22b376"),
        symbol: "TUSD",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
        symbol: "USDC",
        decimals: 6,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("73a15fed60bf67631dc6cd7bc5b6e8da8190acf5"),
        symbol: "USD0",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("C824Bf014539F6bdE6b81ABAaca0D626C2AC5985"),
        symbol: "USD1",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("8E870D67F660D95D5be530380D0eC0bd388289E1"),
        symbol: "USDP",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("A4BDB11dC0a2beC88d24A3AA1e6bb17201112EBE"),
        symbol: "USDS",
        decimals: 6,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("dAC17F958D2ee523a2206206994597C13D831ec7"),
        symbol: "USDT",
        decimals: 6,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("0C10BF8FCB7BF5412187A595aB97A3609160B5C6"),
        symbol: "USDD",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("4C9EDD5852cD905F086c759e8383E09BFF1E68B3"),
        symbol: "USDE",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("59D9356E565AB3A36DD77763FC0D87FEAF85508C"),
        symbol: "USDM",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("20B3B07E9C0E37815E2892AB09496559F57C3603"),
        symbol: "USDV",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("96F6EF951840721ADBF46AC996B59E0235CB985C"),
        symbol: "USDY",
        decimals: 18,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("68749665FF8D2d112Fa859AA293F07A622782F38"),
        symbol: "XAUt",
        decimals: 6,
        unit: "Gold (troy ounce)",
    },
    StablecoinInfo {
        address: address!("ebF2096E01455108bAdCbAF86cE30b6e5A72aa52"),
        symbol: "XIDR",
        decimals: 6,
        unit: "Indonesian Rupiah",
    },
    StablecoinInfo {
        address: address!("70e8dE73cE538DA2bEEd35d14187F6959a8ecA96"),
        symbol: "XSGD",
        decimals: 6,
        unit: "Singapore Dollar",
    },
    StablecoinInfo {
        address: address!("C08e7E23C235073C6807C2eFe7021304cB7C2815"),
        symbol: "XUSD",
        decimals: 6,
        unit: "US Dollar",
    },
    StablecoinInfo {
        address: address!("c56c2b7e71B54d38Aab6d52E94a04Cbfa8F604fA"),
        symbol: "ZUSD",
        decimals: 6,
        unit: "US Dollar",
    },
];

/// Lazy static HashMap for quick lookups by address
pub static STABLECOIN_BY_ADDRESS: Lazy<HashMap<Address, &'static StablecoinInfo>> =
    Lazy::new(|| {
        STABLECOINS
            .iter()
            .map(|info| (info.address, info))
            .collect()
    });

/// Lazy static HashMap for quick lookups by symbol
pub static STABLECOIN_BY_SYMBOL: Lazy<HashMap<&'static str, &'static StablecoinInfo>> =
    Lazy::new(|| STABLECOINS.iter().map(|info| (info.symbol, info)).collect());

/// Check if an address is a stablecoin
pub fn is_stablecoin(address: Address) -> bool {
    STABLECOIN_BY_ADDRESS.contains_key(&address)
}

/// Get stablecoin info by address
pub fn get_stablecoin_by_address(address: Address) -> Option<&'static StablecoinInfo> {
    STABLECOIN_BY_ADDRESS.get(&address).copied()
}

/// Get stablecoin info by symbol
pub fn get_stablecoin_by_symbol(symbol: &str) -> Option<&'static StablecoinInfo> {
    STABLECOIN_BY_SYMBOL.get(symbol).copied()
}
