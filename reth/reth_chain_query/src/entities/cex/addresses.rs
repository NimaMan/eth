//! CEX (Centralized Exchange) addresses
//! 
//! This file is auto-generated from Python address files.
//! Do not edit manually - regenerate using scripts/convert_addresses_to_rust.py

use alloy_primitives::{address, Address};
use std::collections::{HashMap, HashSet};
use once_cell::sync::Lazy;

/// CEX address entry
#[derive(Debug, Clone)]
pub struct CexAddress {
    pub address: Address,
    pub name: &'static str,
    pub exchange: &'static str,
}

/// All CEX addresses
pub const CEX_ADDRESSES: &[CexAddress] = &[
    CexAddress {
        address: address!("5c89724967D76a4f966b013140355789093F6c7D"),
        name: "1xBet",
        exchange: "1xBet",
    },
    CexAddress {
        address: address!("777f415324d56e1d54fa832902d8797dB7A4c57C"),
        name: "1xBet_1",
        exchange: "1xBet",
    },
    CexAddress {
        address: address!("BA3801847037ffe8dE609CCfDd8E02C2F60AdC43"),
        name: "1xBet_2",
        exchange: "1xBet",
    },
    CexAddress {
        address: address!("4dCa3852323Ae417D4C7d735100A629130d50e90"),
        name: "AAX",
        exchange: "AAX",
    },
    CexAddress {
        address: address!("80edADF751946A71a0131a495BB7abBC75F46f6C"),
        name: "AAX_1",
        exchange: "AAX",
    },
    CexAddress {
        address: address!("8Dc11398263ffF36eB91602A18877112eaD2EDE4"),
        name: "AAX_2",
        exchange: "AAX",
    },
    CexAddress {
        address: address!("A4EE5581fDcBAAf52c5EdB65c8fBb7D3DE78b838"),
        name: "AAX_3",
        exchange: "AAX",
    },
    CexAddress {
        address: address!("c25DC289Edce5227cf15d42539824509e826b54D"),
        name: "AAX_4",
        exchange: "AAX",
    },
    CexAddress {
        address: address!("d25f30CA035250cff6810187e2Da6F924Bd9616e"),
        name: "AAX_5",
        exchange: "AAX",
    },
    CexAddress {
        address: address!("05f51AAb068CAa6Ab7eeb672f88c180f67F17eC7"),
        name: "ABCC Exchange",
        exchange: "ABCC",
    },
    CexAddress {
        address: address!("0A26d96143f8C5754A588dE5Bef1738cF0B6A28F"),
        name: "ABCC Exchange_1",
        exchange: "ABCC Exchange",
    },
    CexAddress {
        address: address!("AA9133EeC3ae5f9440C1a1E61E2D2Cc571675527"),
        name: "ABCC Exchange_2",
        exchange: "ABCC Exchange",
    },
    CexAddress {
        address: address!("aecBE94703Df39B49Ac440fEB177c7f1f782c064"),
        name: "APROBIT",
        exchange: "APROBIT",
    },
    CexAddress {
        address: address!("4dF5f3610e2471095a130D7d934D551f3ddE01ED"),
        name: "ATAIX",
        exchange: "ATAIX",
    },
    CexAddress {
        address: address!("0e0066aca9ef6B8102D8Dbc66AB0091f9370a7cb"),
        name: "Abra",
        exchange: "Abra",
    },
    CexAddress {
        address: address!("0e8D02aE96b229f112f37502C2A26D66BDBcff1F"),
        name: "Aeroswap",
        exchange: "Aeroswap",
    },
    CexAddress {
        address: address!("397Be73c6160E5E66408924dA4aD32eA5e9ea9db"),
        name: "Aeroswap_1",
        exchange: "Aeroswap",
    },
    CexAddress {
        address: address!("7D411c4279A96069Fe32De0c0EAB4a964E8ED9C4"),
        name: "Aeroswap_2",
        exchange: "Aeroswap",
    },
    CexAddress {
        address: address!("8EB871bbB6F754a04bCa23881A7D25A30aAD3f23"),
        name: "Aeroswap_3",
        exchange: "Aeroswap",
    },
    CexAddress {
        address: address!("2DDD202174A72514ed522E77972b461b03155525"),
        name: "Alcumex Exchange",
        exchange: "Alcumex",
    },
    CexAddress {
        address: address!("dc1882F350b42ac9a23508996254b1915c78b204"),
        name: "Allbit",
        exchange: "Allbit",
    },
    CexAddress {
        address: address!("Ff6b1cdfD2d3e37977d7938AA06b6d89D6675e27"),
        name: "Allbit_1",
        exchange: "Allbit",
    },
    CexAddress {
        address: address!("183A6cF1Fc6504138d92C9d663094EE774f80038"),
        name: "AlphaPo",
        exchange: "AlphaPo",
    },
    CexAddress {
        address: address!("6dfc34609a05bC22319fA4Cce1d1E2929548c0D7"),
        name: "AlphaPo_1",
        exchange: "AlphaPo",
    },
    CexAddress {
        address: address!("808d0aeE8db7E7c74FaF4b264333aFE8c9cCDBA4"),
        name: "AlphaPo_2",
        exchange: "AlphaPo",
    },
    CexAddress {
        address: address!("0Da044B16bB53AB6d6691e25b1ec7aD380Fa5Fa7"),
        name: "AltCoinTrader",
        exchange: "AltCoinTrader",
    },
    CexAddress {
        address: address!("1BC972Db7B169C2D79719485803aF1A23645108d"),
        name: "AltCoinTrader_1",
        exchange: "AltCoinTrader",
    },
    CexAddress {
        address: address!("2Cf4E5B4Fa8Afa15F8dc7B5adA853e42776f3455"),
        name: "AltCoinTrader_2",
        exchange: "AltCoinTrader",
    },
    CexAddress {
        address: address!("87b7bA194eD714a871C91F6B9Ba8Dc8182eD3A5d"),
        name: "AltCoinTrader_3",
        exchange: "AltCoinTrader",
    },
    CexAddress {
        address: address!("a1495F85d30aCAB5126f5C1920cD9cb1CE265263"),
        name: "AltCoinTrader_4",
        exchange: "AltCoinTrader",
    },
    CexAddress {
        address: address!("a2Fe6EC4244ee94853326F70893ce5eE20AA4fFb"),
        name: "AltCoinTrader_5",
        exchange: "AltCoinTrader",
    },
    CexAddress {
        address: address!("b56F9e1aecb821413C9F14822D3918A363D83226"),
        name: "AltCoinTrader_6",
        exchange: "AltCoinTrader",
    },
    CexAddress {
        address: address!("c58Bb74606b73c5043B75d7Aa25ebe1D5D4E7c72"),
        name: "AltCoinTrader_7",
        exchange: "AltCoinTrader",
    },
    CexAddress {
        address: address!("E1336fA8165Bc4DbaDC7FB95718A3C9DcAe23fdc"),
        name: "AltCoinTrader_8",
        exchange: "AltCoinTrader",
    },
    CexAddress {
        address: address!("Ff2E0C46C7673ccD00cB5B59Dc1686229A483Fc8"),
        name: "AltCoinTrader_9",
        exchange: "AltCoinTrader",
    },
    CexAddress {
        address: address!("2425B5c48327DA2a8bE22E57207ae8056c3f42ee"),
        name: "AlterDice",
        exchange: "AlterDice",
    },
    CexAddress {
        address: address!("690e96f32A225F661A1881a484F858276CB82984"),
        name: "AlterDice_1",
        exchange: "AlterDice",
    },
    CexAddress {
        address: address!("3161b9660cc36C00dfC36307De2B8C53960164dC"),
        name: "Anchorage Digital",
        exchange: "Anchorage",
    },
    CexAddress {
        address: address!("A44D54Ae6A00e095dAA000365C99c4A27303b6f3"),
        name: "Anchorage Digital_1",
        exchange: "Anchorage Digital",
    },
    CexAddress {
        address: address!("d52055A39a3d2f7505C739f981f296Ea31B50191"),
        name: "Anchorage Digital_2",
        exchange: "Anchorage Digital",
    },
    CexAddress {
        address: address!("3EEBBEBeCde31d36D6AA7AA4FE2A06159119b659"),
        name: "Anycoin Direct",
        exchange: "Anycoin",
    },
    CexAddress {
        address: address!("5B31Bb52bdd7006ae57F9d9506c0FF995229b63c"),
        name: "Anycoin Direct_1",
        exchange: "Anycoin Direct",
    },
    CexAddress {
        address: address!("0323718324218dcBfF7c9f89bA5a5954F61A6c74"),
        name: "Arkham",
        exchange: "Arkham",
    },
    CexAddress {
        address: address!("34407900475cEF87acE1597670A9A42F31961d02"),
        name: "Arkham_1",
        exchange: "Arkham",
    },
    CexAddress {
        address: address!("679Fb19dEc9d66C34450a8563FfDFD29C04e615A"),
        name: "Arkham_2",
        exchange: "Arkham",
    },
    CexAddress {
        address: address!("Dc2822D0685c0CcEAb07b35d6de4aC9280FB9cFF"),
        name: "Arkham_3",
        exchange: "Arkham",
    },
    CexAddress {
        address: address!("94597850916a49b3B152EE374E97260B99249f5B"),
        name: "Artis Turba Exchange",
        exchange: "Artis",
    },
    CexAddress {
        address: address!("f0c80FB9FB22BEF8269CB6fEB9a51130288a671f"),
        name: "Artis Turba Exchange_1",
        exchange: "Artis Turba Exchange",
    },
    CexAddress {
        address: address!("82a403c14483931B2fF6e4440c8373ccFEe698B8"),
        name: "ArzPaya.com",
        exchange: "ArzPaya.com",
    },
    CexAddress {
        address: address!("03BDf69B1322D623836aFBD27679A1C0AfA067E9"),
        name: "AscendEX",
        exchange: "AscendEX",
    },
    CexAddress {
        address: address!("09344477fDc71748216a7b8BbE7F2013B893DeF8"),
        name: "AscendEX_1",
        exchange: "AscendEX",
    },
    CexAddress {
        address: address!("4240781A9ebDB2EB14a183466E8820978b7DA4e2"),
        name: "AscendEX_2",
        exchange: "AscendEX",
    },
    CexAddress {
        address: address!("4B1a99467a284Cc690e3237bC69105956816f762"),
        name: "AscendEX_3",
        exchange: "AscendEX",
    },
    CexAddress {
        address: address!("80Ca27268d4603E00B8d4D98Aa309dB438127d19"),
        name: "AscendEX_4",
        exchange: "AscendEX",
    },
    CexAddress {
        address: address!("8FaB0D3E5eE6FFdb73589c2C47dcD3802360694f"),
        name: "AscendEX_5",
        exchange: "AscendEX",
    },
    CexAddress {
        address: address!("9715254754284a0b3e4C7BF8f57E790415041c1C"),
        name: "AscendEX_6",
        exchange: "AscendEX",
    },
    CexAddress {
        address: address!("983873529f95132BD1812A3B52c98Fb271d2f679"),
        name: "AscendEX_7",
        exchange: "AscendEX",
    },
    CexAddress {
        address: address!("986a2fCa9eDa0e06fBf7839B89BfC006eE2a23Dd"),
        name: "AscendEX_8",
        exchange: "AscendEX",
    },
    CexAddress {
        address: address!("9BD376BFce4B97c6fAe3F438d516Ae1582168596"),
        name: "AscendEX_9",
        exchange: "AscendEX",
    },
    CexAddress {
        address: address!("054C64741dBafDC19784505494029823D89c3b13"),
        name: "AtomSolutions",
        exchange: "AtomSolutions",
    },
    CexAddress {
        address: address!("112b12089611749406fde450FDa9917F7F4Ac3CB"),
        name: "AtomSolutions_1",
        exchange: "AtomSolutions",
    },
    CexAddress {
        address: address!("2F5A7b563E6C4761d478273F5f9B4A444BDd2E3C"),
        name: "AtomSolutions_2",
        exchange: "AtomSolutions",
    },
    CexAddress {
        address: address!("5EcA044b580a86e7ab4b2076330978d6D125a270"),
        name: "AtomSolutions_3",
        exchange: "AtomSolutions",
    },
    CexAddress {
        address: address!("A28d81bF8e4823cf4d6Cc2767507dEe271994e1A"),
        name: "AtomSolutions_4",
        exchange: "AtomSolutions",
    },
    CexAddress {
        address: address!("22682575E073736Ed25258409B09e0e7AF6D9C61"),
        name: "Azbit",
        exchange: "Azbit",
    },
    CexAddress {
        address: address!("36e75f48c5D67e0c619d6F56a3481A21bE57e322"),
        name: "Azbit_1",
        exchange: "Azbit",
    },
    CexAddress {
        address: address!("92dBD8e0A46EdD62AA42d1f7902D0e496Bddc15A"),
        name: "Azbit_2",
        exchange: "Azbit",
    },
    CexAddress {
        address: address!("20Dc0b9520CC2C2BE89F247061A2c8e310045949"),
        name: "B2BinPay",
        exchange: "B2BinPay",
    },
    CexAddress {
        address: address!("849A02be4c2ec8BbD06052C5A0Cd51147994Ad96"),
        name: "B2BinPay_1",
        exchange: "B2BinPay",
    },
    CexAddress {
        address: address!("a7fB5cA286Fc3FD67525629048a4de3bA24Cba2E"),
        name: "B2BinPay_2",
        exchange: "B2BinPay",
    },
    CexAddress {
        address: address!("3BC643A841915A267eE067b580BD802a66001C1d"),
        name: "BTC Markets",
        exchange: "BTC",
    },
    CexAddress {
        address: address!("8a44DC02E250F0f0f388B73a257C53E3BB50321d"),
        name: "BTC Markets_1",
        exchange: "BTC Markets",
    },
    CexAddress {
        address: address!("aecfb1af29B96011EC9AA1Ff98D8C49b49bB3dDc"),
        name: "BTC Markets_2",
        exchange: "BTC Markets",
    },
    CexAddress {
        address: address!("C55EdDadEeB47fcDE0B3B6f25BD47D745BA7E7fa"),
        name: "BTC Markets_3",
        exchange: "BTC Markets",
    },
    CexAddress {
        address: address!("cA46fBDC3Dfe107d033682Acb1EF212fe555E731"),
        name: "BTC Markets_4",
        exchange: "BTC Markets",
    },
    CexAddress {
        address: address!("1c00d840ccAa67c494109F46E55cFEB2D8562F5c"),
        name: "BTC-Alpha Exchange",
        exchange: "BTC-Alpha",
    },
    CexAddress {
        address: address!("91337A300e0361BDDb2e377DD4e88CCB7796663D"),
        name: "BTC-e",
        exchange: "BTC-e",
    },
    CexAddress {
        address: address!("c73f25a029352931a64b328C82511e88188A8c96"),
        name: "BTC-e_1",
        exchange: "BTC-e",
    },
    CexAddress {
        address: address!("EEa5B82B61424dF8020f5feDD81767f2d0D25Bfb"),
        name: "BTC.com",
        exchange: "BTC.com",
    },
    CexAddress {
        address: address!("4f26B5961210F295542B0c5C13c4887E24F0910E"),
        name: "BTCEX",
        exchange: "BTCEX",
    },
    CexAddress {
        address: address!("59EdC943735Aa4f2b1a1e4D7bCb46ebE09F42C8C"),
        name: "BTCEX_1",
        exchange: "BTCEX",
    },
    CexAddress {
        address: address!("BbdaEA89Ced53Bf9E31A4cEb926832fBB1bC0bB4"),
        name: "BTCEX_2",
        exchange: "BTCEX",
    },
    CexAddress {
        address: address!("d020220CA4841229514d1bc02a8aBCE4C162C015"),
        name: "BTCEX_3",
        exchange: "BTCEX",
    },
    CexAddress {
        address: address!("fa66605B88e16c2fD011622dEe6F35A976098eDb"),
        name: "BTCEX_4",
        exchange: "BTCEX",
    },
    CexAddress {
        address: address!("1619d743d7DC612E99d5D94Ebd6b9695D46f0BF3"),
        name: "BTSE",
        exchange: "BTSE",
    },
    CexAddress {
        address: address!("661BC014Ba045A1918215cbe2aF8121ADA09638D"),
        name: "BTSE_1",
        exchange: "BTSE",
    },
    CexAddress {
        address: address!("9036B1eB7630d9A45720FD80D05D46262f460529"),
        name: "BTSE_2",
        exchange: "BTSE",
    },
    CexAddress {
        address: address!("b0afFFd6f6Ad77f61927803ADE6dbD47f1a1C356"),
        name: "BTSE_3",
        exchange: "BTSE",
    },
    CexAddress {
        address: address!("bB4D1DC5c1ABec4Ea11166ec97E714862863aD1D"),
        name: "BTSE_4",
        exchange: "BTSE",
    },
    CexAddress {
        address: address!("DDAad971BE05321FD541372CD710a7f0555972eD"),
        name: "BTSE_5",
        exchange: "BTSE",
    },
    CexAddress {
        address: address!("de279a5cD86860Cd3D039AA1B74bc29E74cABB12"),
        name: "BTSE_6",
        exchange: "BTSE",
    },
    CexAddress {
        address: address!("73957709695E73Fd175582105c44743CF0fB6f2f"),
        name: "BW.com",
        exchange: "BW.com",
    },
    CexAddress {
        address: address!("bCDFC35b86BedF72F0Cda046A3c16829A2Ef41d1"),
        name: "BW.com_1",
        exchange: "BW.com",
    },
    CexAddress {
        address: address!("94fa70d079D76279e1815ce403e9B985bcCC82AC"),
        name: "Bake",
        exchange: "Bake",
    },
    CexAddress {
        address: address!("6D932cB67760F6a5343998bebAc85c0DE7C9aA10"),
        name: "Beaxy",
        exchange: "Beaxy",
    },
    CexAddress {
        address: address!("Adb72986EAd16bDbc99208086BD431C1Aa38938e"),
        name: "Beaxy_1",
        exchange: "Beaxy",
    },
    CexAddress {
        address: address!("258B7B9A1BA92f47f5F4f5e733293477620a82Cb"),
        name: "Beldex",
        exchange: "Beldex",
    },
    CexAddress {
        address: address!("52A258ED593C793251a89bfd36caE158EE9fC4F8"),
        name: "BetFury",
        exchange: "BetFury",
    },
    CexAddress {
        address: address!("7A10Ec7d68a048BdaE36A70E93532D31423170fA"),
        name: "Bgogo",
        exchange: "Bgogo",
    },
    CexAddress {
        address: address!("Ce1bF8E51F8b39e51c6184e059786D1c0eAF360F"),
        name: "Bgogo_1",
        exchange: "Bgogo",
    },
    CexAddress {
        address: address!("06BA294D190b5F9788FfCa86cE19a43CD746d36A"),
        name: "BiKi",
        exchange: "BiKi",
    },
    CexAddress {
        address: address!("6eFb20f61B80F6a7ebe7a107baCe58288a51FB34"),
        name: "BiKi_1",
        exchange: "BiKi",
    },
    CexAddress {
        address: address!("6efF3372fa352b239Bb24ff91b423A572347000D"),
        name: "BiKi_2",
        exchange: "BiKi",
    },
    CexAddress {
        address: address!("F71CBF6758aaAaF06eBCcA5447019c31bB145782"),
        name: "BiKi_3",
        exchange: "BiKi",
    },
    CexAddress {
        address: address!("fd736FAA01073D35c148B61093E6AE562B2f8544"),
        name: "BiKi_4",
        exchange: "BiKi",
    },
    CexAddress {
        address: address!("24D55BF5031D46b8Ebb656e905e3CcE759EA526F"),
        name: "Bibox",
        exchange: "Bibox",
    },
    CexAddress {
        address: address!("76bD39DBc1cc977c03d38dc8a70ECbF21177c0Df"),
        name: "Bibox_1",
        exchange: "Bibox",
    },
    CexAddress {
        address: address!("B0d3C2D9F7D3C65D83a3Af84A8584C2AD6Bee3E4"),
        name: "Bibox_2",
        exchange: "Bibox",
    },
    CexAddress {
        address: address!("EA7B33d264F4B7e6fd283A8250a572f2cEEfaCD4"),
        name: "Bibox_3",
        exchange: "Bibox",
    },
    CexAddress {
        address: address!("f73C3c65bde10BF26c2E1763104e609A41702EFE"),
        name: "Bibox_4",
        exchange: "Bibox",
    },
    CexAddress {
        address: address!("856cb5c3cBBe9e2E21293A644aA1f9363CEE11E8"),
        name: "Biconomy",
        exchange: "Biconomy",
    },
    CexAddress {
        address: address!("94D3E62151B12A12A4976F60EdC18459538FaF08"),
        name: "Biconomy_1",
        exchange: "Biconomy",
    },
    CexAddress {
        address: address!("c864019047B864B6ab609a968ae2725DFaee808A"),
        name: "Biconomy_2",
        exchange: "Biconomy",
    },
    CexAddress {
        address: address!("0639bFE9e40A08e7E08A04e119B13AFFbEb9AeD9"),
        name: "Bidesk",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("0bB5DE248DbbD31eE6c402C3c4a70293024ACf74"),
        name: "Bidesk_1",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("2fFe12462A7415A154a9db89266F257061c83f3D"),
        name: "Bidesk_10",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("3aaD4FF78052fDF407CD4eb856923D3180e941D3"),
        name: "Bidesk_11",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("3c047B9Dfa4fd8F812f264f1611b959a4DD980f8"),
        name: "Bidesk_12",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("40A6C95f8809D7b0363Eb559Fbe3481d3731Df68"),
        name: "Bidesk_13",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("42289749f57C0fB81f3C079291F9D3513b76FbF8"),
        name: "Bidesk_14",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("45E5b30803C3a5c6f9E68451026D912CD9C9EFd6"),
        name: "Bidesk_15",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("46f696DDDBb9edBF504A4e0226016190995B6dc6"),
        name: "Bidesk_16",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("5388cBdB0A5B760953FBBAE2bD341c4a06b8dd79"),
        name: "Bidesk_17",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("599814467C8AcBD77B761702f741325af5018a1f"),
        name: "Bidesk_18",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("5D027af50Ae06f210BC8ED64f183c248Eb95eAc8"),
        name: "Bidesk_19",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("0C2F8e43cc0d7184D1057620994fa32F03F64a8d"),
        name: "Bidesk_2",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("6358A984302f5c7C261743423aCa4B3fF575765B"),
        name: "Bidesk_20",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("6ec17B53659C71427cFeb19ACFAE5ABd29b69a16"),
        name: "Bidesk_21",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("7423931617700331FA34D2Df2b58Ee809626625B"),
        name: "Bidesk_22",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("7915E66B491C0e0D6e4f2d00C398866332B7c167"),
        name: "Bidesk_23",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("7FeC5b3d62f357Dc0431f38ed73c8198420430D5"),
        name: "Bidesk_24",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("8c699A889413893Ca1f03bA6405ae66c66F955E6"),
        name: "Bidesk_25",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("8D76166C22658A144c0211d87Abf152e6a2d9D95"),
        name: "Bidesk_26",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("96C604c94DCc5E356d1818F321cec9826675964b"),
        name: "Bidesk_27",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("99bD796babE675B410943387b2e0Bf87a54296B9"),
        name: "Bidesk_28",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("A31974408AAD18C1e5C4648d435795Fa757EA47f"),
        name: "Bidesk_29",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("103f11005A9521F5CC3D6331A2C0c441ED827735"),
        name: "Bidesk_3",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("a4938E606eDfe350D08813a0785Ed85D4640d365"),
        name: "Bidesk_30",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("b85a1fb66A67804A017E7B9270Db5c6c06c21A3C"),
        name: "Bidesk_31",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("C13CA46A9C895b6aFd8a44A1A8238a242DBd176D"),
        name: "Bidesk_32",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("c2BA04E89016f417e1219Af7eF82a5B6A9214793"),
        name: "Bidesk_33",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("CdFf68f58470E19c1D77643e864a9D4437754db4"),
        name: "Bidesk_34",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("Ce76a14dABe64acb8b044c489374C8ca1f456837"),
        name: "Bidesk_35",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("d61Ff104C480A225AB4666b95b5Ddeb101352A56"),
        name: "Bidesk_36",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("De528EceC4F16cc20bFAfAD23B41388343C243C4"),
        name: "Bidesk_37",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("e05b4BDf83274545E64f4f19Db75e64E6492A3b3"),
        name: "Bidesk_38",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("E383E773245a93574B528b5415C752BB866AD23A"),
        name: "Bidesk_39",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("1a9e7F7D49f4E9FddF063617Cde6514b711758cB"),
        name: "Bidesk_4",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("Ed5cdB0D02152046E6f234aD578613831b9184D4"),
        name: "Bidesk_40",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("ed643A3cf76AB65F7F9a7833656881e91097B6d2"),
        name: "Bidesk_41",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("F0d04D794053936807D553319dd0988D77Ab78df"),
        name: "Bidesk_42",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("F1F178e5Cf884134A72D48d07f259894390FF1f9"),
        name: "Bidesk_43",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("f3385c7aE159057Bbe1Aae4eB4292E84c2679365"),
        name: "Bidesk_44",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("F6cf41af1dc8aec47dBf92Eb0Ef23099b22b803E"),
        name: "Bidesk_45",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("FA3F128bE5ce8c42082cA1468544e6e1F9F22824"),
        name: "Bidesk_46",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("fbc865A47c741Be6a245e1Cbf9A7Fcfae048aEDb"),
        name: "Bidesk_47",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("1b9e8A6caE8Dc7796cEbb40997eBC56e798E7578"),
        name: "Bidesk_5",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("1Ee7C37EFF6a37d38d805A0ceDc134147a9DD3Cf"),
        name: "Bidesk_6",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("1F13246D1D34f5Ae1C588DbEA54248451801065f"),
        name: "Bidesk_7",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("233Da32CA8CD6CE0928C9893382216E6F81F8F16"),
        name: "Bidesk_8",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("23ce3e09CeCF4D0B1C876224731F0B73B5e523bC"),
        name: "Bidesk_9",
        exchange: "Bidesk",
    },
    CexAddress {
        address: address!("17Bc58b788808DaB201a9A90817fF3C168BF3d61"),
        name: "BigONE",
        exchange: "BigONE",
    },
    CexAddress {
        address: address!("1A84b64Cc85BFab627dA6537cBE8E9E26C0B3ed0"),
        name: "BigONE_1",
        exchange: "BigONE",
    },
    CexAddress {
        address: address!("493144718a78DfBEF79F825AE71b29134b5cF60E"),
        name: "BigONE_2",
        exchange: "BigONE",
    },
    CexAddress {
        address: address!("8D61120Cf18069139220875C745B5A62C39Bd024"),
        name: "BigONE_3",
        exchange: "BigONE",
    },
    CexAddress {
        address: address!("a30D8157911ef23c46C0eB71889eFe6a648a41F7"),
        name: "BigONE_4",
        exchange: "BigONE",
    },
    CexAddress {
        address: address!("20DBb3496eBa23337F68461B6eCc3c79d4c78fd9"),
        name: "Bilaxy",
        exchange: "Bilaxy",
    },
    CexAddress {
        address: address!("4bb6E665e962EF2E0fD8400C43eBc7F9d2c24435"),
        name: "Bilaxy_1",
        exchange: "Bilaxy",
    },
    CexAddress {
        address: address!("9BA3560231e3E0aD7dde23106F5B98C72E30b468"),
        name: "Bilaxy_2",
        exchange: "Bilaxy",
    },
    CexAddress {
        address: address!("CCE8D59AFFdd93be338FC77FA0A298C2CB65Da59"),
        name: "Bilaxy_3",
        exchange: "Bilaxy",
    },
    CexAddress {
        address: address!("F22a4B9d9D1B20b44699E36A1e3903E78143f9Da"),
        name: "Bilaxy_4",
        exchange: "Bilaxy",
    },
    CexAddress {
        address: address!("f7793d27A1b76CDF14Db7C83e82C772cF7C92910"),
        name: "Bilaxy_5",
        exchange: "Bilaxy",
    },
    CexAddress {
        address: address!("001866Ae5B3de6cAa5a51543FD9fB64f524F5478"),
        name: "Binance",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("211Ee0129A67e7D44514152eB43D9f31103Ac46B"),
        name: "Binance US",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("001cEb373C83ae75b9f5CF78Fc2aBa3e185d09E2"),
        name: "Binance_1",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("141FeF8cd8397a390AFe94846c8bD6F4ab981c48"),
        name: "Binance_10",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("E0F0CfDe7Ee664943906f17F7f14342E76A5CeC7"),
        name: "Binance_100",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("e2fc31F816A9b94326492132018C3aEcC4a93aE1"),
        name: "Binance_101",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("e7804c37c13166fF0b37F5aE0BB07A3aEbb6e245"),
        name: "Binance_102",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("Eb25DF7c79a85640c4420680461DCDFD91F0dfAd"),
        name: "Binance_103",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("EB2d2F1b8c558a40207669291Fda468E50c8A0bB"),
        name: "Binance_104",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("ef7fb88F709aC6148C07D070BC71d252E8E13b92"),
        name: "Binance_105",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("F17ACEd3c7A8DAA29ebb90Db8D1b6efD8C364a18"),
        name: "Binance_106",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("f2DE20Dbf4b224Af77AA4FF446F43318800bD6b4"),
        name: "Binance_107",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("F3084ed5596c3eF9fCf53689da3B998e621a34C4"),
        name: "Binance_108",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("f92402bB795Fd7CD08fb83839689DB79099C8c9C"),
        name: "Binance_109",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("15ecE0d7de25436bCfcF3D62A9085Ddc7838aeE9"),
        name: "Binance_11",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("F977814e90dA44bFA03b6295A0616a897441aceC"),
        name: "Binance_110",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("Fc19E4Ce0e0a27B09f2011eF0512669A0F76367A"),
        name: "Binance_111",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("FDD2Ba77DB02Caa6a9869735dAC577d809CaDd11"),
        name: "Binance_112",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("fE9e8709d3215310075d67E3ed32A380CCf451C8"),
        name: "Binance_113",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("161bA15A5f335c9f06BB5BbB0A9cE14076FBb645"),
        name: "Binance_12",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("1763F1A93815Ee6e6bc3C4475d31cC9570716dB2"),
        name: "Binance_13",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("17B692ae403a8Ff3a3B2eD7676cF194310ddE9Af"),
        name: "Binance_14",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("19184aB45C40c2920B0E0e31413b9434ABD243eD"),
        name: "Binance_15",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("1B5B4e441F5A22bfd91B7772C780463F66A74b35"),
        name: "Binance_16",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("1D40B233CdF2cC0CDC347d5401D5b02c2831A0c1"),
        name: "Binance_17",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("1FBe2AcEe135D991592f167Ac371f3DD893A508B"),
        name: "Binance_18",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("21a31Ee1afC51d94C2eFcCAa2092aD1028285549"),
        name: "Binance_19",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("00799bbc833D5B168F0410312d2a8fD9e0e3079c"),
        name: "Binance_2",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("25681Ab599B4E2CEea31F8B498052c53FC2D74db"),
        name: "Binance_20",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("28C6c06298d514Db089934071355E5743bf21d60"),
        name: "Binance_21",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("290275e3db66394C52272398959845170E4DCb88"),
        name: "Binance_22",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("294B9B133cA7Bc8ED2CdD03bA661a4C6d3a834D9"),
        name: "Binance_23",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("29bDfbf7D27462a2d115748ace2bd71A2646946c"),
        name: "Binance_24",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("29Fe6c66097F7972d8e47c4f691576327Fcf9A12"),
        name: "Binance_25",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("2E581a5aE722207Aa59aCD3939771E7c7052DD3d"),
        name: "Binance_26",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("2f47A1c2Db4a3B78CDA44eADE915c3b19107DDcc"),
        name: "Binance_27",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("2f7e209e0F5F645c7612D7610193Fe268F118b28"),
        name: "Binance_28",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("3304E22DDaa22bCdC5fCa2269b418046aE7b566A"),
        name: "Binance_29",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("01C952174C24E1210d26961D456A77A39e1F0BB0"),
        name: "Binance_3",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("345D8e3A1F62eE6B1D483890976fD66168e390F2"),
        name: "Binance_30",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("370b8EAad4e5970a853d25cD26a499150FD38274"),
        name: "Binance_31",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("3931dAb967C3E2dbb492FE12460a66d0fe4cC857"),
        name: "Binance_32",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("3c783c21a0383057D128bae431894a5C19F9Cf06"),
        name: "Binance_33",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("3CDfB47b0E910d9190eD788726cD72489bf10499"),
        name: "Binance_34",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("3f5CE5FBFe3E9af3971dD833D26bA9b5C936f0bE"),
        name: "Binance_35",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("417850c1Cd0fB428eb63649E9DC4C78edE9A34E8"),
        name: "Binance_36",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("44592b81c05b4c35Efb8424eB9D62538b949eBbF"),
        name: "Binance_37",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("47ac0Fb4F2D84898e4D9E7b4DaB3C24507a6D503"),
        name: "Binance_38",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("4976A4A02f38326660D17bf34b431dC6e2eb2327"),
        name: "Binance_39",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("0681d8Db095565FE8A346fA0277bFfdE9C0eDBBF"),
        name: "Binance_4",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("4A9E49A45A4b2545Cb177F79C7381A30e1Dc261F"),
        name: "Binance_40",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("4aeFa39caEAdD662aE31ab0CE7c8C2c9c0a013E8"),
        name: "Binance_41",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("4D072A68d0428A9A3054e03Ad7Ee61C557b537ab"),
        name: "Binance_42",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("4D9fF50EF4dA947364BB9650892B2554e7BE5E2B"),
        name: "Binance_43",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("4E9ce36E442e55EcD9025B9a6E0D88485d628A67"),
        name: "Binance_44",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("50460c4CD74094CD591F455caD457e99c4AB8Be0"),
        name: "Binance_45",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("505e71695E9bc45943c58adEC1650577BcA68fD9"),
        name: "Binance_46",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("50d669F43b484166680Ecc3670E4766cdb0945CE"),
        name: "Binance_47",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("515b72Ed8a97F42C568D6A143232775018f133C8"),
        name: "Binance_48",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("564286362092D8e7936f0549571a803B203aAceD"),
        name: "Binance_49",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("06a0048079ec6571Cd1b537418869CDE6191d42D"),
        name: "Binance_5",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("56Eddb7aa87536c09CCc2793473599fD21A8b17F"),
        name: "Binance_50",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("5a52E96BAcdaBb82fd05763E25335261B270Efcb"),
        name: "Binance_51",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("5D7F34372FA8708E09689D400A613EeE67F75543"),
        name: "Binance_52",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("631Fc1EA2270e98fbD9D92658eCe0F5a269Aa161"),
        name: "Binance_53",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("66F791456b82921CBc3F89A98c24Ea21784973a1"),
        name: "Binance_54",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("6bE5A267B04E9f24CdC1824fd38d63c436be91aB"),
        name: "Binance_55",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("6D8bE5cdf0d7DEE1f04E25FD70B001AE3B907824"),
        name: "Binance_56",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("708396f17127c42383E3b9014072679b2F60B82f"),
        name: "Binance_57",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("73f5ebe90f27B46ea12e5795d16C4b408B19cc6F"),
        name: "Binance_58",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("7a8A34DB9acD10C3b6277473b192FE47192569cA"),
        name: "Binance_59",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("07B664C8aF37EdDAa7e3b6030ed1F494975e9DFB"),
        name: "Binance_6",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("7Ab33AD1E91dDF6d5edf69a79D5d97a9c49015D4"),
        name: "Binance_60",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("7AeD074cA56F5050D5A2E512eCc5bf7103937d76"),
        name: "Binance_61",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("7DFe9A368B6Cf0C0309b763bb8d16da326e8F46e"),
        name: "Binance_62",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("7E278a68A35D76A7E4b2C9d8b778aCD775C6d832"),
        name: "Binance_63",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("835678a611B28684005a5e2233695fB6cbbB0007"),
        name: "Binance_64",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("85b931A32a0725Be14285B66f1a22178c672d69B"),
        name: "Binance_65",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("87917D879ba83CE3Ada6e02d49A10c1eC1988062"),
        name: "Binance_66",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("8894E0a0c962CB723c1976a4421c95949bE2D4E3"),
        name: "Binance_67",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("892e9e24AeA3f27f4C6E9360e312Cce93cc98Ebe"),
        name: "Binance_68",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("8B99F3660622e21f2910ECCA7fBe51d654a1517D"),
        name: "Binance_69",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("082489A616aB4D46d1947eE3F912e080815b08DA"),
        name: "Binance_7",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("8F22F2063D253846B53609231eD80FA571Bc0C8F"),
        name: "Binance_70",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("8f80C66C70cBC52009babB04c1CadF9b40109289"),
        name: "Binance_71",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("8fF804cc2143451F454779A40DE386F913dCff20"),
        name: "Binance_72",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("923fc76cB13A14e5A87843d309C9f401EC498E2d"),
        name: "Binance_73",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("9430801EBAf509Ad49202aaBC5F5Bc6fd8A3dAf8"),
        name: "Binance_74",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("9696f59E4d72E237BE84fFD425DCaD154Bf96976"),
        name: "Binance_75",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("972Bed5493F7E7bdc760265Fbb4d8e73ea89e453"),
        name: "Binance_76",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("9CD1AC952951fe63C658589db0DdE32fc55b815B"),
        name: "Binance_77",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("9f8c163cBA728e99993ABe7495F06c0A3c8Ac8b9"),
        name: "Binance_78",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("a180Fe01B906A1bE37BE6c534a3300785b20d947"),
        name: "Binance_79",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("0b95993A39A363d99280Ac950f5E4536Ab5C5566"),
        name: "Binance_8",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("a344c7aDA83113B3B56941F6e85bf2Eb425949f3"),
        name: "Binance_80",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("a7C0D36c4698981FAb42a7d8c783674c6Fe2592d"),
        name: "Binance_81",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("A84fD90d8640FA63D194601E0B2D1c9094297083"),
        name: "Binance_82",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("AB83D182f3485cf1D6ccdd34C7CFEf95b4C08da4"),
        name: "Binance_83",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("acD03D601e5bB1B275Bb94076fF46ED9D753435A"),
        name: "Binance_84",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("AD9ffffd4573b642959D3B854027735579555Cbc"),
        name: "Binance_85",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("B1256D6b31E4Ae87DA1D56E5890C66be7f1C038e"),
        name: "Binance_86",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("b32e9A84Ae0B55b8ab715e4Ac793a61B277bAFA3"),
        name: "Binance_87",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("B38e8c17e38363aF6EbdCb3dAE12e0243582891D"),
        name: "Binance_88",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("B3f923eaBAF178fC1BD8E13902FC5C61D3DdEF5B"),
        name: "Binance_89",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("0E4158C85FF724526233c1aeB4fF6f0C46827FbE"),
        name: "Binance_9",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("BD612a3f30dcA67bF60a39Fd0D35e39B7aB80774"),
        name: "Binance_90",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("BdD75A97c29294FF805FB2fEe65aBd99492b32A8"),
        name: "Binance_91",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("BE0eB53F46cd790Cd13851d5EFf43D12404d33E8"),
        name: "Binance_92",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("c365c3315cF926351CcAf13fA7D19c8C4058C8E1"),
        name: "Binance_93",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("C3C8E0A39769e2308869f7461364ca48155D1d9E"),
        name: "Binance_94",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("D551234Ae421e3BCBA99A0Da6d736074f22192FF"),
        name: "Binance_95",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("d88B55467f58af508dBfDC597E8Ebd2Ad2De49b3"),
        name: "Binance_96",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("d9D93951896B4eF97D251334EF2A0e39F6F6D7d7"),
        name: "Binance_97",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("dccF3B77dA55107280bd850ea519DF3705D1a75a"),
        name: "Binance_98",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("DFd5293D8e347dFe59E90eFd55b2956a1343963d"),
        name: "Binance_99",
        exchange: "Binance",
    },
    CexAddress {
        address: address!("21d45650db732cE5dF77685d6021d7D5d1da807f"),
        name: "Binance US_1",
        exchange: "Binance US",
    },
    CexAddress {
        address: address!("34ea4138580435B5A521E460035edb19Df1938c1"),
        name: "Binance US_2",
        exchange: "Binance US",
    },
    CexAddress {
        address: address!("43c5b1C2bE8EF194a509cF93Eb1Ab3Dbd07B97eD"),
        name: "Binance US_3",
        exchange: "Binance US",
    },
    CexAddress {
        address: address!("61189Da79177950A7272c88c6058b96d4bcD6BE2"),
        name: "Binance US_4",
        exchange: "Binance US",
    },
    CexAddress {
        address: address!("9223C017a39d4806d1D92c15046Ae28c32c6D8E7"),
        name: "Binance US_5",
        exchange: "Binance US",
    },
    CexAddress {
        address: address!("b14A67c63BDA5024D2EffD53Aa16A00bB7F9A30a"),
        name: "Binance US_6",
        exchange: "Binance US",
    },
    CexAddress {
        address: address!("b650B0b1f183D59343dA753d94C068bFEA92693D"),
        name: "Binance US_7",
        exchange: "Binance US",
    },
    CexAddress {
        address: address!("D5C08681719445A5Fdce2Bda98b341A49050d821"),
        name: "Binance US_8",
        exchange: "Binance US",
    },
    CexAddress {
        address: address!("f60c2Ea62EDBfE808163751DD0d8693DCb30019c"),
        name: "Binance US_9",
        exchange: "Binance US",
    },
    CexAddress {
        address: address!("1651D700cD4020334bD185BA4c6E0271ffc0C732"),
        name: "BingX",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("29F3144d84Da21F9A6788f14680d0A2aa44D6F0e"),
        name: "BingX_1",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("b48C5CA99D33a8625E125f69AC8e07F3dFfE34A0"),
        name: "BingX_10",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("BB936d7CD3cDc6AC01088916d939cA5207cD84c2"),
        name: "BingX_11",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("C4334A9AF50C80A12C484de643149f6159Bdd110"),
        name: "BingX_12",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("da43c54Ce5083885F561E05fd6220b7096bE246c"),
        name: "BingX_13",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("DEc815281519F6cB080090317E0BA3E446Fafe43"),
        name: "BingX_14",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("E4516477ADacc6682Cf18069475f676a5B5667f9"),
        name: "BingX_15",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("ED5e461999B1Fedd00797a818Edd75C727Ed948F"),
        name: "BingX_16",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("EfA02443139F31E0336608e5a2F99D26784e4bfd"),
        name: "BingX_17",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("406C22b8740ae955b04fD11c2061E053807E2A69"),
        name: "BingX_2",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("4597A8206978C5DE22173432B2f0Cb899EEf9Fa3"),
        name: "BingX_3",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("6c69fa64EC451b1Bc5b5FBAa56CF648a281634Be"),
        name: "BingX_4",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("766182bFA8B8790d61c4D7E7912C1C3A6F42cef6"),
        name: "BingX_5",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("7C217Eb128337C4B44CA9093eA9a9984b1803488"),
        name: "BingX_6",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("A0FCA8fA8E9C6aA77305f94bE0e03908d0a42900"),
        name: "BingX_7",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("a88f86E5685FCa7C5D6de0e4D944875b007137b5"),
        name: "BingX_8",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("AF1e33f8153f25e304dEC5Cb544b5B6CcC5520eD"),
        name: "BingX_9",
        exchange: "BingX",
    },
    CexAddress {
        address: address!("0DE4b2BE45Ae233D5F782a5C70dFc8BFAB736528"),
        name: "Bit-Z",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("0F63AF93bf5d2E786FE4b47cBd6e264669E64456"),
        name: "Bit-Z_1",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("6EC1b4e6315eA30F52E5858d1F2A9Cb2E462D157"),
        name: "Bit-Z_10",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("8658264955e288275E8cD8788b4a7f10ca257836"),
        name: "Bit-Z_11",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("9c2cd7092c693D89E4106CE36125ccD754E589a8"),
        name: "Bit-Z_12",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("9F2b734417eC00b6E2c474Bd26d7e8cC737e7C96"),
        name: "Bit-Z_13",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("a1E522376F8f96A96e8Fb4722A1D4f26D6d17E2D"),
        name: "Bit-Z_14",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("A399Bf13e39ecA44a943ac02Da76913FE7aE0043"),
        name: "Bit-Z_15",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("a8aE6549c66C59aa55D50377948dFBE362d56B03"),
        name: "Bit-Z_16",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("Af2a0a5589eC469bda06deaA938D5B3b231D5FE8"),
        name: "Bit-Z_17",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("B67b5c792F6725CA3606626D679A28b551f2B4Ad"),
        name: "Bit-Z_18",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("b6F8b42396B012DC124c61012D5E9354966DB9ef"),
        name: "Bit-Z_19",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("237A734da47A70626B3A11e1928a5dcE12cb9E46"),
        name: "Bit-Z_2",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("bA5BFfd6F8fC5A2E65Cdc37043678a990178B009"),
        name: "Bit-Z_20",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("CF3618D4680817AF786a1D93465a19aB4225E69e"),
        name: "Bit-Z_21",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("dFDaCDab40bc0b339E15EDAEcDaF120C389D4dAe"),
        name: "Bit-Z_22",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("E65bD8fa44d9D6D37410891b98Cef29A96A84AaF"),
        name: "Bit-Z_23",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("f695c9e5F0E017F82B8f7b968075d949263795aa"),
        name: "Bit-Z_24",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("fC51869C1f33514bd315F1831eb17DFEA5E53C7a"),
        name: "Bit-Z_25",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("2D48B865D7b322E510BCF36953A06608E20e323e"),
        name: "Bit-Z_3",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("30146933A3A0BABc74eC0b3403beC69281Ba5914"),
        name: "Bit-Z_4",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("3C365fFb42Ae67ed147cF413a90E887f59ba9b24"),
        name: "Bit-Z_5",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("4B729cF402CfCfFd057E254924B32241AeDC1795"),
        name: "Bit-Z_6",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("5633764e2299253525Ea1b9AD9032C69A2b21D76"),
        name: "Bit-Z_7",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("65951c3A417e9b5e749e688b1bea1280a052c0f1"),
        name: "Bit-Z_8",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("68dD1c5cb1cE86341Ca9475f3FB81Cb2C2e8Dee3"),
        name: "Bit-Z_9",
        exchange: "Bit-Z",
    },
    CexAddress {
        address: address!("7c49e1c0e33F3efB57d64b7690Fa287C8D15B90A"),
        name: "Bit2C",
        exchange: "Bit2C",
    },
    CexAddress {
        address: address!("0d8824cA76e627E9CC8227Faa3B3993986ce9e48"),
        name: "BitBase",
        exchange: "BitBase",
    },
    CexAddress {
        address: address!("6DCD15A0dbeFd0700063a4445382D3506391A41A"),
        name: "BitBase_1",
        exchange: "BitBase",
    },
    CexAddress {
        address: address!("5D375281582791A38E0348915Fa9CBc6139E9C2a"),
        name: "BitBlinx",
        exchange: "BitBlinx",
    },
    CexAddress {
        address: address!("2125d0d68a13b1E7Fe73641Ef2098f621486e457"),
        name: "BitForex",
        exchange: "BitForex",
    },
    CexAddress {
        address: address!("3A723e58C4808DDE4591543282adC7D6b378715b"),
        name: "BitForex_1",
        exchange: "BitForex",
    },
    CexAddress {
        address: address!("3C48f8457Dbfbcea63AAf936A71d24DE7D37cc99"),
        name: "BitForex_2",
        exchange: "BitForex",
    },
    CexAddress {
        address: address!("704dDD09eF7A6D3034d76ae8Ca8c854eFA59B669"),
        name: "BitForex_3",
        exchange: "BitForex",
    },
    CexAddress {
        address: address!("a546E1D9D3748E9F9fE784A221fB8A8081702514"),
        name: "BitForex_4",
        exchange: "BitForex",
    },
    CexAddress {
        address: address!("eeC0Ed9E41C209c1c53a35900a06BF5DcA927405"),
        name: "BitForex_5",
        exchange: "BitForex",
    },
    CexAddress {
        address: address!("294c6F1Ec18494abe9f608eCD97a307C80586775"),
        name: "BitGo",
        exchange: "BitGo",
    },
    CexAddress {
        address: address!("6Fb3934EE371F5ea06c5F6a71cF7c7C6688fBd8D"),
        name: "BitGo_1",
        exchange: "BitGo",
    },
    CexAddress {
        address: address!("758982386D532d977B462F5b80E16928ea6652dc"),
        name: "BitGo_2",
        exchange: "BitGo",
    },
    CexAddress {
        address: address!("95EEaDDe20306a602cBa20AE8B4F29A95c5d6405"),
        name: "BitGo_3",
        exchange: "BitGo",
    },
    CexAddress {
        address: address!("99126dAF078c693d200155E2dd7a668479120745"),
        name: "BitGo_4",
        exchange: "BitGo",
    },
    CexAddress {
        address: address!("a5D29237a8F14FF25ab9683C38e64671E3bB5cc8"),
        name: "BitGo_5",
        exchange: "BitGo",
    },
    CexAddress {
        address: address!("34F1b0d87BB332d3D99A410376AC30499a9F97B9"),
        name: "BitKeep",
        exchange: "BitKeep",
    },
    CexAddress {
        address: address!("4E29fa717FB61753e26885421b84ff7E06Df585e"),
        name: "BitKeep_1",
        exchange: "BitKeep",
    },
    CexAddress {
        address: address!("603D022611BfE6A101DCdaB207D96C527F1d4d8e"),
        name: "BitKeep_2",
        exchange: "BitKeep",
    },
    CexAddress {
        address: address!("77E7c5CBeAaD915cf5462064B02984E16A902e67"),
        name: "BitKeep_3",
        exchange: "BitKeep",
    },
    CexAddress {
        address: address!("7d1288E5dbC91b9d2e7be736Cc789114e8D58f69"),
        name: "BitKeep_4",
        exchange: "BitKeep",
    },
    CexAddress {
        address: address!("8967711D157561656b236F36dB5F448bD63F7029"),
        name: "BitKeep_5",
        exchange: "BitKeep",
    },
    CexAddress {
        address: address!("a366Ac4542bedd0ab96b9e80687BbA2A0F1B7b19"),
        name: "BitKeep_6",
        exchange: "BitKeep",
    },
    CexAddress {
        address: address!("ee0ca9ca2deFF0F8be6A1229D89555689f8fe365"),
        name: "BitKeep_7",
        exchange: "BitKeep",
    },
    CexAddress {
        address: address!("F8C7e1e2f92cCBFb6911dbE62F38966Fe836eb9E"),
        name: "BitKeep_8",
        exchange: "BitKeep",
    },
    CexAddress {
        address: address!("EEA81C4416d71CeF071224611359F6F99A4c4294"),
        name: "BitMEX",
        exchange: "BitMEX",
    },
    CexAddress {
        address: address!("fB8131c260749c7835a08ccBdb64728De432858E"),
        name: "BitMEX_1",
        exchange: "BitMEX",
    },
    CexAddress {
        address: address!("03231B778a16D2d5222b4CED947c7Ad3fEa14635"),
        name: "BitMart",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("03Ca1829f4D3839467701592b9aDCC7bAbBD8769"),
        name: "BitMart_1",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("5c7beD3Cca42e4562877eD88B9Aa0F5898Ed59B0"),
        name: "BitMart_10",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("68b22215FF74E3606BD5E6c1DE8c2D68180c85F7"),
        name: "BitMart_11",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("6D0D19bddDC5ED1dD501430c9621DD37ebd9062d"),
        name: "BitMart_12",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("701f38C8c0eE48b4c1e5aEfc0a3C6880f1d3d445"),
        name: "BitMart_13",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("7563758243A262E96880F178aeE7817DcF47Ab0f"),
        name: "BitMart_14",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("79288AC3525c4E7669481571658A867F7E18f0B2"),
        name: "BitMart_15",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("8c128DBA2cB66399341AA877315BE1054be75da8"),
        name: "BitMart_16",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("8EaFEE3d0DF538A1e04487a43239c1C73B50032d"),
        name: "BitMart_17",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("a1F54002f695E79380C7A0A27B14C8e33f1E1228"),
        name: "BitMart_18",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("A9E4332448318dA58CDD398286c0809684eD9BD4"),
        name: "BitMart_19",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("11FC614E2218b479F94636C234BC352EE490EFd1"),
        name: "BitMart_2",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("Abb239191ab5d0482Ab7C74F412d2B117f4EeD3a"),
        name: "BitMart_20",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("d11616e66b128c0b756b91cC13466deFaae67D07"),
        name: "BitMart_21",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("e79eeF9b9388A4fF70ed7ec5Bccd5B928ebB8Bd1"),
        name: "BitMart_22",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("eACB50a28630a4C44a884158eE85cBc10d2B3F10"),
        name: "BitMart_23",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("f3d4aa3C6925B38D40C2ae4C7A935d83666Ae5f7"),
        name: "BitMart_24",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("F8f21a32648a540dd8e982ff47BEF6Be2e823F9E"),
        name: "BitMart_25",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("F990D51057Ee5EBd2EB627E5F179dAc90e3B2b25"),
        name: "BitMart_26",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("1Aac8BC17DA523b9bC7470B0C9eD47a83760ACef"),
        name: "BitMart_3",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("328130164d0F2B9D7a52edC73b3632e713ff0ec6"),
        name: "BitMart_4",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("3752a0F9BeA7cDf593F46533E23853161233BD04"),
        name: "BitMart_5",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("3aB28eCeDEa6cdb6feeD398E93Ae8c7b316B1182"),
        name: "BitMart_6",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("3b53e17E54c64e83185954726d251c040200D80F"),
        name: "BitMart_7",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("3f0A468c36E575a994e0166bdc2C62896f3A4A80"),
        name: "BitMart_8",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("4bd3F43C6bfbFda03664ee3Ce4C2bcceAe2AAab0"),
        name: "BitMart_9",
        exchange: "BitMart",
    },
    CexAddress {
        address: address!("2730ef3C0c180E7f7bCFCA249c757421B208e333"),
        name: "BitPay",
        exchange: "BitPay",
    },
    CexAddress {
        address: address!("5763A2A8194E9BD0b8140ABccB9171F005470324"),
        name: "BitPay_1",
        exchange: "BitPay",
    },
    CexAddress {
        address: address!("F2a14015EaA3F9cC987f2c3b62FC93Eee41aA5d0"),
        name: "BitPay_2",
        exchange: "BitPay",
    },
    CexAddress {
        address: address!("1b8a38ea02cEDA9440E00C1Aeba26eE2DC570423"),
        name: "BitStorage",
        exchange: "BitStorage",
    },
    CexAddress {
        address: address!("aa90b4aaE74CEE41e004BC45e45A427406C4dcAe"),
        name: "BitUN.io",
        exchange: "BitUN.io",
    },
    CexAddress {
        address: address!("F8D04A720520d0bCbc722B1d21CA194AA22699f2"),
        name: "BitUN.io_1",
        exchange: "BitUN.io",
    },
    CexAddress {
        address: address!("25Ee4Ce905Da85df8620cB82884adDf96A14498A"),
        name: "BitVenus",
        exchange: "BitVenus",
    },
    CexAddress {
        address: address!("2B097741854EEdeB9e5c3ef9D221fb403d8d8609"),
        name: "BitVenus_1",
        exchange: "BitVenus",
    },
    CexAddress {
        address: address!("4785e47aE7061632C2782384DA28B9F68a5647a3"),
        name: "BitVenus_2",
        exchange: "BitVenus",
    },
    CexAddress {
        address: address!("5631aA1fc1868703a962e2fD713dc02cad07C1DB"),
        name: "BitVenus_3",
        exchange: "BitVenus",
    },
    CexAddress {
        address: address!("686b9202a36C09CE8aBa8b49Ae5F75707EDEc5fE"),
        name: "BitVenus_4",
        exchange: "BitVenus",
    },
    CexAddress {
        address: address!("E1E5F8caCc6B9Ace0894Fe7ba467328587e60bE7"),
        name: "BitVenus_5",
        exchange: "BitVenus",
    },
    CexAddress {
        address: address!("E43C53c466A282773F204df0b0A58fb6F6A88633"),
        name: "BitVenus_6",
        exchange: "BitVenus",
    },
    CexAddress {
        address: address!("ef7A2610a7C9cfB2537d68916B6A87FeA8Acfec3"),
        name: "BitVenus_7",
        exchange: "BitVenus",
    },
    CexAddress {
        address: address!("3727cfCBD85390Bb11B3fF421878123AdB866be8"),
        name: "Bitbank",
        exchange: "Bitbank",
    },
    CexAddress {
        address: address!("620a3E5cDdD2748E111A11810757f419d10B1AaC"),
        name: "Bitbank_1",
        exchange: "Bitbank",
    },
    CexAddress {
        address: address!("DBfC4549b325b6f00013A3861B87AAB6696FdBEe"),
        name: "Bitbank_2",
        exchange: "Bitbank",
    },
    CexAddress {
        address: address!("eB6c4bE4b92a52e969F4bF405025D997703D5383"),
        name: "Bitbank_3",
        exchange: "Bitbank",
    },
    CexAddress {
        address: address!("F9225f3288f6cb0d0f80A5561e73102565E8bD8C"),
        name: "Bitbank_4",
        exchange: "Bitbank",
    },
    CexAddress {
        address: address!("2b49cE21Ad2004CFb3d0b51B2E8eC0406d632513"),
        name: "Bitbee",
        exchange: "Bitbee",
    },
    CexAddress {
        address: address!("6B59210aDE46B62B25e82e95ab390A7CcAdd4c3a"),
        name: "Bitberry",
        exchange: "Bitberry",
    },
    CexAddress {
        address: address!("094b4cf43908F0AdB3dBDb5025F52470AAc3B160"),
        name: "Bitcasino",
        exchange: "Bitcasino",
    },
    CexAddress {
        address: address!("5BCbdfB6cc624b959c39A2D16110D1f2D9204F72"),
        name: "Bitcasino_1",
        exchange: "Bitcasino",
    },
    CexAddress {
        address: address!("910c00A13F2AF11c1e35fE6f6C43B8Ae4c82cF8a"),
        name: "Bitcasino_2",
        exchange: "Bitcasino",
    },
    CexAddress {
        address: address!("97180753F93E250D846d51034bd2bD62375Dc7b0"),
        name: "Bitcasino_3",
        exchange: "Bitcasino",
    },
    CexAddress {
        address: address!("9e3ef83728a399d6f5f757E3FA04f4850BBfB5f0"),
        name: "Bitcasino_4",
        exchange: "Bitcasino",
    },
    CexAddress {
        address: address!("a339aAEE0acC7A96fB34F3F65e600Fd5237dEe22"),
        name: "Bitcasino_5",
        exchange: "Bitcasino",
    },
    CexAddress {
        address: address!("E94d9b695fF36AFa8Db1f764beDC604FB04eCb95"),
        name: "Bitcasino_6",
        exchange: "Bitcasino",
    },
    CexAddress {
        address: address!("7A91a362d4f2c9C4627688D5B7090BBB12e5715f"),
        name: "Bitci",
        exchange: "Bitci",
    },
    CexAddress {
        address: address!("E954B098b80D43FD66AF4a58400C05E62B087b72"),
        name: "Bitci_1",
        exchange: "Bitci",
    },
    CexAddress {
        address: address!("D57fe94225A8Fd8e1a1826de1c6d6b3AFc97C062"),
        name: "Bitcoin Meester",
        exchange: "Bitcoin",
    },
    CexAddress {
        address: address!("2a7077399B3e90F5392D55A1Dc7046ad8D152348"),
        name: "Bitcoin Suisse",
        exchange: "Bitcoin",
    },
    CexAddress {
        address: address!("31dFf0cf605F9719b9171f6049150595CC1240F1"),
        name: "Bitcoin Suisse_1",
        exchange: "Bitcoin Suisse",
    },
    CexAddress {
        address: address!("3f262579E4332e1Be2722684EAa1C1b111F7a8d8"),
        name: "Bitcoin Suisse_2",
        exchange: "Bitcoin Suisse",
    },
    CexAddress {
        address: address!("7B4576d06D0Ce1F83F9a9B76BF8077bFFD34FcB1"),
        name: "Bitcoin Suisse_3",
        exchange: "Bitcoin Suisse",
    },
    CexAddress {
        address: address!("c2288B408Dc872A1546F13E6eBFA9c94998316a2"),
        name: "Bitcoin Suisse_4",
        exchange: "Bitcoin Suisse",
    },
    CexAddress {
        address: address!("FCB7Edb966d320c7f3AE1f751a8c86F30fA5ad37"),
        name: "Bitcoin Suisse_5",
        exchange: "Bitcoin Suisse",
    },
    CexAddress {
        address: address!("28eBe764B8F9A853509840645216D3C2c0fd774b"),
        name: "BiteBTC",
        exchange: "BiteBTC",
    },
    CexAddress {
        address: address!("53bA297c0FF8973B470436360e74aE92eA332399"),
        name: "BiteBTC_1",
        exchange: "BiteBTC",
    },
    CexAddress {
        address: address!("76F1Cd864fc153Eda7A9f5c407380d9a80154A16"),
        name: "BiteBTC_2",
        exchange: "BiteBTC",
    },
    CexAddress {
        address: address!("d8ee4A76B99292c1BB0F45d5c6Fa5F99919b7e64"),
        name: "BiteBTC_3",
        exchange: "BiteBTC",
    },
    CexAddress {
        address: address!("57A47cFE647306A406118B6cF36459a1756823D0"),
        name: "Bitexlive",
        exchange: "Bitexlive",
    },
    CexAddress {
        address: address!("7217d64f77041Ce320c356D1a2185Bcb89798A0A"),
        name: "Bitexlive_1",
        exchange: "Bitexlive",
    },
    CexAddress {
        address: address!("dfc38911F6E0bfDD0472F6f68d83E8A0115768b2"),
        name: "Bitfex.trade",
        exchange: "Bitfex.trade",
    },
    CexAddress {
        address: address!("f2e0e06771414a14d9d1bb70cD81030434421Cb3"),
        name: "Bitfex.trade_1",
        exchange: "Bitfex.trade",
    },
    CexAddress {
        address: address!("0b73F67A49273fc4B9A65DBD25D7d0918E734E63"),
        name: "Bitfinex",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("0cD76cD43992C665FdC2d8aC91B935CA3165E782"),
        name: "Bitfinex_1",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("482F02e8BC15b5EAbC52C6497b425B3Ca3c821E8"),
        name: "Bitfinex_10",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("4fdd5Eb2FB260149A3903859043e962Ab89D8ED4"),
        name: "Bitfinex_11",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("53B36141490c419fa27ecabFEB8Be1ecAdc82431"),
        name: "Bitfinex_12",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("5754284f345afc66a98fbB0a0Afe71e0F007B949"),
        name: "Bitfinex_13",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("58AE42A38D6b33A1E31492B60465fA80dA595755"),
        name: "Bitfinex_14",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("59448fe20378357F206880c58068f095ae63d5A5"),
        name: "Bitfinex_15",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("5a710a3cDF2AF218740384c52a10852D8870626A"),
        name: "Bitfinex_16",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("618F37D7ff7B140E604172466CD42D1Ec35E0544"),
        name: "Bitfinex_17",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("7180EB39A6264938FDB3EfFD7341C4727c382153"),
        name: "Bitfinex_18",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("742d35Cc6634C0532925a3b844Bc454e4438f44e"),
        name: "Bitfinex_19",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("1151314c646Ce4E0eFD76d1aF4760aE66a9Fe30F"),
        name: "Bitfinex_2",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("77134cbC06cB00b66F4c7e623D5fdBF6777635EC"),
        name: "Bitfinex_20",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("7727E5113D1d161373623e5f49FD568B4F543a9E"),
        name: "Bitfinex_21",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("8103683202aa8DA10536036EDef04CDd865C225E"),
        name: "Bitfinex_22",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("876EabF441B2EE5B5b0554Fd502a8E0600950cFa"),
        name: "Bitfinex_23",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("88037f361891A0B5De3C0C30632fBcA7DB2D341F"),
        name: "Bitfinex_24",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("ab7c74abC0C4d48d1bdad5DCB26153FC8780f83E"),
        name: "Bitfinex_25",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("C56fEFd1028B0534bfaDCdB580d3519b5586246E"),
        name: "Bitfinex_26",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("C58B32218A746B70813A057275591966deb5920e"),
        name: "Bitfinex_27",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("cAfB10eE663f465f9d10588AC44eD20eD608C11e"),
        name: "Bitfinex_28",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("dcD0272462140D0A3cEd6C4bf970c7641f08CD2c"),
        name: "Bitfinex_29",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("1b29DD8fF0EB3240238bF97CaFD6edeA05D5Ba82"),
        name: "Bitfinex_3",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("E92d1A43df510F82C66382592a047d288f85226f"),
        name: "Bitfinex_30",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("Ed9Eef56E64A8E779cFaE9ddEDb25d11Ba2B2425"),
        name: "Bitfinex_31",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("f4B51B14b9EE30dc37EC970B50a486F37686E2a8"),
        name: "Bitfinex_32",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("1B8766d041567EeD306940c587e21C06aB968663"),
        name: "Bitfinex_4",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("28140CB1AC771d4Add91eE23788E50249C10263d"),
        name: "Bitfinex_5",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("2EE3B2dF6534abc759ffE994f7b8DcDFAa02cd31"),
        name: "Bitfinex_6",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("30a2EBF10f34c6C4874b0bDD5740690fD2f3B70C"),
        name: "Bitfinex_7",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("36a85757645E8e8AeC062a1dEE289c7d615901Ca"),
        name: "Bitfinex_8",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("3F7E77B627676763997344a1AD71aCb765fc8aC5"),
        name: "Bitfinex_9",
        exchange: "Bitfinex",
    },
    CexAddress {
        address: address!("dF5021a4C1401F1125cD347e394d977630e17Cf7"),
        name: "Bitfront",
        exchange: "Bitfront",
    },
    CexAddress {
        address: address!("0639556F03714A74a5fEEaF5736a4A64fF70D206"),
        name: "Bitget",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("149Ded7438Caf5e5BFDc507a6c25436214d445E1"),
        name: "Bitget_1",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("4dFc15890972eceA7A213bDA2b478DAbC382e7a1"),
        name: "Bitget_10",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("5051e9860c1889Eb1bfa394365364B3dd61787F1"),
        name: "Bitget_11",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("51971c86b04516062c1e708CDC048CB04fbe959f"),
        name: "Bitget_12",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("5bdf85216ec1e38D6458C870992A69e38e03F7Ef"),
        name: "Bitget_13",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("6a3F28C47542bD5811AE37ab358D5d7E3ab84127"),
        name: "Bitget_14",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("731309E453972598eA05D706C6Ee6c3c21AB4D2a"),
        name: "Bitget_15",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("7651fC1605a58Fe9a99F1fe0d6db05D4182a9a93"),
        name: "Bitget_16",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("842Ea89f73ADD9e4fe963Ae7929fDc1e80AcdB52"),
        name: "Bitget_17",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("97b9D2102A9a65A26E1EE82D59e42d1B73B68689"),
        name: "Bitget_18",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("9E00816F61a709fa124D36664Cd7b6f14c13eE05"),
        name: "Bitget_19",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("1a96E5dA1315efCF9b75100F5757d5E8B76abb0C"),
        name: "Bitget_2",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("B8cda8d72DA558Ef8F76A0d928f9652D2b003e2e"),
        name: "Bitget_20",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("bC942E2250Ec7AB83bFC4516BCa4e281Dbfbb393"),
        name: "Bitget_21",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("dFE4B89cf009BFfa33D9BCA1f19694FC2d4d943d"),
        name: "Bitget_22",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("E2B406EC9227143A8830229eEb3Eb6E24b5c60Be"),
        name: "Bitget_23",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("e6a421f24d330967a3Af2F4cDB5c34067E7e4d75"),
        name: "Bitget_24",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("e80623a9d41f2f05780D9cD9cea0F797Fd53062A"),
        name: "Bitget_25",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("f646d9B7d20BABE204a89235774248BA18086dae"),
        name: "Bitget_26",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("1AB4973a48dc892Cd9971ECE8e01DcC7688f8F23"),
        name: "Bitget_3",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("1Ae3739E17d8500F2b2D80086ed092596A116E0b"),
        name: "Bitget_4",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("1d5BA5414f2983212E03Bf7725adD9eB4CdB00dC"),
        name: "Bitget_5",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("2bf7494111a59bD51f731DCd4873D7d71F8feEEC"),
        name: "Bitget_6",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("31A36512D4903635b7dd6828a934C3915A5809Be"),
        name: "Bitget_7",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("3A7d1A8C3A8dC9d48a68e628432198a2eAD4917c"),
        name: "Bitget_8",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("461f6dCdd5Be42D41FE71611154279d87c06B406"),
        name: "Bitget_9",
        exchange: "Bitget",
    },
    CexAddress {
        address: address!("0016C0d0343e8f2c3A7b6A51606B84B1545Ec606"),
        name: "Bithumb",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("03599A2429871E6be1B154Fb9c24691F9D301865"),
        name: "Bithumb_1",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("3052cD6BF951449A984fe4B5a38B46AEF9455c8E"),
        name: "Bithumb_10",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("3154f72F0A0b9023F1c18505c2b73f9cd1990caE"),
        name: "Bithumb_11",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("31D03f07178BcD74F9099AfeBD23B0AE30184ab5"),
        name: "Bithumb_12",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("3B83Cd1a8e516B6Eb9f1Af992E9354b15A6F9672"),
        name: "Bithumb_13",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("3fBE1f8Fc5dDb27d428aA60f661EAAaB0d2000ce"),
        name: "Bithumb_14",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("47E3aC26C5A8f1715DabFE1DB00e4bf1F54aFe23"),
        name: "Bithumb_15",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("5521a68D4F8253fC44BFb1490249369b3E299A4A"),
        name: "Bithumb_16",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("558553D54183a8542F7832742e7B4Ba9c33Aa1E6"),
        name: "Bithumb_17",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("560e389a2b032319E742A59AE8Bafa62671089fe"),
        name: "Bithumb_18",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("6fC48dE8f167456b7aa27dD4ecfaBBA329EA623D"),
        name: "Bithumb_19",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("0f863d0bBC760D7EF8d47c43639c13Fc4B0f2e04"),
        name: "Bithumb_2",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("771a2dA83236a230c15CE3EB07be4dE7164E3cfF"),
        name: "Bithumb_20",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("7Be98E1D8f29D23338E61694007D50D67144C7eF"),
        name: "Bithumb_21",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("83761c6785427F5A27a07c92a9dcFa99947bC4AD"),
        name: "Bithumb_22",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("88D34944cF554e9CCCf4a24292D891f620e9c94F"),
        name: "Bithumb_23",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("8FA8aF91C675452200e49b4683a33Ca2E1A34e42"),
        name: "Bithumb_24",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("97122dDca38c29b7653D52b07998d06a7128fa0B"),
        name: "Bithumb_25",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("A0Ff1e0F30b5DDA2dc01e7e828290Bc72b71E57d"),
        name: "Bithumb_26",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("a5Dab3C7a3821F6440a10d634E766bFD2750E54e"),
        name: "Bithumb_27",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("a84Aa98cA1C5DBFf58F825e28FaD700653439d5F"),
        name: "Bithumb_28",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("b08f3dC596fE40bE860ceE549b5c0a2f61De9f9F"),
        name: "Bithumb_29",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("15878e87c685f866edFaF454BE6Dc06Fa517B35B"),
        name: "Bithumb_3",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("b4460b75254ce0563Bb68eC219208344C7EA838c"),
        name: "Bithumb_30",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("B470faEa0dc6E1a99DbB4fA95B5bE47D9C27e00A"),
        name: "Bithumb_31",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("bb5A0408Fa54287B9074A2f47AB54c855e95EF82"),
        name: "Bithumb_32",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("c1dA8F69e4881efe341600620268934ef01a3E63"),
        name: "Bithumb_33",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("d273Bd546b11Bd60214A2F9d71f22A088AAfe31B"),
        name: "Bithumb_34",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("d341Dd814Eb0937Caf3517Ff2203C6d26F306898"),
        name: "Bithumb_35",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("D5a11A51fD0CDA5F119b78D87EaeAa970D77c55e"),
        name: "Bithumb_36",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("E320D449F11560ed9ff917799CB7CF10fFD7d6Ba"),
        name: "Bithumb_37",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("ed48DC0628789c2956B1E41726d062a86ec45bFF"),
        name: "Bithumb_38",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("EFb2E870b14D7e555a31B392541ACf002Dae6aE9"),
        name: "Bithumb_39",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("186549a4aE594fc1F70bA4CFFDAc714b405bE3F9"),
        name: "Bithumb_4",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("F1BafEFDED83fe7D2b754b3393445D01A7d3573B"),
        name: "Bithumb_40",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("2140eFD7Ba31169c69dfff6CDC66C542f0211825"),
        name: "Bithumb_5",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("22B84d5FFeA8b801C0422AFe752377A64Aa738c2"),
        name: "Bithumb_6",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("26BC3D0BB634E242359849Fe257a4cF76444D504"),
        name: "Bithumb_7",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("2F41Ea745c67724FDC65FF909318EDeB73Cfd6e7"),
        name: "Bithumb_8",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("2fFFb384d9bFB5F824958503B60D0d5962b080Ce"),
        name: "Bithumb_9",
        exchange: "Bithumb",
    },
    CexAddress {
        address: address!("8c00dDA00F9E2AabDE88a4d17f9EF9fCe265592F"),
        name: "Bitkan",
        exchange: "Bitkan",
    },
    CexAddress {
        address: address!("Cfbbf8dc80bb324d2F8634cc73d6e8F6784D3230"),
        name: "Bitkan_1",
        exchange: "Bitkan",
    },
    CexAddress {
        address: address!("1579B5f6582C7a04f5fFEec683C13008C4b0A520"),
        name: "Bitkub",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("1fd9393359b825156e8353e390E23664BB5ab4B8"),
        name: "Bitkub_1",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("B9C764114C5619a95d7f232594e3B8dDDF95b9CF"),
        name: "Bitkub_10",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("C488E4AfEC0414511d1518d8d6B8Dc5f820Fb92a"),
        name: "Bitkub_11",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("Ca7404EED62a6976Afc335fe08044B04dBB7e97D"),
        name: "Bitkub_12",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("DB044B8298E04D442FdBE5ce01B8cc8F77130e33"),
        name: "Bitkub_13",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("326D9f47BA49BBAac279172634827483af70a601"),
        name: "Bitkub_2",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("3d1D8A1d418220fd53C18744d44c182C46f47468"),
        name: "Bitkub_3",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("49876520C866D138dd749d6C2C33e4DA5bfAeC66"),
        name: "Bitkub_4",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("59E0cDA5922eFbA00a57794faF09BF6252d64126"),
        name: "Bitkub_5",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("6254B927ecC25DDd233aAECD5296D746B1C006B4"),
        name: "Bitkub_6",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("79169E7818968cD0C6DBd8929f24d797CC1Af9A1"),
        name: "Bitkub_7",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("9be7B0f285d04701f27682F591a60417C47d095A"),
        name: "Bitkub_8",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("Adf4c208d546E7F1Ec24cab1CcDA9B47B90B8540"),
        name: "Bitkub_9",
        exchange: "Bitkub",
    },
    CexAddress {
        address: address!("0B01450061e68c4f0f89167EFEAd245a3d393750"),
        name: "BitoPro",
        exchange: "BitoPro",
    },
    CexAddress {
        address: address!("16905697103Cd4AD6a22193B242941b62f660bBc"),
        name: "BitoPro_1",
        exchange: "BitoPro",
    },
    CexAddress {
        address: address!("1DfEFF70fbcC5949b29FaF52196f0711e4915B2c"),
        name: "BitoPro_2",
        exchange: "BitoPro",
    },
    CexAddress {
        address: address!("Aa65CC699F9B0258064d952aD987269f7C9DBFAD"),
        name: "BitoPro_3",
        exchange: "BitoPro",
    },
    CexAddress {
        address: address!("3da452fd6947BEa2735C5EAea04F1C22587cb5B2"),
        name: "Bitpanda",
        exchange: "Bitpanda",
    },
    CexAddress {
        address: address!("7071F121C038A98F8A7d485648a27FCD48891Ba8"),
        name: "Bitpanda_1",
        exchange: "Bitpanda",
    },
    CexAddress {
        address: address!("74dEc05E5b894b0EfEc69Cdf6316971802A2F9a1"),
        name: "Bitpanda_2",
        exchange: "Bitpanda",
    },
    CexAddress {
        address: address!("8cFa2d9047cA6573219C252E345506dE5e9da5d9"),
        name: "Bitpanda_3",
        exchange: "Bitpanda",
    },
    CexAddress {
        address: address!("B10EdD6fa6067DbA8d4326F1c8f0d1C791594F13"),
        name: "Bitpanda_4",
        exchange: "Bitpanda",
    },
    CexAddress {
        address: address!("b8Cbbf78c7Ad1cDF4cA0e111B35491f3bFE027AC"),
        name: "Bitpanda_5",
        exchange: "Bitpanda",
    },
    CexAddress {
        address: address!("F197c6F2aC14d25eE2789A73e4847732C7F16bC9"),
        name: "Bitpanda_6",
        exchange: "Bitpanda",
    },
    CexAddress {
        address: address!("F32682d5F99ba4143532618d6f516859a055Ea06"),
        name: "Bitpanda_7",
        exchange: "Bitpanda",
    },
    CexAddress {
        address: address!("19E460AC0a23706F156909b545De9D5072802065"),
        name: "Bitpie",
        exchange: "Bitpie",
    },
    CexAddress {
        address: address!("346B46826Be175c943a45cDBeC2E9D95dc52FB38"),
        name: "Bitpie_1",
        exchange: "Bitpie",
    },
    CexAddress {
        address: address!("48Fc0b223b818b55F2a6feb3ab329FF816edD2bc"),
        name: "Bitpie_2",
        exchange: "Bitpie",
    },
    CexAddress {
        address: address!("5c9F3ffF6ee846a83080F373F8ceA1451bB4a3D9"),
        name: "Bitpie_3",
        exchange: "Bitpie",
    },
    CexAddress {
        address: address!("7b138cc83ecCcD6331Cd7651c75ee3EB2ec6682E"),
        name: "Bitpie_4",
        exchange: "Bitpie",
    },
    CexAddress {
        address: address!("8979d1E0EcaB3cF5aE8A5a7b2d792aa119f34bE1"),
        name: "Bitpie_5",
        exchange: "Bitpie",
    },
    CexAddress {
        address: address!("ADEf6b2c5e848A4F04145ee88f20aC079f17bEab"),
        name: "Bitpie_6",
        exchange: "Bitpie",
    },
    CexAddress {
        address: address!("B685d0DAea607Fe75e02c1A7679261eAc661b9bc"),
        name: "Bitpie_7",
        exchange: "Bitpie",
    },
    CexAddress {
        address: address!("f64edD94558Ca8B3a0e3b362e20BB13ff52eA513"),
        name: "Bitpie_8",
        exchange: "Bitpie",
    },
    CexAddress {
        address: address!("4945cE2d1B5BD904CAc839b7FDAbAfd19Cab982b"),
        name: "Bitrefill",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("4d31f0089a27E7AFe56AC47C1551e09102d2834D"),
        name: "Bitrefill_1",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("B5de51cb0BF61554e47a7a6B6c7f429feC925B4f"),
        name: "Bitrefill_10",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("c5B4a8B2414C339a471CeF9663E67D57E33f41f9"),
        name: "Bitrefill_11",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("CBD17EE0703B92f858744F658c53Ad26CE206dfA"),
        name: "Bitrefill_12",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("EAdf413c2AB5B1488430469551241CcA29dF65b4"),
        name: "Bitrefill_13",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("F5E410B97aabdFbd019f1428fDe4E12815CABBAE"),
        name: "Bitrefill_14",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("6438beeB08cf4fABa533eCde748180DD56D43513"),
        name: "Bitrefill_2",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("69B8F9E1f633fBB8De3e279aAF34D4EfF05e61AF"),
        name: "Bitrefill_3",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("79267CD876AFe3E400917C1B26743630cC29f4e0"),
        name: "Bitrefill_4",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("88C633D244ee11269B33a5d29133c5E66E8951A5"),
        name: "Bitrefill_5",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("8f662404D92908324D25d7E9b814c89Ae2bFAD5E"),
        name: "Bitrefill_6",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("94Aa0b75A10bf4e92A00dE8A115a6f61095cF663"),
        name: "Bitrefill_7",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("95ba04c934d231C51435A266fFf2b2288e1D0AF3"),
        name: "Bitrefill_8",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("A381c04ebFDf07F9A63Eb8DbE2eF55c0122620AB"),
        name: "Bitrefill_9",
        exchange: "Bitrefill",
    },
    CexAddress {
        address: address!("6cc8dCbCA746a6E4Fdefb98E1d0DF903b107fd21"),
        name: "Bitrue",
        exchange: "Bitrue",
    },
    CexAddress {
        address: address!("d205a958527f083F1b222061B4D60D147fEc5044"),
        name: "Bitrue_1",
        exchange: "Bitrue",
    },
    CexAddress {
        address: address!("F4C62B4f8b7b1b1c4BA88BFD3a8ea392641516e9"),
        name: "Bitrue_2",
        exchange: "Bitrue",
    },
    CexAddress {
        address: address!("20bEea119E70255A8c36E4009C94AedB1F8B8Eea"),
        name: "Bitso",
        exchange: "Bitso",
    },
    CexAddress {
        address: address!("29D5527CaA78f1946a409FA6aCaf14A0a4A0274b"),
        name: "Bitso_1",
        exchange: "Bitso",
    },
    CexAddress {
        address: address!("58b704065B7aFF3ED351052f8560019E05925023"),
        name: "Bitso_2",
        exchange: "Bitso",
    },
    CexAddress {
        address: address!("A01AA2196724a39290f465b3925E5dCaFe7F2256"),
        name: "Bitso_3",
        exchange: "Bitso",
    },
    CexAddress {
        address: address!("f9D2D8D90B4b35aADA5AEd2F46FCD15DA0608B4e"),
        name: "Bitso_4",
        exchange: "Bitso",
    },
    CexAddress {
        address: address!("00BDb5699745f5b860228c8f939ABF1b9Ae374eD"),
        name: "Bitstamp",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("059799F2261d37b829c2850cEe67b5b975432271"),
        name: "Bitstamp_1",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("518B82370bc31eBB96922EC257D92517d7387615"),
        name: "Bitstamp_10",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("538d72dEd42A76A30f730292Da939e0577f22F57"),
        name: "Bitstamp_11",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("593aebEE9117EEA447279E5973F64C68D8e977a0"),
        name: "Bitstamp_12",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("6dCa94b6173c28a4900ea257121e6002C0B96968"),
        name: "Bitstamp_13",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("772396dD44Ce3d347838bFEC437CB32F534963F2"),
        name: "Bitstamp_14",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("7E677CaCaaE0D465Cfd336869f1F575a48BF012a"),
        name: "Bitstamp_15",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("808e7133C700cF3a66E6A25AAdB1fBEF6be468b4"),
        name: "Bitstamp_16",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("8366DCAB4Cc14c826fC9D51bd4c16567bD07B02a"),
        name: "Bitstamp_17",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("964771F6dF31EeA2D927Fa71d7bd78e81bcdce05"),
        name: "Bitstamp_18",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("9A9BED3Eb03E386D66f8a29DC67dC29Bbb1ccB72"),
        name: "Bitstamp_19",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("0b0F7ebF967146566799229394171FC47f1a765a"),
        name: "Bitstamp_2",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("9feC89e34efaa4FC9f19c02F474c71373e6effe7"),
        name: "Bitstamp_20",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("A3Fb85C3A2c50D8C0e1Dd7Fa7746F97C9E1D9591"),
        name: "Bitstamp_21",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("AB7bb7959332888E44d795c6F28eE876a8469EAA"),
        name: "Bitstamp_22",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("B8e73ba7C6c0b50a0cd94fe9F6622762b0401c02"),
        name: "Bitstamp_23",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("BcdDebA6a9672c1F76a8b8EDD3190BDFe6D4ef11"),
        name: "Bitstamp_24",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("C0AC2f4A3cF22fD504D8835B07f5acCcfa9b27F9"),
        name: "Bitstamp_25",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("C20b79CFf9d2C89bA8aeb9ABf4BfEf0314Ca7bD2"),
        name: "Bitstamp_26",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("C3b7336D5A5158215599572012CeDd4403A81629"),
        name: "Bitstamp_27",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("c5b611f502a0DCF6C3188Fd494061aE29B2baa4f"),
        name: "Bitstamp_28",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("Cddf488f1c826160eE832D4f1492f00cf8557Ff6"),
        name: "Bitstamp_29",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("1522900B6daFac587d499a862861C0869Be6E428"),
        name: "Bitstamp_3",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("d4FCC07a8da7d55599167991D4AB47f976d0A306"),
        name: "Bitstamp_30",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("De0f7Df88678E2aee576a2f3D9B18d4DfAd0155C"),
        name: "Bitstamp_31",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("e1576685451986e3f93C2fb87CCa3AEC5b5d45D0"),
        name: "Bitstamp_32",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("eE9FB7A615cb76b46d26BE6EbC9114a627A81C5B"),
        name: "Bitstamp_33",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("fbb23038Fe6CFa16Aa898d7DbCa7C3269bDAf258"),
        name: "Bitstamp_34",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("fCA70E67b3f93f679992Cd36323eEB5a5370C8e4"),
        name: "Bitstamp_35",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("182E1259eF6Ee45Dc811132eF4Ba5871F1536822"),
        name: "Bitstamp_4",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("1F69d824c3b4F906ac3FC8826e2391Bcb9330E02"),
        name: "Bitstamp_5",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("333C100AE1A2743a1e55D73913cAC6d95deB7F62"),
        name: "Bitstamp_6",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("3F3E23249F38d35A4CdAf44eDFD99eeb4325b401"),
        name: "Bitstamp_7",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("4c0907f7ad337635A7fd414A0c7a938e0d64BF4D"),
        name: "Bitstamp_8",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("4c766dEf136F59f6494f0969B1355882080CF8E0"),
        name: "Bitstamp_9",
        exchange: "Bitstamp",
    },
    CexAddress {
        address: address!("bDeF6e20692c302044B98284090922F683F3b523"),
        name: "Bitsten",
        exchange: "Bitsten",
    },
    CexAddress {
        address: address!("fe0cb30AFCb0EB1c27Ae33D11Be7ef749Ed25072"),
        name: "Bitsten_1",
        exchange: "Bitsten",
    },
    CexAddress {
        address: address!("66f820a414680B5bcda5eECA5dea238543F42054"),
        name: "Bittrex",
        exchange: "Bittrex",
    },
    CexAddress {
        address: address!("a3C1E324CA1ce40db73eD6026c4A177F099B5770"),
        name: "Bittrex_1",
        exchange: "Bittrex",
    },
    CexAddress {
        address: address!("E94b04a0FeD112f3664e45adb2B8915693dD5FF3"),
        name: "Bittrex_2",
        exchange: "Bittrex",
    },
    CexAddress {
        address: address!("FBb1b73C4f0BDa4f67dcA266ce6Ef42f520fBB98"),
        name: "Bittrex_3",
        exchange: "Bittrex",
    },
    CexAddress {
        address: address!("04A6b67FAC93e507d2642f129f5cB8aD474f9823"),
        name: "Bitvavo",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("079A892628EBf28d0Ed8f00151cff225A093dc63"),
        name: "Bitvavo_1",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("9d5DB4B86AfD9Fe80720cA0F5637d6B790CE5Bcb"),
        name: "Bitvavo_10",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("aB782bc7D4a2b306825de5a7730034F8F63ee1bC"),
        name: "Bitvavo_11",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("B2C9fFdbe4b4BBB6aA2951390506C39be6998751"),
        name: "Bitvavo_12",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("c15dc0B4223b05588A257E48Ed129Fb59C6eB7E3"),
        name: "Bitvavo_13",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("c8E0EF05B7f250fc63702fBdaDA6972aFDCae8C2"),
        name: "Bitvavo_14",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("Cc9Febe81EaBC8e1B0dcC29dEc2bdc70DEF9180f"),
        name: "Bitvavo_15",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("ccF8594f833FE5eBBF0e72FDa51F9fb291b39A57"),
        name: "Bitvavo_16",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("d2674dA94285660c9b2353131bef2d8211369A4B"),
        name: "Bitvavo_17",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("dd25cA203C060213Eaffe59E37101a01dAC463c1"),
        name: "Bitvavo_18",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("dF07BCb647Ca70Bd8e78b51d898777426c6d121A"),
        name: "Bitvavo_19",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("1A1c87d9A6F55D3BbB064bfF1059ad37B6Bdc097"),
        name: "Bitvavo_2",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("edC6BacdC1e29D7c5FA6f6ECA6FDD447B9C487c9"),
        name: "Bitvavo_20",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("20840B20f1eEE3B7b225866Dc2e0d669CC2553fb"),
        name: "Bitvavo_3",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("2586Db422EC15d5999b50C43bE219756C46Bb0Bd"),
        name: "Bitvavo_4",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("2f1752e8B49C6E7391Dc6e55934ceedC7AbF18F7"),
        name: "Bitvavo_5",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("75f594083de65FF173Eb7CE505aa26332fe389f5"),
        name: "Bitvavo_6",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("87eAB4F119113e0E5920288936aD94C307b4849a"),
        name: "Bitvavo_7",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("897321a0Ee56b57ef56F284037FFE4B63287D52A"),
        name: "Bitvavo_8",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("95B564F3B3BaE3f206aa418667bA000AFAFAcc8a"),
        name: "Bitvavo_9",
        exchange: "Bitvavo",
    },
    CexAddress {
        address: address!("fb9F7F41319157ac5C5dccaE308A63a4337ad5d9"),
        name: "Bity",
        exchange: "Bity",
    },
    CexAddress {
        address: address!("00cdC153Aa8894D08207719Fe921FfF964f28Ba3"),
        name: "Bitzlato",
        exchange: "Bitzlato",
    },
    CexAddress {
        address: address!("d1b7cCa582578bFF37faab547a4D50FF6342c6D0"),
        name: "Bitzlato_1",
        exchange: "Bitzlato",
    },
    CexAddress {
        address: address!("04046027549f739eDFd5b2a78EfDBAf0F0bf4514"),
        name: "BlockFi",
        exchange: "BlockFi",
    },
    CexAddress {
        address: address!("22FFDA6813f4F34C520bf36E5Ea01167bC9DF159"),
        name: "BlockFi_1",
        exchange: "BlockFi",
    },
    CexAddress {
        address: address!("2A549b4AF9Ec39B03142DA6dC32221fC390B5533"),
        name: "BlockFi_2",
        exchange: "BlockFi",
    },
    CexAddress {
        address: address!("3FDA25F27211a138ADF211F4C060f2149674Be6D"),
        name: "BlockFi_3",
        exchange: "BlockFi",
    },
    CexAddress {
        address: address!("530e0A6993eA99ffc96615aF43f327225a5fe536"),
        name: "BlockFi_4",
        exchange: "BlockFi",
    },
    CexAddress {
        address: address!("808b4dA0Be6c9512E948521452227EFc619BeA52"),
        name: "BlockFi_5",
        exchange: "BlockFi",
    },
    CexAddress {
        address: address!("A26A8e242A7470476DF1dc11dEd5DBc9FCa610fA"),
        name: "BlockFi_6",
        exchange: "BlockFi",
    },
    CexAddress {
        address: address!("007174732705604bBbf77038332Dc52FD5A5000C"),
        name: "BlockTrades",
        exchange: "BlockTrades",
    },
    CexAddress {
        address: address!("23f4569002a5A07f0Ecf688142eEB6bcD883eeF8"),
        name: "Blockchain.com",
        exchange: "Blockchain.com",
    },
    CexAddress {
        address: address!("46E0813Dcb480517e3E73449761BDc596e424A79"),
        name: "Blockchain.com_1",
        exchange: "Blockchain.com",
    },
    CexAddress {
        address: address!("52749D8c2f70f7ea6343054b7a04FB16337e5F58"),
        name: "Blockchain.com_2",
        exchange: "Blockchain.com",
    },
    CexAddress {
        address: address!("9AA65464b4cFbe3Dc2BDB3dF412AeE2B3De86687"),
        name: "Blockchain.com_3",
        exchange: "Blockchain.com",
    },
    CexAddress {
        address: address!("A00E2A7652248AbEb209398227DAE413E9479e52"),
        name: "Blockchain.com_4",
        exchange: "Blockchain.com",
    },
    CexAddress {
        address: address!("C88F7666330b4b511358b7742dC2a3234710e7B1"),
        name: "Blockchain.com_5",
        exchange: "Blockchain.com",
    },
    CexAddress {
        address: address!("fC3E21f959551512D68b6b00F8931593D3106151"),
        name: "Blockchain.com_6",
        exchange: "Blockchain.com",
    },
    CexAddress {
        address: address!("DF8752caa319668006580dDf48DB25A23728b926"),
        name: "Bololex.com",
        exchange: "Bololex.com",
    },
    CexAddress {
        address: address!("16769E533352798deB664bA570230A758346Ca1A"),
        name: "BtcTurk",
        exchange: "BtcTurk",
    },
    CexAddress {
        address: address!("1C17622cfa9B6fD2043A76DfC39A5B5a109aa708"),
        name: "BtcTurk_1",
        exchange: "BtcTurk",
    },
    CexAddress {
        address: address!("D2589c4061bF45a9a5212846AA72C2eD46377145"),
        name: "BtcTurk_10",
        exchange: "BtcTurk",
    },
    CexAddress {
        address: address!("dE0f5F79dffd1E8daA6051bd960AefB964D9845f"),
        name: "BtcTurk_11",
        exchange: "BtcTurk",
    },
    CexAddress {
        address: address!("fc3bFe0ECb87c1eea44e5338828149480e8D2B74"),
        name: "BtcTurk_12",
        exchange: "BtcTurk",
    },
    CexAddress {
        address: address!("2eE555C9006A9DC4674f01E0d4Dfc58e013708f0"),
        name: "BtcTurk_2",
        exchange: "BtcTurk",
    },
    CexAddress {
        address: address!("40E832C3Df9562DfaE5A86A4849F27F687A9B46B"),
        name: "BtcTurk_3",
        exchange: "BtcTurk",
    },
    CexAddress {
        address: address!("46f80018211D5cBBc988e853A8683501FCA4ee9b"),
        name: "BtcTurk_4",
        exchange: "BtcTurk",
    },
    CexAddress {
        address: address!("628a6fe299589B4b5A67d4B43CcDCB4821b3D80C"),
        name: "BtcTurk_5",
        exchange: "BtcTurk",
    },
    CexAddress {
        address: address!("832F166799A407275500430b61b622F0058f15d6"),
        name: "BtcTurk_6",
        exchange: "BtcTurk",
    },
    CexAddress {
        address: address!("8C54EbDD960056d2CfF5998df5695dACA1FC0190"),
        name: "BtcTurk_7",
        exchange: "BtcTurk",
    },
    CexAddress {
        address: address!("9FCaFcca8aec0367abB35fBd161c241f7b79891B"),
        name: "BtcTurk_8",
        exchange: "BtcTurk",
    },
    CexAddress {
        address: address!("B02f1329d6a6AcEF07a763258f8509c2847A0a3E"),
        name: "BtcTurk_9",
        exchange: "BtcTurk",
    },
    CexAddress {
        address: address!("6908C23476D3c8Aa59994662e95B40414B396bf0"),
        name: "Bullish",
        exchange: "Bullish",
    },
    CexAddress {
        address: address!("756D64Dc5eDb56740fC617628dC832DDBCfd373c"),
        name: "Bullish_1",
        exchange: "Bullish",
    },
    CexAddress {
        address: address!("a96853390C776ce8307B46B4D1a857Ab310Ab018"),
        name: "Bullish_2",
        exchange: "Bullish",
    },
    CexAddress {
        address: address!("C08FB884576cc89957e9058eF11587C468c2952F"),
        name: "Bullish_3",
        exchange: "Bullish",
    },
    CexAddress {
        address: address!("1e32760a3285550278aEAFA776E5641bC581C845"),
        name: "Bybit",
        exchange: "Bybit",
    },
    CexAddress {
        address: address!("2C7DAb4B02b77603eF16a7aeA8E30F137e8C1b93"),
        name: "Bybit_1",
        exchange: "Bybit",
    },
    CexAddress {
        address: address!("F5F3436A05B5CEd2490DAE07B86EB5BbD02782aA"),
        name: "Bybit_10",
        exchange: "Bybit",
    },
    CexAddress {
        address: address!("F65d698D18bC37bF36e4C8D4Fe4F051EF570e2B6"),
        name: "Bybit_11",
        exchange: "Bybit",
    },
    CexAddress {
        address: address!("f89d7b9c864f589bbF53a82105107622B35EaA40"),
        name: "Bybit_12",
        exchange: "Bybit",
    },
    CexAddress {
        address: address!("2deD5ce31a0C61eCaf6429A1ba1A00b2bFe67099"),
        name: "Bybit_2",
        exchange: "Bybit",
    },
    CexAddress {
        address: address!("3D5202A0564De9B05eCd07C955BcCA964585ea03"),
        name: "Bybit_3",
        exchange: "Bybit",
    },
    CexAddress {
        address: address!("4230C402c08cB66DCf3820649A115e54661FCe9D"),
        name: "Bybit_4",
        exchange: "Bybit",
    },
    CexAddress {
        address: address!("88a1493366D48225fc3cEFbdae9eBb23E323Ade3"),
        name: "Bybit_5",
        exchange: "Bybit",
    },
    CexAddress {
        address: address!("a95B83af96d0B8A90BD507f2Bd82aD8F3dbb86BC"),
        name: "Bybit_6",
        exchange: "Bybit",
    },
    CexAddress {
        address: address!("ab97925eB84fe0260779F58B7cb08d77dcB1ee2B"),
        name: "Bybit_7",
        exchange: "Bybit",
    },
    CexAddress {
        address: address!("BaeD383EDE0e5d9d72430661f3285DAa77E9439F"),
        name: "Bybit_8",
        exchange: "Bybit",
    },
    CexAddress {
        address: address!("ee5B5B923fFcE93A870B3104b7CA09c3db80047A"),
        name: "Bybit_9",
        exchange: "Bybit",
    },
    CexAddress {
        address: address!("4c8F52106D72dA5E090bc08Fb4d8063F3d591beb"),
        name: "C-Patex",
        exchange: "C-Patex",
    },
    CexAddress {
        address: address!("789ddCAC6A48A593CFcD957637bc1CEBD65eBAeb"),
        name: "C2CX",
        exchange: "C2CX",
    },
    CexAddress {
        address: address!("D7C866d0D536937bF9123E02F7C052446588189f"),
        name: "C2CX_1",
        exchange: "C2CX",
    },
    CexAddress {
        address: address!("1f973B233f5Ebb1E5D7CFe51B9aE4A32415A3A08"),
        name: "CEX.IO",
        exchange: "CEX.IO",
    },
    CexAddress {
        address: address!("c9f5296Eb3ac266c94568D790b6e91ebA7D76a11"),
        name: "CEX.IO_1",
        exchange: "CEX.IO",
    },
    CexAddress {
        address: address!("0D6B5A54F940BF3D52E438CaB785981aAeFDf40C"),
        name: "COSS Exchange",
        exchange: "COSS",
    },
    CexAddress {
        address: address!("38C939ECD144788a50aAE1dB7EB17D35Ffe16eb0"),
        name: "COSS Exchange_1",
        exchange: "COSS Exchange",
    },
    CexAddress {
        address: address!("43F07efe28E092A0fE4ec5B5662022B461fFac80"),
        name: "COSS Exchange_2",
        exchange: "COSS Exchange",
    },
    CexAddress {
        address: address!("6fa0a717c1073402a963E38ac8CB0d52C271b36E"),
        name: "COSS Exchange_3",
        exchange: "COSS Exchange",
    },
    CexAddress {
        address: address!("d1560b3984B7481CD9a8F40435a53C860187174d"),
        name: "COSS Exchange_4",
        exchange: "COSS Exchange",
    },
    CexAddress {
        address: address!("521dB06bF657Ed1D6C98553A70319a8DdBAc75A3"),
        name: "CREX24",
        exchange: "CREX24",
    },
    CexAddress {
        address: address!("a63fdc6684c9E454433Ceec20eFfcf9Fbc96BAfB"),
        name: "Calypso Exchange",
        exchange: "Calypso",
    },
    CexAddress {
        address: address!("4dC98C79A52968a6c20cE9A7A08d5e8D1C2D5605"),
        name: "CamboChanger",
        exchange: "CamboChanger",
    },
    CexAddress {
        address: address!("88988D6Ef12d7084e34814b9edafA01aE0D05082"),
        name: "CamboChanger_1",
        exchange: "CamboChanger",
    },
    CexAddress {
        address: address!("41A313bC923927A86a384c9128718300Fd75C34F"),
        name: "Cashierest",
        exchange: "Cashierest",
    },
    CexAddress {
        address: address!("6999603912eeB1B3f10A2058384DB5E109B4DF6F"),
        name: "Cashierest_1",
        exchange: "Cashierest",
    },
    CexAddress {
        address: address!("72BCFA6932FeACd91CB2Ea44b0731ed8Ae04d0d3"),
        name: "Cashierest_2",
        exchange: "Cashierest",
    },
    CexAddress {
        address: address!("7A56F645DCB513D0326CBAA048E9106fF6D4CD5f"),
        name: "Catex Exchange",
        exchange: "Catex",
    },
    CexAddress {
        address: address!("845437bD99dBE5595494077b6b0b6C6A16ec1878"),
        name: "Catex Exchange_1",
        exchange: "Catex Exchange",
    },
    CexAddress {
        address: address!("33e55B9bADc446f7FDDd8BDA1b5F33c8E3e82a2c"),
        name: "Ceffu",
        exchange: "Ceffu",
    },
    CexAddress {
        address: address!("3a3C006053a9B40286B9951A11bE4C5808c11dc8"),
        name: "Ceffu_1",
        exchange: "Ceffu",
    },
    CexAddress {
        address: address!("4Ed6Cf63bd9C009d247ee51224Fc1c7041f517F1"),
        name: "Ceffu_2",
        exchange: "Ceffu",
    },
    CexAddress {
        address: address!("756B86fD69914db92229C07c7393d7d9948fEd33"),
        name: "Ceffu_3",
        exchange: "Ceffu",
    },
    CexAddress {
        address: address!("84dBd7da7464B54d2F271b383106795d7dcd9069"),
        name: "Ceffu_4",
        exchange: "Ceffu",
    },
    CexAddress {
        address: address!("9f34A5fe4846d89dB84e30aF799459a691834cA0"),
        name: "Ceffu_5",
        exchange: "Ceffu",
    },
    CexAddress {
        address: address!("D3a22590f8243f8E83Ac230D1842C9Af0404C4A1"),
        name: "Ceffu_6",
        exchange: "Ceffu",
    },
    CexAddress {
        address: address!("d3Fdd98162902844342E7F8b59aB17CbD03BA36B"),
        name: "Ceffu_7",
        exchange: "Ceffu",
    },
    CexAddress {
        address: address!("e91268537B6dc2a43D8c81c4c4CAE0D5312B9C2c"),
        name: "Ceffu_8",
        exchange: "Ceffu",
    },
    CexAddress {
        address: address!("eA01C23b82F5B13A811ae49100E2b9008b1e6AEE"),
        name: "Ceffu_9",
        exchange: "Ceffu",
    },
    CexAddress {
        address: address!("06FC63b5C211aC29A9dA0cc24461581786163a67"),
        name: "Celsuis",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("0A9872e72b86C682032d3cAbEE8ab70e38DE7f35"),
        name: "Celsuis_1",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("44b68b196148e426402b3e9aB3bE974176Ad32C0"),
        name: "Celsuis_10",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("4f6742bADB049791CD9A37ea913f2BAC38d01279"),
        name: "Celsuis_11",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("5132d0a2fC15FBA4a9a64EA714854270BEc382FB"),
        name: "Celsuis_12",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("5670b3E44AdE4BD9D04A73fB88994E37A0d48f27"),
        name: "Celsuis_13",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("59A7F0779829Bf51DbB1EcD9dB873B6a2cF57F42"),
        name: "Celsuis_14",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("6101B69C06EFC3c1Ae68EbEa61AA1c746FE628D7"),
        name: "Celsuis_15",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("6A3528677e598B47952749b08469CE806C2524e7"),
        name: "Celsuis_16",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("6d9db9275C144533071FeA75866993Ef2AfcaAB7"),
        name: "Celsuis_17",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("6E216a1D8b19a505511a7f1B10084BAd57D31A48"),
        name: "Celsuis_18",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("752C8191E6b1Db38B41A8c8921F7a703F2969d18"),
        name: "Celsuis_19",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("0b6438F10FDae49E48815c4B5222B562789FB9F6"),
        name: "Celsuis_2",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("8d0EfDeE1cf48710F2E02d81660E2A6654d65a8D"),
        name: "Celsuis_20",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("99fa1561E63DDd3CB318ef6Cb306f04FA6E149b0"),
        name: "Celsuis_21",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("a16a857292228228D34ad99255f4CbDd3fcbbBF9"),
        name: "Celsuis_22",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("a8631Fa3B38C84C22B2e4B6E33997Bc7563A4baA"),
        name: "Celsuis_23",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("c602dc3fb4a966cd6AED233db2ae4a5E596fcC27"),
        name: "Celsuis_24",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("cb8BBFa45541a95C1de883eB3606708cAe9fd45C"),
        name: "Celsuis_25",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("D44DB45fff057C2e08c49FF046A3CC3A5D199B12"),
        name: "Celsuis_26",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("Db31651967684A40A05c4aB8Ec56FC32f060998d"),
        name: "Celsuis_27",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("DffFb7bA70eaBd7138D9c89e02d9e5A15D7fE096"),
        name: "Celsuis_28",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("Ef22c14F46858d5aC61326497b056974167F2eE1"),
        name: "Celsuis_29",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("11889C10CA33FBAbdbEB0C5Ffc016c8eE56f87F4"),
        name: "Celsuis_3",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("Ef8dc2b0005eeF85902C78f8E5040609d72Ea9C0"),
        name: "Celsuis_30",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("F03E41A34e97737ecF424e48127a21aE47945AA8"),
        name: "Celsuis_31",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("f0d54551a359D5b57B7035d847B5C8D8Eb374b73"),
        name: "Celsuis_32",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("F9D89Dc506c55738379C44Dc27205fD6f68e1974"),
        name: "Celsuis_33",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("1CeDC0f3Af8f9841B0a1F5c1a4DDc6e1a1629074"),
        name: "Celsuis_4",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("2156A3d636Ee80f437A21f6C41df8F14c39aDb19"),
        name: "Celsuis_5",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("2339a732DfA3dB16b3fFB550a42b0fbCbe2435D5"),
        name: "Celsuis_6",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("3473779Fd4D366774fE7D2Ceb089B30d94D7F1d1"),
        name: "Celsuis_7",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("41318419CFa25396b47A94896FfA2C77c6434040"),
        name: "Celsuis_8",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("4135F67bE29CE9EF8Bc9b2f37f3c466304902F6C"),
        name: "Celsuis_9",
        exchange: "Celsuis",
    },
    CexAddress {
        address: address!("0e747EB2ff0F26fB77c3a1eA67EE07FAc2DbB783"),
        name: "ChainUp",
        exchange: "ChainUp",
    },
    CexAddress {
        address: address!("10349DaaE3D75bABBC00B4Ec70416590DEdaA7e8"),
        name: "ChainUp_1",
        exchange: "ChainUp",
    },
    CexAddress {
        address: address!("b563C72D3b5514Fa098F9a588523915469DCFD88"),
        name: "ChainUp_10",
        exchange: "ChainUp",
    },
    CexAddress {
        address: address!("E17338Bd00Bf71D16953D4A00f1e02c42823F25C"),
        name: "ChainUp_11",
        exchange: "ChainUp",
    },
    CexAddress {
        address: address!("E66b1c7EDE325133e51346b7ee4814c7831F2542"),
        name: "ChainUp_12",
        exchange: "ChainUp",
    },
    CexAddress {
        address: address!("10EE46C16BE0fcfD506F3eed301254e4bc434FF1"),
        name: "ChainUp_2",
        exchange: "ChainUp",
    },
    CexAddress {
        address: address!("1cD9eF04c7833f4e3182Bf49FB806F75bB674152"),
        name: "ChainUp_3",
        exchange: "ChainUp",
    },
    CexAddress {
        address: address!("1f8F16a29251fA399D89e1005E3f95427Bf5B1dE"),
        name: "ChainUp_4",
        exchange: "ChainUp",
    },
    CexAddress {
        address: address!("49EC29B54acc0F85595C2F9bDbe58fb856094665"),
        name: "ChainUp_5",
        exchange: "ChainUp",
    },
    CexAddress {
        address: address!("5f77a4C9F962349ac2AAEC4459594031d1829AF8"),
        name: "ChainUp_6",
        exchange: "ChainUp",
    },
    CexAddress {
        address: address!("6b45921E693AC503a03f5E46f793Aff877698Fdc"),
        name: "ChainUp_7",
        exchange: "ChainUp",
    },
    CexAddress {
        address: address!("7f92c0eB5765FD0a97B9F227ED183c7397234FaD"),
        name: "ChainUp_8",
        exchange: "ChainUp",
    },
    CexAddress {
        address: address!("ae7FDD687bc6C802B55FaA373b468F3Fb6620a06"),
        name: "ChainUp_9",
        exchange: "ChainUp",
    },
    CexAddress {
        address: address!("fd648cC72F1b4E71CbDDa7A0a91Fe34D32abD656"),
        name: "ChainX",
        exchange: "ChainX",
    },
    CexAddress {
        address: address!("077D360f11D220E4d5D831430c81C26c9be7C4A4"),
        name: "ChangeNOW",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("0a1cE4496471867Fac0Ad71b785E5258993C9b33"),
        name: "ChangeNOW_1",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("754993C330E8aE2ce276a9c792B40047BA9f8b1f"),
        name: "ChangeNOW_10",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("7a3BEa333246efcD74ebf5835987a5398Eac10fE"),
        name: "ChangeNOW_11",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("8f54972F4Ca40bD3ffC8b085f6Ece1739C40c65f"),
        name: "ChangeNOW_12",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("975d9Bd9928F398c7E01F6ba236816Fa558cD94B"),
        name: "ChangeNOW_13",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("9bc2f223026c252c8ef5f7f33F00F4bEE21434B8"),
        name: "ChangeNOW_14",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("a12e1462d0ceD572f396F58B6E2D03894cD7C8a4"),
        name: "ChangeNOW_15",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("A4e5961B58DBE487639929643dCB1Dc3848dAF5E"),
        name: "ChangeNOW_16",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("A6ba490e1aF9849B6220aB9F709A32f7A82afaD6"),
        name: "ChangeNOW_17",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("A96Be652A08D9905F15B7FbE2255708709BeCD09"),
        name: "ChangeNOW_18",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("Ba1955c198FcA20834340414F0Ea951452BDC7DE"),
        name: "ChangeNOW_19",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("3421230289980b8EA81781B170Ef7d475673102B"),
        name: "ChangeNOW_2",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("Bac051BBF79C5321c0f825ea9BCA71f992144029"),
        name: "ChangeNOW_20",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("be6439B25E2C6560590407731bB9FE2908F30c94"),
        name: "ChangeNOW_21",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("c275119660Fefe4519083Ea6e57CBd1B672bc020"),
        name: "ChangeNOW_22",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("D421bB540B5f980a9f7D4206e1404C107AAD9ecc"),
        name: "ChangeNOW_23",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("D5b73fC035d4d679234323e0d891cAB4A4f5a1Ab"),
        name: "ChangeNOW_24",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("d98CfE4A2B9FCE9b884D2ec3698E775ee54753AF"),
        name: "ChangeNOW_25",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("D9f2C88Ae7372E7E418E5304105b918D6CF2CE1f"),
        name: "ChangeNOW_26",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("3525d3a883F743CA146288c146dE7CCD59d48BF5"),
        name: "ChangeNOW_3",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("3A0d24d59Af3a3444dc6Ef12cdb0c6e38C985288"),
        name: "ChangeNOW_4",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("4657F866a0D9B46f288893FFF466e8d87C556B1a"),
        name: "ChangeNOW_5",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("48c04ed5691981C42154C6167398f95e8f38a7fF"),
        name: "ChangeNOW_6",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("637c86B5F2C8399716964C702BcAEABC69B900dF"),
        name: "ChangeNOW_7",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("6ba7Fe01EeC4C6A4308C8E2B35970e0488ce9a86"),
        name: "ChangeNOW_8",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("6cb399D73cA29859d795c68A68F0DBF6a74F55D4"),
        name: "ChangeNOW_9",
        exchange: "ChangeNOW",
    },
    CexAddress {
        address: address!("96fC4553a00C117C5b0bED950Dd625d1c16Dc894"),
        name: "Changelly",
        exchange: "Changelly",
    },
    CexAddress {
        address: address!("0BB9Fc3Ba7BCF6e5d6F6fC15123ff8d5F96cEE00"),
        name: "Cobinhood",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("18a2111eB97884caE46BEeb560b3e67fB0978773"),
        name: "Cobinhood_1",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("87a42198419c2381fE19d9028D794608B8e19A93"),
        name: "Cobinhood_10",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("8958618332dF62AF93053cb9c535e26462c959B0"),
        name: "Cobinhood_11",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("9A1a3e23E9c781d9f4A549D390f120C76751C245"),
        name: "Cobinhood_12",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("a2daa486Bb03c0eb966133345c8AbA89bf114E50"),
        name: "Cobinhood_13",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("A7E53B8De2620a8d0072b06eCa271Fd4EbD8D262"),
        name: "Cobinhood_14",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("b2d0B80dCeca962766C742630305066F700dc074"),
        name: "Cobinhood_15",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("B726dA4fbdc3E4dBda97bb20998cF899b0e727E0"),
        name: "Cobinhood_16",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("C5663294f16d3bEf912bA06a62AeC73C32d1bEe4"),
        name: "Cobinhood_17",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("2B58521c4493210C172862D22E630537B3B63302"),
        name: "Cobinhood_2",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("2e6B57F24B7c0b1d4De8bC514B89234432d60221"),
        name: "Cobinhood_3",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("3FC4163546e64b5c98eb18c2E6A35d84a601590f"),
        name: "Cobinhood_4",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("41Fced841a2c4dBAf9B31a2AAeeaEC2C5fb655B3"),
        name: "Cobinhood_5",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("4886d2E96aBB131083B4072D9d102e77ffCFf74b"),
        name: "Cobinhood_6",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("69e9Bf9b1e5A725F5910EaD276Dc6932f687dCD0"),
        name: "Cobinhood_7",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("7917aF9E9D49504956108d86537Aa3dE61Fc9045"),
        name: "Cobinhood_8",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("7a6fF2Ac9A80d2fA56587BBEc63B59BfAEA31Ae0"),
        name: "Cobinhood_9",
        exchange: "Cobinhood",
    },
    CexAddress {
        address: address!("2487cb1A359c942312259BBc64a01CEe32E9f539"),
        name: "Cobo",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("39A80b830a4b77a56EF952df33CaabA70F27Fd5D"),
        name: "Cobo_1",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("B9711550ec6Dc977f26B73809A2D6791c0F0E9C8"),
        name: "Cobo_10",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("BF957e1c121FA769580D29bF320Ee8BfF138Ad12"),
        name: "Cobo_11",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("DAC967C10444267EE2e30De4D94AD997AdA96Be6"),
        name: "Cobo_12",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("F1300cc9c2Cf347f7902742fEC4dF9dbA952fD7b"),
        name: "Cobo_13",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("5733edE0F7109acf4565336b59E530272446f470"),
        name: "Cobo_2",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("6B365AF8d060E7F7989985D62485357E34e2e8f5"),
        name: "Cobo_3",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("7a843e6ff730CB48Ac94fD88235d393a41d23c5b"),
        name: "Cobo_4",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("7EF2D7B88D43F1831241F0dD63E0bdeF048Ba8aC"),
        name: "Cobo_5",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("A443320A40fdd0e8B3c21c94ed4363Efb7621679"),
        name: "Cobo_6",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("A9C7d31BB1879BfF8BE25EaD2F59B310a52b7c5a"),
        name: "Cobo_7",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("Af850683a03Af068d2870bFD1C7852980e754ee8"),
        name: "Cobo_8",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("B8001C3eC9AA1985f6c747E25c28324E4A361ec1"),
        name: "Cobo_9",
        exchange: "Cobo",
    },
    CexAddress {
        address: address!("0a33Fa8D5037940C33BEA55dF21e3A101F16F784"),
        name: "CoinBene",
        exchange: "CoinBene",
    },
    CexAddress {
        address: address!("194A0d4139Bd1eA6f471dD6B7c3a241479292AC6"),
        name: "CoinBene_1",
        exchange: "CoinBene",
    },
    CexAddress {
        address: address!("2E771b31E7A2DA659F5eC05E3f5bFF2Eeb382aff"),
        name: "CoinBene_2",
        exchange: "CoinBene",
    },
    CexAddress {
        address: address!("33683b94334eeBc9BD3EA85DDBDA4a86Fb461405"),
        name: "CoinBene_3",
        exchange: "CoinBene",
    },
    CexAddress {
        address: address!("3788539703c1e469fE0EB408095E97B0C247042a"),
        name: "CoinBene_4",
        exchange: "CoinBene",
    },
    CexAddress {
        address: address!("6A3eB79E1C4023f1610FF046C5dc30f9790d326f"),
        name: "CoinBene_5",
        exchange: "CoinBene",
    },
    CexAddress {
        address: address!("9539e0b14021a43cDE41d9d45Dc34969bE9c7cb0"),
        name: "CoinBene_6",
        exchange: "CoinBene",
    },
    CexAddress {
        address: address!("Ee57141075EA3FFFC41F06AF2cEba23e521019a8"),
        name: "CoinBene_7",
        exchange: "CoinBene",
    },
    CexAddress {
        address: address!("06051836ac6C5112b890f8b6Ec78e33D1AfeaE7c"),
        name: "CoinDCX",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("07E114C06462D8892Ae4574A7502b8c1c0FBdFbb"),
        name: "CoinDCX_1",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("3698cc7F524BAde1a05e02910538F436a3E94384"),
        name: "CoinDCX_10",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("37b6bD5fECE5b88B6E8e825196bcc868a2FeEd51"),
        name: "CoinDCX_11",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("38f76d1C8fcC854fb4d2416dDAeC8Df41Ab60867"),
        name: "CoinDCX_12",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("44A69D8443521969FFDC8a0721AD1013582d7097"),
        name: "CoinDCX_13",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("4928B328d4261120a493dfdb32B6d4Cc57D58850"),
        name: "CoinDCX_14",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("4D24EecEcb86041F47bca41265319e9f06aE2Fcb"),
        name: "CoinDCX_15",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("50B0063161e507bEc6c21cC23FD11EC2945b7b52"),
        name: "CoinDCX_16",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("59548449926551ACb5480d610292c4266b872beF"),
        name: "CoinDCX_17",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("5fED0d9bE6FD42e086E0D4F1bF6ceCd19635dEAd"),
        name: "CoinDCX_18",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("660e3Bd3bcDa11538fa331282666F1d001b87A42"),
        name: "CoinDCX_19",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("0B5ffAE844a4B21c7beeDed2595F22215288173C"),
        name: "CoinDCX_2",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("6D92f2E52481ce219D201EaA1b0Cf6839270152F"),
        name: "CoinDCX_20",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("7157Bb38613c5362e59dFd769C2Fcf4996B4cc8b"),
        name: "CoinDCX_21",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("763104507945B6b7f21Ee68b92048A53F7debF18"),
        name: "CoinDCX_22",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("78bba2389c2cEEb6f94C70eD133712E3B3e2C4D0"),
        name: "CoinDCX_23",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("7cCCf66DB1A0d4069a52A0C26859EDbC34177065"),
        name: "CoinDCX_24",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("881f982575a3EcBEA6fe133ddB0951303215d130"),
        name: "CoinDCX_25",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("892787C947fdd1CF6C525C6107d80265D3D7EBb4"),
        name: "CoinDCX_26",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("8c7Efd5B04331EFC618e8006f19019A3Dc88973e"),
        name: "CoinDCX_27",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("90f76616d34Cb6A1F4423B33c0201B2A1980Fc81"),
        name: "CoinDCX_28",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("978E746330870627CC353092eDd2dBA3Cc99461c"),
        name: "CoinDCX_29",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("1CE0c2827e2eF14D5C4f29a091d735A204794041"),
        name: "CoinDCX_3",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("A15B94629727152c952a6979d899F71426cE7976"),
        name: "CoinDCX_30",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("A4FE2F90a8991A410c825C983CbB6A92d03607fc"),
        name: "CoinDCX_31",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("A916a54af7553BAe6172e510D067826Bd204d0dD"),
        name: "CoinDCX_32",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("AA8bC1fc0FCfdcA5b7E5D35e5AC13800850d90C7"),
        name: "CoinDCX_33",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("Abd9193388C58D7e9f45fF5E4ca741212d1Ec827"),
        name: "CoinDCX_34",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("Ada1fA671651D335998c6a0dE336a78F5b49Ad3F"),
        name: "CoinDCX_35",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("b188a49Da0836c289dcB4Fa0E856647a33DE537F"),
        name: "CoinDCX_36",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("b6DFCF39503dddDe140105954a819e944CE543A7"),
        name: "CoinDCX_37",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("b79421720b92180487f71F13c5D5D8B9ecA27BF1"),
        name: "CoinDCX_38",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("b85E9868a0E8492353Db5C3022e6F96fc62F2306"),
        name: "CoinDCX_39",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("2407b9B9662d970ecE2224A0403D3B15c7e4D1FE"),
        name: "CoinDCX_4",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("C1723Af0Dc5400A1cAAa47E76a45c39538A6AD49"),
        name: "CoinDCX_40",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("C4e805208D8d25b71BDcfB558F29259544EC4fcC"),
        name: "CoinDCX_41",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("CCFA6f3b01c7bf07B033A9d496Fdf22F0cdF5293"),
        name: "CoinDCX_42",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("D4D7Aedb9AbeEC03101dB6f8426DeeE390E3cCF9"),
        name: "CoinDCX_43",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("Df263Bd241B694b1Fbe30eE954757ddDEd7D95e6"),
        name: "CoinDCX_44",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("e298dC1c377e4511f32Afd2362726c4F3A644356"),
        name: "CoinDCX_45",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("EF0Fc6322b2b5b02f0Db68f8eA74819560124b2d"),
        name: "CoinDCX_46",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("f031356Fa1bD3949420884db1a35A99422E36df1"),
        name: "CoinDCX_47",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("f250EE103b8e4C5b9825a270e5c70Ea0C113c854"),
        name: "CoinDCX_48",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("F25d1D2507ce1f956F5BAb45aD2341e3c0DB6d3C"),
        name: "CoinDCX_49",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("274c427B1BF0bB4a137EDE688c6D621263CA7Ce8"),
        name: "CoinDCX_5",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("F379FcD9C996d85de025985bA9B1C9C96DAa4a72"),
        name: "CoinDCX_50",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("f809c975eFAD2Bc33E21B5972DB765A6230E956A"),
        name: "CoinDCX_51",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("29A62a542b6EA441abB6F03C2bca54aD72fF750C"),
        name: "CoinDCX_6",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("2DEFfdde0867B6EcA9a63fE53ef0FC3106d3DBa0"),
        name: "CoinDCX_7",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("2e5129e77c928D96b5A70c0effB97Ee6e95D77b6"),
        name: "CoinDCX_8",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("35efE40CeDdb8DfA27F0c3e4cd65B711C29CB5D8"),
        name: "CoinDCX_9",
        exchange: "CoinDCX",
    },
    CexAddress {
        address: address!("bf1a97D8D4229d61B031214d5BbE9a5cB1e737f9"),
        name: "CoinDhan",
        exchange: "CoinDhan",
    },
    CexAddress {
        address: address!("93f36930F94FBB5aFc5fB506D3f7ABB9179a4e4e"),
        name: "CoinEgg",
        exchange: "CoinEgg",
    },
    CexAddress {
        address: address!("187E3534f461d7C59a7d6899a983A5305b48f93F"),
        name: "CoinEx",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("1e450c2A1870A52606eDd37ac0bF593dca9C1c3F"),
        name: "CoinEx_1",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("89F2ab029DcD11bD5A00Ed6A77ccBE46315212e8"),
        name: "CoinEx_10",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("8dA2931006453e0285682314af5bcd2377deaEB1"),
        name: "CoinEx_11",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("90f86774e792e91cf81B2Ff9F341EfcA649343A6"),
        name: "CoinEx_12",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("AFedF06777839D59eED3163cC3e0A5057b514399"),
        name: "CoinEx_13",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("B55270a75D548de1f7b89eA571438E9745465644"),
        name: "CoinEx_14",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("B85bd2A1D76586BB8F81494BFA061DfBF803b4db"),
        name: "CoinEx_15",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("b9ee1e551f538A464E8F8C41E9904498505B49b0"),
        name: "CoinEx_16",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("cF7105FDE573695D62666aeFa4e1691bf3Ab50d5"),
        name: "CoinEx_17",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("D782E53A49D564F5fce4bA99555DD25d16d02a75"),
        name: "CoinEx_18",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("DA07F1603a1c514b2F4362F3eae7224A9CDEfAF9"),
        name: "CoinEx_19",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("3339Afcb8e237aA8AaF3e1040473173830EfD09C"),
        name: "CoinEx_2",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("E70b8dc28E795738A772379E9D456E7d74f50aB5"),
        name: "CoinEx_20",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("f54635836862aAD6e255E9B4FE49275fA5047E5d"),
        name: "CoinEx_21",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("33Ddd548FE3a082d753E5fE721a26E1Ab43e3598"),
        name: "CoinEx_3",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("53Eb3Ea47643E87e8f25dd997A37B3b5260e7336"),
        name: "CoinEx_4",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("5Ad4D300FA795e9C2FE4221F0e64A983aCdBCaC9"),
        name: "CoinEx_5",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("5Cf44f2cB65aF7D56B30719312eCD13151A0470b"),
        name: "CoinEx_6",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("601A63C50448477310feDb826ED0295499bAf623"),
        name: "CoinEx_7",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("6fdbe6bEC0B63334c0dC26623Ca9C58dFc158c4e"),
        name: "CoinEx_8",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("85Cf05F35b6D542aC1d777d3F8cfde57578696fc"),
        name: "CoinEx_9",
        exchange: "CoinEx",
    },
    CexAddress {
        address: address!("4B01721F0244E7c5B5F63c20942850E447f5a5Ee"),
        name: "CoinExchange",
        exchange: "CoinExchange",
    },
    CexAddress {
        address: address!("01E79FF04a66CE034758172f8B422B0f8115921B"),
        name: "CoinFLEX",
        exchange: "CoinFLEX",
    },
    CexAddress {
        address: address!("1FA2157797a4Ef47DCb5cD4d50835F0c69F70Af2"),
        name: "CoinFLEX_1",
        exchange: "CoinFLEX",
    },
    CexAddress {
        address: address!("544cd22157aBF2d06c683B71A97B33FAAE65a791"),
        name: "CoinFLEX_2",
        exchange: "CoinFLEX",
    },
    CexAddress {
        address: address!("D56E9Af0243FFbbE4c6a801ad8f20a097D247122"),
        name: "CoinFLEX_3",
        exchange: "CoinFLEX",
    },
    CexAddress {
        address: address!("a1813f73448a7392e6f069299c23120aeEAb879A"),
        name: "CoinField",
        exchange: "CoinField",
    },
    CexAddress {
        address: address!("187c0E0aa33282096b39a33457939f1dC3Ea8e0f"),
        name: "CoinList",
        exchange: "CoinList",
    },
    CexAddress {
        address: address!("8D1f2eBFACCf1136dB76FDD1b86f1deDE2D23852"),
        name: "CoinList_1",
        exchange: "CoinList",
    },
    CexAddress {
        address: address!("D1669Ac6044269b59Fa12c5822439F609Ca54F41"),
        name: "CoinList_2",
        exchange: "CoinList",
    },
    CexAddress {
        address: address!("D2C82F2e5FA236E114A81173e375a73664610998"),
        name: "CoinList_3",
        exchange: "CoinList",
    },
    CexAddress {
        address: address!("003E36550908907c2a2dA960FD19A419B9A774b7"),
        name: "CoinPayments.net",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("005AD9b93955Bf76d180C66f33Fe84bD0f3310b5"),
        name: "CoinPayments.net_1",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("8AE0b6bb5ea5b8AB7C9322FCB0Dfcd8f7eFfd169"),
        name: "CoinPayments.net_10",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("a3fa59C716908290AB8611b33e33CD55876aEdDF"),
        name: "CoinPayments.net_11",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("C49688C286FA5FDC9f4DAA0f09F35861115a16f4"),
        name: "CoinPayments.net_12",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("cA15769b2b683F414bd4B95DFEF405101D883a4b"),
        name: "CoinPayments.net_13",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("ce3DaEbEC9A5cB4904bEb01C8998bbca6e96Ed9F"),
        name: "CoinPayments.net_14",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("e1B0F467BB4667BcAF77a147F784cFE1edffD6cb"),
        name: "CoinPayments.net_15",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("ffCd95D059bA265194CEc9b1FB6609bf2aA6B436"),
        name: "CoinPayments.net_16",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("0833B566dD4D3a2F55B8416E7Ca7e43921510885"),
        name: "CoinPayments.net_2",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("1a459813Db7758B0309b7785FEc854966685Ce3b"),
        name: "CoinPayments.net_3",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("278425B97f719bC8cFA1d8aA1bBf7520CA81610c"),
        name: "CoinPayments.net_4",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("31d3a108FAc60b0D5584226d9D2aa2294C637d76"),
        name: "CoinPayments.net_5",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("4685BDB11C75aDE469d6Aa294c332D4f60736EBB"),
        name: "CoinPayments.net_6",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("5A060db7cAc8786818B752EE3cc802548D86a30C"),
        name: "CoinPayments.net_7",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("632d17e6f450B0b89cf1D8C61E93b9e9a4b8608B"),
        name: "CoinPayments.net_8",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("7DE68A47A861A549Dbe2baE37884FC6c84D12026"),
        name: "CoinPayments.net_9",
        exchange: "CoinPayments.net",
    },
    CexAddress {
        address: address!("09363887A4096b142f3F6b58A7eeD2F1A0FF7343"),
        name: "CoinSpot",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("20312e96b1a0568Ac31C6630844a962383cc66c2"),
        name: "CoinSpot_1",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("91a0a3043f68986043D7083C4D85B558B21F0A7B"),
        name: "CoinSpot_10",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("9239dF3E9996c776D539EB9f01A8aE8E7957b3c3"),
        name: "CoinSpot_11",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("a89a1278Ac85367F38BDF6746658CE2B9875526E"),
        name: "CoinSpot_12",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("a9Bd318A4Ca1747E6068D100e18711B529386e29"),
        name: "CoinSpot_13",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("db6FDc30AB61C7cCA742D4c13D1b035F3F82019A"),
        name: "CoinSpot_14",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("DF1553A2130cbAFA70a35e68eFC6cCF67F0A278C"),
        name: "CoinSpot_15",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("E4b3dD9839ed1780351Dc5412925cf05F07A1939"),
        name: "CoinSpot_16",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("e6f79f8B46b30f293cfBDe50eF787d2Fe0610782"),
        name: "CoinSpot_17",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("eeD86B90448C371Eab47b7f16E294297C27E4F51"),
        name: "CoinSpot_18",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("f1088841Ab08FC1Cb3835fd75207bCB3137F6EE3"),
        name: "CoinSpot_19",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("32143A02Fb6484D18C79Fa0401c9bF760DD3DE68"),
        name: "CoinSpot_2",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("f35A6bD6E0459A4B53A27862c51A2A7292b383d1"),
        name: "CoinSpot_20",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("32E567E8B527d3194C60ea3C6a5c009d58A0B36d"),
        name: "CoinSpot_3",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("33A64dcDfa041bEfebC9161a3e0c6180cd94Fa89"),
        name: "CoinSpot_4",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("4207837D4Cd914467EB76bf88c4d6e7Ba11ccDf9"),
        name: "CoinSpot_5",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("56de1961fDA5454E6F8e6D0e3124fF648FD69400"),
        name: "CoinSpot_6",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("60F9e80D0D40b2958ac39006635dE782096866C3"),
        name: "CoinSpot_7",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("867bfA133D64fAd734C89f886D2A169B6504Ab2b"),
        name: "CoinSpot_8",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("916ED5586bB328E0eC1a428af060DC3D10919d84"),
        name: "CoinSpot_9",
        exchange: "CoinSpot",
    },
    CexAddress {
        address: address!("0E58a7652143a0A275750E5ac57a4Fb14eb4A18D"),
        name: "CoinTiger",
        exchange: "CoinTiger",
    },
    CexAddress {
        address: address!("5Dd697c77E80C7Fda3dA1cccd5214c3a75503727"),
        name: "CoinTiger_1",
        exchange: "CoinTiger",
    },
    CexAddress {
        address: address!("707C6F7d35798780CEFCE81B747392198566a186"),
        name: "CoinTiger_2",
        exchange: "CoinTiger",
    },
    CexAddress {
        address: address!("77f814F8ccba595B83c69E587b159E9887890639"),
        name: "CoinTiger_3",
        exchange: "CoinTiger",
    },
    CexAddress {
        address: address!("8A023F23d0DC363A74dc5CB480FB393cfef0CCB9"),
        name: "CoinTiger_4",
        exchange: "CoinTiger",
    },
    CexAddress {
        address: address!("97d31bF5271052888863791FeD990c17cE656B95"),
        name: "CoinTiger_5",
        exchange: "CoinTiger",
    },
    CexAddress {
        address: address!("BCB301999bB1FA44A9181Fbe5E279218B02Aae32"),
        name: "CoinTiger_6",
        exchange: "CoinTiger",
    },
    CexAddress {
        address: address!("e9d2AFFF18F08375D5B7a8A804e272b6c81Ceb9F"),
        name: "CoinTiger_7",
        exchange: "CoinTiger",
    },
    CexAddress {
        address: address!("fBdEB87969F5610d006d1a4ed79308A5778E77E5"),
        name: "CoinTiger_8",
        exchange: "CoinTiger",
    },
    CexAddress {
        address: address!("092738E25c5d132e235e8bd4451d180e86dA6736"),
        name: "CoinW",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("204828c049AaBC79405510bd287E066A64c1f307"),
        name: "CoinW_1",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("639d9BA0f11Ff73A25c0a26849D4d5A7175169b6"),
        name: "CoinW_10",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("64919eB2823e0dcCb3bD29D24B7d4bc6ab992f0c"),
        name: "CoinW_11",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("67EC7eA2dE759458B9A7f28956618bA841C9Bf9f"),
        name: "CoinW_12",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("696CFd63F98DCd1FeA2d6BD27f3CF85C2007a2FD"),
        name: "CoinW_13",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("6A9F6ab41A90D59fC5Ef0D373559F39002c22a0f"),
        name: "CoinW_14",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("6f31D347457962c9811ff953742870EF5a755dE3"),
        name: "CoinW_15",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("7009407B417759C34a35d63530D30A818661B857"),
        name: "CoinW_16",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("80E29AcB842498fE6591F020bd82766DCe619D43"),
        name: "CoinW_17",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("83a0B03E76c88052cd7989edeaB80aA4cACc66b5"),
        name: "CoinW_18",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("8705CcFd8A6dF3785217C307cbEbf9b793310B94"),
        name: "CoinW_19",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("2aE40EeAf27a28D31eAbB050dAC2e0568c7B1a7E"),
        name: "CoinW_2",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("87B748aC0a8eAe308882F13522fa81f384588Cb1"),
        name: "CoinW_20",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("940Bdc6844d9b801c60a775397f2945b97a6E1C6"),
        name: "CoinW_21",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("9998d99daAE38B87F0f43c40974720A0B3D3D95C"),
        name: "CoinW_22",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("9f8646A35db0f466aC9322e2D194cc18f209Fc75"),
        name: "CoinW_23",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("a20f10289248717374e9B7776dC368aa526cb6F2"),
        name: "CoinW_24",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("a75bae74E39DC2f38eE879eC3De9489ABE65280e"),
        name: "CoinW_25",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("ab59487C43211da1d8B8e60479CAd6aADC52cd1c"),
        name: "CoinW_26",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("B840fe2b3fd8F75275240c671d6EC659e4c9a500"),
        name: "CoinW_27",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("BEf77b5f2B7434333b13f6441cB88866e07ECa2D"),
        name: "CoinW_28",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("bf2d58698a8A215F868CF24BAba360c77266b466"),
        name: "CoinW_29",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("2Ce80f8d81fa594c4dBc1374A974d277b1188598"),
        name: "CoinW_3",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("C4e08dd8Dfc72cDBB61bc9Cfa82F8738F3ab6D05"),
        name: "CoinW_30",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("cb243bf48FB443082FAE7db47eC96Cb120Cd6801"),
        name: "CoinW_31",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("DC8f0f2Aef01EB3a11b7F73B087A2F6A2eE3067f"),
        name: "CoinW_32",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("Dcc06BB2a55169a897E8b4148f151D7574EfF66C"),
        name: "CoinW_33",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("e3Be8206E74773Dbc4902480344ae1F94323170b"),
        name: "CoinW_34",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("E48A4E20BE4EA888748c56BDCb632D960Cbfb011"),
        name: "CoinW_35",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("eCAB44B10b8faEE48d7eEe03F81a815078070624"),
        name: "CoinW_36",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("F110c5f6e04Eea18e415C5753a43479851F27530"),
        name: "CoinW_37",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("F27e7FA401B991A593A9E83AA6f23f2A212A9aC4"),
        name: "CoinW_38",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("F6E9D98134194fcc78bBF5908491fDd81a6Ee13D"),
        name: "CoinW_39",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("2D6323cc438B96F0aE942280762Cc507B5398563"),
        name: "CoinW_4",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("3864D8F360bA98212A2edDF05A357599F25196c1"),
        name: "CoinW_5",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("3974D816A969908A82428384C728fbd8fAFfbFe1"),
        name: "CoinW_6",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("429Bf8EC3330E02401D72bEadE86000d9a2E19EB"),
        name: "CoinW_7",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("46499f2dff9f551bC71646a0448396B1C4702343"),
        name: "CoinW_8",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("611F32E5d7F6640ecAf3e66759318aBB9CbEce64"),
        name: "CoinW_9",
        exchange: "CoinW",
    },
    CexAddress {
        address: address!("00aac037C2cA137972F963693d38A57d0E9f7475"),
        name: "Coinbase",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("1565f0c48c06bC006095591A4c3FE4A6F39712cF"),
        name: "Coinbase Prime",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("02466E547BFDAb679fC49e96bBfc62B9747D997C"),
        name: "Coinbase_1",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("14AF92363379f3548958f9de1fb2e6E5DF74476e"),
        name: "Coinbase_10",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("9DCaF485e72f812DEe725Ce64B6667a7A83f73Dd"),
        name: "Coinbase_100",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("9eBE8AE7DbC0285B04E93dAB86a081cA32ccf52e"),
        name: "Coinbase_101",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("A090e606E30bD747d4E6245a1517EbE430F0057e"),
        name: "Coinbase_102",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("A14D57f5Ea867572b0d239798D2C1Dde13153902"),
        name: "Coinbase_103",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("a2908F1758d1cC3990F4A2dA8DEA0aA2ecf1b913"),
        name: "Coinbase_104",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("A2B57dD51c464E863a5EFc70C8116eC46791e38f"),
        name: "Coinbase_105",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("a3682Fe8fD73B90A7564585A436EC2D2AEb612eE"),
        name: "Coinbase_106",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("a656f7d2A93A6F5878AA768f24eB38Ec8C827fE2"),
        name: "Coinbase_107",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("A7927364E94E162102E7cF2447e33E01fc629e68"),
        name: "Coinbase_108",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("a818c01C2fdf04f24f8D6BDa3512e901F33a575e"),
        name: "Coinbase_109",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("1553767e6Ab6d26695B34366f61340B48d8b7a62"),
        name: "Coinbase_11",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("A9D1e08C7793af67e9d92fe308d5697FB81d3E43"),
        name: "Coinbase_110",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("adBbE373B5b5F72C59c0311cFfBded51f0C5F434"),
        name: "Coinbase_111",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("b0fa34C866e1e1E7030820B4f846BB58d6F75b04"),
        name: "Coinbase_112",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("b35D425a3F49c49181eF8e72583efB070B54202E"),
        name: "Coinbase_113",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("B4807865A786E9E9E26E6A9610F2078e7fc507fB"),
        name: "Coinbase_114",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("b5d85CBf7cB3EE0D56b3bB207D5Fc4B82f43F511"),
        name: "Coinbase_115",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("B624219480543C54603fb6b07d5eb347E51bffe0"),
        name: "Coinbase_116",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("b739D0895772DBB71A89A3754A160269068f0D45"),
        name: "Coinbase_117",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("b8487eeD31Cf5C559BF3f4eDD166b949553D0d11"),
        name: "Coinbase_118",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("Bc8Ec259E3026aE0D87bc442D034d6882ce4a35C"),
        name: "Coinbase_119",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("1846DeB34cc19c11e7DdF0de48B13FD1F231Ca7F"),
        name: "Coinbase_12",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("BE3c68821D585Cf1552214897a1c091014B1EB0a"),
        name: "Coinbase_120",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("C070A61D043189D99bbf4baA58226bf0991c7b11"),
        name: "Coinbase_121",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("c7bf35C9A3Bdd1B1c19A6963De669cb45191A019"),
        name: "Coinbase_122",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("C8373EDFaD6d5C5f600b6b2507F78431C5271fF5"),
        name: "Coinbase_123",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("c9AAA6cA0e05B87d53A3E51Edbc44b406EEaF299"),
        name: "Coinbase_124",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("c9ebC59a7590E52B0904817f172AC82fc66a530b"),
        name: "Coinbase_125",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("cad50671877Eb564d42DdD76b5Fec46ac60EF1BD"),
        name: "Coinbase_126",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("Ce352e98934499be70F641353f16A47D9E1E3aBd"),
        name: "Coinbase_127",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("cF63Fc571aDceC4FD4A750ecACC3af1f5b748101"),
        name: "Coinbase_128",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("D34EA7278e6BD48DefE656bbE263aEf11101469c"),
        name: "Coinbase_129",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("1985EA6E9c68E1C272d8209f3B478AC2Fdb25c87"),
        name: "Coinbase_13",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("D451e3919950963e9C1CA2F78A987DBD7937C0FB"),
        name: "Coinbase_130",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("d5c41FD4a31Eaaf5559FfCC60Ec051fcB8eCC375"),
        name: "Coinbase_131",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("D688AEA8f7d450909AdE10C47FaA95707b0682d9"),
        name: "Coinbase_132",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("d6974E28eCee4005a4d03c4C6ED0Aac71fb46b94"),
        name: "Coinbase_133",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("D69B42d93aF96Cf278b1149bFfB2D668D0154B7d"),
        name: "Coinbase_134",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("D839C179a4606F46abD7A757f7Bb77D7593aE249"),
        name: "Coinbase_135",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("ddfAbCdc4D8FfC6d5beaf154f18B778f892A0740"),
        name: "Coinbase_136",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("dF61e3281FB5B1C3Da3fF92865875D7751aD830B"),
        name: "Coinbase_137",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("e04Cf52e9Fafa3D9bF14c407AFfF94165EF835f7"),
        name: "Coinbase_138",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("E0E65c40BCB1225D4aB3a13f3a9E21Abbd83F9d0"),
        name: "Coinbase_139",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("19aB546E77d0cD3245B2AAD46bd80dc4707d6307"),
        name: "Coinbase_14",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("e1597DF1F0E1920F7a296CeF27babB40BaEeabFC"),
        name: "Coinbase_140",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("E1A0DDeb9b5b55E489977b438764e60e314E917c"),
        name: "Coinbase_141",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("e3aaC971590635F601Ea751096f11343C70ebaDF"),
        name: "Coinbase_142",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("e3D6d8BCDC4Eb4e24aD7523D98C394960e8C1d32"),
        name: "Coinbase_143",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("e49916B3ff411fC6A83dC31f130E2e85Be4a9385"),
        name: "Coinbase_144",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("E68Ee8A12c611fd043fB05d65E1548dC1383f2b9"),
        name: "Coinbase_145",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("E7Ee701BdAA5b446C985BFeCC8933f3E5eeed867"),
        name: "Coinbase_146",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("E86F3aaA57F63B2AfeCA68178182a91bC3909962"),
        name: "Coinbase_147",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("eB2629a2734e272Bcc07BDA959863f316F4bD4Cf"),
        name: "Coinbase_148",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("EbA20D0f74ECc13130579d21bB53D63C96258652"),
        name: "Coinbase_149",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("19d599012788b991FF542F31208bAB21Ea38403E"),
        name: "Coinbase_15",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("EDc7001e99a37c3D23b5f7974F837387e09f9C93"),
        name: "Coinbase_150",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("ee81B5Afc73Cf528778E0ED98622e434E5eFADb4"),
        name: "Coinbase_151",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("F0eeFa46ee509eB8fAB42F62917db8a93432E2ef"),
        name: "Coinbase_152",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("F27182c5568beAFb967140286E25807250EacC4C"),
        name: "Coinbase_153",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("F27daFf52c38b2c373Ad2B9392652DdF433303c4"),
        name: "Coinbase_154",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("F2f07ef4a923F48fC4D47F0Af115F8895177F075"),
        name: "Coinbase_155",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("f491d040110384DBcf7F241fFE2A546513fD873d"),
        name: "Coinbase_156",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("FF9FBB429C186029c28f1e30361271EA002847AD"),
        name: "Coinbase_157",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("1b4C15b991543a6082213a88276e7a83c9985676"),
        name: "Coinbase_16",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("204ad5bed7eb66e002329A018BB96d87a8dB1AA8"),
        name: "Coinbase_17",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("20FE51A9229EEf2cF8Ad9E89d91CAb9312cF3b7A"),
        name: "Coinbase_18",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("21bD501F86A0B5cE0907651Df3368DA905B300A9"),
        name: "Coinbase_19",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("02d24cAB4f2c3Bf6e6EB07ea07e45F96baccFfE7"),
        name: "Coinbase_2",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("221cEc08C3DF34763eed468705DC779Cbe750ceb"),
        name: "Coinbase_20",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("251e93d51c5F2A1e60b7BC90BC8B2534b68e8f40"),
        name: "Coinbase_21",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("27724B0d4fb98A89a092E6a4ADbC09154c182637"),
        name: "Coinbase_22",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("281055Afc982d96fAB65b3a49cAc8b878184Cb16"),
        name: "Coinbase_23",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("28C5B0445d0728bc25f143f8EbA5C5539fAe151A"),
        name: "Coinbase_24",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("28E71d0b7f7f29106a1bE2A5B289cab331E7B56f"),
        name: "Coinbase_25",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("292BF41E2506f88Aa3E73721Aabc75B4f08e664e"),
        name: "Coinbase_26",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("2a410f11A6F520398447bF423DceDd25DFd3a568"),
        name: "Coinbase_27",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("2bDCDa44D935C12c3a76972C4339975782842a12"),
        name: "Coinbase_28",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("2cc5146929A893D1d73BC34Fb37815cC1a44ae33"),
        name: "Coinbase_29",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("04D4876932C7C375efcaEB7aE0AD00591acF09F6"),
        name: "Coinbase_3",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("333d17d3B42bf7930Dbc6e852cA7Bcf560A69003"),
        name: "Coinbase_30",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("336307F2d8390035Ba926a61a86b45CA9dC91E57"),
        name: "Coinbase_31",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("33AE106bc06EffA33e6dB5813b619710f2dfb5d4"),
        name: "Coinbase_32",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("38024b883C72F23D6d6c1f3f869034e849e8b121"),
        name: "Coinbase_33",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("382fFCe2287252F930E1C8DC9328dac5BF282bA1"),
        name: "Coinbase_34",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("3C070dcEaC7f99AeC493De5b56619D71fa31E131"),
        name: "Coinbase_35",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("3cD751E6b0078Be393132286c442345e5DC49699"),
        name: "Coinbase_36",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("3D2e397F94e415D7773E72e44D5B5338a99E77d9"),
        name: "Coinbase_37",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("3DC474a2A65507f32b05C5f80D852515B25b2134"),
        name: "Coinbase_38",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("3DD1D15b3c78d6aCFD75a254e857Cbe5b9fF0aF2"),
        name: "Coinbase_39",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("05e3a758FdD29d28435019ac453297eA37b61b62"),
        name: "Coinbase_4",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("3DD87411a3754deea8cc52C4CF57E2fC254924Cc"),
        name: "Coinbase_40",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("3f1137CF8e6468669959C3b99a9250A400a76574"),
        name: "Coinbase_41",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("40EbC1Ac8d4Fedd2E144b75fe9C0420BE82750c6"),
        name: "Coinbase_42",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("441CACfD43856409b163B90e094BB42aeb70a70e"),
        name: "Coinbase_43",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("46Ce00Ec0866e7A045D2F2778558efee05c4fB08"),
        name: "Coinbase_44",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("47D43AC7EeA761C4959B2a4318e620603bE7Fd4F"),
        name: "Coinbase_45",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("4a4E859565D9B563afC8e63641542455cff0dFD2"),
        name: "Coinbase_46",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("4a797A6fA930118b9BC267911751241B8c514fAA"),
        name: "Coinbase_47",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("4B23d52eFf7C67F5992C2aB6D3f69b13a6a33561"),
        name: "Coinbase_48",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("4D8336bDa6C11BD2a805C291Ec719BaeDD10AcB9"),
        name: "Coinbase_49",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("0a74dB66f8248554D406832E305555043fE3BfD7"),
        name: "Coinbase_5",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("4F86D1d365434bfBC1E818534d353FfC1A06F8Fe"),
        name: "Coinbase_50",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("4FD166478B440FB2a0BC64321Ec35eD48F9CDB16"),
        name: "Coinbase_51",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("503828976D22510aad0201ac7EC88293211D23Da"),
        name: "Coinbase_52",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("5122E9AA635C13AFD2fc31De3953E0896bac7aB4"),
        name: "Coinbase_53",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("55c4bf24450ecB6F818f6Be5881A4fc491dEb01e"),
        name: "Coinbase_54",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("55E6513DBCdD1dCAf4c6710AC423C698a56Eb68A"),
        name: "Coinbase_55",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("563537412ad5D49fAA7FA442b9193B8238d98c3c"),
        name: "Coinbase_56",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("57a7560D0eC28065762203c0d633943298eaC7C0"),
        name: "Coinbase_57",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("5FfC99B5B23c5aB8f463F6090342879c286a29bE"),
        name: "Coinbase_58",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("607094ed3a8361bB5e94dD21bcBef2997b687478"),
        name: "Coinbase_59",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("0Af5035a8dFf53F5a89bfC083FdEC94C7332aD7A"),
        name: "Coinbase_6",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("60EB0250E3A428A51ccb1E44E0aadBD1fD213Ff3"),
        name: "Coinbase_60",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("6321F9F02D9d56261c8C79131aE74D7b427ccAF5"),
        name: "Coinbase_61",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("660629fe43b0825ACF3402E10a999C563075A32c"),
        name: "Coinbase_62",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("67857eE12929E74082f1cAe64eF4221830c39113"),
        name: "Coinbase_63",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("6AAC5E7C12D3C9259cff8E10b5bBdB1064C382a5"),
        name: "Coinbase_64",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("6b76F8B1e9E59913BfE758821887311bA1805cAB"),
        name: "Coinbase_65",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("6c37a68DBB44c3F437D15Dc405c67BF5778b8637"),
        name: "Coinbase_66",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("6c8dd0e9cC58c07429e065178d88444B60e60b80"),
        name: "Coinbase_67",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("6dcBCe46a8B494c885D0e7b6817d2b519dF64467"),
        name: "Coinbase_68",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("6F52730DBA7B02beeFcAF0D6998c9AE901Ea04f9"),
        name: "Coinbase_69",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("0B0A5886664376F59C351ba3f598C8A8B4D0A6f3"),
        name: "Coinbase_7",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("707e8eaf4C1586fea01F8fd0242DaaC009d4f60E"),
        name: "Coinbase_70",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("71660c4005BA85c37ccec55d0C4493E66Fe775d3"),
        name: "Coinbase_71",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("731307f3B12cC56191aDE83ea630a377D9a941F6"),
        name: "Coinbase_72",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("739120AdE7ED878FcA5bbDB806263a8258FE2360"),
        name: "Coinbase_73",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("75590f473d2A7b377c1Ba991790900f28532f329"),
        name: "Coinbase_74",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("760DcE7eA6e8BA224BFFBEB8a7ff4Dd1Ef122BfF"),
        name: "Coinbase_75",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("76a1B88Be943BC5dC6c9b641A0835970a2cC2dc4"),
        name: "Coinbase_76",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("77696bb39917C91A0c3908D577d5e322095425cA"),
        name: "Coinbase_77",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("7830c87C02e56AFf27FA8Ab1241711331FA86F43"),
        name: "Coinbase_78",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("7c195D981AbFdC3DDecd2ca0Fed0958430488e34"),
        name: "Coinbase_79",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("0Bf5Ec06f44E9AA068732B1c203F007855A5EEd8"),
        name: "Coinbase_8",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("7C310a03f4CFa19F7f3d7F36DD3E05828629fa78"),
        name: "Coinbase_80",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("7c41FDceD2Ea646eD85665D1a9b28e6632b61c41"),
        name: "Coinbase_81",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("7eD53F6E3dE6B2b4156FA8E618506E60D8E65843"),
        name: "Coinbase_82",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("80cF6275294bddcE597789c855691cF4CDF01386"),
        name: "Coinbase_83",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("8196f70b2c17Ba58d8Ef56AD62087Ee8231be33a"),
        name: "Coinbase_84",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("829E3c7781a6AC8Cd864Cd8437a664Ec07DA75a8"),
        name: "Coinbase_85",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("8535d5Ed435e405a881545FFb2f8D6B81F6c9d41"),
        name: "Coinbase_86",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("881D4032abe4188e2237eFCD27aB435E81FC6bb1"),
        name: "Coinbase_87",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("898F293E97d59b375939eC3654ac548be2759d41"),
        name: "Coinbase_88",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("8af8485e1F178be06386CD3877Fde20626e0284F"),
        name: "Coinbase_89",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("122fDD9fEcbc82F7d4237C0549a5057E31c8EF8D"),
        name: "Coinbase_9",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("90E18a6920985DBACc3d76Cf27a3F2131923C720"),
        name: "Coinbase_90",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("90e63c3d53E0Ea496845b7a03ec7548B70014A91"),
        name: "Coinbase_91",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("91d66b38ae24292e9e12dd962bBB3AEcF4Ab769A"),
        name: "Coinbase_92",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("93745FEEb3C42F6aB0f9890CbCe65B360145b4ab"),
        name: "Coinbase_93",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("95A9bd206aE52C4BA8EecFc93d18EACDd41C88CC"),
        name: "Coinbase_94",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("95f90ce2e3abaeD29eEEbDb42E1FdB146e0F848a"),
        name: "Coinbase_95",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("9620e530995D4B497F16652419dBB60f7834010A"),
        name: "Coinbase_96",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("9810762578aCCF1F314320CCa5B72506aE7D7630"),
        name: "Coinbase_97",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("9a1eD80eBc9936ceE2d3DB944Ee6bD8D407e7f9F"),
        name: "Coinbase_98",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("9b4Fc9E22b46487F0810eF5dFa230b9f139E5179"),
        name: "Coinbase_99",
        exchange: "Coinbase",
    },
    CexAddress {
        address: address!("1E7016f7C23859d097668C27B72C170eD7129A10"),
        name: "Coinbase Prime_1",
        exchange: "Coinbase Prime",
    },
    CexAddress {
        address: address!("72CEf07728B199e7aD11FF03f02D2D65F91AD553"),
        name: "Coinbase Prime_2",
        exchange: "Coinbase Prime",
    },
    CexAddress {
        address: address!("A86309988947559b6E72Ef716C5058F479386C0F"),
        name: "Coinbase Prime_3",
        exchange: "Coinbase Prime",
    },
    CexAddress {
        address: address!("abF7503e05a9c82726Ba6d7BBfFDfF8C2f3388c6"),
        name: "Coinbase Prime_4",
        exchange: "Coinbase Prime",
    },
    CexAddress {
        address: address!("CD531Ae9EFCCE479654c4926dec5F6209531Ca7b"),
        name: "Coinbase Prime_5",
        exchange: "Coinbase Prime",
    },
    CexAddress {
        address: address!("ceB69F6342eCE283b2F5c9088Ff249B5d0Ae66ea"),
        name: "Coinbase Prime_6",
        exchange: "Coinbase Prime",
    },
    CexAddress {
        address: address!("DfD76BbFEB9Eb8322F3696d3567e03f894C40d6c"),
        name: "Coinbase Prime_7",
        exchange: "Coinbase Prime",
    },
    CexAddress {
        address: address!("189B9cBd4AfF470aF2C0102f365FC1823d857965"),
        name: "Coincheck",
        exchange: "Coincheck",
    },
    CexAddress {
        address: address!("8696e84aB5e78983f2456bCB5c199eEa9648C8C2"),
        name: "Coincheck_1",
        exchange: "Coincheck",
    },
    CexAddress {
        address: address!("9C19B0497997Fe9E75862688a295168070456951"),
        name: "Coincheck_2",
        exchange: "Coincheck",
    },
    CexAddress {
        address: address!("d52814615DC129e1eEa3520e7e7f8d44DBfc6c5b"),
        name: "Coincheck_3",
        exchange: "Coincheck",
    },
    CexAddress {
        address: address!("B6ba1931e4E74FD080587688F6DB10E830F810d5"),
        name: "Coindelta",
        exchange: "Coindelta",
    },
    CexAddress {
        address: address!("1562Bd4f90EB997611B5D7579ab38e5a23aCb33e"),
        name: "Coinhako",
        exchange: "Coinhako",
    },
    CexAddress {
        address: address!("19d97aa29cd33Bd966d52e2Bc9dFc719f2Bb9aE1"),
        name: "Coinhako_1",
        exchange: "Coinhako",
    },
    CexAddress {
        address: address!("1d1bD550197c7c0787b9ad0aEA9c1CCa66eE0E90"),
        name: "Coinhako_2",
        exchange: "Coinhako",
    },
    CexAddress {
        address: address!("a193C943980A9340F306b3d59Deb183Dc501B35f"),
        name: "Coinhako_3",
        exchange: "Coinhako",
    },
    CexAddress {
        address: address!("bb44E3349C23cC430CAe6EbBaf0256c9f2a1872f"),
        name: "Coinhako_4",
        exchange: "Coinhako",
    },
    CexAddress {
        address: address!("d4BDDf5E3D0435D7A6214A0B949C7BB58621F37C"),
        name: "Coinhako_5",
        exchange: "Coinhako",
    },
    CexAddress {
        address: address!("E66BAa0B612003AF308D78f066Bbdb9a5e00fF6c"),
        name: "Coinhako_6",
        exchange: "Coinhako",
    },
    CexAddress {
        address: address!("86D3E894b5CDb6a80affFd35eD348868fb98DD3f"),
        name: "Coinify",
        exchange: "Coinify",
    },
    CexAddress {
        address: address!("0363bFC09B48616190d15ddEe5987Ae2D05Da6F4"),
        name: "Coinjar",
        exchange: "Coinjar",
    },
    CexAddress {
        address: address!("6e5bBE3f4Fe7b0CCd74d7dA1Cbd5CfE3c868BbdC"),
        name: "Coinjar_1",
        exchange: "Coinjar",
    },
    CexAddress {
        address: address!("C1C88785E9B5c9c85B3f6c99255C3bef9e3B14A7"),
        name: "Coinjar_2",
        exchange: "Coinjar",
    },
    CexAddress {
        address: address!("d6a062CAE6123C158768A5C444CA0896CC60D6B1"),
        name: "Coinjar_3",
        exchange: "Coinjar",
    },
    CexAddress {
        address: address!("ddC50252A3080d5028D1c25261f78f023E9117d5"),
        name: "Coinjar_4",
        exchange: "Coinjar",
    },
    CexAddress {
        address: address!("0BC7b31BF8ffAc8213E496102167FE73Fc30937a"),
        name: "Coinmetro",
        exchange: "Coinmetro",
    },
    CexAddress {
        address: address!("165Fe6a10812fAA49515522d685A27c6Bf12DbA9"),
        name: "Coinmetro_1",
        exchange: "Coinmetro",
    },
    CexAddress {
        address: address!("Fad672DC92C2d2Db0aa093331bD1098e30249AB8"),
        name: "Coinmetro_10",
        exchange: "Coinmetro",
    },
    CexAddress {
        address: address!("1E7b450322E42b53D55ABC795913537fD680fe0c"),
        name: "Coinmetro_2",
        exchange: "Coinmetro",
    },
    CexAddress {
        address: address!("46a189C7F37953b91E5bd75D0E608efdAfefAF1C"),
        name: "Coinmetro_3",
        exchange: "Coinmetro",
    },
    CexAddress {
        address: address!("4CF2220105995F006813923019f02BE1CCcA8132"),
        name: "Coinmetro_4",
        exchange: "Coinmetro",
    },
    CexAddress {
        address: address!("706Ee6cF36a4bF95ec7Fe5d468571B9E9e63FBB6"),
        name: "Coinmetro_5",
        exchange: "Coinmetro",
    },
    CexAddress {
        address: address!("a270F3ad1a7a82E6a3157F12a900f1E25BC4FbFD"),
        name: "Coinmetro_6",
        exchange: "Coinmetro",
    },
    CexAddress {
        address: address!("bAC7c449689A2d3c51c386D8E657338C41ab3030"),
        name: "Coinmetro_7",
        exchange: "Coinmetro",
    },
    CexAddress {
        address: address!("DD06B66c76D9c6fdC41935A7b32566C646325005"),
        name: "Coinmetro_8",
        exchange: "Coinmetro",
    },
    CexAddress {
        address: address!("F3e35734B7413F87c2054A16cE04230d803E4dC3"),
        name: "Coinmetro_9",
        exchange: "Coinmetro",
    },
    CexAddress {
        address: address!("167A9333BF582556f35Bd4d16A7E80E191aa6476"),
        name: "Coinone",
        exchange: "Coinone",
    },
    CexAddress {
        address: address!("1e2FCfd26d36183f1A5d90f0e6296915b02BCb40"),
        name: "Coinone_1",
        exchange: "Coinone",
    },
    CexAddress {
        address: address!("7A1BA53C0E2D218DF39E76e4eFBF0455978cc23E"),
        name: "Coinone_2",
        exchange: "Coinone",
    },
    CexAddress {
        address: address!("17Ac1517c30d5Cd7f065cb28C4703431bb69D47D"),
        name: "Coins.ph",
        exchange: "Coins.ph",
    },
    CexAddress {
        address: address!("4577F1A9b54492fA1B9EF3b58d7CDc1ea8b3225C"),
        name: "Coins.ph_1",
        exchange: "Coins.ph",
    },
    CexAddress {
        address: address!("292f04a44506c2fd49Bac032E1ca148C35A478c8"),
        name: "CoinsPaid",
        exchange: "CoinsPaid",
    },
    CexAddress {
        address: address!("bd0fCcdC19bC3b979e8E256b7B88AAe7C77A5BEC"),
        name: "CoinsPaid_1",
        exchange: "CoinsPaid",
    },
    CexAddress {
        address: address!("Dce92f40cAdDE2C4e3EA78b8892c540e6bFe2f81"),
        name: "CoinsPaid_2",
        exchange: "CoinsPaid",
    },
    CexAddress {
        address: address!("21Dd5c13925407e5bCec3f27aB11a355a9Dafbe3"),
        name: "Coinsbit",
        exchange: "Coinsbit",
    },
    CexAddress {
        address: address!("252C8bbeF63101eBcA22C27951175C6B33c86b07"),
        name: "Coinsbit_1",
        exchange: "Coinsbit",
    },
    CexAddress {
        address: address!("75987b9edB5463CE1a3a857E11671424600927A4"),
        name: "Coinsbit_2",
        exchange: "Coinsbit",
    },
    CexAddress {
        address: address!("02fdc44Bf226E49DCecA4775Afaef3360e9C4EE9"),
        name: "Coinsquare",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("0fcFF154753e337983613889b69dd85Fe8a1a145"),
        name: "Coinsquare_1",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("7061d86A274B398a1fB7Cdb74B3abBc7601e105f"),
        name: "Coinsquare_10",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("7ee87dd5BB9924Cb85CA2916Bd4E04299D3A8EcC"),
        name: "Coinsquare_11",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("82Be7cFeF05B70c4AF47F8fd70F636201121341b"),
        name: "Coinsquare_12",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("8623c08A4B880799CF65E75137ec9759DB336637"),
        name: "Coinsquare_13",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("89813b57AE92e74Fb808eb7639d3A0050c9b3D7D"),
        name: "Coinsquare_14",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("8e080C5d233F2A14A37d024c0382bF0585146993"),
        name: "Coinsquare_15",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("910695E5C7c14499B554fb132A9710988a42fC38"),
        name: "Coinsquare_16",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("91ADf14f4C0782634E04Dfc6e9Be16d950AA4daA"),
        name: "Coinsquare_17",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("9C6D4A1922Eed56Ee9de148c5BA9b1b477FEcBb6"),
        name: "Coinsquare_18",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("C4d75abAb14Ef006d5Ac9fe901a8ed616C4e2627"),
        name: "Coinsquare_19",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("14AA1AD09664c33679aE5689d93085B8F7c84bd3"),
        name: "Coinsquare_2",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("d093F2Ee92cf32B4D3EBefd965447415074DD6c8"),
        name: "Coinsquare_20",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("D381347EE757F53aE4B3b6822DAeC3E2A14B2005"),
        name: "Coinsquare_21",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("D5B2C371808018ee131ad387877C4d58e08e7A06"),
        name: "Coinsquare_22",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("f9c91937737cCaFE9bBb662b1917B54F9606Ca13"),
        name: "Coinsquare_23",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("fac596Facd1901458C1C6347397a6e5D0769736c"),
        name: "Coinsquare_24",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("2f671a39613861EC17ad35de44F4f84e3D4C69f4"),
        name: "Coinsquare_3",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("3858A27eeCB5f1144473E35A293cb1B2bda6DfF4"),
        name: "Coinsquare_4",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("476B067CbFF8ACB805038E9dAEF5D51c7612d593"),
        name: "Coinsquare_5",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("48a0B5f7DE8789a3962918C6DF4A766c0c8857B0"),
        name: "Coinsquare_6",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("56E89a4b2E3924c336d52CE0ad98fF23E1a51627"),
        name: "Coinsquare_7",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("6A73f209d25CC9c089170cc5b54962e0c7614E0c"),
        name: "Coinsquare_8",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("6d712f120bD65aD54a5F56670976788a044Cb987"),
        name: "Coinsquare_9",
        exchange: "Coinsquare",
    },
    CexAddress {
        address: address!("20664cacdcfeb318C8e145a03C75e34bc2CC4A3b"),
        name: "Coinstore",
        exchange: "Coinstore",
    },
    CexAddress {
        address: address!("65e1615eFC11c63E15c00aC4447C56aF294135a9"),
        name: "Coinstore_1",
        exchange: "Coinstore",
    },
    CexAddress {
        address: address!("86790abbaCcD1B21F5ecFDaA67EC6282AFbf3E83"),
        name: "Coinstore_2",
        exchange: "Coinstore",
    },
    CexAddress {
        address: address!("93639ba2c0138c9EF450C247D33c49aC9dbE97D5"),
        name: "Coinstore_3",
        exchange: "Coinstore",
    },
    CexAddress {
        address: address!("A33Ea0BACaCb74800f89762779AC03073b1182A3"),
        name: "Coinstore_4",
        exchange: "Coinstore",
    },
    CexAddress {
        address: address!("DF12A6f5C800eCE10D3A53636fF0a41a1AcA4850"),
        name: "Coinstore_5",
        exchange: "Coinstore",
    },
    CexAddress {
        address: address!("F2067aBFaB8BC621211935431519d41825d2f344"),
        name: "Coinstore_6",
        exchange: "Coinstore",
    },
    CexAddress {
        address: address!("f83F7ae1403fAd899108F99D9f427dC6981806C4"),
        name: "Coinstore_7",
        exchange: "Coinstore",
    },
    CexAddress {
        address: address!("d0808Da05cc71a9F308D330bC9c5C81Bbc26FC59"),
        name: "Coinswitch",
        exchange: "Coinswitch",
    },
    CexAddress {
        address: address!("014C18ad92838E8bB62b0135a0Cf3f5CDD5Bd6F4"),
        name: "Coinzix",
        exchange: "Coinzix",
    },
    CexAddress {
        address: address!("48077400FAF11183c043Feb5184a13ea628Bb0DB"),
        name: "Coinzix_1",
        exchange: "Coinzix",
    },
    CexAddress {
        address: address!("4f50226BBc651EEB8E766591c4Dac762ad832de2"),
        name: "Coinzix_2",
        exchange: "Coinzix",
    },
    CexAddress {
        address: address!("9eE23657e6764f6DbAE17af8a8191ea3c29A6D1D"),
        name: "Coinzix_3",
        exchange: "Coinzix",
    },
    CexAddress {
        address: address!("0349923aE2B35FF4f0099869aeea99d1f3FD12a9"),
        name: "Copper",
        exchange: "Copper",
    },
    CexAddress {
        address: address!("7A1D38cacA09F408D58837E60E172ea3785f9ff0"),
        name: "Copper_1",
        exchange: "Copper",
    },
    CexAddress {
        address: address!("7C6782476e29fB26E1556FbA058eb7ECED93D327"),
        name: "Copper_2",
        exchange: "Copper",
    },
    CexAddress {
        address: address!("a205fD7344656c72FDC645b72fAF5a3DE0B3E825"),
        name: "Copper_3",
        exchange: "Copper",
    },
    CexAddress {
        address: address!("A5995359b9941E060e366B4Ee3ebB6A9f47649be"),
        name: "Copper_4",
        exchange: "Copper",
    },
    CexAddress {
        address: address!("Af64555DDD61FcF7D094824dd9B4eBea165aFc5b"),
        name: "Copper_5",
        exchange: "Copper",
    },
    CexAddress {
        address: address!("B7a2D83d94fce4A4cC8f92c961Af418D5C797565"),
        name: "Copper_6",
        exchange: "Copper",
    },
    CexAddress {
        address: address!("cFbba243c567738C2d3D958026b1Bf6Ad894A19e"),
        name: "Copper_7",
        exchange: "Copper",
    },
    CexAddress {
        address: address!("1A0324046933dDa97F0296fcCff033966278B532"),
        name: "Cryptal",
        exchange: "Cryptal",
    },
    CexAddress {
        address: address!("a046FDd5Dd224BA5498F7821A757c4b67622Db2b"),
        name: "Cryptal_1",
        exchange: "Cryptal",
    },
    CexAddress {
        address: address!("F3294A05Fe37673B96c6fF166aFe50cDd6B33391"),
        name: "Cryptal_2",
        exchange: "Cryptal",
    },
    CexAddress {
        address: address!("0Ecc16D3fa38E1a59c10e44CDA4e2e9d9941275A"),
        name: "Crypto.com",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("1714400FF23dB4aF24F9fd64e7039e6597f18C2b"),
        name: "Crypto.com_1",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("72A53cDBBcc1b9efa39c834A540550e23463AAcB"),
        name: "Crypto.com_10",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("7758E507850dA48cd47df1fB5F875c23E3340c50"),
        name: "Crypto.com_11",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("7Aad7840F119f3876EE3569e488C7C4135f695fa"),
        name: "Crypto.com_12",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("8a161a996617f130d0F37478483AfC8c1914DB6d"),
        name: "Crypto.com_13",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("92BD687953Da50855AeE2Df0Cff282cC2d5F226b"),
        name: "Crypto.com_14",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("9a552417cfc942A5C88Ab474756d3D9962f917C0"),
        name: "Crypto.com_15",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("9Fb538820D4FDe2FCC509Dc01Ae73a192f36cfcC"),
        name: "Crypto.com_16",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("Ae45a8240147E6179ec7c9f92c5A18F9a97B3fCA"),
        name: "Crypto.com_17",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("CFFAd3200574698b78f32232aa9D63eABD290703"),
        name: "Crypto.com_18",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("D3d877fc323De661Ff9E1a38147A1AC679ce7C64"),
        name: "Crypto.com_19",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("187b2d576ba7ec2141c180A96eDd0f202492f36B"),
        name: "Crypto.com_2",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("D7a827FBaf38c98E8336C5658E4BcbCD20a4fd2d"),
        name: "Crypto.com_20",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("f3B0073E3a7F747C7A38B36B805247B222C302A3"),
        name: "Crypto.com_21",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("fa0b641678F5115ad8a8De5752016bD1359681b9"),
        name: "Crypto.com_22",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("18ae7a92f5261208bb5366Fa213c966D65988C95"),
        name: "Crypto.com_3",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("20fA1822A87D4e7A3CcF20f86e716Ef3772eCff1"),
        name: "Crypto.com_4",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("2C2301FDB0bfA06EAABaA0122CbCEb2265337C25"),
        name: "Crypto.com_5",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("46340b20830761efd32832A74d7169B29FEB9758"),
        name: "Crypto.com_6",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("546553718b1B255742566f10A34D86FC22F02b1f"),
        name: "Crypto.com_7",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("625b02b687Ec38f3085Af5B108Dda410775fA76a"),
        name: "Crypto.com_8",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("6262998Ced04146fA42253a5C0AF90CA02dfd2A3"),
        name: "Crypto.com_9",
        exchange: "Crypto.com",
    },
    CexAddress {
        address: address!("0975CA9F986EeE35F5CbbA2d672ad9bc8D2a0844"),
        name: "Cryptonator",
        exchange: "Cryptonator",
    },
    CexAddress {
        address: address!("4b407c966C2cacD502a73E46bD0B97aAf78929ee"),
        name: "Cryptonator_1",
        exchange: "Cryptonator",
    },
    CexAddress {
        address: address!("4FED1fC4144c223aE3C1553be203cDFcbD38C581"),
        name: "Cryptonator_2",
        exchange: "Cryptonator",
    },
    CexAddress {
        address: address!("687c8EcBc10ed1Ff6b1D700037FC4b873cf7e424"),
        name: "Cryptonator_3",
        exchange: "Cryptonator",
    },
    CexAddress {
        address: address!("1B3d794bbEECD9240F46dBb3b79F4f71a972e00A"),
        name: "Cryptopia",
        exchange: "Cryptopia",
    },
    CexAddress {
        address: address!("2984581eCE53A4390d1F568673cf693139C97049"),
        name: "Cryptopia_1",
        exchange: "Cryptopia",
    },
    CexAddress {
        address: address!("5BaEac0a0417a05733884852aa068B706967e790"),
        name: "Cryptopia_2",
        exchange: "Cryptopia",
    },
    CexAddress {
        address: address!("CEED7802EA80992aF9dA3811c455fD7BAA3f644C"),
        name: "Cryptopia_3",
        exchange: "Cryptopia",
    },
    CexAddress {
        address: address!("e269E891A2Ec8585a378882fFA531141205e92E9"),
        name: "DDEX",
        exchange: "DDEX",
    },
    CexAddress {
        address: address!("7600977Eb9eFFA627D6BD0DA2E5be35E11566341"),
        name: "DEx.top",
        exchange: "DEx.top",
    },
    CexAddress {
        address: address!("276766330eA4447289df29648474d1BBDb3fee90"),
        name: "DIFX",
        exchange: "DIFX",
    },
    CexAddress {
        address: address!("58A5B842A629B9D134DFD348C714b7f1d8212253"),
        name: "DIFX_1",
        exchange: "DIFX",
    },
    CexAddress {
        address: address!("bE774bB4A11c033C68FF0CC515B3316e93e94465"),
        name: "DIFX_2",
        exchange: "DIFX",
    },
    CexAddress {
        address: address!("2101e480e22C953b37b9D0FE6551C1354Fe705E6"),
        name: "DMEX",
        exchange: "DMEX",
    },
    CexAddress {
        address: address!("6c73b1cA08bBC3F44340603b1Fb9E331C2ABaCa7"),
        name: "Deepcoin",
        exchange: "Deepcoin",
    },
    CexAddress {
        address: address!("1a7574D48c4960278e89b7e7e069E5b9809D7b67"),
        name: "Delta Exchange",
        exchange: "Delta",
    },
    CexAddress {
        address: address!("50a3F3B8855c2DA88e56C7B0EF6E0e4A79F853f9"),
        name: "Delta Exchange_1",
        exchange: "Delta Exchange",
    },
    CexAddress {
        address: address!("c07b9DDC7F87e76E682a7a4F3859586eEF1c7efd"),
        name: "Delta Exchange_2",
        exchange: "Delta Exchange",
    },
    CexAddress {
        address: address!("062448f804191128D71FC72e10a1D13Bd7308e7E"),
        name: "Deribit",
        exchange: "Deribit",
    },
    CexAddress {
        address: address!("2EeD6A08Fb89a5CD111efA33F8DcA46CfdBE370F"),
        name: "Deribit_1",
        exchange: "Deribit",
    },
    CexAddress {
        address: address!("A0F6121319a34f24653fB82aDdC8dD268Af5b9e1"),
        name: "Deribit_10",
        exchange: "Deribit",
    },
    CexAddress {
        address: address!("A7e15eF7C01B58eBe5eF74Aa73625Ae4b11FE754"),
        name: "Deribit_11",
        exchange: "Deribit",
    },
    CexAddress {
        address: address!("cFEe6efEc3471874022e205f4894733C42CbBF64"),
        name: "Deribit_12",
        exchange: "Deribit",
    },
    CexAddress {
        address: address!("58F56615180A8eeA4c462235D9e215F72484B4A3"),
        name: "Deribit_2",
        exchange: "Deribit",
    },
    CexAddress {
        address: address!("5f397B62502e255f68382791947D54C4B2d37F09"),
        name: "Deribit_3",
        exchange: "Deribit",
    },
    CexAddress {
        address: address!("63F41034871535ceE49996Cc47719891Fe03dff9"),
        name: "Deribit_4",
        exchange: "Deribit",
    },
    CexAddress {
        address: address!("6B378bE3c9642ccF25b1A27faCb8ace24aC34A12"),
        name: "Deribit_5",
        exchange: "Deribit",
    },
    CexAddress {
        address: address!("77021d475E36b3ab1921a0e3A8380f069d3263de"),
        name: "Deribit_6",
        exchange: "Deribit",
    },
    CexAddress {
        address: address!("904cC2B2694FFa78F04708D6F7dE205108213126"),
        name: "Deribit_7",
        exchange: "Deribit",
    },
    CexAddress {
        address: address!("9cf8d36F4Aab14Ef4975aC6C98f896865F0900c5"),
        name: "Deribit_8",
        exchange: "Deribit",
    },
    CexAddress {
        address: address!("9FaE72D291949Ed6fa8b74881328FDc123C645D3"),
        name: "Deribit_9",
        exchange: "Deribit",
    },
    CexAddress {
        address: address!("4081A867B3F3fB8761a054024d09d2575d81D381"),
        name: "Dex-Trade",
        exchange: "Dex-Trade",
    },
    CexAddress {
        address: address!("881368E08CC5353E0188b2cA0401b5de35F319F4"),
        name: "Dex-Trade_1",
        exchange: "Dex-Trade",
    },
    CexAddress {
        address: address!("a0f66086C32c692033b664A775Cb3484C62c2C9e"),
        name: "Dex-Trade_2",
        exchange: "Dex-Trade",
    },
    CexAddress {
        address: address!("B355104B0AE14265b69AfC31d6a117f982337280"),
        name: "Dex-Trade_3",
        exchange: "Dex-Trade",
    },
    CexAddress {
        address: address!("bde577B9582aA0305f623b51f1b70c5a118B922A"),
        name: "Dex-Trade_4",
        exchange: "Dex-Trade",
    },
    CexAddress {
        address: address!("D0ACB9C61cD72E0f57B19268D70c73b77DbDd553"),
        name: "Dex-Trade_5",
        exchange: "Dex-Trade",
    },
    CexAddress {
        address: address!("1b930C43526B09191a74175eAA47F2A650aEB73d"),
        name: "DigiFinex",
        exchange: "DigiFinex",
    },
    CexAddress {
        address: address!("3B73D7e1266e02a68185f5221a6718dB04dF6301"),
        name: "DigiFinex_1",
        exchange: "DigiFinex",
    },
    CexAddress {
        address: address!("96C01fA1d6A732F7888b564453cED1cD6c1fa5b9"),
        name: "DigiFinex_2",
        exchange: "DigiFinex",
    },
    CexAddress {
        address: address!("b2cA9495F3e1972801c12a964fA344a6630F427a"),
        name: "DigiFinex_3",
        exchange: "DigiFinex",
    },
    CexAddress {
        address: address!("B37640f5F7ef7b0fDCce2c0C053DB4f976945647"),
        name: "DigiFinex_4",
        exchange: "DigiFinex",
    },
    CexAddress {
        address: address!("e17ee7B3c676701c66B395A35f0DF4C2276a344E"),
        name: "DigiFinex_5",
        exchange: "DigiFinex",
    },
    CexAddress {
        address: address!("Fb75B231C307738ce506c242bABaD2FD2e77B0bf"),
        name: "Digital Surge",
        exchange: "Digital",
    },
    CexAddress {
        address: address!("3E97ae61Ceda35B857b05e6B579c54aBD5568C36"),
        name: "Duelbits",
        exchange: "Duelbits",
    },
    CexAddress {
        address: address!("4E80744fa23cEC76e1621ce0DfACeB4B1D532e12"),
        name: "Duelbits_1",
        exchange: "Duelbits",
    },
    CexAddress {
        address: address!("147a95FA1cCeCCc20B341824952106CB0EE1eded"),
        name: "EXMO",
        exchange: "EXMO",
    },
    CexAddress {
        address: address!("1Fd6267f0D86F62D88172B998390AfEE2a1F54B6"),
        name: "EXMO_1",
        exchange: "EXMO",
    },
    CexAddress {
        address: address!("495a7e55dAf85d5C098a2aD5ba9a46312329D63c"),
        name: "EXMO_2",
        exchange: "EXMO",
    },
    CexAddress {
        address: address!("b36CBe7f95A39984384E6AA4068B02C1697Ef80E"),
        name: "EXMO_3",
        exchange: "EXMO",
    },
    CexAddress {
        address: address!("C179FBDDC946694d11185d4e15DbBa5Fd0aDac0a"),
        name: "EXMO_4",
        exchange: "EXMO",
    },
    CexAddress {
        address: address!("d7B9A9b2F665849C4071Ad5af77d8c76aa30fb32"),
        name: "EXMO_5",
        exchange: "EXMO",
    },
    CexAddress {
        address: address!("ffF52a282c215aeD0C4C71786D85519bbcC86464"),
        name: "EXMO_6",
        exchange: "EXMO",
    },
    CexAddress {
        address: address!("07ec30473eF6e1b9a434b1D48B97f79D46C13D5f"),
        name: "Eidoo",
        exchange: "Eidoo",
    },
    CexAddress {
        address: address!("aCF9e2469fBc57E6Ad391f97a41f8Aad4d68B097"),
        name: "Eidoo_1",
        exchange: "Eidoo",
    },
    CexAddress {
        address: address!("F1C525a488a848b58B95D79dA48C21Ce434290f7"),
        name: "Eidoo_2",
        exchange: "Eidoo",
    },
    CexAddress {
        address: address!("608F94DF1c1D89ea13e5984D7bF107DF137A6541"),
        name: "Eigen Fx",
        exchange: "Eigen",
    },
    CexAddress {
        address: address!("eB9EbF2c624eBee42e0853da6443dDC6C8020de7"),
        name: "Eigen Fx_1",
        exchange: "Eigen Fx",
    },
    CexAddress {
        address: address!("2F006050F172f3C2a85F5E72A5A2c6c67790d6B7"),
        name: "Emirex",
        exchange: "Emirex",
    },
    CexAddress {
        address: address!("fA5ccf9FC5828B589aF6f321770a7260f31b4746"),
        name: "Emirex_1",
        exchange: "Emirex",
    },
    CexAddress {
        address: address!("d3808c5D48903be1490989F3fcE2a2b3890E8eB6"),
        name: "Exchange A",
        exchange: "Exchange",
    },
    CexAddress {
        address: address!("915d7915f2b469bb654A7D903A5d4417Cb8eA7Df"),
        name: "FCoin",
        exchange: "FCoin",
    },
    CexAddress {
        address: address!("71D7cb7F3F4731Ec26E281E58d9FcE443E7B3f83"),
        name: "FINXFLO",
        exchange: "FINXFLO",
    },
    CexAddress {
        address: address!("7C63a3f37d3d18e29a683b8769964a98b056b542"),
        name: "FINXFLO_1",
        exchange: "FINXFLO",
    },
    CexAddress {
        address: address!("a31e062155B3387aeeF9A476e162b0b4AF298935"),
        name: "FINXFLO_2",
        exchange: "FINXFLO",
    },
    CexAddress {
        address: address!("25eAff5B179f209Cf186B1cdCbFa463A69Df4C45"),
        name: "FTX",
        exchange: "FTX",
    },
    CexAddress {
        address: address!("3701F04E25b57e549A2a348C18e2d925C2805602"),
        name: "FTX US",
        exchange: "FTX",
    },
    CexAddress {
        address: address!("279f8940ca2a44C35ca3eDf7d28945254d0F0aE6"),
        name: "FTX_1",
        exchange: "FTX",
    },
    CexAddress {
        address: address!("C098B2a3Aa256D2140208C3de6543aAEf5cd3A94"),
        name: "FTX_10",
        exchange: "FTX",
    },
    CexAddress {
        address: address!("d8019a114e86ad41D71a3EeB6620b19Dd166A969"),
        name: "FTX_11",
        exchange: "FTX",
    },
    CexAddress {
        address: address!("2FAF487A4414Fe77e2327F0bf4AE2a264a776AD2"),
        name: "FTX_2",
        exchange: "FTX",
    },
    CexAddress {
        address: address!("41772eDd47D9DDF9ef848cDB34fE76143908c7Ad"),
        name: "FTX_3",
        exchange: "FTX",
    },
    CexAddress {
        address: address!("51bfacfcE67821EC05d3C9bC9a8BC8300fB29564"),
        name: "FTX_4",
        exchange: "FTX",
    },
    CexAddress {
        address: address!("6001CE416FF9801dba27c6eb217DfD7C258f6d27"),
        name: "FTX_5",
        exchange: "FTX",
    },
    CexAddress {
        address: address!("6E685A45Db4d97BA160FA067cB81b40Dfed47245"),
        name: "FTX_6",
        exchange: "FTX",
    },
    CexAddress {
        address: address!("772589e99bC9C54DD40acb7d73F88Ccbc9D9CF47"),
        name: "FTX_7",
        exchange: "FTX",
    },
    CexAddress {
        address: address!("9ade1c17d25246c405604344f89E8F23F8c1c632"),
        name: "FTX_8",
        exchange: "FTX",
    },
    CexAddress {
        address: address!("A60113f7d43130919802b0863abdCdb956664fD5"),
        name: "FTX_9",
        exchange: "FTX",
    },
    CexAddress {
        address: address!("42d6Ce661bB2e5F5cc639E7BEFE74Ff9Fd649541"),
        name: "FTX US_1",
        exchange: "FTX US",
    },
    CexAddress {
        address: address!("46cE2DA01290EA2D1BCED39BbA149d017DF34CB9"),
        name: "FTX US_2",
        exchange: "FTX US",
    },
    CexAddress {
        address: address!("7abE0cE388281d2aCF297Cb089caef3819b13448"),
        name: "FTX US_3",
        exchange: "FTX US",
    },
    CexAddress {
        address: address!("A182aAB7B51232FBFABC22d989f21d264B0B246f"),
        name: "FTX US_4",
        exchange: "FTX US",
    },
    CexAddress {
        address: address!("94fe3AD91dACbA8eC4B82F56ff7C122181f1535d"),
        name: "Faa.st",
        exchange: "Faa.st",
    },
    CexAddress {
        address: address!("0f69fd9Fee9D01034D612497fCf07e1f5AB9ED3C"),
        name: "Fairdesk",
        exchange: "Fairdesk",
    },
    CexAddress {
        address: address!("63295D75B5A01eFF3D7F7Cd199fa48f9C3De6395"),
        name: "Fairdesk_1",
        exchange: "Fairdesk",
    },
    CexAddress {
        address: address!("cc5F916C45E7a3506186ffF370fb985bA93a5167"),
        name: "Fairdesk_2",
        exchange: "Fairdesk",
    },
    CexAddress {
        address: address!("FfD0eCEE885FcD2eBCe0FeC9add69Bf1Ed03194B"),
        name: "Fairdesk_3",
        exchange: "Fairdesk",
    },
    CexAddress {
        address: address!("1157A2076b9bB22a85CC2C162f20fAB3898F4101"),
        name: "FalconX",
        exchange: "FalconX",
    },
    CexAddress {
        address: address!("9FC6beF0702CF47dCD2e5a42b48e19aed8732499"),
        name: "FalconX_1",
        exchange: "FalconX",
    },
    CexAddress {
        address: address!("e1eD4DA4284924dDAf69983B4D813FB1be58c380"),
        name: "FalconX_2",
        exchange: "FalconX",
    },
    CexAddress {
        address: address!("F2eF6Ac0F00BC91B3CB5FFb621B64c69E87b4f77"),
        name: "FalconX_3",
        exchange: "FalconX",
    },
    CexAddress {
        address: address!("85E1De87a7575C6581F7930F857a3813B66A14d8"),
        name: "FastEx",
        exchange: "FastEx",
    },
    CexAddress {
        address: address!("c21A1D213f64FeDEA3415737CCe2BE37Eb59be81"),
        name: "FastEx_1",
        exchange: "FastEx",
    },
    CexAddress {
        address: address!("66A0be112EFE2cc3bc2f09Fa2aCaaf9f593B0265"),
        name: "Firi",
        exchange: "Firi",
    },
    CexAddress {
        address: address!("a6F617f873684ED062C9Df281145250b3E4EE2D2"),
        name: "Firi_1",
        exchange: "Firi",
    },
    CexAddress {
        address: address!("0bC6E8ceAc156acBE85E2c46A505fF33407c45F5"),
        name: "FixedFloat",
        exchange: "FixedFloat",
    },
    CexAddress {
        address: address!("440Efc740Ca49ED88051c05CF756cdE6c8Be3b27"),
        name: "FixedFloat_1",
        exchange: "FixedFloat",
    },
    CexAddress {
        address: address!("4727250679294802377dD6cA6541B8E459077c95"),
        name: "FixedFloat_2",
        exchange: "FixedFloat",
    },
    CexAddress {
        address: address!("4E5B2e1dc63F6b91cb6Cd759936495434C7e972F"),
        name: "FixedFloat_3",
        exchange: "FixedFloat",
    },
    CexAddress {
        address: address!("5959dBA0123D7a60DF9E1409DE4d7b5604976060"),
        name: "FixedFloat_4",
        exchange: "FixedFloat",
    },
    CexAddress {
        address: address!("6c344a0bBf8Ef7aF72C141c7EdA129AF4A71a9b5"),
        name: "FixedFloat_5",
        exchange: "FixedFloat",
    },
    CexAddress {
        address: address!("71d4249079684479F2651745fA2fcD79c9b45f53"),
        name: "FixedFloat_6",
        exchange: "FixedFloat",
    },
    CexAddress {
        address: address!("95d3B980D54CC9580DBFe88A7E3A6c48f27FfC47"),
        name: "FixedFloat_7",
        exchange: "FixedFloat",
    },
    CexAddress {
        address: address!("f5CBD91EaBD71Dab7343F56aBe03250BD0C2fFf4"),
        name: "FixedFloat_8",
        exchange: "FixedFloat",
    },
    CexAddress {
        address: address!("f6Ee635C28Dda95a74BA989807000cbe00317d27"),
        name: "FixedFloat_9",
        exchange: "FixedFloat",
    },
    CexAddress {
        address: address!("14301566b9669b672878d86fF0B1d18Dd58054e9"),
        name: "Flata Exchange",
        exchange: "Flata",
    },
    CexAddress {
        address: address!("C517dFAcBee5DFa82FeefB5cA29A2BB40b2d4fAA"),
        name: "Flata Exchange_1",
        exchange: "Flata Exchange",
    },
    CexAddress {
        address: address!("91e18eE76483FA2eC5Cfe2959DF46673c2565BE0"),
        name: "Flybit",
        exchange: "Flybit",
    },
    CexAddress {
        address: address!("0021845f4c2604c58F9ba5b7BFF58d16A2aB372c"),
        name: "Folgory Exchange",
        exchange: "Folgory",
    },
    CexAddress {
        address: address!("4834E61F91EC2304Cf51b590073f3c9ff8161446"),
        name: "Freewallet",
        exchange: "Freewallet",
    },
    CexAddress {
        address: address!("6025D96932D378BE7D0A46343b437678A126eCCa"),
        name: "Freewallet_1",
        exchange: "Freewallet",
    },
    CexAddress {
        address: address!("7eD1E469fCb3EE19C0366D829e291451bE638E59"),
        name: "Freewallet_2",
        exchange: "Freewallet",
    },
    CexAddress {
        address: address!("8Dd640228e1eb91e05d58206f3D9B0cCaf21bcF1"),
        name: "Freewallet_3",
        exchange: "Freewallet",
    },
    CexAddress {
        address: address!("9F5ca0012B9B72E8F3Db57092a6f26bF4f13DC69"),
        name: "GBX",
        exchange: "GBX",
    },
    CexAddress {
        address: address!("5735fBAC26BB21CA3C5228022cc382136038087c"),
        name: "GDAC",
        exchange: "GDAC",
    },
    CexAddress {
        address: address!("9f4745DF0d6713B08323e0d39Ab4CeF6891C11E1"),
        name: "GDAC_1",
        exchange: "GDAC",
    },
    CexAddress {
        address: address!("35c049D01b3782EFf7Fcf8C5770d891CC5A03561"),
        name: "GGBTC.com",
        exchange: "GGBTC.com",
    },
    CexAddress {
        address: address!("9fB01A2584Aac5aAE3faB1ed25F86c5269b32999"),
        name: "GGBTC.com_1",
        exchange: "GGBTC.com",
    },
    CexAddress {
        address: address!("52b4567c37b48D51198B25CaA6e79e4fDa6D9734"),
        name: "GMO Coin",
        exchange: "GMO",
    },
    CexAddress {
        address: address!("85C0d1110C57a329ccD6Cc91e50fB4666F2287c3"),
        name: "GMO Coin_1",
        exchange: "GMO Coin",
    },
    CexAddress {
        address: address!("86E284421664840Cb65C5b918Da59c01ED8fA666"),
        name: "GMO Coin_2",
        exchange: "GMO Coin",
    },
    CexAddress {
        address: address!("976b251AB62371d6d154EF5989F5fA11452fB260"),
        name: "GMO Coin_3",
        exchange: "GMO Coin",
    },
    CexAddress {
        address: address!("9C4fA4Ee466dfF080ADcCb12D39b99e83cdb1077"),
        name: "GMO Coin_4",
        exchange: "GMO Coin",
    },
    CexAddress {
        address: address!("a8EBa6dA3801489093C0299529AC79fC16C59b6d"),
        name: "GMO Coin_5",
        exchange: "GMO Coin",
    },
    CexAddress {
        address: address!("b77c64A1fd89d57e0f26e1a5c0a4d5934aD84350"),
        name: "GMO Coin_6",
        exchange: "GMO Coin",
    },
    CexAddress {
        address: address!("c39BDF685F289B1F261EE9b0b1B2Bf9eae4C1980"),
        name: "GMO Coin_7",
        exchange: "GMO Coin",
    },
    CexAddress {
        address: address!("e89943Ec20d856F064A87349a00Ef6aB00AED042"),
        name: "GMO Coin_8",
        exchange: "GMO Coin",
    },
    CexAddress {
        address: address!("E978d95B437D75826ABa2ED1A0BDb534f173E28c"),
        name: "GMO Coin_9",
        exchange: "GMO Coin",
    },
    CexAddress {
        address: address!("2331f22f1701D79bf102a66672AB4293dB1bF72F"),
        name: "GOPAX",
        exchange: "GOPAX",
    },
    CexAddress {
        address: address!("6707A6763c6Dc64DE7C4048A27b6303292F88F50"),
        name: "GOPAX_1",
        exchange: "GOPAX",
    },
    CexAddress {
        address: address!("84c609976C73C38cDd36895cc998E1ac95734e4e"),
        name: "GOPAX_2",
        exchange: "GOPAX",
    },
    CexAddress {
        address: address!("e3031C1BfaA7825813c562CbDCC69d96FCad2087"),
        name: "GOPAX_3",
        exchange: "GOPAX",
    },
    CexAddress {
        address: address!("15abb66bA754F05cBC0165A64A11cDed1543dE48"),
        name: "Galaxy Digital",
        exchange: "Galaxy",
    },
    CexAddress {
        address: address!("220eaDF10315dCB744Cb4C12d3d8d5F3d1028336"),
        name: "Galaxy Digital_1",
        exchange: "Galaxy Digital",
    },
    CexAddress {
        address: address!("33566c9D8BE6Cf0B23795E0d380E112Be9d75836"),
        name: "Galaxy Digital_2",
        exchange: "Galaxy Digital",
    },
    CexAddress {
        address: address!("46f34C24A7bA7a2Ac6DD76c3F09B32D41C144d08"),
        name: "Galaxy Digital_3",
        exchange: "Galaxy Digital",
    },
    CexAddress {
        address: address!("5797F722b1FeE36e3D2c3481D938d1372bCD99A7"),
        name: "Galaxy Digital_4",
        exchange: "Galaxy Digital",
    },
    CexAddress {
        address: address!("6AE55181F90c954993789546956A8453E63B0015"),
        name: "Galaxy Digital_5",
        exchange: "Galaxy Digital",
    },
    CexAddress {
        address: address!("b6949a1A9C335cE1b73D490b1e134086Ec5718F5"),
        name: "Galaxy Digital_6",
        exchange: "Galaxy Digital",
    },
    CexAddress {
        address: address!("b9C3832e688736ed8c8954d158cB6b00FCa6C8F2"),
        name: "Galaxy Digital_7",
        exchange: "Galaxy Digital",
    },
    CexAddress {
        address: address!("caF66b3C3c343064F19c697E53B126cBb9E7Ad99"),
        name: "Galaxy Digital_8",
        exchange: "Galaxy Digital",
    },
    CexAddress {
        address: address!("F4561C710ba450Aab302Ffc3557EE59Bbce94CA6"),
        name: "Galaxy Digital_9",
        exchange: "Galaxy Digital",
    },
    CexAddress {
        address: address!("d5FBDa4C79F38920159fE5f22DF9655FDe292d47"),
        name: "Gamdom",
        exchange: "Gamdom",
    },
    CexAddress {
        address: address!("05EE546c1a62f90D7aCBfFd6d846c9C54C7cF94c"),
        name: "Gate.io",
        exchange: "Gate.io",
    },
    CexAddress {
        address: address!("0D0707963952f2fBA59dD06f2b425ace40b492Fe"),
        name: "Gate.io_1",
        exchange: "Gate.io",
    },
    CexAddress {
        address: address!("1C4b70a3968436B9A0a9cf5205c787eb81Bb558c"),
        name: "Gate.io_2",
        exchange: "Gate.io",
    },
    CexAddress {
        address: address!("234EE9e35f8e9749A002fc42970D570DB716453B"),
        name: "Gate.io_3",
        exchange: "Gate.io",
    },
    CexAddress {
        address: address!("6596Da8B65995d5feaCfF8c2936f0b7a2051B0D0"),
        name: "Gate.io_4",
        exchange: "Gate.io",
    },
    CexAddress {
        address: address!("7793cD85c11a924478d358D49b05b37E91B5810F"),
        name: "Gate.io_5",
        exchange: "Gate.io",
    },
    CexAddress {
        address: address!("85FAa6C1F2450b9caEA300838981C2e6E120C35c"),
        name: "Gate.io_6",
        exchange: "Gate.io",
    },
    CexAddress {
        address: address!("C882b111A75C0c657fC507C04FbFcD2cC984F071"),
        name: "Gate.io_7",
        exchange: "Gate.io",
    },
    CexAddress {
        address: address!("D793281182A0e3E023116004778F45c29fc14F19"),
        name: "Gate.io_8",
        exchange: "Gate.io",
    },
    CexAddress {
        address: address!("eB01f8cdAE433E7B55023fF0B2DA44C4c712DCE2"),
        name: "Gate.io_9",
        exchange: "Gate.io",
    },
    CexAddress {
        address: address!("07Ee55aA48Bb72DcC6E9D78256648910De513eca"),
        name: "Gemini",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("183b1Ffb0Aa9213b9335AdFAd82E47bfb02f8d24"),
        name: "Gemini_1",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("B1CCe076E720300C4E49b529a7cE0E58d3C0e8fe"),
        name: "Gemini_10",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("b302BFE9c246c6e150AF70b1cAAA5e3Df60dAc05"),
        name: "Gemini_11",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("d24400ae8BfEBb18cA49Be86258a3C749cf46853"),
        name: "Gemini_12",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("D69B0089D9CA950640F5DC9931A41a5965f00303"),
        name: "Gemini_13",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("Dd51F01D9fc0Fd084C1a4737BbFa5Becb6CEd9BC"),
        name: "Gemini_14",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("F51710015536957A01f32558402902A2D9c35d82"),
        name: "Gemini_15",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("3e6722f32CBE5b3C7BD3dcA7017c7FfE1b9E5A2A"),
        name: "Gemini_2",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("485b9a41e8BF06E57BB64c6Ba7cB04f9d53D2d76"),
        name: "Gemini_3",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("4c2F150Fc90fed3d8281114c2349f1906cdE5346"),
        name: "Gemini_4",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("5f65f7b609678448494De4C87521CdF6cEf1e932"),
        name: "Gemini_5",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("61EDCDf5bb737ADffE5043706e7C5bb1f1a56eEA"),
        name: "Gemini_6",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("6Fc82a5fe25A5cDb58bc74600A40A69C065263f8"),
        name: "Gemini_7",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("8D6F396D210d385033b348bCae9e4f9Ea4e045bD"),
        name: "Gemini_8",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("9d549AD2CE668271fB1354Af19B1668fDb86d818"),
        name: "Gemini_9",
        exchange: "Gemini",
    },
    CexAddress {
        address: address!("C1a012E58aD5a229E4a7051e07Bc5bdF2EcB91a2"),
        name: "Graviex",
        exchange: "Graviex",
    },
    CexAddress {
        address: address!("034f854B44D28E26386c1BC37ff9B20C6380b00d"),
        name: "HTX",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("04645AF26b54BD85Dc02Ac65054e87362A72CB22"),
        name: "HTX_1",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("119346062a580Ee98774DF7F0c1C5D1dd7f1AdC0"),
        name: "HTX_10",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("aB5C66752a9e8167967685F1450532fB96d5d24f"),
        name: "HTX_100",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("aC6a692d2cE44fec287084fFa65244baF2f578df"),
        name: "HTX_101",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("adB2B42F6bD96F5c65920b9ac88619DcE4166f94"),
        name: "HTX_102",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("afdfd157d9361e621e476036FEE62f688450692B"),
        name: "HTX_103",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("B2a48f542dc56B89b24C04076cbE565b3Dc58e7b"),
        name: "HTX_104",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("B48931a3673B6351D45f0d01a296F4db7148f485"),
        name: "HTX_105",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("B4Cd0386d2Db86f30C1A11c2B8c4F4185c1Dade9"),
        name: "HTX_106",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("B5E919c001F752501AC603D0c581e24c77165bb2"),
        name: "HTX_107",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("b633a44c8464c9C625E25C18cf210529A1E4cc99"),
        name: "HTX_108",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("B9a4873d8D2C22e56B8574e8605644d08E047549"),
        name: "HTX_109",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("11E4184C68C3d673c96aCc85A03510073d6EB2c8"),
        name: "HTX_11",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("b9F775179bcC7FcF4534700a48F09C590E390eAd"),
        name: "HTX_110",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("BA5cfbc7c1e08156d9E6e0D91a66ad3bCFf7956a"),
        name: "HTX_111",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("bc53B706B165D2B7E98F254095D9d342E845e5aC"),
        name: "HTX_112",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("c589b275e60dDa57aD7E117C6DD837Ab524a5666"),
        name: "HTX_113",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("c837F51A0eFa33F8ECA03570e3D01a4B2CF97FfD"),
        name: "HTX_114",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("c9610bE2843F1618EdFeDd0860DC43551c727061"),
        name: "HTX_115",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("CAc725beF4f114F728cbCfd744a731C2a463c3Fc"),
        name: "HTX_116",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("Ce7Ec11a5F306c6B896526149dB1a86c7d1531E2"),
        name: "HTX_117",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("Cf35a0dE7FE1671c997e32Be36BD2307BaA3DD72"),
        name: "HTX_118",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("d10E08325c0E95D59c607a693483680FE5B755B3"),
        name: "HTX_119",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("1205E4f0D2f02262E667fd72f95a68913b4F7462"),
        name: "HTX_12",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("d3a2f775e973c1671f2047E620448B8662dCD3cA"),
        name: "HTX_120",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("d3Cc0C7d40366A061397274Eae7C387D840e6ff8"),
        name: "HTX_121",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("D4F7C1047E43016434B177222e0706cE71084514"),
        name: "HTX_122",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("d70250731A72C33BFB93016E3D1F0CA160dF7e42"),
        name: "HTX_123",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("d8a83b72377476D0a66683CDe20A8aAD0B628713"),
        name: "HTX_124",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("dB0E89a9B003A28A4055ef772E345E8089987bfd"),
        name: "HTX_125",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("Dc76CD25977E0a5Ae17155770273aD58648900D3"),
        name: "HTX_126",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("dd3CB5c974601BC3974d908Ea4A86020f9999E0c"),
        name: "HTX_127",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("DF95de30cDff4381B69F9e4FA8DDDce31a0128DF"),
        name: "HTX_128",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("dFCCd61E2E919489062224989A54AC6E59F832aa"),
        name: "HTX_129",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("12FA4951CBFC51102ccFd5580beb135cA3D74c54"),
        name: "HTX_13",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("e0B7A39Fef902c21bAd124b144c62E7F85f5f5fA"),
        name: "HTX_130",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("E195B82Df6A797551Eb1ACd506e892531824Af27"),
        name: "HTX_131",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("E3314bbF3334228b257779E28228CfB86fA4261B"),
        name: "HTX_132",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("E3C274D2e2180aFd89EdA4D25320Cff08AaDB680"),
        name: "HTX_133",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("E4818f8fDe0C977A01DA4Fa467365B8bF22b071E"),
        name: "HTX_134",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("e8D8a02601f54AcB6fB69537Be1F1D7cC76ccd8C"),
        name: "HTX_135",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("E93381fB4c4F14bDa253907b18faD305D799241a"),
        name: "HTX_136",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("EA0cFeF143182d7B9208FBfEda9D172c2ACED972"),
        name: "HTX_137",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("EB6D43Fe241fb2320b5A3c9BE9CDfD4dd8226451"),
        name: "HTX_138",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("EBA290cf248cB14442A071fbCb58a9Cc5dcdE28E"),
        name: "HTX_139",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("137ad9C4777E1d36e4b605e745e8F37B2b62E9c5"),
        name: "HTX_14",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("eCD8b3877D8E7cD0739dE18a5b545bc0B3538566"),
        name: "HTX_140",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("Eec606A66edB6f497662Ea31b5eb1610da87AB5f"),
        name: "HTX_141",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("eEe28d484628d41A82d01e21d12E2E78D69920da"),
        name: "HTX_142",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("EF54F559B5e3b55b783C7Bc59850F83514B6149c"),
        name: "HTX_143",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("f0458aAAf6d49192D3b4711960635D5FA2114E71"),
        name: "HTX_144",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("f056F435Ba0CC4fCD2F1B17e3766549fFc404B94"),
        name: "HTX_145",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("F2dbC42875E7764EDBd89732A15214A9a0Deb085"),
        name: "HTX_146",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("f66852bC122fD40bFECc63CD48217E88bda12109"),
        name: "HTX_147",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("f726dc178D1A4d9292A8d63f01e0fA0A1235E65C"),
        name: "HTX_148",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("f775a9a0Ad44807bc15936dF0Ee68902aF1A0EEE"),
        name: "HTX_149",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("170af0A02339743687aFD3dC8d48cfFd1f660728"),
        name: "HTX_15",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("f7A8Af16Acb302351D7Ea26ffc380575B741724c"),
        name: "HTX_150",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("F881bCB3705926cEa9C598aB05a837cf41a833a9"),
        name: "HTX_151",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("FA4B5Be3f2f84f56703C42eB22142744E95a2c58"),
        name: "HTX_152",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("Fd54078bAdD5653571726C3370AfB127351a6f26"),
        name: "HTX_153",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("fdb16996831753d5331fF813c29a93c76834A0AD"),
        name: "HTX_154",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("18709E89BD403F470088aBDAcEbE86CC60dda12e"),
        name: "HTX_16",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("18916e1a2933Cb349145A280473A5DE8EB6630cb"),
        name: "HTX_17",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("1B93129F05cc2E840135AAB154223C75097B69bf"),
        name: "HTX_18",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("1C515Eaa87568c850043a89C2D2C2e8187Adb056"),
        name: "HTX_19",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("0511509A39377F1C6c78DB4330FBfcC16D8A602f"),
        name: "HTX_2",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("1D1E10e8c66B67692f4C002C0cB334De5D485e41"),
        name: "HTX_20",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("2177c77A1f3c4900De7668662706633DB4688726"),
        name: "HTX_21",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("229b5c097F9b35009CA1321Ad2034D4b3D5070F6"),
        name: "HTX_22",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("25C6459e5c5b01694F6453E8961420CcD1EDF3b1"),
        name: "HTX_23",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("27E9F4748a2eb776bE193a1F7dec2Bb6DAAfE9Cf"),
        name: "HTX_24",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("2886DAC7bAe6C45B05BB483A6C709fA6501A765a"),
        name: "HTX_25",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("28FFE35688fFFfd0659AEE2E34778b0ae4E193aD"),
        name: "HTX_26",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("296d69073d5D2E0Dc51B768ee83c0f9F14A0bfE2"),
        name: "HTX_27",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("2Abc22eb9A09EbBE7b41737CCde147F586EfeB6A"),
        name: "HTX_28",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("2bB1881b1DdE050fdf756c25E025aA9367b4aFFC"),
        name: "HTX_29",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("0577a79Cfc63Bbc0Df38833Ff4C4a3BF2095b404"),
        name: "HTX_3",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("30741289523c2e4d2A62c7D6722686D14E723851"),
        name: "HTX_30",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("32598293906b5b17c27d657dB3AD2c9b3f3E4265"),
        name: "HTX_31",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("34189c75Cbb13Bdb4F5953CDa6c3045CFcA84a9e"),
        name: "HTX_32",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("3634b91e54c4dCd7919772F225fC5532F91d8F92"),
        name: "HTX_33",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("36d86971BebACd424AEb0BB164c629d61f4557A2"),
        name: "HTX_34",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("39d9f4640b98189540A9C0edCFa95C5e657706aA"),
        name: "HTX_35",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("3c979fB790c86e361738ED17588c1e8b4C4cc49A"),
        name: "HTX_36",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("3cb013F6704EB8E22923f02Bb8E7C8D4Bd7541CF"),
        name: "HTX_37",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("3d655889D197125fb90dcB72e4a287A8410ED1B9"),
        name: "HTX_38",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("42dc966b7eCc3c6cc73e7bc04862859D5bDDCE65"),
        name: "HTX_39",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("07EF60deCa209Ea0F3f3f08C1aD21a6DB5EF9D33"),
        name: "HTX_4",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("46705dfff24256421A05D056c29E81Bdc09723B8"),
        name: "HTX_40",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("48AB9f29795DFB44B36587C50da4b30c0E84d3ed"),
        name: "HTX_41",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("49517CA7b7a50f592886D4c74175F4C07D460a70"),
        name: "HTX_42",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("4d77a1144dC74f26838B69391a6D3B1e403D0990"),
        name: "HTX_43",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("508Ab7C228951BEc9c33EB5613Fc9AaB726Fc74C"),
        name: "HTX_44",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("53cFD2c9eB387Cce8F5d111a4352ab7aa4333b18"),
        name: "HTX_45",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("5401dBf7da53e1C9Dbf484E3d69505815F2f5e6e"),
        name: "HTX_46",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("56a5D322AE2E2F5AfC6716addacA4E62276aE9B6"),
        name: "HTX_47",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("5861b8446A2F6e19a067874c133f04c578928727"),
        name: "HTX_48",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("58c2cb4a6BeE98C309215D0d2A38d7F8aa71211c"),
        name: "HTX_49",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("08DeB6278D671E2a1aDc7b00839b402B9cF3375d"),
        name: "HTX_5",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("598273eA2CAbD9F798564877851788c5e0d5b7b9"),
        name: "HTX_50",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("5BD5EdcCa85c0c035C9B2F5b4e22683FE37F6677"),
        name: "HTX_51",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("5c8A9F68eFF7Df9f291cE8408b797C4e7B9a95CB"),
        name: "HTX_52",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("5C985E89DDe482eFE97ea9f1950aD149Eb73829B"),
        name: "HTX_53",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("5E9dB2797C2F2775247881d3535271b212bD79D6"),
        name: "HTX_54",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("5F477d105EFa3bCc2F74f0e9A008bcAA988a8E8A"),
        name: "HTX_55",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("60aA247146d47Aee2B1C136540C55261a4b8342c"),
        name: "HTX_56",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("60B45F993223DcB8bdF05e3391f7630E5a51D787"),
        name: "HTX_57",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("636B76AE213358b9867591299E5c62B8d014E372"),
        name: "HTX_58",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("6496Be0EaeF40cd4497a6E9DCA28Fe5441b4AcD1"),
        name: "HTX_59",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("0A98fB70939162725aE66E626Fe4b52cFF62c2e5"),
        name: "HTX_6",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("664C900aD79A7E9e3E1BbdD238715744AC9b575B"),
        name: "HTX_60",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("6663613FbD927cE78abBF7F5Ca7e2c3FE0d96d18"),
        name: "HTX_61",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("6748F50f686bfbcA6Fe8ad62b22228b87F31ff2b"),
        name: "HTX_62",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("6b2286FC3a9265bab3F064808022acA54dE4B6cE"),
        name: "HTX_63",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("6EdB9d6547BEFc3397801C94Bb8C97d2e8087E2F"),
        name: "HTX_64",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("6F48a3E70F0251d1e83a989e62aAa2281A6d5380"),
        name: "HTX_65",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("6F50C6Bff08Ec925232937B204B0ae23C488402a"),
        name: "HTX_66",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("70383BfF83a9504049A3759b892e4bb1Ec10b806"),
        name: "HTX_67",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("73f8FC2e74302eb2EfdA125A326655aCF0DC2D1B"),
        name: "HTX_68",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("74956D3Ab75af393cd74c8410129d27997f797f9"),
        name: "HTX_69",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("0c6C34CDd915845376fb5407E0895196C9DD4eeC"),
        name: "HTX_7",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("75a83599dE596cBC91a1821fFA618C40e22ac8CA"),
        name: "HTX_70",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("76b66147709f45eDB1a95c469F8F076b2aBb8f2D"),
        name: "HTX_71",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("77Fe06EF6A614d292A6342df1bc2CB7aEBBbe856"),
        name: "HTX_72",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("794d28aC31bCB136294761a556b68D2634094153"),
        name: "HTX_73",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("79795ecce9e9538E886Df5312088999D7Fa817bC"),
        name: "HTX_74",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("7EF35bb398E0416b81b019fEa395219B65c52164"),
        name: "HTX_75",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("80E63D2735789Fa1F676B7644236d501133986dD"),
        name: "HTX_76",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("82D015d74670d8645b56c3f453398a3E799Ee582"),
        name: "HTX_77",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("8AaBBA0077f1565Df73e9D15dd3784a2b0033DAd"),
        name: "HTX_78",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("8b6a3587676719A4FeCBb24b503a3634C44A44d5"),
        name: "HTX_79",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("0C92EfA186074Ba716d0E2156A6FFAbD579f8035"),
        name: "HTX_8",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("8E8bc99b79488c276D6f3Ca11901E9AbD77efEa4"),
        name: "HTX_80",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("90E9dDD9d8D5ae4E3763d0CF856C97594DEA7325"),
        name: "HTX_81",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("90f49e24A9554126F591D28174e157CA267194Ba"),
        name: "HTX_82",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("918800E018A0Eeea672740F88A60091c7D327a79"),
        name: "HTX_83",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("91dFa9d9E062A50D2f351bfbA0d35A9604993DaC"),
        name: "HTX_84",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("926fC576b7facF6aE2d08eE2D4734C134a743988"),
        name: "HTX_85",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("956e0DBEcC0e873d34a5e39B25f364b2CA036730"),
        name: "HTX_86",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("99fe5D6383289CDD56e54Fc0bAF7F67c957A8888"),
        name: "HTX_87",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("9A31CFBDDd97276e43442f34e971decf6DC2D211"),
        name: "HTX_88",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("9A755332D874c893111207b0b220Ce2615cd036F"),
        name: "HTX_89",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("1062a747393198f70F71ec65A582423Dba7E5Ab3"),
        name: "HTX_9",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("9A7ffD7F6c42ab805e0eDF16c25101964C6326B6"),
        name: "HTX_90",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("9d6d492bD500DA5B33cf95A5d610a73360FcaAa0"),
        name: "HTX_91",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("9Eebb2815dbA2166d8287AFa9A2C89336ba9DEaA"),
        name: "HTX_92",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("9ef21bE1C270AA1c3c3d750F458442397fBFFCB6"),
        name: "HTX_93",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("a03400E098F4421b34a3a44A1B4e571419517687"),
        name: "HTX_94",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("A23d7dd4b8a1060344CAF18A29b42350852AF481"),
        name: "HTX_95",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("a5D7F0F7027fa8F4D1BE8042E1e43bbdEc36951e"),
        name: "HTX_96",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("a77ff0e1C52f58363a53282624C7BaA5fA91687D"),
        name: "HTX_97",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("a8660c8ffD6D578F657B72c0c811284aef0B735e"),
        name: "HTX_98",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("A91183d7DfcFe39D923071F5527552ab52DA6d44"),
        name: "HTX_99",
        exchange: "HTX",
    },
    CexAddress {
        address: address!("ffe15FF598e719d29DFe5E1d60BE1A5521A779Ae"),
        name: "HashKey Exchange",
        exchange: "HashKey",
    },
    CexAddress {
        address: address!("0113a6b755fBaD36B4249Fd63002e2035E401143"),
        name: "HitBTC",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("04EcC5ba3258840725FD808BE0fAE3430B8346F2"),
        name: "HitBTC_1",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("1aD9461fda9C678ea768B9161E9EFe26b9965f64"),
        name: "HitBTC_10",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("1af06e390EC67cf9B14cc092558279AED0Dd923A"),
        name: "HitBTC_11",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("1F07D0C207bBba4B7745C3604215b0262e56c6A0"),
        name: "HitBTC_12",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("24e54449c11aAe288033b12746CF552071d449e8"),
        name: "HitBTC_13",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("265a9cb831EAa7FdD7cd2FF80aFEc4eA79E2Ac7A"),
        name: "HitBTC_14",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("300615A4AFDBefDC79819D7434defD18a16463C7"),
        name: "HitBTC_15",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("303C8F04773A7Ef6608D9E6c7dFb497D0A0B0A9D"),
        name: "HitBTC_16",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("379E981f8E51628143bDbaF5464F9436b937D321"),
        name: "HitBTC_17",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("37d3e950dc5cEb6309cbc5Bfa456cAAEDd179Dd4"),
        name: "HitBTC_18",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("3a7d3565DE0038E0AEEe32c02D47d5d8b599E87A"),
        name: "HitBTC_19",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("05EAC110625CB1715988b1757A94CB0f0548cF05"),
        name: "HitBTC_2",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("3E3B18E821dB7B6d44ea445730847eb18fF8135F"),
        name: "HitBTC_20",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("422026c9fE5DD65Dc259Ef1140735bDd952CC2eB"),
        name: "HitBTC_21",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("469A0EFadF0A5404859f269A97459d6A48cB444a"),
        name: "HitBTC_22",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("487628D81Ce77fA8CE5aebdeF6eFF7C940ADD15B"),
        name: "HitBTC_23",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("4C34aE54Dc716808e94Af3d1d638b8EA3A23fA9B"),
        name: "HitBTC_24",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("4FE6C1a64E7fBFf55a6517eF85489CA187A91BBF"),
        name: "HitBTC_25",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("5255eCdC334c3D3f4D0DeB0cB5657b64881db46F"),
        name: "HitBTC_26",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("5871FCC9af1016185F1f6732C518c73f4821C16F"),
        name: "HitBTC_27",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("59a5208B32e627891C389EbafC644145224006E8"),
        name: "HitBTC_28",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("5B7934cdBb5cD076bd486e0f017AeB777bf0D04c"),
        name: "HitBTC_29",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("085489C3A85c51d5a443cFBDeaf9F13dda961CE0"),
        name: "HitBTC_3",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("644bC076f4A098445971E75bE8fb887d7EC1C6Fb"),
        name: "HitBTC_30",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("6543Ea3d1eEAEF316eF1d5EBef30a6a0B8f5dd76"),
        name: "HitBTC_31",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("65E2C5175e2E618F48E70343b14C31B280E42d90"),
        name: "HitBTC_32",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("688C09e6Bb9bE28ae7aE3bfEebC0A65d532B3305"),
        name: "HitBTC_33",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("6C18d587D762A347d025A4Bd027d9f9cBa2E0736"),
        name: "HitBTC_34",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("6E9470664abc929C1bb98f7860cEf82E2aB7b678"),
        name: "HitBTC_35",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("6EB53062ef576dF1c4EB5CA326B866590571bcbd"),
        name: "HitBTC_36",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("6F9E3de5F5cFc53a0ceC58D5CEc8C9abb99a4F78"),
        name: "HitBTC_37",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("6Fd591EAEE4AAf90ACa728F3b02799b99b7fD18e"),
        name: "HitBTC_38",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("7516952EA89721cF97990cF232AbbBd4C6A922f7"),
        name: "HitBTC_39",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("09EB27139BbFe4B97458Cb85875E94A3a4D74a2e"),
        name: "HitBTC_4",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("76d0f5dDB248F03d7d38444E15CEe5161Fa66aDF"),
        name: "HitBTC_40",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("77D0c7F4eBCBB1e1D27036C7a9725391ab3cE8D7"),
        name: "HitBTC_41",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("78C1B2fC2484cD112CC980c3CB2F12c8e30B4c94"),
        name: "HitBTC_42",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("79bE4dCe73B64667BA06c05d011fABBc9FD240a8"),
        name: "HitBTC_43",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("7b8067d5CA62A9BA370360F72C951e821669Ba0a"),
        name: "HitBTC_44",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("7e02AB6a31580e0320266cD53e9E5C49dF0B2Fdd"),
        name: "HitBTC_45",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("85cC44702763ADe958bffD6F3257Fa7cF2378d75"),
        name: "HitBTC_46",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("8c2C0235c9838a48Be4585304D1aedDcc2eaBc85"),
        name: "HitBTC_47",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("9089C54813a793AC6c911bf13e9909b6072790D4"),
        name: "HitBTC_48",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("93bc5582bC721F30f8279d03346A9e708ddB0672"),
        name: "HitBTC_49",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("0DC1DF88B06a437dD22C5CE0466ccBFe1a4C12A4"),
        name: "HitBTC_5",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("9431003113aeC4372d4FdfbA08845f3939a1F8B2"),
        name: "HitBTC_50",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("97952751dc220aa755CA1B7bDa9F0d3D8DeF660D"),
        name: "HitBTC_51",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("989D7a06345F89662De142b6F9Eeb1f8c8022Af4"),
        name: "HitBTC_52",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("9C67e141C0472115AA1b98BD0088418Be68fD249"),
        name: "HitBTC_53",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("9CB95Ee6A4FbA2c8898F32254a26Cea74Ad7E26F"),
        name: "HitBTC_54",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("A12431D0B9dB640034b0CDFcEEF9CCe161e62be4"),
        name: "HitBTC_55",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("A8220Ce798eb9a75883189FBDc9a8cF633aD1795"),
        name: "HitBTC_56",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("aD68942A95fDd56594aA5cF862B358790E37834C"),
        name: "HitBTC_57",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("b02A5c8C7844bf6ce02ad55a95645B03e6bD5c9e"),
        name: "HitBTC_58",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("BFCd86e36D947A9103A7D4a95d178A432723d6aD"),
        name: "HitBTC_59",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("1366A2ca67594fFD5174d0216d60D9Ea8DEB511F"),
        name: "HitBTC_6",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("cb4a847dB63faFA275e4f53114f4DdF09e600b65"),
        name: "HitBTC_60",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("D690a9F0909acb7fBeE8F7915137000D2ff851eF"),
        name: "HitBTC_61",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("DCA8B1E595708fB891009C541BAFFD376BAFbB92"),
        name: "HitBTC_62",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("dee41DE15B86367d39A8BBd021bF6d527630fa74"),
        name: "HitBTC_63",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("E18cE1Fce73e6b8343b616C9ca0Bd50727d9b357"),
        name: "HitBTC_64",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("e5cd51c7523D78C14B004e50526d6D532F56Ba77"),
        name: "HitBTC_65",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("e6e7b66daB1A20A548fA100f4773A5904Bb69158"),
        name: "HitBTC_66",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("f07232Bc85D995c32C1EDf1C985C84A8B7b0DEd7"),
        name: "HitBTC_67",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("F6Ab4b2EF674911C3cBf2197334C72C57e070C33"),
        name: "HitBTC_68",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("F6b63e252ffB19B676410c966d23442e79709210"),
        name: "HitBTC_69",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("1484cAdeE1A93E04D02cab154BB84Bc767270936"),
        name: "HitBTC_7",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("FBA17B00768d9109045433AEE66c70E5215F0B75"),
        name: "HitBTC_70",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("FDeda15e2922C5ed41fc1fdF36DA2FB2623666b3"),
        name: "HitBTC_71",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("15c6A9291e296E5C91727D9C01c55337c6757a55"),
        name: "HitBTC_8",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("17be334446EEef539BE497c297Eb902BE030D227"),
        name: "HitBTC_9",
        exchange: "HitBTC",
    },
    CexAddress {
        address: address!("0000Bf7f4e4B7fb2315fc6D5d0F8854c91dfF1d8"),
        name: "Hoo.com",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("008932bE50098089C6a075D35F4B5182EE549F8A"),
        name: "Hoo.com_1",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("980a4732c8855Ffc8112E6746bd62095b4c2228F"),
        name: "Hoo.com_10",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("993b7fcba51d8F75C2DfAeC0d17B6649ee0C9068"),
        name: "Hoo.com_11",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("D0Ec209AD2134899148bEc8aEF905A6E9997456A"),
        name: "Hoo.com_12",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("Ec293b9C56f06C8f71392269313D7e2Da681D9aC"),
        name: "Hoo.com_13",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("eed85Ddd6828B8C714141d34A12D0c07F7553182"),
        name: "Hoo.com_14",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("0093e5f2A850268c0ca3093c7EA53731296487eB"),
        name: "Hoo.com_2",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("00D46709171b0796c90024f55CAcfc92acbacdA7"),
        name: "Hoo.com_3",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("00DdB997C6b6015e187E9A242FF081df23C2DAC8"),
        name: "Hoo.com_4",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("05f0fDD0E49A5225011fff92aD85cC68e1D1F08e"),
        name: "Hoo.com_5",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("21169023f11B38EF9C95BA3aABd295be8e5a96Ec"),
        name: "Hoo.com_6",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("4d4FfB448194504242267585F0Ea6f9de6A96DE3"),
        name: "Hoo.com_7",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("74a73578e56b43aa9c088e013C5B7Bd91aA5221a"),
        name: "Hoo.com_8",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("9760862d09B70433a91Ff27cBD069f51ef1cbd5c"),
        name: "Hoo.com_9",
        exchange: "Hoo.com",
    },
    CexAddress {
        address: address!("274F3c32C90517975e29Dfc209a23f315c1e5Fc7"),
        name: "Hotbit",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("39aa02D6B499a76A70faB5F164e8Be587C366141"),
        name: "Hotbit_1",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("8533A0bd9310Eb63E7CC8E1116c18a3D67B1976A"),
        name: "Hotbit_10",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("A986266B60390E1d313Ecd3B2311A9aF74c57400"),
        name: "Hotbit_11",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("B18fbFe3d34FdC227eB4508cdE437412B6233121"),
        name: "Hotbit_12",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("B34eD85bc0B9DA2fA3C5e5d2f4B24f8EE96CE4E9"),
        name: "Hotbit_13",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("b4E95e70015eeb04a23e0FCB6BAE06835310B356"),
        name: "Hotbit_14",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("c62A0781934744E05927ceABB94a3043CdCfEA89"),
        name: "Hotbit_15",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("C7029E939075F48fa2D5953381660c7d01570171"),
        name: "Hotbit_16",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("d690a9DfD7e4B02898Cdd1a9E50eD1fd7D3d3442"),
        name: "Hotbit_17",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("fa6cf22527d88270eEa37F45aF1808AdBF3C1B17"),
        name: "Hotbit_18",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("4A41D76B0524a3989998380c033f12bfEb5f7201"),
        name: "Hotbit_2",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("4b81c7Ff6912856AFBb40ACb32084A41F019B433"),
        name: "Hotbit_3",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("4E568bffd661031993d530456d85Fb37DA2C30AE"),
        name: "Hotbit_4",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("562680a4dC50ed2f14d75BF31f494cfE0b8D10a1"),
        name: "Hotbit_5",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("5Ecddc1AC099074ae965D140A7c62bd71B7Fc80a"),
        name: "Hotbit_6",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("66F7Bbf25c07e5D407A80010B0e9Ba96bF5A2A3E"),
        name: "Hotbit_7",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("6C2e8d4F73f6A129843d1b3D2ACAFF1DB22E3366"),
        name: "Hotbit_8",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("768F2A7CcdFDe9eBDFd5Cea8B635dd590Cb3A3F1"),
        name: "Hotbit_9",
        exchange: "Hotbit",
    },
    CexAddress {
        address: address!("3c11c3025ce387D76C2eDDf1493eC55a8cC2A0f7"),
        name: "IDAX",
        exchange: "IDAX",
    },
    CexAddress {
        address: address!("408af76966D49e57D79aAA6bD55f5162476B051B"),
        name: "IDAX_1",
        exchange: "IDAX",
    },
    CexAddress {
        address: address!("6E743CdC08F22578f5FE99779aa2EdC019363483"),
        name: "IDAX_2",
        exchange: "IDAX",
    },
    CexAddress {
        address: address!("BD6d79F3f02584cfcB754437Ac6776c4C6E0a0eC"),
        name: "IDAX_3",
        exchange: "IDAX",
    },
    CexAddress {
        address: address!("8af97264482B59c7AA11010907710DEe6d8D8c6C"),
        name: "IDEX",
        exchange: "IDEX",
    },
    CexAddress {
        address: address!("9A79994Bf5b887FF7e909498c14DD896c62FF891"),
        name: "IDEX_1",
        exchange: "IDEX",
    },
    CexAddress {
        address: address!("A7a7899d944fE658c4B0a1803BAB2F490bd3849e"),
        name: "IDEX_2",
        exchange: "IDEX",
    },
    CexAddress {
        address: address!("154Af3E01eC56Bc55fD585622E33E3dfb8a248d8"),
        name: "Iconomi",
        exchange: "Iconomi",
    },
    CexAddress {
        address: address!("4F003663aB7C6CcAD2B14688B1d1f8332763F0a9"),
        name: "Iconomi_1",
        exchange: "Iconomi",
    },
    CexAddress {
        address: address!("B081886Da8d77Fff190dCA39528ce1ceA6151096"),
        name: "IndoEx LTD",
        exchange: "IndoEx",
    },
    CexAddress {
        address: address!("B1A34309AF7f29B4195a6b589737f86E14597DdC"),
        name: "IndoEx LTD_1",
        exchange: "IndoEx LTD",
    },
    CexAddress {
        address: address!("C43F9c2f2B61f3dEC87E6c3F0f33C150A6526Ace"),
        name: "IndoEx LTD_2",
        exchange: "IndoEx LTD",
    },
    CexAddress {
        address: address!("Da793AC155a2aF190015F7393B2A36AFC94B7EcF"),
        name: "IndoEx LTD_3",
        exchange: "IndoEx LTD",
    },
    CexAddress {
        address: address!("3C02290922a3618A4646E3BbCa65853eA45FE7C6"),
        name: "Indodax",
        exchange: "Indodax",
    },
    CexAddress {
        address: address!("51836A753E344257B361519E948ffCAF5fb8d521"),
        name: "Indodax_1",
        exchange: "Indodax",
    },
    CexAddress {
        address: address!("8Ab399CBB9FDB9a36518a7e7EddF89158E56c595"),
        name: "Indodax_2",
        exchange: "Indodax",
    },
    CexAddress {
        address: address!("9554EFa1669014C25070BC23C2dF262825704228"),
        name: "Indodax_3",
        exchange: "Indodax",
    },
    CexAddress {
        address: address!("9CbADD5Ce7E14742F70414A6DcbD4e7bB8712719"),
        name: "Indodax_4",
        exchange: "Indodax",
    },
    CexAddress {
        address: address!("0810Cc2d46fC76355E33E10f19D3F04d03A11ece"),
        name: "JPEX",
        exchange: "JPEX",
    },
    CexAddress {
        address: address!("50c85E5587d5611cf5cDFBa23640BC18b3571665"),
        name: "JPEX_1",
        exchange: "JPEX",
    },
    CexAddress {
        address: address!("9528043B8Fc2a68380F1583C389a94dcd50d085e"),
        name: "JPEX_2",
        exchange: "JPEX",
    },
    CexAddress {
        address: address!("a72Ad701807e5902F458e1844D560128F3F57750"),
        name: "JPEX_3",
        exchange: "JPEX",
    },
    CexAddress {
        address: address!("D2f41167a391014Bc5df2234BE7f3a64a06a9fD7"),
        name: "JPEX_4",
        exchange: "JPEX",
    },
    CexAddress {
        address: address!("88880809D6345119cCABe8A9015e4b1309456990"),
        name: "Juno",
        exchange: "Juno",
    },
    CexAddress {
        address: address!("42D17b7f3532Ec2f7C4E4e5E239BAA476846E2CD"),
        name: "Kanga Exchange",
        exchange: "Kanga",
    },
    CexAddress {
        address: address!("352BDaBe484499e4c25c3536CC3edA1edbC5ad29"),
        name: "KickEX",
        exchange: "KickEX",
    },
    CexAddress {
        address: address!("3bb14B49Da0c47a0B2a2a112BEde35655C39a032"),
        name: "KickEX_1",
        exchange: "KickEX",
    },
    CexAddress {
        address: address!("aF4ff15C9809e246111802f04A6acC7160992feF"),
        name: "KickEX_2",
        exchange: "KickEX",
    },
    CexAddress {
        address: address!("C153121042832Ac11587eBE361b8dc3cCd90e9E4"),
        name: "KickEX_3",
        exchange: "KickEX",
    },
    CexAddress {
        address: address!("4a5F98e2C2784d359FC0deCc8533Ae27AF0e5974"),
        name: "Klever",
        exchange: "Klever",
    },
    CexAddress {
        address: address!("5a57cfAFE8b9E94419Cc7d0Cb1F4a95C73f40110"),
        name: "Klever_1",
        exchange: "Klever",
    },
    CexAddress {
        address: address!("5AF8da2675DD31BEffa2619145957B15E8013f37"),
        name: "Klever_2",
        exchange: "Klever",
    },
    CexAddress {
        address: address!("91af50AdB57283283C8B442622e95c26D46D911c"),
        name: "Klever_3",
        exchange: "Klever",
    },
    CexAddress {
        address: address!("96c38EEeD002d3Df2e369DefFe6cc84688eAdb01"),
        name: "Klever_4",
        exchange: "Klever",
    },
    CexAddress {
        address: address!("0c01089AEdc45Ab0F43467CCeCA6B4d3E4170bEa"),
        name: "Korbit",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("223674Cc4433A50CAddF13c65F92151d75996E41"),
        name: "Korbit_1",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("d17e26529E5fca53901f65F1A914317877CAB08a"),
        name: "Korbit_10",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("d6e0F7dA4480b3AD7A2C8b31bc5a19325355CA15"),
        name: "Korbit_11",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("D77f7f8868F20835FdFc8c7E851f6f23cE9F651D"),
        name: "Korbit_12",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("e5d7CcC5fc3b3216C4DFF3a59442F1d83038468C"),
        name: "Korbit_13",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("E83a48CaE4d7120e8bA1C2E0409568fFBA532E87"),
        name: "Korbit_14",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("f0bc8FdDB1F358cEf470D63F96aE65B1D7914953"),
        name: "Korbit_15",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("f6230e7E98D2BBeBF96d14888020E9c3E8c27d69"),
        name: "Korbit_16",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("2864DE013415B6c2C7A96333183B20f0F9cC7532"),
        name: "Korbit_2",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("33DE12b5Cf336692c6b7cfD3aAF779425067f21c"),
        name: "Korbit_3",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("3A70c2528265F0624E7ac8495B212A367b5b61b2"),
        name: "Korbit_4",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("455aE9F6d25fD49ED7bE9F3D8Dd1Cf58a79a5958"),
        name: "Korbit_5",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("5BD811987Ee931Ac85ed0eDA8871282F9c5C88A4"),
        name: "Korbit_6",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("8550E644D74536f1DF38B17D5F69aa1BFe28aE86"),
        name: "Korbit_7",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("8E2040aB7A6af6BBA67e6d9b280c6feA7F930C87"),
        name: "Korbit_8",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("D03be958e6b8da2D28aC8231a2291d6E4f0a7ea7"),
        name: "Korbit_9",
        exchange: "Korbit",
    },
    CexAddress {
        address: address!("012480c08d20a14CF3Cb495e942a94dd926DCc8f"),
        name: "Kraken",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("098cAE2DEBceDCeDcAF71e43C1c055C0Ec369492"),
        name: "Kraken_1",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("24B4eAE904632c53ee231e3bd6C4444745Ce22c0"),
        name: "Kraken_10",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("267be1C1D684F78cb4F6a176C4911b741E4Ffdc0"),
        name: "Kraken_11",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("26a78D5b6d7a7acEEDD1e6eE3229b372A624d8b7"),
        name: "Kraken_12",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("2910543Af39abA0Cd09dBb2D50200b3E800A63D2"),
        name: "Kraken_13",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("29728D0efd284D85187362fAA2d4d76C2CfC2612"),
        name: "Kraken_14",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("2a62C4aCcA1A166Ee582877112682cAe8Cc0ffe7"),
        name: "Kraken_15",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("2C7C03cF85ec621bF997E425f550A6683D6d60f3"),
        name: "Kraken_16",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("2d070ed1321871841245D8EE5B84bD2712644322"),
        name: "Kraken_17",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("3FF7215004Fea03c2C745E0476e3f412050e04D1"),
        name: "Kraken_18",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("43984D578803891dfa9706bDEee6078D80cFC79E"),
        name: "Kraken_19",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("098cbdd8eb01b19D37539644821772e9bdE12D55"),
        name: "Kraken_2",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("4442c3E6B5f22B8b4dc3c9329be6c850C5779E85"),
        name: "Kraken_20",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("490b1E689Ca23be864e55B46bf038e007b528208"),
        name: "Kraken_21",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("491f5512751F5dB45c5415049D423aFfAaE70392"),
        name: "Kraken_22",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("4B6f17856215eab57c29ebfA18B0a0F74A3627bb"),
        name: "Kraken_23",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("4C6007e38Ce164Ed80FF8Ff94192225FcdAC68CD"),
        name: "Kraken_24",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("52F5F2adD61c835ff10550402A46621EBd1071D5"),
        name: "Kraken_25",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("53aB4a93B31F480d17D3440a6329bDa86869458A"),
        name: "Kraken_26",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("53d284357ec70cE289D6D64134DfAc8E511c8a3D"),
        name: "Kraken_27",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("555e179d64335945Fc6B155B7235a31B0a595542"),
        name: "Kraken_28",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("62ac55b745F9B08F1a81DCbbE630277095Cf4Be1"),
        name: "Kraken_29",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("0A869d79a7052C7f1b55a8EbAbbEa3420F0D1E13"),
        name: "Kraken_3",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("66c57bF505A85A74609D2C83E94Aabb26d691E1F"),
        name: "Kraken_30",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("6d0Cf1F651f5Ae585d24DcaA188d44E389E93D26"),
        name: "Kraken_31",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("7217F8A697713f6F7DE06CFcD80B76A2CbA375f0"),
        name: "Kraken_32",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("72e93123e8b5D168246739CDC45360ea11209364"),
        name: "Kraken_33",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("735FD3c55A8be1aEb3544C7e29eBa3ea23500A1C"),
        name: "Kraken_34",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("79990a901281bEe059BB3F4D7Db477F7495e2049"),
        name: "Kraken_35",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("808E5374106E820aE54662Fcf8a5e3CCA6aFA13D"),
        name: "Kraken_36",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("89e51fA8CA5D66cd220bAed62ED01e8951aa7c40"),
        name: "Kraken_37",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("8a108e4761386c94b8d2f98A5fFe13E472cFE76a"),
        name: "Kraken_38",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("8aF3827a41c26C7F32C81E93bb66e837e0210D5c"),
        name: "Kraken_39",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("0E33Be39B13c576ff48E14392fBf96b02F40Cd34"),
        name: "Kraken_4",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("8dFeE7DbD859989bF3E80c4f64c42b7Cd283671b"),
        name: "Kraken_40",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("8f9c79B9De8b0713dCAC3E535fc5A1A92DB6EA2D"),
        name: "Kraken_41",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("92927a664c88449318E14D0fD582c787AE2cd934"),
        name: "Kraken_42",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("9c2bd617b77961ee2c5e3038dFb0c822cb75d82a"),
        name: "Kraken_43",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("9Da5812111DCBD65fF9b736874a89751A4F0a2F8"),
        name: "Kraken_44",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("a054611c5B224a5F4ce93Ac2A5f8d0Ed17813402"),
        name: "Kraken_45",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("A1CDBC1a4178c17116Bdb56C946e8B0757C8dcec"),
        name: "Kraken_46",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("a24787320ede4CC19D800bf87B41Ab9539c4dA9D"),
        name: "Kraken_47",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("A25aA6DFBf6d9bbd7a6A9eb47B9f1e57a2BD92d7"),
        name: "Kraken_48",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("A2f443492bbB1b041FCEee5194e2bc133ecC6407"),
        name: "Kraken_49",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("0eF6AEB825dc4c9983d551F8aFEfaAE9d79165C6"),
        name: "Kraken_5",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("a40dFEE99E1C85DC97Fdc594b16A460717838703"),
        name: "Kraken_50",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("a4A6A282A7fC7F939e01D62D884355d79f5046C1"),
        name: "Kraken_51",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("A83B11093c858c86321FBc4c20FE82cdbd58E09E"),
        name: "Kraken_52",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("a861678beE80035114B47615142e9302139a8c32"),
        name: "Kraken_53",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("AdaE2f3B0dB76cb3eaFe76A8Bf99b93f099C140a"),
        name: "Kraken_54",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("Ae2D4617c862309A3d75A0fFB358c7a5009c673F"),
        name: "Kraken_55",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("b874005cbEa25C357b31C62145b3AEF219d105CF"),
        name: "Kraken_56",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("c6bed363b30DF7F35b601a5547fE56cd31Ec63DA"),
        name: "Kraken_57",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("cD0267c7F1A8Ad1b6e33B7fB801F8D935F6B557D"),
        name: "Kraken_58",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("CdC8488E63A403BfD580222ea0F3719477bfea9C"),
        name: "Kraken_59",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("10593a64B7b7BB0Ea29B8c01F1619ca8fF294b2F"),
        name: "Kraken_6",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("cE27fC71139d02f9A3D5Cc1356Add185750660Ac"),
        name: "Kraken_60",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("D0AD6ed2B2920a5744a064af3d585Ee54F528B2F"),
        name: "Kraken_61",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("D4039ECC40AedA0582036437cf3ec02845DA4C13"),
        name: "Kraken_62",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("d88545d0034C245857d1523bb4e8686BcED9Bb85"),
        name: "Kraken_63",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("DA9dfA130Df4dE4673b89022EE50ff26f6EA73Cf"),
        name: "Kraken_64",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("e6a02eeFC2612b13f2B3B914009576ce5495Ec0e"),
        name: "Kraken_65",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("E7178aD747f2C12aB1F8332E61Cf6E756815D5C6"),
        name: "Kraken_66",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("e84F75FC9cAA49876d0Ba18d309da4231d44E94D"),
        name: "Kraken_67",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("e850b7c87F66371035e184C72d4B99e7b2ca4865"),
        name: "Kraken_68",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("E853c56864A2ebe4576a807D26Fdc4A0adA51919"),
        name: "Kraken_69",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("16B2b042f15564Bb8585259f535907F375Bdc415"),
        name: "Kraken_7",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("e9f7eCAe3A53D2A67105292894676b00d1FaB785"),
        name: "Kraken_70",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("EA578Aae84b156010b5df7759a08Cf6E6D6fC288"),
        name: "Kraken_71",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("f1f7648f81F5219C36d75D24D33811f16B426DBe"),
        name: "Kraken_72",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("F7Cd385CB9a442358B892B14301F6310e57CC5c9"),
        name: "Kraken_73",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("Fa52274DD61E1643d2205169732f29114BC240b3"),
        name: "Kraken_74",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("fCAF9c57C26566f96D23F585950Bb1c66E138890"),
        name: "Kraken_75",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("16b34756653f88a89005E96C0622832D8fB6b0B5"),
        name: "Kraken_8",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("1F7bc4dA1a0c2e49d7eF542F74CD46a3FE592cb1"),
        name: "Kraken_9",
        exchange: "Kraken",
    },
    CexAddress {
        address: address!("30B71D015F60e2f959743038cE0aAeC9b4C1ea44"),
        name: "Kryptono",
        exchange: "Kryptono",
    },
    CexAddress {
        address: address!("41C00d79005a78C23D8CF73075F739f12f03bbAA"),
        name: "Kryptono_1",
        exchange: "Kryptono",
    },
    CexAddress {
        address: address!("629a7144235259336ea2694167F3C8b856EDD7dC"),
        name: "Kryptono_2",
        exchange: "Kryptono",
    },
    CexAddress {
        address: address!("E8A0E282e6a3E8023465acCd47FaE39dD5Db010b"),
        name: "Kryptono_3",
        exchange: "Kryptono",
    },
    CexAddress {
        address: address!("00F3e09Abe73AeC2D6AD7B8820049B60eBc73f94"),
        name: "KuCoin",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("03E6FA590CAdcf15A38e86158E9b3D06FF3399Ba"),
        name: "KuCoin_1",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("2B5634C42055806a59e9107ED44D43c426E58258"),
        name: "KuCoin_10",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("3aD7D43702Bc2177cc9EC655b6ee724136891EF4"),
        name: "KuCoin_11",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("41e29c02713929F800419AbE5770fAa8A5b4dADC"),
        name: "KuCoin_12",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("439eF8dd8E50d632f6e5802869fD13af430DE84F"),
        name: "KuCoin_13",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("441454b3D857FE365b7DefE8cb3E4F498EC91eAC"),
        name: "KuCoin_14",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("45300136662dD4e58fc0DF61E6290DFfD992B785"),
        name: "KuCoin_15",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("4ad64983349C49dEfE8d7A4686202d24b25D0CE8"),
        name: "KuCoin_16",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("4E75e27e5Aa74F0c7A9D4897dC10EF651f3A3995"),
        name: "KuCoin_17",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("53f78A071d04224B8e254E243fFfc6D9f2f3Fa23"),
        name: "KuCoin_18",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("58edF78281334335EfFa23101bBe3371b6a36A51"),
        name: "KuCoin_19",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("061F7937B7b2bc7596539959804F86538b6368dC"),
        name: "KuCoin_2",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("635308e731A878741bfeC299e67f5fD28c7553D9"),
        name: "KuCoin_20",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("689C56AEf474Df92D44A1B70850f808488F9769C"),
        name: "KuCoin_21",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("738cF6903E6c4e699D1C2dd9AB8b67fcDb3121eA"),
        name: "KuCoin_22",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("7491f26A0FCb459111b3a1db2fbFC4035D096933"),
        name: "KuCoin_23",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("77f59b595CaC829575E262B4c8bBCB17abAdB33A"),
        name: "KuCoin_24",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("7b403C20ADB5674e31FBBC040945999739A874c8"),
        name: "KuCoin_25",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("7b915c27a0Ed48E2Ce726Ee40F20B2bF8a88a1b3"),
        name: "KuCoin_26",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("83C41363cBee0081dab75cB841FA24f3dB46627e"),
        name: "KuCoin_27",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("88Bd4D3e2997371BCEEFE8D9386c6B5B4dE60346"),
        name: "KuCoin_28",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("899B5d52671830f567BF43A14684Eb14e1f945fe"),
        name: "KuCoin_29",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("14EA40648fC8C1781D19363F5B9Cc9A877ac2469"),
        name: "KuCoin_3",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("9AC5637d295FEA4f51E086C329d791cC157B1C84"),
        name: "KuCoin_30",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("9f4Cf329f4cF376B7ADED854D6054859dd102a2A"),
        name: "KuCoin_31",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("a152F8bb749c55E9943A3a0A3111D18ee2B3f94E"),
        name: "KuCoin_32",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("A1CE37506eadf62d2Be3741C644AA52a009d3A8b"),
        name: "KuCoin_33",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("a1D8d972560C2f8144AF871Db508F0B0B10a3fBf"),
        name: "KuCoin_34",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("a3f45e619cE3AAe2Fa5f8244439a66B203b78bCc"),
        name: "KuCoin_35",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("a649fFC455AC7C5acc1bc35726FcE54e25Eb59f9"),
        name: "KuCoin_36",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("B9F79Fc4B7A2F5fB33493aB5D018dB811c9c2f02"),
        name: "KuCoin_37",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("BF7AEBe0A571CF621F59aD48333a5c75fBf9d5E4"),
        name: "KuCoin_38",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("c91FbeC25F454E1BFc782d215aDA56b3007276D0"),
        name: "KuCoin_39",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("1692E170361cEFD1eb7240ec13D048Fd9aF6d667"),
        name: "KuCoin_4",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("caD621da75a66c7A8f4FF86D30A2bF981Bfc8FdD"),
        name: "KuCoin_40",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("cB014880de8b1E5f6c90CBcD2c232970cF3Aec32"),
        name: "KuCoin_41",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("cD5F3c15120a1021155174719Ec5FCf2c75aDf5b"),
        name: "KuCoin_42",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("ce0B6bfd578A5e90fB827ce6F86Aa06355277F8c"),
        name: "KuCoin_43",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("cE0d2213A0eAFF4176D90B39879b7B4F870fA428"),
        name: "KuCoin_44",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("D6216fC19DB775Df9774a6E33526131dA7D19a2c"),
        name: "KuCoin_45",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("d89350284c7732163765b23338f2ff27449E0Bf5"),
        name: "KuCoin_46",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("dd07813c45CA55731dd12f6e5De59Ce9FE5304ad"),
        name: "KuCoin_47",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("e59Cd29be3BE4461d79C0881D238Cbe87D64595A"),
        name: "KuCoin_48",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("E66845FD840FC7e489bcb61241FFf5B7fc5f1f0e"),
        name: "KuCoin_49",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("17A30350771d02409046A683b18Fe1C13cCFC4A8"),
        name: "KuCoin_5",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("EBb8EA128BbdFf9a1780A4902A9380022371d466"),
        name: "KuCoin_50",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("eC30d02f10353f8EFC9601371f56e808751f396F"),
        name: "KuCoin_51",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("f16E9B0D03470827A95CDfd0Cb8a8A3b46969B91"),
        name: "KuCoin_52",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("F3F094484eC6901FfC9681bCb808B96bAFd0b8a8"),
        name: "KuCoin_53",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("f8Ba3EC49212Ca45325A2335a8Ab1279770dF6c0"),
        name: "KuCoin_54",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("F8dA05c625A6E601281110cbA52b156e714E1DC2"),
        name: "KuCoin_55",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("f97DeB1C0BB4536ff16617D29E5F4B340fE231Df"),
        name: "KuCoin_56",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("fB6a733bf7eC9CE047c1c5199F18401052Eb062D"),
        name: "KuCoin_57",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("1dD9319a115D36bD0f71C276844f67171678E17b"),
        name: "KuCoin_6",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("245654d7Db653FF134EA032f671eF2730333F42c"),
        name: "KuCoin_7",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("2602669a92fCCF44e5319fF51B0F453aAb9Db021"),
        name: "KuCoin_8",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("2a8c8b09bD77c13980495A959B26c1305166A57f"),
        name: "KuCoin_9",
        exchange: "KuCoin",
    },
    CexAddress {
        address: address!("04196627190fF624492427317D853deaa270F9d2"),
        name: "Kuna",
        exchange: "Kuna",
    },
    CexAddress {
        address: address!("77aB999d1e9F152156B4411E1f3E2A42Dab8CD6D"),
        name: "Kuna_1",
        exchange: "Kuna",
    },
    CexAddress {
        address: address!("9030a104a49141459F4B419BD6f56E4bA6fcd800"),
        name: "Kuna_2",
        exchange: "Kuna",
    },
    CexAddress {
        address: address!("EA81CE54A0AfA10A027f65503bd52FBa83d745b8"),
        name: "Kuna_3",
        exchange: "Kuna",
    },
    CexAddress {
        address: address!("FAc0f29459896ED8550e002344734989d958DE01"),
        name: "Kuna_4",
        exchange: "Kuna",
    },
    CexAddress {
        address: address!("00343217B01188388C0E3242278231Ace35E1b61"),
        name: "LAToken",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("0861Fca546225fbF8806986D211C8398f7457734"),
        name: "LAToken_1",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("44C19a079B869A3E74942dF2CfE0152f834A458D"),
        name: "LAToken_10",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("6fb194fc9806fE320E0CBD658e31F13B1bAa3925"),
        name: "LAToken_11",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("7891b20C690605F4E370d6944C8A5DBfAc5a451c"),
        name: "LAToken_12",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("7D8a212940933114DaC826EfBA2f673dc66310D0"),
        name: "LAToken_13",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("8D056D457a52c4dAF71CEf45F540a040c143Ea05"),
        name: "LAToken_14",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("9480D1cc3fd4cb7936D114f7d63124107870A7b8"),
        name: "LAToken_15",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("9976c40e8186a5E0C2a9D50d55b51F905d10ce52"),
        name: "LAToken_16",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("A1a0538D556B3E77f7E1340E3Ebd70C649c4bb84"),
        name: "LAToken_17",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("A614180C69aBF82f3E7AAbB53AD9976EC90aeAC6"),
        name: "LAToken_18",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("BA6C98f1cc6869ECCbeB892b7A603F8F02Db3b29"),
        name: "LAToken_19",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("0F307b17d41acE555620DF5a55Dd5A01637e3b42"),
        name: "LAToken_2",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("c00EEbe4E2bE29679781fc5fC350057eE8132BaB"),
        name: "LAToken_20",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("CE55977E7B33E4e5534Bd370eE31504Fc7Ac9ADc"),
        name: "LAToken_21",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("d76D939B455743e96adbCdf800627b11F3446780"),
        name: "LAToken_22",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("E69963CE13ED742639C8287913682bC008B3e622"),
        name: "LAToken_23",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("ecabeA0fB22f82F3A5a5D6043D7cCf65F3640c85"),
        name: "LAToken_24",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("eD8D8f4Ff53915D80987BCD51C2DE582a05b2322"),
        name: "LAToken_25",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("eE61F5fB0dB81d3A09392375Ee96f723C0620E07"),
        name: "LAToken_26",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("EeC02a6D1a7F9f534b9609c8EE30B9cF9A7fe1B3"),
        name: "LAToken_27",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("EFf6E17Fdc68d56812DA40f7d05FC8cDfd212440"),
        name: "LAToken_28",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("1771C9c8d5AF830d322c2E1D2161D002844679EF"),
        name: "LAToken_3",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("1B6C1A0e20aF81b922Cb454c3E52408496eE7201"),
        name: "LAToken_4",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("235e8ceD6b42eE6E226837EB551E86D810d49f22"),
        name: "LAToken_5",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("26b52C889FCf3B8f449aD1c0F07b8572E6ACE262"),
        name: "LAToken_6",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("2790E66986d4f701443b3F053d7d9ebFa69c990e"),
        name: "LAToken_7",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("3b28358e9CDde80A24f0f811daD13aB9fc2A0d2A"),
        name: "LAToken_8",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("4114d8D509503592175A8E044594b29EC081dbe0"),
        name: "LAToken_9",
        exchange: "LAToken",
    },
    CexAddress {
        address: address!("0E80Abb31aCE45ae26F9A2943Acaab72E77DE9bE"),
        name: "LBank",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("11B1e83818028a8F44EF84613D5c058734b2d5CF"),
        name: "LBank_1",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("a690621b1D2F8cc06dE2De11e4cf69935F82655A"),
        name: "LBank_10",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("b0E5Ec2A0BB8b8f3a727787f90b959611e4062b7"),
        name: "LBank_11",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("C06e0513A150a021104FdCDD20Ce362fA593Ba1F"),
        name: "LBank_12",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("e0F0aA98b4A4d305Ac4A04D830C96A158BdA9cd8"),
        name: "LBank_13",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("E5a8B35eaa3c14E0a6514F800825e1e6687bF23E"),
        name: "LBank_14",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("Ea48643446540fe29A909Ad4aa6Df182c8b7E997"),
        name: "LBank_15",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("EC174c25bba7359b56Fd672c8899664121E7dBCF"),
        name: "LBank_16",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("fa9f7a1cBfBCB688729c522b4F0905CcF4d26D25"),
        name: "LBank_17",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("120051a72966950B8ce12eB5496B5D1eEEC1541B"),
        name: "LBank_2",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("124D9BF2fecBc16b54eC4AcCdB14D44C2144f012"),
        name: "LBank_3",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("22f83e4b9cB95CB99B88E8f4f15ea598C74c2788"),
        name: "LBank_4",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("25b63C988310DE90031B84975aE13a6015ca6a16"),
        name: "LBank_5",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("356dC48d74F107cfBfd61790B0808CdA6a0D364f"),
        name: "LBank_6",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("43cBDc908A0860Ae23fA06AA5B20dFEe43c196a6"),
        name: "LBank_7",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("7fF77C761e5669d1696059A3b46Cda0aE293aAE3"),
        name: "LBank_8",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("9c4Fe1C3D5975E5C5E493F24352969aa280B7CFc"),
        name: "LBank_9",
        exchange: "LBank",
    },
    CexAddress {
        address: address!("2957eA6D4f06bC2BadFB2958c65fc7d1bE5461B1"),
        name: "LCX",
        exchange: "LCX",
    },
    CexAddress {
        address: address!("4631018F63d5E31680FB53C11C9e1B11F1503e6f"),
        name: "LCX_1",
        exchange: "LCX",
    },
    CexAddress {
        address: address!("c0C704BFc375b3FC657Bc378b1cfee009194e2b3"),
        name: "LCX_2",
        exchange: "LCX",
    },
    CexAddress {
        address: address!("c90970DD648415756681163F69Eaeb7EB9c28A9C"),
        name: "LCX_3",
        exchange: "LCX",
    },
    CexAddress {
        address: address!("20bB82F2Db6FF52b42c60cE79cDE4C7094Ce133F"),
        name: "Lemon Cash",
        exchange: "Lemon",
    },
    CexAddress {
        address: address!("1BEDb2AA5d389b13980d7B7cA7B7266a95020781"),
        name: "Liqui",
        exchange: "Liqui",
    },
    CexAddress {
        address: address!("3AE7F3679D63077B4ab30dC96af2DF72239fAaff"),
        name: "Liqui_1",
        exchange: "Liqui",
    },
    CexAddress {
        address: address!("5E575279bf9f4acf0A130c186861454247394C06"),
        name: "Liqui_2",
        exchange: "Liqui",
    },
    CexAddress {
        address: address!("8271B2E8CBe29396e9563229030c89679B9470db"),
        name: "Liqui_3",
        exchange: "Liqui",
    },
    CexAddress {
        address: address!("D9Bd20EFcA7b0e6606b969548b1516C08D37374b"),
        name: "Liqui_4",
        exchange: "Liqui",
    },
    CexAddress {
        address: address!("E17AF102ba3c45301Cca05953892e2eD357486e6"),
        name: "Liqui_5",
        exchange: "Liqui",
    },
    CexAddress {
        address: address!("ea133a07880B99Ceb7A973e64DD3AC18f7A20888"),
        name: "Liqui_6",
        exchange: "Liqui",
    },
    CexAddress {
        address: address!("07445065963c2D563Cd70Ddf2AA49fc771e59a98"),
        name: "Liquid",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("3c0a69CBb7e0830f8fAB41F11F75100f886998a6"),
        name: "Liquid_1",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("dF4B6Fb700C428476Bd3C02E6FA83e110741145b"),
        name: "Liquid_10",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("E11eda4F7ee51bF6EFf7Cb3CA0F1Dc10809e01B1"),
        name: "Liquid_11",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("edBB72E6b3Cf66a792bFF7FaaC5Ea769fe810517"),
        name: "Liquid_12",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("eE0fB34631f0e6503c5fFB4A30F6FA345Cf1BA99"),
        name: "Liquid_13",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("41D5233f434d98b73F22Ce664D48bE06F4eb073F"),
        name: "Liquid_2",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("8e01eF6A4D864698Ff80B419fe9ec2e3C85e9E9e"),
        name: "Liquid_3",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("9cC2dCe817093CEEa82bb67A4Cf43131fA354c06"),
        name: "Liquid_4",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("9F571BB918e98B8DEb462F14C54a3E36Ad43627A"),
        name: "Liquid_5",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("a80EE1D4E41dc43eE5BCc54BD2867B44A8e6A385"),
        name: "Liquid_6",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("ccDfdc87341605200a3582aB52350fb4FB261AB0"),
        name: "Liquid_7",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("Db2caD4f306B47C9b35541988c7656F1BB092e15"),
        name: "Liquid_8",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("Db2E63058A01A3F304c9E337003f425889BA135F"),
        name: "Liquid_9",
        exchange: "Liquid",
    },
    CexAddress {
        address: address!("67655E8FCd903f75aceB82bB3f782687CE985d35"),
        name: "LiteBit",
        exchange: "LiteBit",
    },
    CexAddress {
        address: address!("9Afa066884cE723200438B672C0B8D5769E03A6A"),
        name: "LiteBit_1",
        exchange: "LiteBit",
    },
    CexAddress {
        address: address!("243BEc9256C9A3469DA22103891465B47583d9F1"),
        name: "Livecoin.net",
        exchange: "Livecoin.net",
    },
    CexAddress {
        address: address!("76c674F9bcb5Eb01Ad64629D3f68895B60029305"),
        name: "LordToken",
        exchange: "LordToken",
    },
    CexAddress {
        address: address!("05CdB1526F6e224e02919a4C018D9784Ea25eb3d"),
        name: "Luno",
        exchange: "Luno",
    },
    CexAddress {
        address: address!("3a5cc8689D1b0cEf2c317bC5C0aD6Ce88B27D597"),
        name: "Luno_1",
        exchange: "Luno",
    },
    CexAddress {
        address: address!("416299AAde6443e6F6e8ab67126e65a7F606eeF5"),
        name: "Luno_2",
        exchange: "Luno",
    },
    CexAddress {
        address: address!("Af1931c20ee0c11BEA17A41BfBbAd299B2763bc0"),
        name: "Luno_3",
        exchange: "Luno",
    },
    CexAddress {
        address: address!("2E0279b98000182d1C286da4102AAcdbEe4c2d85"),
        name: "MAX Exchange",
        exchange: "MAX",
    },
    CexAddress {
        address: address!("A9BfF538A906154c80A8dBccd229F3DEddFa52D6"),
        name: "MAX Exchange_1",
        exchange: "MAX Exchange",
    },
    CexAddress {
        address: address!("c4EB040289e0d8A8F38184c52757E691C1D1d112"),
        name: "MAX Exchange_2",
        exchange: "MAX Exchange",
    },
    CexAddress {
        address: address!("cD32602db028cB3827f66cedc2c6d7c0B97A8b34"),
        name: "MAX Exchange_3",
        exchange: "MAX Exchange",
    },
    CexAddress {
        address: address!("0211f3ceDbEf3143223D3ACF0e589747933e8527"),
        name: "MEXC",
        exchange: "MEXC",
    },
    CexAddress {
        address: address!("2e8F79aD740de90dC5F5A9F0D8D9661a60725e64"),
        name: "MEXC_1",
        exchange: "MEXC",
    },
    CexAddress {
        address: address!("9b64203878F24eB0CDF55c8c6fA7D08Ba0cF77E5"),
        name: "MEXC_10",
        exchange: "MEXC",
    },
    CexAddress {
        address: address!("DF90C9B995a3b10A5b8570a47101e6c6a29eb945"),
        name: "MEXC_11",
        exchange: "MEXC",
    },
    CexAddress {
        address: address!("ffB3118124cdaEbD9095fA9a479895042018cac2"),
        name: "MEXC_12",
        exchange: "MEXC",
    },
    CexAddress {
        address: address!("3CC936b795A188F0e246cBB2D74C5Bd190aeCF18"),
        name: "MEXC_2",
        exchange: "MEXC",
    },
    CexAddress {
        address: address!("4982085C9e2F89F2eCb8131Eca71aFAD896e89CB"),
        name: "MEXC_3",
        exchange: "MEXC",
    },
    CexAddress {
        address: address!("4e3ae00E8323558fA5Cac04b152238924AA31B60"),
        name: "MEXC_4",
        exchange: "MEXC",
    },
    CexAddress {
        address: address!("51E3D44172868Acc60D68ca99591Ce4230bc75E0"),
        name: "MEXC_5",
        exchange: "MEXC",
    },
    CexAddress {
        address: address!("576b81F0c21EDBc920ad63FeEEB2b0736b018A58"),
        name: "MEXC_6",
        exchange: "MEXC",
    },
    CexAddress {
        address: address!("75e89d5979E4f6Fba9F97c104c2F0AFB3F1dcB88"),
        name: "MEXC_7",
        exchange: "MEXC",
    },
    CexAddress {
        address: address!("83c1C224044Ef8573e9a728dBb91013CF80827E6"),
        name: "MEXC_8",
        exchange: "MEXC",
    },
    CexAddress {
        address: address!("8E1701CFd85258DDb8DFE89Bc4c7350822B9601D"),
        name: "MEXC_9",
        exchange: "MEXC",
    },
    CexAddress {
        address: address!("477b8D5eF7C2C42DB84deB555419cd817c336b6F"),
        name: "MaiCoin",
        exchange: "MaiCoin",
    },
    CexAddress {
        address: address!("986AAC49D38CEA193e56DF1A114894f8f25333Ee"),
        name: "MaiCoin_1",
        exchange: "MaiCoin",
    },
    CexAddress {
        address: address!("09b1806Df13062B5f653BeDA6998972cabCF7009"),
        name: "MaskEX",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("0B3c7bcE764E6f1B52443e30fcb4f34997A0674c"),
        name: "MaskEX_1",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("3Dd878A95DCAEF2800cD57BB065B5e8f2F438131"),
        name: "MaskEX_10",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("46c75Fc52E0263946f8F1a75A95C23a767D2f26e"),
        name: "MaskEX_11",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("5f23b26C6D76F836aa99F174E71BCD89bdEe3226"),
        name: "MaskEX_12",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("6DB133E840376555A5aD5c1D7616872EF57e7F13"),
        name: "MaskEX_13",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("6e2673095545280F6F10E22eB861a555C6E94bEc"),
        name: "MaskEX_14",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("6f531cf07F2D659DcfB371B1A7f4c0157A168332"),
        name: "MaskEX_15",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("71467Da4c0b0db4e889DA703E6fF1cd740F1F74A"),
        name: "MaskEX_16",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("7Ac724cAC6E4dDc24C102b1006f41bc8A6A5C1C5"),
        name: "MaskEX_17",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("7E0616656934a09373b1E1114DE2c20A77513D16"),
        name: "MaskEX_18",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("80B62F0ea7a89bbc4dF4C95e2ad363e5C153b80e"),
        name: "MaskEX_19",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("0c78fD926A8fC9CFc682bDc6b411942D9C7EDb7a"),
        name: "MaskEX_2",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("823C8E533657B0004B5Ab8553d84502ba2E571f7"),
        name: "MaskEX_20",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("833F3B6fAa717079fb3A1030f6207C57B1c591Bd"),
        name: "MaskEX_21",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("84457412efE8b3A05583cb496E1D2c03E6F36155"),
        name: "MaskEX_22",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("8458c828D602230e92Eb0aAC5a6aed5580011B6A"),
        name: "MaskEX_23",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("95ad8841376058a000F489196F05ecf176bEB8ac"),
        name: "MaskEX_24",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("9F1bB5349d481065561A84CBD7F84982fD533359"),
        name: "MaskEX_25",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("A310b3eecA53B9C115af529faF92Bb5ca4B41494"),
        name: "MaskEX_26",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("a4E71851A8c8eaeFeb20A994159F4A443E46059b"),
        name: "MaskEX_27",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("BE921EA3bd0c879a8688B7fabE6b3c8A471df90d"),
        name: "MaskEX_28",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("c3eDBB9C181016Cef5d76491F835930e9C8c4D2C"),
        name: "MaskEX_29",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("0cE7EeFB9f862aa0374EE7bbC4D8A0Fc2C651517"),
        name: "MaskEX_3",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("C6aCB77BEFebfF0359CC581973859eeE8CbAEDa1"),
        name: "MaskEX_30",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("c7570e308464c7838A3eEA1e5788D0d901cc6A80"),
        name: "MaskEX_31",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("d666aD8D95903BcE9B4dcD2cacdE5145E36405c2"),
        name: "MaskEX_32",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("D7aEd730A7c4cf8dFE313b16712AF3406f6Dca5b"),
        name: "MaskEX_33",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("DCa6951B82e82AF6AAB4bB9e90CA00F5760370e1"),
        name: "MaskEX_34",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("dD9c649Edb7fF80c6C9D238344260184A4f94b88"),
        name: "MaskEX_35",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("FB65377800a7282CF81bAf0f335FBC6f8FF36776"),
        name: "MaskEX_36",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("0cE92D3a15908b53371Ff1afCaE800F28142250c"),
        name: "MaskEX_4",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("0FEABB61f67E859811aAfce83a5aB780f8C53c0a"),
        name: "MaskEX_5",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("1349907C197731c5Ed98D8442309a15107cB6baD"),
        name: "MaskEX_6",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("2161217d22FAC0188775432F8bA32F1D4272dD19"),
        name: "MaskEX_7",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("32FFFc894503eBa55Af4c371fd50198e7356D780"),
        name: "MaskEX_8",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("33fe5557E90a872A065f2acfD973847e33fC4532"),
        name: "MaskEX_9",
        exchange: "MaskEX",
    },
    CexAddress {
        address: address!("0090E10302a3dDeff920BA96a023423F306dc0a8"),
        name: "Matrixport",
        exchange: "Matrixport",
    },
    CexAddress {
        address: address!("0525eB58F9d226ADb659777791ddD994dEF56E1c"),
        name: "Matrixport_1",
        exchange: "Matrixport",
    },
    CexAddress {
        address: address!("Ab5A9FCb27e4F97E87a536E768b9cb49dC8B1A4F"),
        name: "Matrixport_2",
        exchange: "Matrixport",
    },
    CexAddress {
        address: address!("C5b8A59fdaAb89bdbf22Dbd906A03C1F48bc9eC8"),
        name: "Matrixport_3",
        exchange: "Matrixport",
    },
    CexAddress {
        address: address!("D00C56cd2cA9fFe295BfF960a14c65783dd87162"),
        name: "Matrixport_4",
        exchange: "Matrixport",
    },
    CexAddress {
        address: address!("5e6aD578fe3a2DA7bBD0255f04179e1E77317D1a"),
        name: "Mbcbit",
        exchange: "Mbcbit",
    },
    CexAddress {
        address: address!("8CE13C17B9C9caE9193538DC2a64ca7be07E2C00"),
        name: "Mercado Bitcoin",
        exchange: "Mercado",
    },
    CexAddress {
        address: address!("b8bA36E591FAceE901FfD3d5D82dF491551AD7eF"),
        name: "Mercado Bitcoin_1",
        exchange: "Mercado Bitcoin",
    },
    CexAddress {
        address: address!("e03c23519e18D64F144d2800E30E81B0065C48B5"),
        name: "Mercatox",
        exchange: "Mercatox",
    },
    CexAddress {
        address: address!("8C8D7C46219D9205f056f28fee5950aD564d7465"),
        name: "Mercuryo",
        exchange: "Mercuryo",
    },
    CexAddress {
        address: address!("ac338d9fAaC562Df26d702880c796e1024E2698A"),
        name: "MinedTrade.com",
        exchange: "MinedTrade.com",
    },
    CexAddress {
        address: address!("0b5c4a7FcDA49e0a8661419Bb55B86161a86db2a"),
        name: "MoonPay",
        exchange: "MoonPay",
    },
    CexAddress {
        address: address!("1440ec793aE50fA046B95bFeCa5aF475b6003f9e"),
        name: "MoonPay_1",
        exchange: "MoonPay",
    },
    CexAddress {
        address: address!("151B381058f91cF871E7eA1eE83c45326F61e96D"),
        name: "MoonPay_2",
        exchange: "MoonPay",
    },
    CexAddress {
        address: address!("22F6CC8738308a8c92a6a71ea67832463d1Fec0d"),
        name: "MoonPay_3",
        exchange: "MoonPay",
    },
    CexAddress {
        address: address!("7AFC12C8DD2e6591581D95586eB2c2A4905a12a9"),
        name: "MoonPay_4",
        exchange: "MoonPay",
    },
    CexAddress {
        address: address!("8216874887415e2650D12D53Ff53516F04a74FD7"),
        name: "MoonPay_5",
        exchange: "MoonPay",
    },
    CexAddress {
        address: address!("B287eaC48aB21c5FB1d3723830d60b4c797555B0"),
        name: "MoonPay_6",
        exchange: "MoonPay",
    },
    CexAddress {
        address: address!("d108FD0E8c8E71552a167E7a44FF1d345D233BA6"),
        name: "MoonPay_7",
        exchange: "MoonPay",
    },
    CexAddress {
        address: address!("D42f958E1C3e2a10e5d66343c4c9a57726E5b4b6"),
        name: "MoonPay_8",
        exchange: "MoonPay",
    },
    CexAddress {
        address: address!("C21aE7Af41c4dfeEb7eCa07Df573288523076b60"),
        name: "MultiBank Group",
        exchange: "MultiBank",
    },
    CexAddress {
        address: address!("14c4017f81675F7aC1321218d4a385a2D3117328"),
        name: "NDAX",
        exchange: "NDAX",
    },
    CexAddress {
        address: address!("3B56258cFfD6ae5604A4906E55e79B9BFd3cdcfE"),
        name: "NDAX_1",
        exchange: "NDAX",
    },
    CexAddress {
        address: address!("ae7006588d03bd15d6954e3084A7e644596bC251"),
        name: "NEXBIT Pro",
        exchange: "NEXBIT",
    },
    CexAddress {
        address: address!("404460039499c774c48248552F802CE5dd482e32"),
        name: "Netcoins",
        exchange: "Netcoins",
    },
    CexAddress {
        address: address!("ba20996529D2722c4D9D800A7b20E96a0A235336"),
        name: "Netcoins_1",
        exchange: "Netcoins",
    },
    CexAddress {
        address: address!("e3743D1bf4bdc86BF1FaAc0fFe325a23cF495Ad7"),
        name: "Netcoins_2",
        exchange: "Netcoins",
    },
    CexAddress {
        address: address!("0031e147A79c45f24319dc02ca860cB6142FCBA1"),
        name: "Nexo",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("00EE047A66d5cff27587A61559138c26b62F7CEb"),
        name: "Nexo_1",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("57793E249825492212de2aA4306379017301e1Da"),
        name: "Nexo_10",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("65b0BF8Ee4947edD2A500D74E50a3d757DC79de0"),
        name: "Nexo_11",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("6914FC70fAC4caB20a8922E900C4BA57fEECf8E1"),
        name: "Nexo_12",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("7344E478574aCBe6DaC9dE1077430139E17EEc3D"),
        name: "Nexo_13",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("7AB6c736baf1DAc266aAb43884d82974a9ADCcCF"),
        name: "Nexo_14",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("8Fd589AA8bfA402156a6D1ad323FEC0ECee50D9D"),
        name: "Nexo_15",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("9bdB521a97E95177BF252C253E256A60C3e14447"),
        name: "Nexo_16",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("A75EDE99F376Dd47f3993Bc77037F61b5737C6EA"),
        name: "Nexo_17",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("B60C61DBb7456f024f9338c739B02Be68e3F545C"),
        name: "Nexo_18",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("Ba90b5bc12DAAb8d06582967a22c86AE7eed0469"),
        name: "Nexo_19",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("121EFFb8160f7206444f5a57d13c7A4424a237A4"),
        name: "Nexo_2",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("e498E7a77a2ffBd33E4a14253C3d11F97AeBa18B"),
        name: "Nexo_20",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("E6Fa688c09E196a8f9D08911dF84e3b5f350e507"),
        name: "Nexo_21",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("eD212a4A2E82d5ee0D62F70B5deE2f5Ee0f10c5D"),
        name: "Nexo_22",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("f36A47300F002c0C9F8c131962F077C3543B2fC6"),
        name: "Nexo_23",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("Ffec0067F5a79CFf07527f63D83dD5462cCf8BA4"),
        name: "Nexo_24",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("1D85f929EE6AEDc3b4981d8FE408Ae43942b2e53"),
        name: "Nexo_3",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("31E9b3373F2AD5d964CAd0fd01332d6550cBBdE6"),
        name: "Nexo_4",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("354e9Fa5c6Ee7e6092158a8c1B203CcAc932D66d"),
        name: "Nexo_5",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("463E5b673d1029989c9B059d36393c539beF9094"),
        name: "Nexo_6",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("4bb7f4c3d47C4b431cb0658F44287d52006fb506"),
        name: "Nexo_7",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("4d7F1790644Af787933c9fF0e2cff9a9B4299Abb"),
        name: "Nexo_8",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("55e4d16f9c3041EfF17Ca32850662f3e9Dddbce7"),
        name: "Nexo_9",
        exchange: "Nexo",
    },
    CexAddress {
        address: address!("09672F26Cc257F7B216710864c98aB1841453E4a"),
        name: "Nobitex",
        exchange: "Nobitex",
    },
    CexAddress {
        address: address!("598C60Bb7E929F02008A098461Fd2FAec3f74771"),
        name: "Nobitex_1",
        exchange: "Nobitex",
    },
    CexAddress {
        address: address!("641FB555527B9108a1F58eA24E0C04fF86C4Ed0d"),
        name: "Nobitex_2",
        exchange: "Nobitex",
    },
    CexAddress {
        address: address!("7eB6a79587DFd5Da426DFF27ff11da1F09c90A2B"),
        name: "Nobitex_3",
        exchange: "Nobitex",
    },
    CexAddress {
        address: address!("8D56f551b44a6dA6072a9608d63d664ce67681a5"),
        name: "Nobitex_4",
        exchange: "Nobitex",
    },
    CexAddress {
        address: address!("D16E4cdb153B2DCc617061174223a6D4BFaE53f5"),
        name: "Nobitex_5",
        exchange: "Nobitex",
    },
    CexAddress {
        address: address!("D5BcF75c0573B14818C42F0118067BE859131acE"),
        name: "Nobitex_6",
        exchange: "Nobitex",
    },
    CexAddress {
        address: address!("F639d88a89384A4D97f2bA9159567Ddb3890Ea07"),
        name: "Nobitex_7",
        exchange: "Nobitex",
    },
    CexAddress {
        address: address!("10298Be5Abf74D111D133dc3493Dc4C6a9FD924b"),
        name: "Nominex",
        exchange: "Nominex",
    },
    CexAddress {
        address: address!("26804231a528c894AB6790530b237449a817da6A"),
        name: "Nominex_1",
        exchange: "Nominex",
    },
    CexAddress {
        address: address!("A937Eddfd12930F758788BcC936B4762BDE9d54C"),
        name: "Nominex_10",
        exchange: "Nominex",
    },
    CexAddress {
        address: address!("ab2f4297E7e31638eBE8362471b3038018A106D8"),
        name: "Nominex_11",
        exchange: "Nominex",
    },
    CexAddress {
        address: address!("DbF1B10FE3e05397Cd454163F6F1eD0c1181C3B3"),
        name: "Nominex_12",
        exchange: "Nominex",
    },
    CexAddress {
        address: address!("2D8b192eAd2f402867323B072D143d44435EDd74"),
        name: "Nominex_2",
        exchange: "Nominex",
    },
    CexAddress {
        address: address!("5cd67d65Ff07D5BE2488E51F1a8C69273D258338"),
        name: "Nominex_3",
        exchange: "Nominex",
    },
    CexAddress {
        address: address!("63A81d936cb14fA3649A4D071608758cFFb3Bd94"),
        name: "Nominex_4",
        exchange: "Nominex",
    },
    CexAddress {
        address: address!("8326E22a36486ae7D4B85e8DFA732527b962805c"),
        name: "Nominex_5",
        exchange: "Nominex",
    },
    CexAddress {
        address: address!("857083580AeD7b5726860937EF030ED8072BC9aB"),
        name: "Nominex_6",
        exchange: "Nominex",
    },
    CexAddress {
        address: address!("99b674Ba03E896D952983908DbA8D7b560FB10d5"),
        name: "Nominex_7",
        exchange: "Nominex",
    },
    CexAddress {
        address: address!("9Cd2D1A3214c12BB6dbfA7DBc3B0641C26a2f9a6"),
        name: "Nominex_8",
        exchange: "Nominex",
    },
    CexAddress {
        address: address!("A0F2C13e20A11e00acF4e7B47604b24ca8908797"),
        name: "Nominex_9",
        exchange: "Nominex",
    },
    CexAddress {
        address: address!("052Ed0aD68Ffc470386FDAb82F7046E0b55FD663"),
        name: "Norwegian Block Exchange",
        exchange: "Norwegian",
    },
    CexAddress {
        address: address!("0d019414aC7DD7E8262aE7Dc9EFCC6bDe050b0DD"),
        name: "Norwegian Block Exchange_1",
        exchange: "Norwegian Block Exchange",
    },
    CexAddress {
        address: address!("0d075CCD8FE8C5C8e28A308f66422Db3456Eb9cc"),
        name: "Norwegian Block Exchange_2",
        exchange: "Norwegian Block Exchange",
    },
    CexAddress {
        address: address!("29af949c3D218C1133bD16257ed029E92deFb168"),
        name: "Norwegian Block Exchange_3",
        exchange: "Norwegian Block Exchange",
    },
    CexAddress {
        address: address!("8Cad96fB23924Ebc37b8CdAFa8400AD856fE4a2C"),
        name: "Norwegian Block Exchange_4",
        exchange: "Norwegian Block Exchange",
    },
    CexAddress {
        address: address!("AeB81c391Ac427B6443310fF1cB73a21E071e5ad"),
        name: "Norwegian Block Exchange_5",
        exchange: "Norwegian Block Exchange",
    },
    CexAddress {
        address: address!("fACCB74832546a745aaB8Dbd2d155Dc67a222048"),
        name: "Norwegian Block Exchange_6",
        exchange: "Norwegian Block Exchange",
    },
    CexAddress {
        address: address!("03aE1A796DFE0400439211133D065BDA774B9D3e"),
        name: "OKX",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("0475Dd0e4194422A8Cac486Dc69173F535D0baF4"),
        name: "OKX_1",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("12A8BDC0470ab29a229D828526641b7d1f170fcF"),
        name: "OKX_10",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("9e64aFC7bca5F2C6607a5B8c378bCF1EC3531C97"),
        name: "OKX_100",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("A16F524a804BEaED0d791De0aa0b5836295A2a84"),
        name: "OKX_101",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("a2684F75740cFFF46c29bAe79a4eCc43043C003D"),
        name: "OKX_102",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("a27CEF8aF2B6575903b676e5644657FAe96F491F"),
        name: "OKX_103",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("A7EFAe728D2936e78BDA97dc267687568dD593f3"),
        name: "OKX_104",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("aad8AD7DfA05Bc354e011890dd61636842c2Cb96"),
        name: "OKX_105",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("aE0CBABa071D58EFc278A815B2Cb652286e192ff"),
        name: "OKX_106",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("B072b7DC9521d97a3f12b04bEb1e497f8875eC52"),
        name: "OKX_107",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("b47A0AC7798d8308467b2b96aC632ED43c9CB6d7"),
        name: "OKX_108",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("B4eC508ADEB174610B4295E233A458b3475964f7"),
        name: "OKX_109",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("16b5016803bcB4915701EFC0a3B471Bd4c168d93"),
        name: "OKX_11",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("B640e1e5A5f726A054Cd518C968DFDac8C421eE5"),
        name: "OKX_110",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("b8351B61Fa1Eb007A9f80144C489d513e6A76b14"),
        name: "OKX_111",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("B8B0b53b387B061Af2717D642961Af405c79e85A"),
        name: "OKX_112",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("b9656d0393015f92bA642C3A344061E8E2478599"),
        name: "OKX_113",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("b99CC7e10Fe0Acc68C50C7829F473d81e23249cc"),
        name: "OKX_114",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("Ba0a39d37151fA4f938Bec51CdAE4675105760f4"),
        name: "OKX_115",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("ba96A3f3d5E9F13a5f93C37001f946D42eb2E165"),
        name: "OKX_116",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("BDa23B750dD04F792ad365B5F2a6F1d8593796f2"),
        name: "OKX_117",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("bE787D53E09822cC42bfB4ABE1fb4492CAe3D19d"),
        name: "OKX_118",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("Bf94F0AC752C739F623C463b5210a7fb2cbb420B"),
        name: "OKX_119",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("17e91B98988937cB59Dc163b0b11781F9785591A"),
        name: "OKX_12",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("bFbBFacCD1126A11b8F84C60b09859F80f3BD10F"),
        name: "OKX_120",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("BFeF5c888bB7a0A6B14b4C3Ccc4364EA81aA573f"),
        name: "OKX_121",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("c3AE71FE59f5133BA180cbBd76536a70Dec23d40"),
        name: "OKX_122",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("c5451b523d5FFfe1351337a221688a62806ad91a"),
        name: "OKX_123",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("c5a93444Cc4dA6EfB9e6FC6e5D3CB55A53b52396"),
        name: "OKX_124",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("C68c17E6Eec0fDE3605C595C9B98De5c1a4Cc3E4"),
        name: "OKX_125",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("c708A1c712bA26DC618f972ad7A187F76C8596Fd"),
        name: "OKX_126",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("CB0963264231Bb08B2f680aB3ED89A49c9641Bb3"),
        name: "OKX_127",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("CbA38020cd7B6F51Df6AFaf507685aDd148F6ab6"),
        name: "OKX_128",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("CBA6a2397b322CF1389f6d1adc05F75F36B20116"),
        name: "OKX_129",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("236F9F97e0E62388479bf9E5BA4889e46B0273C3"),
        name: "OKX_13",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("Cbc767b519394A9E4682Cc9a15DCd18c46A6045b"),
        name: "OKX_130",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("CbffCB2c38ecd19468d366D392AC0c1DC7F04Bb6"),
        name: "OKX_131",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("cC3947059395A2DfD4eE23DfB1bB586F90d44F2E"),
        name: "OKX_132",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("CD5bD47D3D1D8412b241cE9015c5032142948c12"),
        name: "OKX_133",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("d266529641EeFFAA2B2A0Bc99daA5b32EF241078"),
        name: "OKX_134",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("D30b438DF65f4f788563b2b3611Bd6059bFF4ad9"),
        name: "OKX_135",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("d3d7DBe73BbdD5A5C7a49Ca322763c4d400fC240"),
        name: "OKX_136",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("D576392CB12b7749BA33f8d223f64E65Ee32f03f"),
        name: "OKX_137",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("D7efCbB86eFdD9E8dE014dafA5944AaE36E817e4"),
        name: "OKX_138",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("d9a3c9BA5aa4415a53B9190bcb00f14472790a70"),
        name: "OKX_139",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("24C654f6b143dc5caE3C02Fbb527cA63aa555dBC"),
        name: "OKX_14",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("dad24044E36587d975D7b5BCEb6467Fac21E0c81"),
        name: "OKX_140",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("db0ED345Cb52c2F2457918AFA3EB4b682E264Ad0"),
        name: "OKX_141",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("dc3cE895714844B4775B6d06F0DaE513542cEE10"),
        name: "OKX_142",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("DD6B3aC983AE0427D329A705108C26D2cb8F945E"),
        name: "OKX_143",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("dE01974Fb4A98BAFd7cbf8A06eCf6DCc94d7283f"),
        name: "OKX_144",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("deB6ad2a7820839464Da79BaC953aE6189215443"),
        name: "OKX_145",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("e6eea9812CCc5981cCf0Ab333610C42c2D92D146"),
        name: "OKX_146",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("e7aaFaAD7Eb2bB10771d14CdC4F62D3c7D4A3ba8"),
        name: "OKX_147",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("E7F344E7b95c8A19d7d77eA27B86EE2fD6D776CF"),
        name: "OKX_148",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("e9172Daf64b05B26eb18f07aC8d6D723aCB48f99"),
        name: "OKX_149",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("25236E080106B5387Df201FBBd9a6b870917676D"),
        name: "OKX_15",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("e95f6604A591F6ba33aCCB43a8a885C9c272108c"),
        name: "OKX_150",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("e983845A04c681A295Dd9cE1FA8C2c8505932da3"),
        name: "OKX_151",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("eaEd6576334B003d1c5C4797D9c7Bb025A20c038"),
        name: "OKX_152",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("eB196a61f9A1E35Bf5053b65AAA57c5541dcBa86"),
        name: "OKX_153",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("Ebe80f029b1c02862B9E8a70a7e5317C06F62Cae"),
        name: "OKX_154",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("ED55E0d547FD2fA53Aa64F587B21A9caa9E2D90f"),
        name: "OKX_155",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("Ee1c6537E589a15a15f80961f5594C57beD936fB"),
        name: "OKX_156",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("f332761c673b59B21fF6dfa8adA44d78c12dEF09"),
        name: "OKX_157",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("f51cD688b8744b1bfD2FBa70D050dE85EC4fb9Fb"),
        name: "OKX_158",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("f59869753f41Db720127Ceb8DbB8afAF89030De4"),
        name: "OKX_159",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("267b51c97632225Da2b2e49aEbE59eEF1Af32653"),
        name: "OKX_16",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("f7858Da8a6617f7C6d0fF2bcAFDb6D2eeDF64840"),
        name: "OKX_160",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("F7C63e75B90C60D7b343106A39658F8b3ca6e4D2"),
        name: "OKX_161",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("F81233a61C0D6D13C6FE504DDbbA3E2630eA0c5c"),
        name: "OKX_162",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("fcB21730ac0CD487D8701dFED1170e023B57cf7a"),
        name: "OKX_163",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("Fd92F4e91d54B9EF91cc3f97C011a6aF0C2a7eDa"),
        name: "OKX_164",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("276cdBa3a39aBF9cEdBa0F1948312c0681E6D5Fd"),
        name: "OKX_17",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("297611B7aCcB7F24032c27B3a496465624c7Ef50"),
        name: "OKX_18",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("2c8FBB630289363Ac80705A1a61273f76fD5a161"),
        name: "OKX_19",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("06959153B974D0D5fDfd87D561db6d8d4FA0bb0B"),
        name: "OKX_2",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("2D2cC0eB095e43204E0C087E07Dbf95909650939"),
        name: "OKX_20",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("30C1DcdE81e5dbF3121D0408abC7908980e83aE2"),
        name: "OKX_21",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("313Eb1C5e1970EB5CEEF6AEbad66b07c7338d369"),
        name: "OKX_22",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("3b5a23f6207d87B423C6789D2625eA620423b32D"),
        name: "OKX_23",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("3c5883C650d600bd543A9B5c8D9A3a6f5d16b8f4"),
        name: "OKX_24",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("3D55CCb2a943d88D39dd2E62DAf767C69fD0179F"),
        name: "OKX_25",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("3f482c72a2b3e777746f5755cC0Ff1323eA2Ad16"),
        name: "OKX_26",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("3F83fd98d5fA84F3cBF8B275D6A10dFC5605CDa2"),
        name: "OKX_27",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("4106A4be14867c70E52c51B9805514Ae2e16dE64"),
        name: "OKX_28",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("41205307b6618F03bE2d95747a07311456bdb143"),
        name: "OKX_29",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("06d3a30cBb00660B85a30988D197B1c282c6dCB6"),
        name: "OKX_3",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("42436286A9c8d63AAfC2eEbBCA193064d68068f2"),
        name: "OKX_30",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("42Cf18596EE08E877d532Df1b7cF763059A7EA57"),
        name: "OKX_31",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("45CeaBB79cF4aba1E9781CEc35ce726f8a1A9309"),
        name: "OKX_32",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("461249076B88189f8AC9418De28B365859E46BfD"),
        name: "OKX_33",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("47Eb32dEa1ab1436187939fa72D6d5FF884A87Da"),
        name: "OKX_34",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("48480aAd203e8b030F82754B3c75869C9895C6Bf"),
        name: "OKX_35",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("48C4c83BE7e3884ee5043a3ABE5115eB020b5f4a"),
        name: "OKX_36",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("4a11078a99b118BbFee78a5c187D98D264360433"),
        name: "OKX_37",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("4A8F1F5B2A3652131eAc54a6f183A4a2cF44A9A6"),
        name: "OKX_38",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("4b4e14a3773Ee558b6597070797fd51EB48606e5"),
        name: "OKX_39",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("0799dDbF6F14Db566ca4dF4ff0575c4cC1e7749c"),
        name: "OKX_4",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("4BCbAF34862e6480329477917Cf5cd7b9537a98D"),
        name: "OKX_40",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("4D19C0a5357bC48be0017095d3C871D9aFC3F21d"),
        name: "OKX_41",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("4e2757e46103556f98d4D036D8eFE18389B89f51"),
        name: "OKX_42",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("4E7b110335511F662FDBB01bf958A7844118c0D4"),
        name: "OKX_43",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("5041ed759Dd4aFc3a72b8192C143F72f4724081A"),
        name: "OKX_44",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("52738a51882f35D6b25a3FD0C86089ddBd206821"),
        name: "OKX_45",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("52b311c52436789f3754bD199Bf3886b8CCBab4c"),
        name: "OKX_46",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("539C92186f7C6CC4CbF443F26eF84C595baBBcA1"),
        name: "OKX_47",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("56FD42ECD77C88BDd959Be54aF10d1759b473DfF"),
        name: "OKX_48",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("5793Da1b0c41C7dB8E3Eb8DbcD18fdca94A58535"),
        name: "OKX_49",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("0938C63109801Ee4243a487aB84DFfA2Bba4589e"),
        name: "OKX_5",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("59FAE149A8f8EC74d5bC038F8b76D25b136b9573"),
        name: "OKX_50",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("5A150733cb59Bbdd5C7398a8fE7Da7F97c8a213a"),
        name: "OKX_51",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("5B27e98516fD2Bd5001D4dfE3f5a2263f702f634"),
        name: "OKX_52",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("5b686a7DB873AC258bbC16e8306594e3e862B3FC"),
        name: "OKX_53",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("5C52cC7c96bDE8594e5B77D5b76d042CB5FaE5f2"),
        name: "OKX_54",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("5F8215eE653Cb7225c741C7aA8591468d1f158b8"),
        name: "OKX_55",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("5FF13e9A3EEd7a2Dbcb1Dfa21dfB1c07F3419277"),
        name: "OKX_56",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("6182672EfCB2DdF094F7BFd3E92eCD10E718E4e1"),
        name: "OKX_57",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("62383739D68Dd0F844103Db8dFb05a7EdED5BBE6"),
        name: "OKX_58",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("65A0947BA5175359Bb457D3b34491eDf4cBF7997"),
        name: "OKX_59",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("0F51A310a4Dd79d373eB8bE1c0ddd54570235443"),
        name: "OKX_6",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("6667A4B7EFf4A0B86781fB3B187622CD3c257F09"),
        name: "OKX_60",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("68841a1806fF291314946EebD0cdA8b348E73d6D"),
        name: "OKX_61",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("68b5E9E083BFC28c33cfbf3F19D33e629015E907"),
        name: "OKX_62",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("68E2EA1622AA67FB3a01a66D132daed8A48d1662"),
        name: "OKX_63",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("69a722f0B5Da3aF02b4a205D6F0c285F4ed8F396"),
        name: "OKX_64",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("6a4561eF7874A76B4bf0D3edca31DFCD51603414"),
        name: "OKX_65",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("6b2C0c7be2048Daa9b5527982C29f48062B34D58"),
        name: "OKX_66",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("6B7ba57e1d43c2C975Ba25139A04D193e64a11D0"),
        name: "OKX_67",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("6cC5F688a315f3dC28A7781717a9A798a59fDA7b"),
        name: "OKX_68",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("6d8c32fCF2d95ff410Ba492f6694F18CbEE55CE1"),
        name: "OKX_69",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("0fF9491b236a36Cc183823E39d7532194143FdB1"),
        name: "OKX_7",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("6Dc1A070425f437Ac08BB108f7093b177D7Af3a6"),
        name: "OKX_70",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("6Df7E0F084D46683E811998847Da3832c9dC3b35"),
        name: "OKX_71",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("6E0Ad348Ce07218f772dc4C05e6c747dA12D664e"),
        name: "OKX_72",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("6e5d525FA1B207b0e42fC3FfDFB9bEd507708904"),
        name: "OKX_73",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("6Fb624B48d9299674022a23d92515e76Ba880113"),
        name: "OKX_74",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("728d92A8023bFbe0d4f3fdd549Ed3b4996a0EBa9"),
        name: "OKX_75",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("730dF969955c0A2fA9e8F2484E9741a363afdbb3"),
        name: "OKX_76",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("7332CD352a84673F1413416ef2E321e17df59844"),
        name: "OKX_77",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("74ac8eC0c6fC83B4127816c23930Bea9E1a83df5"),
        name: "OKX_78",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("793Aa889e19A130ee4cB8b63c79Aa3BDccC663CB"),
        name: "OKX_79",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("10374bC1c4cA086e22673dBc4d702Fee74C4ffc5"),
        name: "OKX_8",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("7Afbf56C48d38D732E8B71dB229a20A2eaEa8532"),
        name: "OKX_80",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("7E4aA755550152a522d9578621EA22eDAb204308"),
        name: "OKX_81",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("7eb6c83AB7D8D9B8618c0Ed973cbEF71d1921EF2"),
        name: "OKX_82",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("868daB0b8E21EC0a48b726A1ccf25826c78C6d7F"),
        name: "OKX_83",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("8734BCa44102bf8A663e1ba112308504606E1b08"),
        name: "OKX_84",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("8744f9a43c22c804553835bA33c5c402af3C79D6"),
        name: "OKX_85",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("88288100c2005c5cE9B06956BeD357F3CcC95b9E"),
        name: "OKX_86",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("88956282d52Eee0aE1Bf8Eaf98Bc6Eac2250B681"),
        name: "OKX_87",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("88C94d8e7d4203B185B3Beb0a7B15A6b4F36B2A2"),
        name: "OKX_88",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("89B59726f9C42c350641182536Bb045B9C6A36cA"),
        name: "OKX_89",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("11817afB29279703c5679959417015328cA6A0D1"),
        name: "OKX_9",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("89FD00b8D2dCEE0f40D8699970115BB861241a54"),
        name: "OKX_90",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("8c3CB9665833fD9f79eB14cbA16D82BBAB6F22D8"),
        name: "OKX_91",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("8d0Abeb725Fa260521Ed54985a5F793141329aE9"),
        name: "OKX_92",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("96FDC631F02207B72e5804428DeE274cF2aC0bCD"),
        name: "OKX_93",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("9723b6d608D4841eB4Ab131687a5D4764eb30138"),
        name: "OKX_94",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("98EC059Dc3aDFBdd63429454aEB0c990FBA4A128"),
        name: "OKX_95",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("9B645675E8D64759E5c36E30Dcb766d8CEC3d34F"),
        name: "OKX_96",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("9Cc548536d8Ec1D6c89e6dDd3B0CEEA350Fe73DC"),
        name: "OKX_97",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("9e13bAE2256f968d02Fe0129A0F51788F4e2472f"),
        name: "OKX_98",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("9e3bB2cD5a89FCC4B826230b144c4917881BEa85"),
        name: "OKX_99",
        exchange: "OKX",
    },
    CexAddress {
        address: address!("134530E94d8c603Ca627c114D2d5B206D652e895"),
        name: "OPNX",
        exchange: "OPNX",
    },
    CexAddress {
        address: address!("2335919E01Fa45744815290Ee255dC0C066C47D3"),
        name: "OPNX_1",
        exchange: "OPNX",
    },
    CexAddress {
        address: address!("610887f0AE329557d1aE067F6b7697fF07032116"),
        name: "OPNX_2",
        exchange: "OPNX",
    },
    CexAddress {
        address: address!("E5241AC645cad844e94A2E7486283ED6398Fb3Aa"),
        name: "OPNX_3",
        exchange: "OPNX",
    },
    CexAddress {
        address: address!("AeEc6f5aCA72F3A005af1B3420ab8c8c7009BaC8"),
        name: "OTCBTC",
        exchange: "OTCBTC",
    },
    CexAddress {
        address: address!("7454a609EA877f37FA6D42FEcA76a8f18DE841C7"),
        name: "OceanEx",
        exchange: "OceanEx",
    },
    CexAddress {
        address: address!("904a0600582eD1003016D143756A4F427D5BA344"),
        name: "OceanEx_1",
        exchange: "OceanEx",
    },
    CexAddress {
        address: address!("B385d810CC3Bf0f4A4629528967e18Cd0196E077"),
        name: "OceanEx_2",
        exchange: "OceanEx",
    },
    CexAddress {
        address: address!("03E3fF995863828554282e80870B489cc31dC8bc"),
        name: "Omgfin",
        exchange: "Omgfin",
    },
    CexAddress {
        address: address!("0BC99404E0fCa2028D7665e3B473869C6C6FF002"),
        name: "Oobit",
        exchange: "Oobit",
    },
    CexAddress {
        address: address!("65FB180C8bfAA811e9b2E3d2fa60EF27c15F8732"),
        name: "Oobit_1",
        exchange: "Oobit",
    },
    CexAddress {
        address: address!("a8EE70F68fffAAE3cb8475112409ADB9282f3246"),
        name: "Oobit_2",
        exchange: "Oobit",
    },
    CexAddress {
        address: address!("bF2EbF6C701068CCf046644315Ff7417C879bd8A"),
        name: "Oobit_3",
        exchange: "Oobit",
    },
    CexAddress {
        address: address!("f6BE265fa72148DFb64106247d21BB15CE650E5e"),
        name: "Oobit_4",
        exchange: "Oobit",
    },
    CexAddress {
        address: address!("aFDa8eBA0aC933661F45b41a438840dc07DF1761"),
        name: "Orionx",
        exchange: "Orionx",
    },
    CexAddress {
        address: address!("b709D82f0706476457ae6baD7C3534fBf424382c"),
        name: "Panda Exchange",
        exchange: "Panda",
    },
    CexAddress {
        address: address!("caCc694840eCeBaDD9B4c419E5B7f1D73FEdf999"),
        name: "Panda Exchange_1",
        exchange: "Panda Exchange",
    },
    CexAddress {
        address: address!("04D9199D397Ed9b0C497ad9bbc10F0E0047DD3B7"),
        name: "Paribu",
        exchange: "Paribu",
    },
    CexAddress {
        address: address!("2bB97B6CF6FfE53576032c11711D59Bd056830eE"),
        name: "Paribu_1",
        exchange: "Paribu",
    },
    CexAddress {
        address: address!("EB91A3754E48Fcd8d3e6F89528D67Ac5386Cd0D0"),
        name: "Paribu_10",
        exchange: "Paribu",
    },
    CexAddress {
        address: address!("FB90501083a3b6AF766c8dA35d3Dde01eB0d2a68"),
        name: "Paribu_11",
        exchange: "Paribu",
    },
    CexAddress {
        address: address!("595063172C85B1e8AC2fe74Fcb6b7dC26844CC2D"),
        name: "Paribu_2",
        exchange: "Paribu",
    },
    CexAddress {
        address: address!("64440d8A6E6F949536646C363A4b734819EeDbfd"),
        name: "Paribu_3",
        exchange: "Paribu",
    },
    CexAddress {
        address: address!("9acbB72Cf67103A30333A32CD203459c6a9c3311"),
        name: "Paribu_4",
        exchange: "Paribu",
    },
    CexAddress {
        address: address!("9bC5A1d65cb56288C0d110Ce2Da3D0aFB3F573cd"),
        name: "Paribu_5",
        exchange: "Paribu",
    },
    CexAddress {
        address: address!("abc74170f3Cb8Ab352820C39cC1d1e05cE9e41D3"),
        name: "Paribu_6",
        exchange: "Paribu",
    },
    CexAddress {
        address: address!("Bd8ef191Caa1571e8aD4619ae894e07A75De0C35"),
        name: "Paribu_7",
        exchange: "Paribu",
    },
    CexAddress {
        address: address!("c6da94cA4CdBE74B777B53e42470F676c44B9ab6"),
        name: "Paribu_8",
        exchange: "Paribu",
    },
    CexAddress {
        address: address!("C86562C1E08cac700F0dDb33EB5A148EC1227d62"),
        name: "Paribu_9",
        exchange: "Paribu",
    },
    CexAddress {
        address: address!("17F1a51dA68d27c94D2a51d92B27B5Bd4718b986"),
        name: "Paxful",
        exchange: "Paxful",
    },
    CexAddress {
        address: address!("777d4627E31863b2a49e2985AF46525F21a9846C"),
        name: "Paxful_1",
        exchange: "Paxful",
    },
    CexAddress {
        address: address!("7A20527ba5a749b3b054a821950Bfcc2C01b959f"),
        name: "Paxful_2",
        exchange: "Paxful",
    },
    CexAddress {
        address: address!("0C23fc0Ef06716D2f8ba19bC4bEd56D045581F2d"),
        name: "Paxos",
        exchange: "Paxos",
    },
    CexAddress {
        address: address!("264bd8291fAE1D75DB2c5F573b07faA6715997B5"),
        name: "Paxos_1",
        exchange: "Paxos",
    },
    CexAddress {
        address: address!("286AF5CF60aE834199949bBc815485f07CC9C644"),
        name: "Paxos_2",
        exchange: "Paxos",
    },
    CexAddress {
        address: address!("41b309236C87b1bc6FA8Eb865833E44158Fa991a"),
        name: "Paxos_3",
        exchange: "Paxos",
    },
    CexAddress {
        address: address!("5195427ca88DF768c298721dA791B93AD11ECa65"),
        name: "Paxos_4",
        exchange: "Paxos",
    },
    CexAddress {
        address: address!("7d766B06e7164Be4196EE62E6036c9FCFF68107d"),
        name: "Paxos_5",
        exchange: "Paxos",
    },
    CexAddress {
        address: address!("8a89E016750479Dc1d7Ad32ecfFCeCd76E118697"),
        name: "Paxos_6",
        exchange: "Paxos",
    },
    CexAddress {
        address: address!("E25a329d385f77df5D4eD56265babe2b99A5436e"),
        name: "Paxos_7",
        exchange: "Paxos",
    },
    CexAddress {
        address: address!("5D9fE07813a260857Cf60639daC710EBb9531a20"),
        name: "PayKassa.pro",
        exchange: "PayKassa.pro",
    },
    CexAddress {
        address: address!("8b8a4abc707F16dA24B795e3e46ed22975A9D329"),
        name: "PayKassa.pro_1",
        exchange: "PayKassa.pro",
    },
    CexAddress {
        address: address!("eCceFaB82bb383afC90b94C8d378DE314e62AC5D"),
        name: "PayKassa.pro_2",
        exchange: "PayKassa.pro",
    },
    CexAddress {
        address: address!("D65E0Cbd31977B2E0e23c8330C8B5f020818Fc91"),
        name: "Paybis",
        exchange: "Paybis",
    },
    CexAddress {
        address: address!("203304a42132928Fa77e2285cC05111693795328"),
        name: "Peatio",
        exchange: "Peatio",
    },
    CexAddress {
        address: address!("468B858c964f297Fd8Fce058032BF4B4911A8ad8"),
        name: "Peatio_1",
        exchange: "Peatio",
    },
    CexAddress {
        address: address!("4BCBe67E72F9324F5E9B3F3dB45Ab47768401227"),
        name: "Peatio_2",
        exchange: "Peatio",
    },
    CexAddress {
        address: address!("7B8ce58fAb63B225f776417c903A1191f497D211"),
        name: "Peatio_3",
        exchange: "Peatio",
    },
    CexAddress {
        address: address!("88e343F4599292C2CfFe683C1bb93cD3480BdbAb"),
        name: "Peatio_4",
        exchange: "Peatio",
    },
    CexAddress {
        address: address!("A6c33332A253E0927b320401626999F6E534722a"),
        name: "Peatio_5",
        exchange: "Peatio",
    },
    CexAddress {
        address: address!("D4Dcd2459BB78d7a645Aa7E196857D421b10D93F"),
        name: "Peatio_6",
        exchange: "Peatio",
    },
    CexAddress {
        address: address!("f5EF8590b55dB151D559c3954e994A6c4DAF12Bb"),
        name: "Peatio_7",
        exchange: "Peatio",
    },
    CexAddress {
        address: address!("2710EB8B7f29def9Fc856c87Aeb64d05d068DA3A"),
        name: "Phemex",
        exchange: "Phemex",
    },
    CexAddress {
        address: address!("35D2d03607b9155b42CF673102FE58251AC4F644"),
        name: "Phemex_1",
        exchange: "Phemex",
    },
    CexAddress {
        address: address!("50BE13b54f3EeBBe415d20250598D81280e56772"),
        name: "Phemex_2",
        exchange: "Phemex",
    },
    CexAddress {
        address: address!("576318Ab7A36A40EBaE463a1604396167eC04B35"),
        name: "Phemex_3",
        exchange: "Phemex",
    },
    CexAddress {
        address: address!("c7eC1A730f718Fb6931D10caEBd85742184ee359"),
        name: "Phemex_4",
        exchange: "Phemex",
    },
    CexAddress {
        address: address!("f7D13C7dBec85ff86Ee815f6dCbb3DEDAc78ca49"),
        name: "Phemex_5",
        exchange: "Phemex",
    },
    CexAddress {
        address: address!("fe065653cFf3154F44eD30EE876570E49051A6E8"),
        name: "Phemex_6",
        exchange: "Phemex",
    },
    CexAddress {
        address: address!("007abbe8057433641aCB791d966D33a12cf82d01"),
        name: "Poloniex",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("0536806df512D6cDDE913Cf95c9886f65b1D3462"),
        name: "Poloniex_1",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("6B71834D65C5C4d8eD158D54B47E6Ea4Ff4E5437"),
        name: "Poloniex_10",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("6F803466bCD17f44fa18975bf7c509ba64Bf3825"),
        name: "Poloniex_11",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("8d451AE5ee8F557a9cE7A9D7Be8A8cb40002d5cB"),
        name: "Poloniex_12",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("8fCA4adE3a517133fF23ca55CdAea29C78C990b8"),
        name: "Poloniex_13",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("A910f92ACdAf488fa6eF02174fb86208Ad7722ba"),
        name: "Poloniex_14",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("Aa9fa73dFE17ecAa2C89b39f0bb2779613C5Fc3b"),
        name: "Poloniex_15",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("aB11204cfEacCFfa63C2D23AeF2Ea9aCCDB0a0D5"),
        name: "Poloniex_16",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("b42b20ddbEabdC2a288Be7FF847fF94fB48d2579"),
        name: "Poloniex_17",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("b794F5eA0ba39494cE839613fffBA74279579268"),
        name: "Poloniex_18",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("Bd2Ec7c608a06fE975DBDCA729E84dEdb34eCC21"),
        name: "Poloniex_19",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("209c4784AB1E8183Cf58cA33cb740efbF3FC18EF"),
        name: "Poloniex_2",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("BFC39b6F805a9E40E77291afF27aeE3C96915BDD"),
        name: "Poloniex_20",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("c0e30823e5e628df8bc9bf2636a347E1512F0ecb"),
        name: "Poloniex_21",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("Df21fA922215B1a56f5a6D6294E6E36c85A0Acfb"),
        name: "Poloniex_22",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("EaD6be34CE315940264519f250d8160f369fa5cd"),
        name: "Poloniex_23",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("fbf2173154F7625713be22E0504404EBfE021eae"),
        name: "Poloniex_24",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("2fA2Bc2ce6A4f92952921A4CAA46B3727D24a1ec"),
        name: "Poloniex_3",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("31a2Feb9b5D3b5f4e76C71D6C92FC46eBb3cb1c1"),
        name: "Poloniex_4",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("32Be343B94f860124dC4fEe278FDCBD38C102D88"),
        name: "Poloniex_5",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("36B01066b7fa4a0fdb2968eA0256C848e9135674"),
        name: "Poloniex_6",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("48d466B7c0d32B61E8A82Cd2bCF060F7C3F966df"),
        name: "Poloniex_7",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("65F9B2e4d7aAEB40fFEA8C6F5844d5AD7Da257E0"),
        name: "Poloniex_8",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("6795cf8EB25585EaDC356Ae32AC6641016c550f2"),
        name: "Poloniex_9",
        exchange: "Poloniex",
    },
    CexAddress {
        address: address!("33FE7Ad77394281e43Cc82D86ad0cbb5b9e9575D"),
        name: "Prime Trust",
        exchange: "Prime",
    },
    CexAddress {
        address: address!("352e0242a58c4F43dc40F3Ee9a2eA14CcC6Bb2ea"),
        name: "Prime Trust_1",
        exchange: "Prime Trust",
    },
    CexAddress {
        address: address!("9416fd2bc773C85A65d699cA9fC9525F1424Df94"),
        name: "Prime Trust_2",
        exchange: "Prime Trust",
    },
    CexAddress {
        address: address!("d8b81f6849dFbBe7B8f3c32bbB3A15aD2AdB6898"),
        name: "Prime Trust_3",
        exchange: "Prime Trust",
    },
    CexAddress {
        address: address!("DDBB8d8B5Da3dFAf65D4F8FA846127ACD6A844B1"),
        name: "Prime Trust_4",
        exchange: "Prime Trust",
    },
    CexAddress {
        address: address!("72E5263FF33D2494692D7F94A758aA9F82062F73"),
        name: "ProBit",
        exchange: "ProBit",
    },
    CexAddress {
        address: address!("aD285fDEDFC0D5f944A33e478356524293c7eC68"),
        name: "ProBit_1",
        exchange: "ProBit",
    },
    CexAddress {
        address: address!("dBA24f19Bce0F32ea4273FaeA7C01D7f9D4F91D6"),
        name: "ProBit_2",
        exchange: "ProBit",
    },
    CexAddress {
        address: address!("F71AfE21Cd32959113Fc47aE2EF886B43A9413d5"),
        name: "ProBit_3",
        exchange: "ProBit",
    },
    CexAddress {
        address: address!("07e551E31A793E20dc18494ff6b03095A8F8Ee36"),
        name: "QMall",
        exchange: "QMall",
    },
    CexAddress {
        address: address!("0C0511d1eE844A516B6bDa54db3bcA01E2cE2A19"),
        name: "QMall_1",
        exchange: "QMall",
    },
    CexAddress {
        address: address!("5d636F90B48c9f14BD0BF9D8016d4cB0DD9e1D9f"),
        name: "QMall_2",
        exchange: "QMall",
    },
    CexAddress {
        address: address!("d3e5b815843C31f621f2253c836B34f84debFE29"),
        name: "QMall_3",
        exchange: "QMall",
    },
    CexAddress {
        address: address!("027BEEFcBaD782faF69FAD12DeE97Ed894c68549"),
        name: "QuadrigaCX",
        exchange: "QuadrigaCX",
    },
    CexAddress {
        address: address!("0EE4E2d09AEC35Bdf08083b649033Ac0A41aa75E"),
        name: "QuadrigaCX_1",
        exchange: "QuadrigaCX",
    },
    CexAddress {
        address: address!("5B5B69f4E0add2Df5d2176D7dBd20B4897bc7eC4"),
        name: "QuadrigaCX_2",
        exchange: "QuadrigaCX",
    },
    CexAddress {
        address: address!("B6AaC3b56FF818496B747EA57fCBe42A9aae6218"),
        name: "QuadrigaCX_3",
        exchange: "QuadrigaCX",
    },
    CexAddress {
        address: address!("2a048d9A8fFDd239F063B09854976c3049AE659C"),
        name: "QuantaEx",
        exchange: "QuantaEx",
    },
    CexAddress {
        address: address!("5cA39c42F4dEE3A5Ba8FEc3Ad4902157D48700bf"),
        name: "QuantaEx_1",
        exchange: "QuantaEx",
    },
    CexAddress {
        address: address!("d344539efe31f8b6DE983A0Cab4Fb721fC69C547"),
        name: "QuantaEx_2",
        exchange: "QuantaEx",
    },
    CexAddress {
        address: address!("8a37F0290AE85D08522d2A605617e76128Fd0712"),
        name: "Ramp Network",
        exchange: "Ramp",
    },
    CexAddress {
        address: address!("98DB3a41bF8bF4DeD2C92A84ec0705689DdEEF8B"),
        name: "Ramp Network_1",
        exchange: "Ramp Network",
    },
    CexAddress {
        address: address!("2819c144D5946404C0516B6f817a960dB37D4929"),
        name: "Remitano",
        exchange: "Remitano",
    },
    CexAddress {
        address: address!("7982789Dd8b4D4a76783d5d43b94c55B75bdEe0B"),
        name: "Remitano_1",
        exchange: "Remitano",
    },
    CexAddress {
        address: address!("7Ae17a0f6f8F02b5B6e76b327DB15F91306194E6"),
        name: "Remitano_2",
        exchange: "Remitano",
    },
    CexAddress {
        address: address!("8365EFb25D0822AaF15Ee1D314147B6a7831C403"),
        name: "Remitano_3",
        exchange: "Remitano",
    },
    CexAddress {
        address: address!("8b2f57d12AE055f26Fb643f9C4F64FfFe9F4c6a1"),
        name: "Remitano_4",
        exchange: "Remitano",
    },
    CexAddress {
        address: address!("ac180b9Be764Fb542Cac26D6A2D227fa8E7792EA"),
        name: "Remitano_5",
        exchange: "Remitano",
    },
    CexAddress {
        address: address!("B8CF411b956B3f9013C1d0Ac8C909b086218207c"),
        name: "Remitano_6",
        exchange: "Remitano",
    },
    CexAddress {
        address: address!("28c9386eBab8D52Ead4A327e6423316435B2d4fc"),
        name: "RenrenBit",
        exchange: "RenrenBit",
    },
    CexAddress {
        address: address!("2b3FeD49557bd88f78b898684F82FBb355305DbB"),
        name: "Revolut",
        exchange: "Revolut",
    },
    CexAddress {
        address: address!("9b0c45d46D386cEdD98873168C36efd0DcBa8d46"),
        name: "Revolut_1",
        exchange: "Revolut",
    },
    CexAddress {
        address: address!("b23360CCDd9Ed1b15D45E5d3824Bb409C8D7c460"),
        name: "Revolut_2",
        exchange: "Revolut",
    },
    CexAddress {
        address: address!("C44b7316936E2F004E688fD53a95e060Df1811C3"),
        name: "Revolut_3",
        exchange: "Revolut",
    },
    CexAddress {
        address: address!("2eFB50e952580f4ff32D8d2122853432bbF2E204"),
        name: "Robinhood",
        exchange: "Robinhood",
    },
    CexAddress {
        address: address!("40B38765696e3d5d8d9d834D8AaD4bB6e418E489"),
        name: "Robinhood_1",
        exchange: "Robinhood",
    },
    CexAddress {
        address: address!("4A5b84fb4c7666692C49F2E11664710AA4D0d2a0"),
        name: "Robinhood_2",
        exchange: "Robinhood",
    },
    CexAddress {
        address: address!("6081258689a75d253d87cE902A8de3887239Fe80"),
        name: "Robinhood_3",
        exchange: "Robinhood",
    },
    CexAddress {
        address: address!("7222dE11e132C6F315789eEb5C0182caBD4a9530"),
        name: "Robinhood_4",
        exchange: "Robinhood",
    },
    CexAddress {
        address: address!("73AF3bcf944a6559933396c1577B257e2054D935"),
        name: "Robinhood_5",
        exchange: "Robinhood",
    },
    CexAddress {
        address: address!("97972fA6D980aA9B93D1b584541055840302dE05"),
        name: "Robinhood_6",
        exchange: "Robinhood",
    },
    CexAddress {
        address: address!("A0116A92A032D17a9Ce431EaBE75C5B5F29E2d5E"),
        name: "Robinhood_7",
        exchange: "Robinhood",
    },
    CexAddress {
        address: address!("a26e73C8E9507D50bF808B7A2CA9D5dE4fcC4A04"),
        name: "Robinhood_8",
        exchange: "Robinhood",
    },
    CexAddress {
        address: address!("8aE57A027c63fcA8070D1Bf38622321dE8004c67"),
        name: "Rollbit",
        exchange: "Rollbit",
    },
    CexAddress {
        address: address!("CBD6832Ebc203e49E2B771897067fce3c58575ac"),
        name: "Rollbit_1",
        exchange: "Rollbit",
    },
    CexAddress {
        address: address!("Ef8801eaf234ff82801821FFe2d78D60a0237F97"),
        name: "Rollbit_2",
        exchange: "Rollbit",
    },
    CexAddress {
        address: address!("9eBe47c83C996E5cBBe44c423d7F20DB19dEfc39"),
        name: "Roobet",
        exchange: "Roobet",
    },
    CexAddress {
        address: address!("C94eBB328aC25b95DB0E0AA968371885Fa516215"),
        name: "Roobet_1",
        exchange: "Roobet",
    },
    CexAddress {
        address: address!("000F422887eA7d370FF31173FD3B46c8F66A5B1c"),
        name: "Shakepay",
        exchange: "Shakepay",
    },
    CexAddress {
        address: address!("3B794929566e3Ba0f25e4263e1987828b5c87161"),
        name: "Shakepay_1",
        exchange: "Shakepay",
    },
    CexAddress {
        address: address!("4d846dA8257BB0Ebd164EFf513DfF0F0c2C3c0ba"),
        name: "Shakepay_2",
        exchange: "Shakepay",
    },
    CexAddress {
        address: address!("5EaE73d4D24B2922FE614D4F58018b34A7E20a83"),
        name: "Shakepay_3",
        exchange: "Shakepay",
    },
    CexAddress {
        address: address!("88DCdd4A0A58b7e2208805D547043c37dca2b6Dc"),
        name: "Shakepay_4",
        exchange: "Shakepay",
    },
    CexAddress {
        address: address!("912fD21d7a69678227fE6d08C64222Db41477bA0"),
        name: "Shakepay_5",
        exchange: "Shakepay",
    },
    CexAddress {
        address: address!("114806Fb56456a525199B957a3a04F726d67b847"),
        name: "ShapeShift",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("120A270bbC009644e35F0bB6ab13f95b8199c4ad"),
        name: "ShapeShift_1",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("52B26F14627e2BF706D257BCfA32D71Eb1BFA70e"),
        name: "ShapeShift_10",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("563b377A956c80d77A7c613a9343699Ad6123911"),
        name: "ShapeShift_11",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("5aa107C71A314CADA39db8fe7f9b591f67521C14"),
        name: "ShapeShift_12",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("5D089C8141e58773Bc88fBA973198B3EC95aa501"),
        name: "ShapeShift_13",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("5E44c3E467a49C9Ca0296a9F130fc433041aAa28"),
        name: "ShapeShift_14",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("5F06fc2A5aFe54e926a21FdfeF056F93A7F1E6CB"),
        name: "ShapeShift_15",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("5F42801ac21677008433d70F1060831ff2a04602"),
        name: "ShapeShift_16",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("650C7eA0Ecd6a59b427Ea74c7Aec83f55B47BD02"),
        name: "ShapeShift_17",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("6665B4A947a1E4864E8Ef64a2006Add1899e6EBf"),
        name: "ShapeShift_18",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("70faa28A6B8d6829a4b1E649d26eC9a2a39ba413"),
        name: "ShapeShift_19",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("16A160826b9A2ea7B328b9624ce8971688fC8D4b"),
        name: "ShapeShift_2",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("714D7321fe17aCbcf1D64FD3A48aC22d3d214204"),
        name: "ShapeShift_20",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("75cDd6bEDACfb4841378C21b076d3E1852a8bB64"),
        name: "ShapeShift_21",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("7B9Bc474667Db2fFE5b08d000F1Acc285B2Ae47D"),
        name: "ShapeShift_22",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("7Fa926A56D376925d253E10bde289a749A51BE24"),
        name: "ShapeShift_23",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("8a65ac0E23F31979db06Ec62Af62b132a6dF4741"),
        name: "ShapeShift_24",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("915EC7DE9C792bF9b530178162D5E87E3d22c632"),
        name: "ShapeShift_25",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("923ff2b262f4BBE0bcCE7f477De6652905F31A2B"),
        name: "ShapeShift_26",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("9BcB0733C56B1D8F0c7c4310949E00485cAe4E9d"),
        name: "ShapeShift_27",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("9e6316f44BaEeeE5d41A1070516cc5fA47BAF227"),
        name: "ShapeShift_28",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("a345341B99B36C2eC355333199999c73B17bdA9b"),
        name: "ShapeShift_29",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("2492D1C00953AD258E1ce6363eb474595f5279F2"),
        name: "ShapeShift_3",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("A620958b0F7D59ed764e77423960a3cb9321680b"),
        name: "ShapeShift_30",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("b36eFd48c9912Bd9fd58b67b65f7438F6364a256"),
        name: "ShapeShift_31",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("Ba991DBF44893D237829fE23D3973971B88f8F5e"),
        name: "ShapeShift_32",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("bfc1fC4d2546AF6420D2Fc14819a1Da2A8E9dD6b"),
        name: "ShapeShift_33",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("c7BA53854Ea347EDCe88c37bB99a699EdbB77096"),
        name: "ShapeShift_34",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("caE39061F41686e1Aaf9cf10145e5d4a4265635C"),
        name: "ShapeShift_35",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("D063435D7caB1A792e1D56F7Aab04313B3D87179"),
        name: "ShapeShift_36",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("D3273EBa07248020bf98A8B560ec1576a612102F"),
        name: "ShapeShift_37",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("Da1E5D4Cc9873963f788562354b55A772253b92f"),
        name: "ShapeShift_38",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("DA8b9075f0F3094EC6F614C36E92317813C80957"),
        name: "ShapeShift_39",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("2624eDaFCE546781883c26Fc9C461D4c8E782Ef9"),
        name: "ShapeShift_4",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("df69de4a2a58866afeBb7713e3dd10C2153fF27C"),
        name: "ShapeShift_40",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("e65A88f487F5d26469Cfd37ce7Ef763D6d9BE454"),
        name: "ShapeShift_41",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("e8ed915E208B28c617d20F3F8Ca8e11455933aDf"),
        name: "ShapeShift_42",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("E9319eBA87Af7C2fc1F55ccDe9d10eA8efbd592d"),
        name: "ShapeShift_43",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("E93E588821A00a9F2ff3f9E40E224cAA5118f275"),
        name: "ShapeShift_44",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("eed16856D551569D134530ee3967Ec79995E2051"),
        name: "ShapeShift_45",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("F08BDf21373A09aB7eDD7769A402D3a22826D317"),
        name: "ShapeShift_46",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("f2038B4368FC6F4eEcbeaE93F37262F1De3a23e6"),
        name: "ShapeShift_47",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("F316e9af231C1c5004540c220c78627F8A9f5419"),
        name: "ShapeShift_48",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("F610Fae41259f6FE66dC32fAc20ba6D2d72A506b"),
        name: "ShapeShift_49",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("2e0714166a5095D0730B97110A11028158F1F3b7"),
        name: "ShapeShift_5",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("2f155ddeFC29c414C94b801B91F55B257231825E"),
        name: "ShapeShift_6",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("39D3b15006e580077a2E8B51B93BE90cCF1EC0e0"),
        name: "ShapeShift_7",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("3AEf01dB231c3C9fF844f7E611c63b8c36bc6A02"),
        name: "ShapeShift_8",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("3b0BC51Ab9De1e5B7B6E34E5b960285805C41736"),
        name: "ShapeShift_9",
        exchange: "ShapeShift",
    },
    CexAddress {
        address: address!("3ee1fac3d8cC67A0676830622D3AFc55cF6ffF27"),
        name: "Sideshift",
        exchange: "Sideshift",
    },
    CexAddress {
        address: address!("6cf6a8488D70b1743134d6D69950cDa60325A42F"),
        name: "Sideshift_1",
        exchange: "Sideshift",
    },
    CexAddress {
        address: address!("722b33b843BAca81aa70cEf29C9512de7B3f8767"),
        name: "Sideshift_2",
        exchange: "Sideshift",
    },
    CexAddress {
        address: address!("cDd37Ada79F589c15bD4f8fD2083dc88E34A2af2"),
        name: "Sideshift_3",
        exchange: "Sideshift",
    },
    CexAddress {
        address: address!("F7E00e8Df9d41B891bbA8263f74c7ec23C12Acac"),
        name: "Sideshift_4",
        exchange: "Sideshift",
    },
    CexAddress {
        address: address!("09fe30D5B6e19B38F04a01A217519cECa15B5388"),
        name: "SimpleSwap",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("0FdD8454CdA144b88955e8bb7931456989f853CC"),
        name: "SimpleSwap_1",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("Bb3fd383d1C5540E52EF0A7bcb9433375793aEAF"),
        name: "SimpleSwap_10",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("BBE4B05aAEF7526153888d0cdd054b78c72A7E85"),
        name: "SimpleSwap_11",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("Ca604a3e8B6277492EbC558a4457B6e60e611096"),
        name: "SimpleSwap_12",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("d8F9Ced745e429Ea0723aA72693EFf03B5182DC7"),
        name: "SimpleSwap_13",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("eE7f2D8257Aa658c5895796f070e4046bA8Fb37e"),
        name: "SimpleSwap_14",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("1d05ACf4e760b1E06C735B67818fdc91558Df17d"),
        name: "SimpleSwap_2",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("32E9dc9968Fab4C4528165cd37B613dD5d229650"),
        name: "SimpleSwap_3",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("40Bbfa70b338efD6E81D93eD0a25A2cB67bB7Bb9"),
        name: "SimpleSwap_4",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("4B0401Fe6B84C52d4F4310c371c731a2B6D0964D"),
        name: "SimpleSwap_5",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("59B36B4b1B25bc61C8A81eaf70aD923D149F3d95"),
        name: "SimpleSwap_6",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("7BaCd3E83522F484Bc5128EA93Bf7290f1F1B9E5"),
        name: "SimpleSwap_7",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("876470570C01806261A981D653C4A601CD6875c0"),
        name: "SimpleSwap_8",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("afd99a1a7e2195a8E0fdB6e8bD45EFDff15FEadD"),
        name: "SimpleSwap_9",
        exchange: "SimpleSwap",
    },
    CexAddress {
        address: address!("6ec88a2Cb932eb46dfda0280c0eadB93b6eCa13B"),
        name: "Simplex",
        exchange: "Simplex",
    },
    CexAddress {
        address: address!("77300C71071eCa35Cb673a0b7571B2907dEB77C7"),
        name: "Simplex_1",
        exchange: "Simplex",
    },
    CexAddress {
        address: address!("324cC2c9fb379EA7a0D1C0862C3b48cA28D174A4"),
        name: "SouthXchange",
        exchange: "SouthXchange",
    },
    CexAddress {
        address: address!("91F6d99b232153CB655Ad3E0d05e13EF505F6cd5"),
        name: "Sparrow",
        exchange: "Sparrow",
    },
    CexAddress {
        address: address!("e855283086FbEe485aECF2084345A91424c23954"),
        name: "Sparrow_1",
        exchange: "Sparrow",
    },
    CexAddress {
        address: address!("019D0706D65c4768ec8081eD7CE41F59Eef9b86c"),
        name: "Stake.com",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("0392b64B8BfDA184F0A72cE37D73dC7dF978C4f7"),
        name: "Stake.com_1",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("bBc43C282B2f829176F4Fc3802436D8fAD3413F3"),
        name: "Stake.com_10",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("DebfBE80C8aebA98A32968278463ccB639C6C4e3"),
        name: "Stake.com_11",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("F598b81Ef8c7b52a7F2a89253436e72ec6DC871f"),
        name: "Stake.com_12",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("Fa500178de024BF43CFA69B7e636A28AB68F2741"),
        name: "Stake.com_13",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("6e29f75b0350fd0e85EE34a21eF94767b0186996"),
        name: "Stake.com_2",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("6F419642AD147853A91E1CB50D4B909dde19Cece"),
        name: "Stake.com_3",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("758BE77a3eE14e7193730560daA07dd3fcBFD200"),
        name: "Stake.com_4",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("787B8840100d9BaAdD7463f4a73b5BA73B00C6cA"),
        name: "Stake.com_5",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("974CaA59e49682CdA0AD2bbe82983419A2ECC400"),
        name: "Stake.com_6",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("A29148c2A656E5Ddc68acB95626D6B64A1131c06"),
        name: "Stake.com_7",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("b04c0EB29C72cEBC467b9d4944D29116fa02C44a"),
        name: "Stake.com_8",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("B2723BEacce4BC54F23544343927f048CeF6bD5A"),
        name: "Stake.com_9",
        exchange: "Stake.com",
    },
    CexAddress {
        address: address!("0542Df7daCc8716653Df3fd9F991520AA2f2D0bc"),
        name: "Steam Exchange",
        exchange: "Steam",
    },
    CexAddress {
        address: address!("c0924EDEFB2C0C303de2d0c21BfF07ab763163B5"),
        name: "Steam Exchange_1",
        exchange: "Steam Exchange",
    },
    CexAddress {
        address: address!("7D2d2bC5FB453673c3E31c6b002ef78613165CDC"),
        name: "Stex",
        exchange: "Stex",
    },
    CexAddress {
        address: address!("97E12BD75bdee72d4975D6df410D2d145b3d8457"),
        name: "Stex_1",
        exchange: "Stex",
    },
    CexAddress {
        address: address!("9BF25700727d10a857099D1033Ce2cC493c3B61A"),
        name: "Streamity",
        exchange: "Streamity",
    },
    CexAddress {
        address: address!("0A52368D5a7E70D8c927f75Ea6618C2c468031D4"),
        name: "SwissBorg",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("11444C6389A26C8E41d7FD5CafBfCC511303b7d3"),
        name: "SwissBorg_1",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("691e3Cbb2a8F504fC650F21C9af6226051340559"),
        name: "SwissBorg_10",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("6Cf9AA65EBaD7028536E353393630e2340ca6049"),
        name: "SwissBorg_11",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("7153D2ef9F14a6b1Bb2Ed822745f65E58d836C3F"),
        name: "SwissBorg_12",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("87cbc48075d7aa1760Ac71C41e8Bc289b6A31F56"),
        name: "SwissBorg_13",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("94596096320A6B4EaB43556AD1Ed8c4c3d51C9aA"),
        name: "SwissBorg_14",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("A03D3611B34C3c49DBcb8206eD08fe6467f684a5"),
        name: "SwissBorg_15",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("a5546c4bC006D23B60D690D3033b8dF40Cecc230"),
        name: "SwissBorg_16",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("cDE4c1b984F3F02f997ECfF9980B06316de2577d"),
        name: "SwissBorg_17",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("cDf2bFd0ff20811c98471b331db19AAbf3D3b972"),
        name: "SwissBorg_18",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("D0c3647CB16460A230B4Aa93f3723823Aa17c943"),
        name: "SwissBorg_19",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("22bF0A4C4eff418b3306AbFeE20813D0b6E8Dc74"),
        name: "SwissBorg_2",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("FF4606bd3884554CDbDabd9B6e25E2faD4f6fc54"),
        name: "SwissBorg_20",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("2e8C1131BE1A839B375ac3ED1BA061dF3d87CBFd"),
        name: "SwissBorg_3",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("42b86A269fb3d5368D880c519BadABa77eC00130"),
        name: "SwissBorg_4",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("43fdA7708C97C4C40d5402c6392f0457f23c0b8e"),
        name: "SwissBorg_5",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("5770815B0c2a09A43C9E5AEcb7e2f3886075B605"),
        name: "SwissBorg_6",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("5cEDc1923c33B253aedf24bF038eeE6Cbbb68A6A"),
        name: "SwissBorg_7",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("61488B940E5796b0D4FF096a441ebc48B8eaC6DE"),
        name: "SwissBorg_8",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("67FE3293FC4e877F3CDc3F0ed93721a600f72BdE"),
        name: "SwissBorg_9",
        exchange: "SwissBorg",
    },
    CexAddress {
        address: address!("A96b536eEf496e21F5432FD258b6F78CF3673F74"),
        name: "Switchain",
        exchange: "Switchain",
    },
    CexAddress {
        address: address!("ea3a46BD1dbd0620d80037f70d0bF7c7dc5a837C"),
        name: "TAGZ",
        exchange: "TAGZ",
    },
    CexAddress {
        address: address!("ED8204345a0Cf4639D2dB61a4877128FE5Cf7599"),
        name: "TAGZ_1",
        exchange: "TAGZ",
    },
    CexAddress {
        address: address!("D9D307698e03Db8BE472E92E1c42b0d66245eeAc"),
        name: "TBCC Global",
        exchange: "TBCC",
    },
    CexAddress {
        address: address!("214989c36c5fD378bcBb27F70315049E3D8Aa74c"),
        name: "Thodex",
        exchange: "Thodex",
    },
    CexAddress {
        address: address!("68859697fdC8c3069303Fa87947ADB622Ce990DC"),
        name: "Thodex_1",
        exchange: "Thodex",
    },
    CexAddress {
        address: address!("B6B9bAD197225DEda72f452A2660F813B557cCc2"),
        name: "Thodex_2",
        exchange: "Thodex",
    },
    CexAddress {
        address: address!("0a73573Cf2903d2D8305b1eCb9e9730186a312aE"),
        name: "Tidex",
        exchange: "Tidex",
    },
    CexAddress {
        address: address!("3613ef1125A078EF96Ffc898c4eC28D73C5b8C52"),
        name: "Tidex_1",
        exchange: "Tidex",
    },
    CexAddress {
        address: address!("9acAfC3FE25E7DA2B8bF49c836619d40E395A859"),
        name: "Tidex_2",
        exchange: "Tidex",
    },
    CexAddress {
        address: address!("2F19E5C3C66C44E6405D4c200fE064ECe9bC253a"),
        name: "TigerGaming",
        exchange: "TigerGaming",
    },
    CexAddress {
        address: address!("41292153E7F5e78C3b7382D59E742b92461CBC70"),
        name: "TigerGaming_1",
        exchange: "TigerGaming",
    },
    CexAddress {
        address: address!("bd5CdD1ca9aE5F1443AeC2642D43a538c32a473F"),
        name: "TigerGaming_2",
        exchange: "TigerGaming",
    },
    CexAddress {
        address: address!("D86e3A116Bc98A354253dea47ffd36E86b1A1BC2"),
        name: "TigerGaming_3",
        exchange: "TigerGaming",
    },
    CexAddress {
        address: address!("E21D837cd1437305632ac1660A94c64b1ECd3151"),
        name: "TigerGaming_4",
        exchange: "TigerGaming",
    },
    CexAddress {
        address: address!("c330C1A3c7Db9c75f60AeD0A9B7C0Fc5FA22D5A2"),
        name: "TokenMarket",
        exchange: "TokenMarket",
    },
    CexAddress {
        address: address!("26637e1362A0C9F57D317CB417A9dEDdFe137F2a"),
        name: "Tokenize Xchange",
        exchange: "Tokenize",
    },
    CexAddress {
        address: address!("27D7f6147A8748454b88bE685c2804F96bF69dB1"),
        name: "Tokenize Xchange_1",
        exchange: "Tokenize Xchange",
    },
    CexAddress {
        address: address!("Edc53939315e2EBe28ebc771E99aD17463D28102"),
        name: "Tokenize Xchange_10",
        exchange: "Tokenize Xchange",
    },
    CexAddress {
        address: address!("F12Db6a6B8ECc2EA6245F8590135ca372ABB36E1"),
        name: "Tokenize Xchange_11",
        exchange: "Tokenize Xchange",
    },
    CexAddress {
        address: address!("f4FcaBded10b2d3D18d5040EcAE3a6D0FBBa10BC"),
        name: "Tokenize Xchange_12",
        exchange: "Tokenize Xchange",
    },
    CexAddress {
        address: address!("2c2F95DC8d3558F490CC0ca431a3BEF0B1E13aC8"),
        name: "Tokenize Xchange_2",
        exchange: "Tokenize Xchange",
    },
    CexAddress {
        address: address!("5f1F90B762baFA7F964050A347228B3b36425A55"),
        name: "Tokenize Xchange_3",
        exchange: "Tokenize Xchange",
    },
    CexAddress {
        address: address!("6cdc7C73345410dB99945433278df0bcbEEf4716"),
        name: "Tokenize Xchange_4",
        exchange: "Tokenize Xchange",
    },
    CexAddress {
        address: address!("b5E8C25f34A84613229BaBf4D0899157D74568F9"),
        name: "Tokenize Xchange_5",
        exchange: "Tokenize Xchange",
    },
    CexAddress {
        address: address!("B64d9784E8516983243434ce3BadF967Fd5cc71e"),
        name: "Tokenize Xchange_6",
        exchange: "Tokenize Xchange",
    },
    CexAddress {
        address: address!("B911c9ab63600B84b17Ef37720B332c73231E904"),
        name: "Tokenize Xchange_7",
        exchange: "Tokenize Xchange",
    },
    CexAddress {
        address: address!("BDB2aD8B5e5606013506c160B75264f9B1b48794"),
        name: "Tokenize Xchange_8",
        exchange: "Tokenize Xchange",
    },
    CexAddress {
        address: address!("CEf15405edCB31942c29792C113a818789259c18"),
        name: "Tokenize Xchange_9",
        exchange: "Tokenize Xchange",
    },
    CexAddress {
        address: address!("0068eB681EC52DBd9944517d785727310B494575"),
        name: "Tokocrypto",
        exchange: "Tokocrypto",
    },
    CexAddress {
        address: address!("7D8Dd9A8Be1c3eeE9101Ffb77C3BF0E89CBe74bA"),
        name: "Tokocrypto_1",
        exchange: "Tokocrypto",
    },
    CexAddress {
        address: address!("9A2f5556e9A637e8fBcE886d8e3cf8b316a1D8a2"),
        name: "Tokocrypto_2",
        exchange: "Tokocrypto",
    },
    CexAddress {
        address: address!("9f589e3eabe42ebC94A44727b3f3531C0c877809"),
        name: "Tokocrypto_3",
        exchange: "Tokocrypto",
    },
    CexAddress {
        address: address!("B6c1a7cCd41530A35Cd3d8ae5F1eaF40b588FaEf"),
        name: "Tokocrypto_4",
        exchange: "Tokocrypto",
    },
    CexAddress {
        address: address!("ef136c9beA3e397Ffed3cd8aD12511A7421116A0"),
        name: "Tokocrypto_5",
        exchange: "Tokocrypto",
    },
    CexAddress {
        address: address!("B2cc3cDd53fC9A1AEAf3A68Edeba2736238DDC5D"),
        name: "TopBTC",
        exchange: "TopBTC",
    },
    CexAddress {
        address: address!("0C38C14188CF42c2ab4bDc55084085059F0a507B"),
        name: "Trade.io",
        exchange: "Trade.io",
    },
    CexAddress {
        address: address!("1119AaEfB02bF12b84d28A5D8ea48ec3C90Ef1Db"),
        name: "Trade.io_1",
        exchange: "Trade.io",
    },
    CexAddress {
        address: address!("5652555D47430E948Ca15660c5652A23410d5072"),
        name: "Trade.io_2",
        exchange: "Trade.io",
    },
    CexAddress {
        address: address!("58f75dDACFFB183a30F69fE58a67a0d0985fce0F"),
        name: "Trade.io_3",
        exchange: "Trade.io",
    },
    CexAddress {
        address: address!("5A2FAd810f990C4535ADa938400B6b67eF7646af"),
        name: "Trade.io_4",
        exchange: "Trade.io",
    },
    CexAddress {
        address: address!("4648451b5F87FF8F0F7D622bD40574bb97E25980"),
        name: "TradeOgre",
        exchange: "TradeOgre",
    },
    CexAddress {
        address: address!("5E38AD84A902078D61Ca8D3BEbd378bC0e32C422"),
        name: "TradeOgre_1",
        exchange: "TradeOgre",
    },
    CexAddress {
        address: address!("27899ffaCe558bdE9F284Ba5C8c91ec79EE60FD6"),
        name: "Transak",
        exchange: "Transak",
    },
    CexAddress {
        address: address!("339Fe932809E39A95B621A7f88BbF6C08eb6C978"),
        name: "Txbit",
        exchange: "Txbit",
    },
    CexAddress {
        address: address!("53EdC98CB6C21dFCDdAF7F91Ebf39789B93E2Ac6"),
        name: "Txbit_1",
        exchange: "Txbit",
    },
    CexAddress {
        address: address!("E4FEb3e94B4128d973A366dc4814167a90629A08"),
        name: "Txbit_2",
        exchange: "Txbit",
    },
    CexAddress {
        address: address!("2f1233Ec3a4930Fd95874291DB7da9E90dfB2F03"),
        name: "UEX",
        exchange: "UEX",
    },
    CexAddress {
        address: address!("a78976995AE1a5670F6faC64a8a3E802fdcF1208"),
        name: "UEX_1",
        exchange: "UEX",
    },
    CexAddress {
        address: address!("18B7517cf34a3277f3DB3381c3B2679Cc3dc1116"),
        name: "Ultimate Champions",
        exchange: "Ultimate",
    },
    CexAddress {
        address: address!("03747F06215B44E498831dA019B27f53E483599F"),
        name: "Upbit",
        exchange: "Upbit",
    },
    CexAddress {
        address: address!("1938A448d105D26c40A52a1Bfe99B8Ca7a745aD0"),
        name: "Upbit_1",
        exchange: "Upbit",
    },
    CexAddress {
        address: address!("390dE26d772D2e2005C6d1d24afC902bae37a4bB"),
        name: "Upbit_2",
        exchange: "Upbit",
    },
    CexAddress {
        address: address!("4F01001cf69785d4c37f03Fd87398849411ccbBa"),
        name: "Upbit_3",
        exchange: "Upbit",
    },
    CexAddress {
        address: address!("5E032243d507C743b061eF021e2EC7fcc6d3ab89"),
        name: "Upbit_4",
        exchange: "Upbit",
    },
    CexAddress {
        address: address!("BA826fEc90CEFdf6706858E5FbaFcb27A290Fbe0"),
        name: "Upbit_5",
        exchange: "Upbit",
    },
    CexAddress {
        address: address!("c9cf0eC93d764f5c9571fD12f764Bae7fC87C84e"),
        name: "Upbit_6",
        exchange: "Upbit",
    },
    CexAddress {
        address: address!("1C727a55eA3c11B0ab7D3a361Fe0F3C47cE6de5d"),
        name: "Uphold",
        exchange: "Uphold",
    },
    CexAddress {
        address: address!("340d693ED55d7bA167D184ea76Ea2Fd092a35BDc"),
        name: "Uphold_1",
        exchange: "Uphold",
    },
    CexAddress {
        address: address!("352E504813B9E0b30F9cA70eFc27A52D298F6697"),
        name: "Uphold_2",
        exchange: "Uphold",
    },
    CexAddress {
        address: address!("3D8FC1CFfAa110F7A7F9f8BC237B73d54C4aBf61"),
        name: "Uphold_3",
        exchange: "Uphold",
    },
    CexAddress {
        address: address!("6E5d4a29833e51a83539a57461E803BCff409050"),
        name: "Uphold_4",
        exchange: "Uphold",
    },
    CexAddress {
        address: address!("a95350d70B18FA29f6B5EB8D627cEEEEE499340d"),
        name: "Uphold_5",
        exchange: "Uphold",
    },
    CexAddress {
        address: address!("07ac908cCa9c69AF022541D8fc0Bb29485FEB4bf"),
        name: "VinDAX",
        exchange: "VinDAX",
    },
    CexAddress {
        address: address!("c5a7C9D185A47DA11878F46932D23c0fdc56F275"),
        name: "VinDAX_1",
        exchange: "VinDAX",
    },
    CexAddress {
        address: address!("b436c96c6DE1f50A160eD307317C275424DBe4F2"),
        name: "Vinex",
        exchange: "Vinex",
    },
    CexAddress {
        address: address!("05a5E62CEBFB3FC3790F6C85fA620E82b5C58BD1"),
        name: "Voyager",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("149090aefD763c3348dbf862bd9D7A2B53C54F97"),
        name: "Voyager_1",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("663fD1D7658cD428b256057C3a85d13b710804b6"),
        name: "Voyager_10",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("73dd3e09A0A75012480F9753efC37d6ccC9A4058"),
        name: "Voyager_11",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("746350bFC022f90cACa573124F7396D46837874e"),
        name: "Voyager_12",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("764735E89a15EE53aA9C353f042e1CB637Aa4AC6"),
        name: "Voyager_13",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("7ccEf9ed17824214D60403171d889Bd4cE878B27"),
        name: "Voyager_14",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("91962711a4D2E4a830b366ce7276D99001e8564b"),
        name: "Voyager_15",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("A6aac20c2F51101A92a01E28c8da87927677f9Cc"),
        name: "Voyager_16",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("ac7E222B95A1e764186Ebe7C10fC32F6C969D076"),
        name: "Voyager_17",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("e120Ed33cAffdcF269EfD822fa0b77CD8c31BFdF"),
        name: "Voyager_18",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("E8724f21Aa13f86BcEb8c9c86e3EbE5da643c730"),
        name: "Voyager_19",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("203520F4ec42Ea39b03F62B20e20Cf17DB5fdfA7"),
        name: "Voyager_2",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("Ee977dC5F20D8e4771f2AF5711C9F75E795bFCbA"),
        name: "Voyager_20",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("F27C5989B39b0BFa537C5aa9617d78235eccD7F7"),
        name: "Voyager_21",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("F91A11B31ECd9a93Aed7060680b5f7899D7CC98d"),
        name: "Voyager_22",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("fAc87e892800F73feA9bD4a81B4E0269f4363fE3"),
        name: "Voyager_23",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("30D02D3351a43220a249C6e87426D1250c976e91"),
        name: "Voyager_3",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("31C741dF303F8F3506c70E69B1893d93236a360B"),
        name: "Voyager_4",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("3F31DbDC099761156244a5Bd5e06CDADbd2F30c0"),
        name: "Voyager_5",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("43E9A1Dc8a6d2f0DD9F1903b82c43AAc69eADCB4"),
        name: "Voyager_6",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("500A746c9a44f68Fe6AA86a92e7B3AF4F322Ae66"),
        name: "Voyager_7",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("5157740482B8AcAD392D696f1ED1e2C09D028ac2"),
        name: "Voyager_8",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("5Bc4599621485566ceF235A8a01D3985c03D7b62"),
        name: "Voyager_9",
        exchange: "Voyager",
    },
    CexAddress {
        address: address!("38Bb8A02eF1EDeBccA370f178F2974417Ed95F12"),
        name: "WEX",
        exchange: "WEX",
    },
    CexAddress {
        address: address!("b3AAAae47070264f3595c5032eE94b620A583a39"),
        name: "WEX_1",
        exchange: "WEX",
    },
    CexAddress {
        address: address!("0E293a9E57D22F6ec575b376e3E3Bd8e642Fd5fc"),
        name: "WazirX",
        exchange: "WazirX",
    },
    CexAddress {
        address: address!("1b9509aeb1FBb605868D21cAf2EEB6aC8351264D"),
        name: "WazirX_1",
        exchange: "WazirX",
    },
    CexAddress {
        address: address!("618fFD1cDAbeE36CE5992a857Cc7463f21272bD7"),
        name: "WazirX_2",
        exchange: "WazirX",
    },
    CexAddress {
        address: address!("7aeB3314E041153c4F6bbea19AbECBCe20946fD4"),
        name: "WazirX_3",
        exchange: "WazirX",
    },
    CexAddress {
        address: address!("7db77E4c967C95A5a9E2ec57Ec21788daB481893"),
        name: "WazirX_4",
        exchange: "WazirX",
    },
    CexAddress {
        address: address!("cDeF28FE85aB08a6632C97fd90534666c1ae96a3"),
        name: "WazirX_5",
        exchange: "WazirX",
    },
    CexAddress {
        address: address!("fA54B4085811aef6ACf47D51B05FdA188DEAe28b"),
        name: "WazirX_6",
        exchange: "WazirX",
    },
    CexAddress {
        address: address!("a21A16EC22a940990922220E4ab5bF4C2310F556"),
        name: "Wealthsimple",
        exchange: "Wealthsimple",
    },
    CexAddress {
        address: address!("1689a089AA12d6CbBd88bC2755E4c192f8702000"),
        name: "WhiteBIT",
        exchange: "WhiteBIT",
    },
    CexAddress {
        address: address!("33Eac50b7fAf4B8842A621d0475335693F5D21fe"),
        name: "WhiteBIT_1",
        exchange: "WhiteBIT",
    },
    CexAddress {
        address: address!("39F6a6C85d39d5ABAd8A398310c52E7c374F2bA3"),
        name: "WhiteBIT_2",
        exchange: "WhiteBIT",
    },
    CexAddress {
        address: address!("515281812Fdf5b0D7bE5Fb25b823a2aB79E0A621"),
        name: "WhiteBIT_3",
        exchange: "WhiteBIT",
    },
    CexAddress {
        address: address!("5c3Be6fA73CaB89f27744e886dB983e64B689Bf9"),
        name: "WhiteBIT_4",
        exchange: "WhiteBIT",
    },
    CexAddress {
        address: address!("98cEA98BE2a37A8bB52451Bd46259b2FBeE1bDc0"),
        name: "WhiteBIT_5",
        exchange: "WhiteBIT",
    },
    CexAddress {
        address: address!("aB928E30bEdE5919D4Bd9ec244711495769d2d85"),
        name: "WhiteBIT_6",
        exchange: "WhiteBIT",
    },
    CexAddress {
        address: address!("e3dB465646EA2aD39ff5672bB5FF0E83dcC91F0E"),
        name: "WhiteBIT_7",
        exchange: "WhiteBIT",
    },
    CexAddress {
        address: address!("eeFBd9626704DCd9C672c1031fC81e7f346ff3B8"),
        name: "WhiteBIT_8",
        exchange: "WhiteBIT",
    },
    CexAddress {
        address: address!("0A1820f0ff7Dc9FCE0A4F0B589ee14DdAe88233C"),
        name: "Wirex",
        exchange: "Wirex",
    },
    CexAddress {
        address: address!("2f13d388b85e0eCd32e7C3D7F36D1053354EF104"),
        name: "Wirex_1",
        exchange: "Wirex",
    },
    CexAddress {
        address: address!("4afDBa85a0E24A0D3F3245C8d91f5A0e2914E3C3"),
        name: "Wirex_2",
        exchange: "Wirex",
    },
    CexAddress {
        address: address!("4B8bd5ad437D43Babd20bBD618F73e3f50D9475f"),
        name: "Wirex_3",
        exchange: "Wirex",
    },
    CexAddress {
        address: address!("6966cE74E593df08e33F9eF0D8a4c0c9E0336bE5"),
        name: "Wirex_4",
        exchange: "Wirex",
    },
    CexAddress {
        address: address!("935f64B44B5C48A1539C4AdA5161D27ace4205b5"),
        name: "Wirex_5",
        exchange: "Wirex",
    },
    CexAddress {
        address: address!("b57decCD4B0Aa811FE1eC947c66Ee65C08617A76"),
        name: "Wirex_6",
        exchange: "Wirex",
    },
    CexAddress {
        address: address!("E3f277382419535245a345e923898c2d43f7CBE5"),
        name: "Wirex_7",
        exchange: "Wirex",
    },
    CexAddress {
        address: address!("03Dd167D62E1DFC223Ffd7b37Fc8bF45fB973478"),
        name: "Woo X",
        exchange: "Woo",
    },
    CexAddress {
        address: address!("15271E572267dEf474366bB683719Cc59489eFBe"),
        name: "Woo X_1",
        exchange: "Woo X",
    },
    CexAddress {
        address: address!("EeF97691d3307b4E61522170F648Ee2df1312fEE"),
        name: "Woo X_10",
        exchange: "Woo X",
    },
    CexAddress {
        address: address!("fA2d1f15557170F6c4A4C5249e77f534184cdb79"),
        name: "Woo X_11",
        exchange: "Woo X",
    },
    CexAddress {
        address: address!("1E6DCe7cE381774286abb8c9AAc461Bb7B1C4b05"),
        name: "Woo X_2",
        exchange: "Woo X",
    },
    CexAddress {
        address: address!("594203E46e0B41B1eDB54a551E7784c194d1335b"),
        name: "Woo X_3",
        exchange: "Woo X",
    },
    CexAddress {
        address: address!("607e062E3986a16283047BEAeD1A7dC3E220ff0E"),
        name: "Woo X_4",
        exchange: "Woo X",
    },
    CexAddress {
        address: address!("63DFE4e34A3bFC00eB0220786238a7C6cEF8Ffc4"),
        name: "Woo X_5",
        exchange: "Woo X",
    },
    CexAddress {
        address: address!("D7d8bCaE65537CB5079a4fB249b9fbB4526e4084"),
        name: "Woo X_6",
        exchange: "Woo X",
    },
    CexAddress {
        address: address!("e2933566f172D08f8C90144fEd5Ae28E9d54B1ec"),
        name: "Woo X_7",
        exchange: "Woo X",
    },
    CexAddress {
        address: address!("E505Bf08C03cc0FA4e0FDFa2487E2c11085b3FD9"),
        name: "Woo X_8",
        exchange: "Woo X",
    },
    CexAddress {
        address: address!("E64eB20471491956338eEdC0F98242Bc3aD0C91b"),
        name: "Woo X_9",
        exchange: "Woo X",
    },
    CexAddress {
        address: address!("104703893f56243c0e56441a99eb3F32E1ed6AD2"),
        name: "XT.com",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("24Ca039E1F71EF468d2946045c1F3e90D31FDB71"),
        name: "XT.com_1",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("B98486f2dF13ba242F173fFCBfF5C122B4d95A16"),
        name: "XT.com_10",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("cAB044b301D94875aC1502B73e54d63b0162C70f"),
        name: "XT.com_11",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("E0a616C3659bE29567E08819772e6905307AdF21"),
        name: "XT.com_12",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("E5B8ff1ca1c3Ef2ac704783d6473Ee5a9BE7e02d"),
        name: "XT.com_13",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("eFDA0cB780A8564903285ED25df3CC024f3b2982"),
        name: "XT.com_14",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("2Bc0DdE194d722FE98Ed3912cad464380F2225aC"),
        name: "XT.com_2",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("3Fcad6c6fB6BFfBb3585F2A401fbC87A6719D28d"),
        name: "XT.com_3",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("4Fd7c9BDb194f3Cf0589803d0A664A80e59ebFa6"),
        name: "XT.com_4",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("716198E735e1DF5F5423E74Fb98d493a1F789c1e"),
        name: "XT.com_5",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("724fD0870996faAcf4F6C169F484c678cf2Aa209"),
        name: "XT.com_6",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("80309282231e9e01c74C4E0EFeD35b063E6c5D27"),
        name: "XT.com_7",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("8c395E3fB26939438ACD5ca4bE7684752829e269"),
        name: "XT.com_8",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("98a79dbf216DaeF423bab001E23795f606bF7c2E"),
        name: "XT.com_9",
        exchange: "XT.com",
    },
    CexAddress {
        address: address!("20FfE0D07D7f7c2C21A24537538b4cDE06c9048a"),
        name: "XeggeX",
        exchange: "XeggeX",
    },
    CexAddress {
        address: address!("5fB29283c2cF472B86DB4f74A2B32F9CaF5578a4"),
        name: "XeggeX_1",
        exchange: "XeggeX",
    },
    CexAddress {
        address: address!("a0387AdBA7636722ABE119cbF9220Ce0B9938b0b"),
        name: "XeggeX_2",
        exchange: "XeggeX",
    },
    CexAddress {
        address: address!("8F3AB2c3B651382b07A76653D2be9EB4b87E1630"),
        name: "YOOBTC",
        exchange: "YOOBTC",
    },
    CexAddress {
        address: address!("8c240D98E179A9e283A2394e5969a2EEA95CA810"),
        name: "YoBit",
        exchange: "YoBit",
    },
    CexAddress {
        address: address!("F5bEC430576fF1b82e44DDB5a1C93F6F9d0884f3"),
        name: "YoBit_1",
        exchange: "YoBit",
    },
    CexAddress {
        address: address!("3BF5a62CCD5ea7a0f56EF5E42616A0D5fEc3Fa95"),
        name: "YouBank",
        exchange: "YouBank",
    },
    CexAddress {
        address: address!("683b8615d6099a71941CD693c151ef7C0573Fb44"),
        name: "YouBank_1",
        exchange: "YouBank",
    },
    CexAddress {
        address: address!("D894A19CF72e2Ef2EF6A0B51A2fCb36ECB733A01"),
        name: "YouBank_10",
        exchange: "YouBank",
    },
    CexAddress {
        address: address!("F509AcCd096A82Ef2562D316669d0AA4b60f3796"),
        name: "YouBank_11",
        exchange: "YouBank",
    },
    CexAddress {
        address: address!("82817C5528F8593e9E88a6A583BF5E77326Bdb55"),
        name: "YouBank_2",
        exchange: "YouBank",
    },
    CexAddress {
        address: address!("8Fa7E42E4ac6C1d876c99018Bb4A210e7Bb7564c"),
        name: "YouBank_3",
        exchange: "YouBank",
    },
    CexAddress {
        address: address!("938534B724e7ea82Da66f22eed82Dd75bB486194"),
        name: "YouBank_4",
        exchange: "YouBank",
    },
    CexAddress {
        address: address!("aD2Ef235E673a23DAB2CDd1EA834509AA6EA8bEB"),
        name: "YouBank_5",
        exchange: "YouBank",
    },
    CexAddress {
        address: address!("c5821feD609BeB4943AEd9CCf7fD20f93c8743E2"),
        name: "YouBank_6",
        exchange: "YouBank",
    },
    CexAddress {
        address: address!("c8f1df1b67F5E151d7DeE0738D50c6152c36BfFB"),
        name: "YouBank_7",
        exchange: "YouBank",
    },
    CexAddress {
        address: address!("d2349d84187580c73f1c77669F6f409DC425C3A3"),
        name: "YouBank_8",
        exchange: "YouBank",
    },
    CexAddress {
        address: address!("D2762EFEa5B803b8E87Aa62049794355BB5F74F3"),
        name: "YouBank_9",
        exchange: "YouBank",
    },
    CexAddress {
        address: address!("260Ee8F2B0C167e0cd6119b2DF923FD061dc1093"),
        name: "YouHodler",
        exchange: "YouHodler",
    },
    CexAddress {
        address: address!("2F1f7FC76e6CF5ab3e186A2A2FD4Fc31952a77Cc"),
        name: "YouHodler_1",
        exchange: "YouHodler",
    },
    CexAddress {
        address: address!("42dA8a05CB7eD9A43572b5BA1B8F82A0a6E263DC"),
        name: "Yunbi",
        exchange: "Yunbi",
    },
    CexAddress {
        address: address!("700f6912e5753e91ea3Fae877A2374A2db1245D7"),
        name: "Yunbi_1",
        exchange: "Yunbi",
    },
    CexAddress {
        address: address!("A32eeab263c7542958258BBeB52F8d4039b76511"),
        name: "Yunbi_2",
        exchange: "Yunbi",
    },
    CexAddress {
        address: address!("d94c9ff168dc6aEbf9b6CC86dEfF54f3fb0AFC33"),
        name: "Yunbi_3",
        exchange: "Yunbi",
    },
    CexAddress {
        address: address!("0e394D3fAcF0Ce3BD5fCcE584E16E0cBAc164346"),
        name: "ZB.com",
        exchange: "ZB.com",
    },
    CexAddress {
        address: address!("5E91E8CcAe2Dd2c6db87F677e161Ed1e07D6cCC8"),
        name: "ZB.com_1",
        exchange: "ZB.com",
    },
    CexAddress {
        address: address!("F0D9FcB4FefdBd3e7929374b4632f8AD511BD7e3"),
        name: "ZB.com_10",
        exchange: "ZB.com",
    },
    CexAddress {
        address: address!("FD6724B4b3e8eca764F0DD07ccd903aD348D70F8"),
        name: "ZB.com_11",
        exchange: "ZB.com",
    },
    CexAddress {
        address: address!("60d0cC2aE15859f69bF74DADb8AE3Bd58434976b"),
        name: "ZB.com_2",
        exchange: "ZB.com",
    },
    CexAddress {
        address: address!("6ba3FFBd026a4ec164aD477092000B9CF1e4C351"),
        name: "ZB.com_3",
        exchange: "ZB.com",
    },
    CexAddress {
        address: address!("734Ac651Dd95a339c633cdEd410228515F97fAfF"),
        name: "ZB.com_4",
        exchange: "ZB.com",
    },
    CexAddress {
        address: address!("b793FE6745fD79a42d7491B0861ef438c45e8A0f"),
        name: "ZB.com_5",
        exchange: "ZB.com",
    },
    CexAddress {
        address: address!("c97A4ed29F03FD549c4ae79086673523122d2Bc5"),
        name: "ZB.com_6",
        exchange: "ZB.com",
    },
    CexAddress {
        address: address!("cB45822DFA8A65b97bc854A1A61B674153a42967"),
        name: "ZB.com_7",
        exchange: "ZB.com",
    },
    CexAddress {
        address: address!("d371fBa58AB4C209f35001B30B39539f05091148"),
        name: "ZB.com_8",
        exchange: "ZB.com",
    },
    CexAddress {
        address: address!("Db7248d26Ea60170a4Fdc2ea44dc839C9E8C9ee4"),
        name: "ZB.com_9",
        exchange: "ZB.com",
    },
    CexAddress {
        address: address!("85D9aef1Ec67356A0D60c3ABE5aDb2E1dB9DE963"),
        name: "Zero Hash",
        exchange: "Zero",
    },
    CexAddress {
        address: address!("a1271A8A80748abd3F0DaFD4914aD2F481264447"),
        name: "Zero Hash_1",
        exchange: "Zero Hash",
    },
    CexAddress {
        address: address!("b02a9b7400545925746d8B9B985BC74A0601fB8D"),
        name: "Zero Hash_2",
        exchange: "Zero Hash",
    },
    CexAddress {
        address: address!("C1712944C5A544eF1287D6959068EAe6090b89Aa"),
        name: "Zero Hash_3",
        exchange: "Zero Hash",
    },
    CexAddress {
        address: address!("CfC0F98f30742B6d880f90155d4EbB885e55aB33"),
        name: "Zero Hash_4",
        exchange: "Zero Hash",
    },
    CexAddress {
        address: address!("1FD5dAA9BB707afb8A3197dEf72005757F8d1E36"),
        name: "Zipmex",
        exchange: "Zipmex",
    },
    CexAddress {
        address: address!("4a87135693A5b7dd0653C151c16EDFA3c524403F"),
        name: "Zipmex_1",
        exchange: "Zipmex",
    },
    CexAddress {
        address: address!("B487562715aC79C81C44830F964c8c65a4b76CBD"),
        name: "Zipmex_2",
        exchange: "Zipmex",
    },
    CexAddress {
        address: address!("b94Db18c60A429CB5080FE36c9333c07Cd158598"),
        name: "Zipmex_3",
        exchange: "Zipmex",
    },
    CexAddress {
        address: address!("cBe3f2e3E5da7E19F973BD07db4D22c28fC71c68"),
        name: "Zipmex_4",
        exchange: "Zipmex",
    },
    CexAddress {
        address: address!("0FF24158220A14398F047a80a513617Ddc4f5289"),
        name: "Zonda",
        exchange: "Zonda",
    },
    CexAddress {
        address: address!("2b645268E2fbb384B423e50089657395F749763a"),
        name: "Zonda_1",
        exchange: "Zonda",
    },
    CexAddress {
        address: address!("5BfF49EeC8F76C066F979A818187b9732AC69503"),
        name: "Zonda_2",
        exchange: "Zonda",
    },
    CexAddress {
        address: address!("6EDF968DA408a9640b8865826429a977a11C5048"),
        name: "Zonda_3",
        exchange: "Zonda",
    },
    CexAddress {
        address: address!("781229c7a798c33EC788520a6bBe12a79eD657FC"),
        name: "Zonda_4",
        exchange: "Zonda",
    },
    CexAddress {
        address: address!("818ab3c61f66e975b8E6290c20999d6749F60d8D"),
        name: "Zonda_5",
        exchange: "Zonda",
    },
    CexAddress {
        address: address!("d388009f01bbE5e6D2Cb6bA8525ca50B56308046"),
        name: "Zonda_6",
        exchange: "Zonda",
    },
    CexAddress {
        address: address!("f646CBe3B030fb6c2569215F0117dbA58baDB95E"),
        name: "Zonda_7",
        exchange: "Zonda",
    },
    CexAddress {
        address: address!("111cFf45948819988857BBF1966A0399e0D1141e"),
        name: "bitFlyer",
        exchange: "bitFlyer",
    },
    CexAddress {
        address: address!("89460424c14378c2407518cEEA6D427830084822"),
        name: "bitFlyer_1",
        exchange: "bitFlyer",
    },
    CexAddress {
        address: address!("8abe3ac564098adcaD1Cb0f812358E5E4555e2bA"),
        name: "bitFlyer_2",
        exchange: "bitFlyer",
    },
    CexAddress {
        address: address!("B01cb49fe0D6D6E47EDf3A072d15dfe73155331C"),
        name: "bitFlyer_3",
        exchange: "bitFlyer",
    },
    CexAddress {
        address: address!("2953452dF5D7285b9a3a8a1E876A4bAcb09a976E"),
        name: "eToro",
        exchange: "eToro",
    },
    CexAddress {
        address: address!("1681D536B8A47C01d7ea07f0D80A2eB10E7Ce842"),
        name: "eXch.sc",
        exchange: "eXch.sc",
    },
    CexAddress {
        address: address!("f1dA173228fcf015F43f3eA15aBBB51f0d8f1123"),
        name: "eXch.sc_1",
        exchange: "eXch.sc",
    },
    CexAddress {
        address: address!("15C5312E24482547FF35899AFeDCAEB572ECB029"),
        name: "xs2.exchange",
        exchange: "xs2.exchange",
    }
];

/// Total number of CEX addresses
pub const CEX_ADDRESS_COUNT: usize = 2776;

/// Lazy static HashSet for quick lookups
pub static CEX_ADDRESS_SET: Lazy<HashSet<Address>> = Lazy::new(|| {
    CEX_ADDRESSES.iter().map(|entry| entry.address).collect()
});

/// Lazy static HashMap for address to exchange mapping
pub static CEX_BY_ADDRESS: Lazy<HashMap<Address, &'static CexAddress>> = Lazy::new(|| {
    CEX_ADDRESSES
        .iter()
        .map(|entry| (entry.address, entry))
        .collect()
});

/// Group addresses by exchange
pub static ADDRESSES_BY_EXCHANGE: Lazy<HashMap<&'static str, Vec<Address>>> = Lazy::new(|| {
    let mut map: HashMap<&'static str, Vec<Address>> = HashMap::new();
    for entry in CEX_ADDRESSES {
        map.entry(entry.exchange)
            .or_insert_with(Vec::new)
            .push(entry.address);
    }
    map
});

/// Check if an address belongs to a CEX
pub fn is_cex_address(address: Address) -> bool {
    CEX_ADDRESS_SET.contains(&address)
}

/// Get CEX info by address
pub fn get_cex_by_address(address: Address) -> Option<&'static CexAddress> {
    CEX_BY_ADDRESS.get(&address).copied()
}

/// Get all addresses for a specific exchange
pub fn get_exchange_addresses(exchange: &str) -> Option<&'static Vec<Address>> {
    ADDRESSES_BY_EXCHANGE.get(exchange)
}


/// Exchange counts:
/// - OKX: 165 addresses
/// - Coinbase: 159 addresses
/// - HTX: 155 addresses
/// - Binance: 115 addresses
/// - Kraken: 76 addresses
/// - HitBTC: 72 addresses
/// - KuCoin: 58 addresses
/// - CoinDCX: 52 addresses
/// - ShapeShift: 50 addresses
/// - Bidesk: 48 addresses
/// Exchange names with counts
pub fn exchange_stats() -> Vec<(&'static str, usize)> {
    let mut stats: Vec<_> = ADDRESSES_BY_EXCHANGE
        .iter()
        .map(|(name, addrs)| (*name, addrs.len()))
        .collect();
    stats.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    stats
}
