//! ETF (Exchange-Traded Fund) addresses
//!
//! This file is auto-generated from Python address files.
//! Do not edit manually - regenerate using scripts/convert_addresses_to_rust.py

use alloy_primitives::{address, Address};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};

/// ETF address entry
#[derive(Debug, Clone)]
pub struct EtfAddress {
    pub address: Address,
    pub name: &'static str,
    pub provider: &'static str,
}

/// All ETF addresses
pub const ETF_ADDRESSES: &[EtfAddress] = &[
    EtfAddress {
        address: address!("1ae3adaB1c43f97D53Ee3619CD8220C294059Dec"),
        name: "21Shares CETH",
        provider: "21Shares",
    },
    EtfAddress {
        address: address!("25e1a34e480443433cC7D16664c62c5A4d9dd43E"),
        name: "21Shares CETH_1",
        provider: "21Shares",
    },
    EtfAddress {
        address: address!("405cBa46e0cBa39961fe2a813B4E403b841A0EE5"),
        name: "21Shares CETH_2",
        provider: "21Shares",
    },
    EtfAddress {
        address: address!("7846e4F966C708799A81927cF34Bcf2544142428"),
        name: "21Shares CETH_3",
        provider: "21Shares",
    },
    EtfAddress {
        address: address!("7b9B5cd00ebb0d4854a73dAfa0609003cF97ea31"),
        name: "21Shares CETH_4",
        provider: "21Shares",
    },
    EtfAddress {
        address: address!("Cdc74Cb68695fD2da6dA8c717e22600cC624744d"),
        name: "21Shares CETH_5",
        provider: "21Shares",
    },
    EtfAddress {
        address: address!("d28F68Feed9a47cb3CF63B058581761abff45FB3"),
        name: "21Shares CETH_6",
        provider: "21Shares",
    },
    EtfAddress {
        address: address!("ff1dBB9e1D2e15B70869ab3BcBe7c1ac09048882"),
        name: "21Shares CETH_7",
        provider: "21Shares",
    },
    EtfAddress {
        address: address!("3339AAD5f1a0CC95d6Dee6DDF10b444F080c7cB5"),
        name: "Bitwise ETHW",
        provider: "Bitwise",
    },
    EtfAddress {
        address: address!("6F28ebf3170AA00Bfdf7131A4635a04976c657Ed"),
        name: "Bitwise ETHW_1",
        provider: "Bitwise",
    },
    EtfAddress {
        address: address!("7716d9e70779ee3d2580DccA818521A85E59eec8"),
        name: "Bitwise ETHW_2",
        provider: "Bitwise",
    },
    EtfAddress {
        address: address!("a15c9d4aF12d42c612D5a7445d76f5cC3aC92A69"),
        name: "Bitwise ETHW_3",
        provider: "Bitwise",
    },
    EtfAddress {
        address: address!("Ed9258097cC80e1E6eBF2c9B132Eb135C81bb4eF"),
        name: "Bitwise ETHW_4",
        provider: "Bitwise",
    },
    EtfAddress {
        address: address!("FBaC831C5A71BF8f517B3a33cBFfFdD448eBB5C6"),
        name: "Bitwise ETHW_5",
        provider: "Bitwise",
    },
    EtfAddress {
        address: address!("0171F896002665C3ea3Ed0c55f21026cA0A734A0"),
        name: "BlackRock ETHA",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("0195Bd0E9Dc6F98BD9bB3d0bFF69749a52065FA5"),
        name: "BlackRock ETHA_1",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("14f175ba60C6be3087ac5ebe322396c99c2a2f54"),
        name: "BlackRock ETHA_10",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("bD96dB1676C9B9136030C69A08eC93507917fCdB"),
        name: "BlackRock ETHA_100",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("bE344dEcD5dE7798f54aa0e0159d494413585c51"),
        name: "BlackRock ETHA_101",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("C074DD1D43342E797cdB680bb48aeF5E8f1b4B27"),
        name: "BlackRock ETHA_102",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("c0b13bBe75717Db4218Ce8031276B7859c2114A1"),
        name: "BlackRock ETHA_103",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("c1aC6dF64f397C324a8bf570fD12E59892b18cA7"),
        name: "BlackRock ETHA_104",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("C2b40e86178d7611d4bF906f52B0174778a8aAA9"),
        name: "BlackRock ETHA_105",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("c3Fe7c7084eA95730Dbc67DF66E509346D7e691a"),
        name: "BlackRock ETHA_106",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("c4C0D26A53A1d9646055623447Dc66fE1aA3d2F6"),
        name: "BlackRock ETHA_107",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("C5685D5dD92f9003cb5087fcDeE27e7401da6d71"),
        name: "BlackRock ETHA_108",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("c89D09B6F6Cf1783D4baF0bb6eAf7E4EDF6b0e2C"),
        name: "BlackRock ETHA_109",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("17939de7446C7cB703ab2ebDA1f63167f1b90693"),
        name: "BlackRock ETHA_11",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("ca060AEad23c6ef6dBc8ca281D7275985d979AfB"),
        name: "BlackRock ETHA_110",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("Cb9F54Ec8Cd14A31BFF10C9C843C843ed67eFa60"),
        name: "BlackRock ETHA_111",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("Cf0C293a6da23A7006Be793B3320aaCB274dEe6E"),
        name: "BlackRock ETHA_112",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("d1A599C4903eb195aCd0BB45f2AB400718AF0907"),
        name: "BlackRock ETHA_113",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("d1dD34E87aEB9DC900c766d98b5376E41938bF03"),
        name: "BlackRock ETHA_114",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("D546f97125f7969e4CA3BA44c6Af33a85a22B00c"),
        name: "BlackRock ETHA_115",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("d56c29eb4F4B3468902dB986FCbeCEbe6FB7ADB4"),
        name: "BlackRock ETHA_116",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("D6CcDACc837f00ec5125fc173ffD01BA7849cFB7"),
        name: "BlackRock ETHA_117",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("D7ac899F6f76bE7278C70cc71B37e21350669743"),
        name: "BlackRock ETHA_118",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("daeDD4f2150f547ebFd49459E7B6468CC90065a2"),
        name: "BlackRock ETHA_119",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("1bFc9c727d03fEc405347E9ECc6A71AAfcED0c66"),
        name: "BlackRock ETHA_12",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("DBFBAdBDFC83aD032E343A4dd3515F5e1241837e"),
        name: "BlackRock ETHA_120",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("DC6205cD2A4BccF39f0f2C9106dE6Cb8f46Ffa03"),
        name: "BlackRock ETHA_121",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("DCD47cF2DF8811272F43e3Eea2412F175cBb6A33"),
        name: "BlackRock ETHA_122",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("dff29e9bCCa88222d2e32Ea5F72aCdd69B0BBD10"),
        name: "BlackRock ETHA_123",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("e3a0E6ACFeb43F4E1Bf5be753088dDe379161273"),
        name: "BlackRock ETHA_124",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("E3ad36592F502eF926e265E4252868F5FbA7aE51"),
        name: "BlackRock ETHA_125",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("E740606845c3FB7F527195dceF4523C0b1966d0D"),
        name: "BlackRock ETHA_126",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("e815C44b7Cab8A5f559B4080a5d0536830B6cf5b"),
        name: "BlackRock ETHA_127",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("eb47E5353256E196687Eb0649b48EB14bc4C6c92"),
        name: "BlackRock ETHA_128",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("eD75D85ca83D10Fd05F7640574b56cc1dAd5B5F0"),
        name: "BlackRock ETHA_129",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("1Cad5f1359224003fFf72096e4d02EC0587e66a0"),
        name: "BlackRock ETHA_13",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("ED9666354fEa95a24195EC40f27eF861Cf08B981"),
        name: "BlackRock ETHA_130",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("eDbcd2E00bADCC3d3D09a8a2823A3f08c1879fe3"),
        name: "BlackRock ETHA_131",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("EdF56687fd7304dD91b3bd8661F95497E2F9e9BC"),
        name: "BlackRock ETHA_132",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("F08BF6Dc5406C7c3Ae579894DEefd5c13ea8933D"),
        name: "BlackRock ETHA_133",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("f0b3e9c849fFE71EF66aCE92b3b4A4b6E20d9f73"),
        name: "BlackRock ETHA_134",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("F3bd8F3381cDD795E9727789d0B7972EF8F5666E"),
        name: "BlackRock ETHA_135",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("F4c008e48150b83Df538bc5B96b4EbFe77Fc8f96"),
        name: "BlackRock ETHA_136",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("f55d07aad3864B34016e31674Aa7C5B3b2598A4D"),
        name: "BlackRock ETHA_137",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("FD2001Ab71878C33C462e32FAD3b5Cbd7602f7ed"),
        name: "BlackRock ETHA_138",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("fdFBE80e1a3b1E7df769054DB3aC89Bc3FCC8958"),
        name: "BlackRock ETHA_139",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("1EbA5f237543CC08d67195649E9E06ACE6AA46B3"),
        name: "BlackRock ETHA_14",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("ff99FaD518f0733765F28181252Dc1F479E7fA9a"),
        name: "BlackRock ETHA_140",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("200d0D4ED39da71BBF2eAA86d7c0923041a8292e"),
        name: "BlackRock ETHA_15",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("24efa0f9AA3C020e980Ab9CF5E396327e9D54CE5"),
        name: "BlackRock ETHA_16",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("28c4f4082748A61Aa959f2f9FA8BE4e27E7Bb2a5"),
        name: "BlackRock ETHA_17",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("296C93E66bCAEBCFFf0c873e54806964c0F63aED"),
        name: "BlackRock ETHA_18",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("2a3e882057769d304c0F43e91aBD2c81e2470c41"),
        name: "BlackRock ETHA_19",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("02d05BA91b77f664122E86cb42CAaE5eb4107144"),
        name: "BlackRock ETHA_2",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("2C110C2a1eCDA251441B2CCea49Ca27910e92e5e"),
        name: "BlackRock ETHA_20",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("2F1d3dA11eAbb07D33d185314B198abd4dF656f0"),
        name: "BlackRock ETHA_21",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("2fd84202979DbAF4B835656b72ABde24C5652436"),
        name: "BlackRock ETHA_22",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("3007C5F232A61206418a1d487ac86699eb71217F"),
        name: "BlackRock ETHA_23",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("304426f628C96Ac4517738d6857ECFd6a0D00D72"),
        name: "BlackRock ETHA_24",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("324be15Bcf22Ef6A0090e04d4164959d60E40EE1"),
        name: "BlackRock ETHA_25",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("36d5868e7f1012eB45F6702B801AD6736edf6e26"),
        name: "BlackRock ETHA_26",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("3877D4faDabDB513A370A4381bBB1509E60290bb"),
        name: "BlackRock ETHA_27",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("3A8EFdA4f6bbE967fC4FDd6ff4742080Bc1DE3bF"),
        name: "BlackRock ETHA_28",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("3EBBaD53F6d00210069BDedd7d9b5C9486b51ece"),
        name: "BlackRock ETHA_29",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("059488Aa193371C47E5cc6e5Be7FCF0ed3404267"),
        name: "BlackRock ETHA_3",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("4003399e77aF1f556455516dC983102351cdeb01"),
        name: "BlackRock ETHA_30",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("40457f403f76fDA0213790EF3aB7a7AC23cC7F1D"),
        name: "BlackRock ETHA_31",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("4244a400c55958de14D47064E89DC039105EF20f"),
        name: "BlackRock ETHA_32",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("435449B156Cbb6473166d599Bb925BbAb6E26F96"),
        name: "BlackRock ETHA_33",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("4dC9b37e794D0ec6c2faBf59b64A421BAdE92b82"),
        name: "BlackRock ETHA_34",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("50Dd722DC8b2d735Fc36C992164612E2834Cf1E3"),
        name: "BlackRock ETHA_35",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("52B19A80695A09D553D805877F437DDa5a18aC1e"),
        name: "BlackRock ETHA_36",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("53071380278512C05A8DcbF253B43C69dB425f04"),
        name: "BlackRock ETHA_37",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("54471c6357A90664f5Fda663619FF12C4942DB50"),
        name: "BlackRock ETHA_38",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("5552Be2668E180621A916C81d382f746b3019094"),
        name: "BlackRock ETHA_39",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("059551481976475ed8B62890F570457d61682d71"),
        name: "BlackRock ETHA_4",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("5557c8c45B4f55016707cf22B6e54E9189A3FBd5"),
        name: "BlackRock ETHA_40",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("56a6e187660B69ec36ed4b421742a74a442d6021"),
        name: "BlackRock ETHA_41",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("59dF1ce7A32f89e58EF56090D7D81b163e6cCddb"),
        name: "BlackRock ETHA_42",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("59e40b07fCB23E45335CFd367f516a69c081d179"),
        name: "BlackRock ETHA_43",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("5AFB8D40C6577D4810214e4B80BD5317E53A33a0"),
        name: "BlackRock ETHA_44",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("5C106A0Ea5f5c0a8D81Cf330e53F058fd3b770B7"),
        name: "BlackRock ETHA_45",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("604Ce9c9236A6252b9D3b935423CF61b335CEcee"),
        name: "BlackRock ETHA_46",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("624f8d573BFfDEbB3e54c3C9571308dc1b944179"),
        name: "BlackRock ETHA_47",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("6277F7B2eb4ec35630b3b76917AB7C142dDF4445"),
        name: "BlackRock ETHA_48",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("639d6101E9c65D352b85b2252D82A4AE02F701a9"),
        name: "BlackRock ETHA_49",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("09AAB9E78A539e8a26cB693cA208A3c99F14Ef21"),
        name: "BlackRock ETHA_5",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("669C35897857BA63cfF1cde9618Ee40bE3C04eA6"),
        name: "BlackRock ETHA_50",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("679C31256DFf9FD6385AeA736012dF5A5F783Bde"),
        name: "BlackRock ETHA_51",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("67e49F57DD2B69288BA7A898466824BE3C433137"),
        name: "BlackRock ETHA_52",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("68d60195E97A75E85D13Dec327ffEFACc4cDB9fE"),
        name: "BlackRock ETHA_53",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("716Dd77Bc1D1f89115E1FB8a7Cb6568a0561E941"),
        name: "BlackRock ETHA_54",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("73800c8E828637a7A4319aeF28c6cf09e408af66"),
        name: "BlackRock ETHA_55",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("79B86B98B1aA714E7CB834330980b8A0f270b37e"),
        name: "BlackRock ETHA_56",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("7C07d3BEd88e23DA29A406Eb41bDA7Ae3B64747b"),
        name: "BlackRock ETHA_57",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("7E2a9D3B486CaeEd30312f5A20b02bA7b77E8e8F"),
        name: "BlackRock ETHA_58",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("7e8f035b613D8073E37FEAC146CDB87cCeff7198"),
        name: "BlackRock ETHA_59",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("09c9f203E6E1BCC4EDE5c594771E9dA6937699Ee"),
        name: "BlackRock ETHA_6",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("7fAE237f63992d32407cc74B4Ad5C495f5709996"),
        name: "BlackRock ETHA_60",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("81127954046a1aBa305F2b562ba1ad7129D73c1D"),
        name: "BlackRock ETHA_61",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("85152a72AA24A3D0b667aDF9e81624925043bFbe"),
        name: "BlackRock ETHA_62",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("862A495cE4487C56F40A338dE0b36b4c89594d6B"),
        name: "BlackRock ETHA_63",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("878D0d68D260E265Eb051688375b50b5DAf7e2b0"),
        name: "BlackRock ETHA_64",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("88c0484300e9BDba36A1F82C15421Bf2c86e0A65"),
        name: "BlackRock ETHA_65",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("89F7DA9cD853dD055C1f7BF4CF15327bf537469b"),
        name: "BlackRock ETHA_66",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("8A4536dc0719896838C5a203b1e1b45359151339"),
        name: "BlackRock ETHA_67",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("8A5Cf0c48A35A1035d03fE2DF53d80062f631ae4"),
        name: "BlackRock ETHA_68",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("8C140be941D4888C035F2316185A214016F6DE2f"),
        name: "BlackRock ETHA_69",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("0Ab743140Fb2F788DA56f4569a012D049cdca41e"),
        name: "BlackRock ETHA_7",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("8D1197Ba508d95D8d1921aAf525F1BF1eA983e25"),
        name: "BlackRock ETHA_70",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("8d1faAbaF2A5d0FF33E6A74c6bE290a60f8550E0"),
        name: "BlackRock ETHA_71",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("8d3aF64C5919735069f46Af6518Dc8654D5Fad8d"),
        name: "BlackRock ETHA_72",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("8fAc44E97E90312fCe3254cff74FD61F0Fe1e7E8"),
        name: "BlackRock ETHA_73",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("906DdD6005407a1252999726676dcd71b3E76e7f"),
        name: "BlackRock ETHA_74",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("944B076bb5DC1F2aC3921b6Df035432d1A05B707"),
        name: "BlackRock ETHA_75",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("95f120A2aD5448041c1910Fc36d60380c24CD4Fd"),
        name: "BlackRock ETHA_76",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("9645edD5BD30b6fB9447A17FAaA029056e6AD329"),
        name: "BlackRock ETHA_77",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("97505Eb589b9Cb68C9f993f20081009d05cb0E8C"),
        name: "BlackRock ETHA_78",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("9C2e2D46234C1C3B690202cFA6098F85d71E84Bb"),
        name: "BlackRock ETHA_79",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("1353B50091b6A5E3643d3b08255F794eC8579955"),
        name: "BlackRock ETHA_8",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("a0D402f74923793b0e9CC42Af0928323bfad55b1"),
        name: "BlackRock ETHA_80",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("A1B84DbADee6E535e38c9Aef545C037A5f60e84e"),
        name: "BlackRock ETHA_81",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("A2858077cddfC3b2FA16ea17B4121717915651C1"),
        name: "BlackRock ETHA_82",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("A29EdF3c4d7349709F57fb5Affb1Ea7CeCEE1B7f"),
        name: "BlackRock ETHA_83",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("a2cbd7bA4b5767E7EdA25ECCA1ce5b65e2C9409b"),
        name: "BlackRock ETHA_84",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("a95a23115B0aB94182C5893E1A45140F0F3B6D53"),
        name: "BlackRock ETHA_85",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("aB9566c24dF471eaF433FFBAF179f464A557D94E"),
        name: "BlackRock ETHA_86",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("ad53B2177d517772797aD502fC436216Ffe0426F"),
        name: "BlackRock ETHA_87",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("aEBFf1fF9d5b0c3E646d790c1d5F0C16c8afaa6D"),
        name: "BlackRock ETHA_88",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("B05cB0758F1E3b59583dd19d5Cc147D082794BB7"),
        name: "BlackRock ETHA_89",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("135427379848c023DE50Ed551E4FA4f192F2D376"),
        name: "BlackRock ETHA_9",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("B0781d1b70386B4aa5206c3565ade5dF128bE161"),
        name: "BlackRock ETHA_90",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("b0F78b3e230e22EF85eB759BB73A04614F5Bb1eb"),
        name: "BlackRock ETHA_91",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("b3569Eb2b6Bf220814308fBB407f24144F45368E"),
        name: "BlackRock ETHA_92",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("B402F0b9D277B6C74f2d8503641B86E19Eb42321"),
        name: "BlackRock ETHA_93",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("b437F43Ad845F966fE18950C4C540BEf26802fD3"),
        name: "BlackRock ETHA_94",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("b4C0C83645411a2d40f51735B33371D018e2BFe5"),
        name: "BlackRock ETHA_95",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("b79E9b4e6E280Fb913Aa03F8D11ed11e30D9640C"),
        name: "BlackRock ETHA_96",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("b85BCb14f0F305106b9B9B61E33bFD4b14169DED"),
        name: "BlackRock ETHA_97",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("BAE3828911e90331325B7D07b07290cEB4Ec4b3e"),
        name: "BlackRock ETHA_98",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("BcbC56D24595a231fC8DFD2a2aA1cdC3683A5138"),
        name: "BlackRock ETHA_99",
        provider: "BlackRock",
    },
    EtfAddress {
        address: address!("0e21a608c7939D0F8005a8DCEA034FBcF5f6f194"),
        name: "Fidelity FETH",
        provider: "Fidelity",
    },
    EtfAddress {
        address: address!("6279fb0FE59125c290E0a9BA7A5c98e8f0c5Ab23"),
        name: "Fidelity FETH_1",
        provider: "Fidelity",
    },
    EtfAddress {
        address: address!("8aFA3d0A70b2f3A92F5b8D63afFafD31d3d46eB7"),
        name: "Fidelity FETH_2",
        provider: "Fidelity",
    },
    EtfAddress {
        address: address!("8b4b2B268766E28224CC03384d80518A91a6fD37"),
        name: "Fidelity FETH_3",
        provider: "Fidelity",
    },
    EtfAddress {
        address: address!("9371be03662333331D1C5D7d9Ee2e1c25A574743"),
        name: "Fidelity FETH_4",
        provider: "Fidelity",
    },
    EtfAddress {
        address: address!("9D18F81dE45a2ed1EF4B269A8d4C6E390A8C1C68"),
        name: "Fidelity FETH_5",
        provider: "Fidelity",
    },
    EtfAddress {
        address: address!("b3E392BEDD6956187F590D50C5Aa071C08DCfd6E"),
        name: "Fidelity FETH_6",
        provider: "Fidelity",
    },
    EtfAddress {
        address: address!("bE775c8a98e33DD6Fd5a139827F2c9339D1F9fdd"),
        name: "Fidelity FETH_7",
        provider: "Fidelity",
    },
    EtfAddress {
        address: address!("CB4460C60F9Ea6fB64F758f7E8dECFA847f7F71A"),
        name: "Fidelity FETH_8",
        provider: "Fidelity",
    },
    EtfAddress {
        address: address!("eF54c7bf79f089f868150D3c0684213237c184D7"),
        name: "Fidelity FETH_9",
        provider: "Fidelity",
    },
    EtfAddress {
        address: address!("21818666eA7218Fa2579146Ccffb05C113fcD132"),
        name: "Franklin Templeton EZET",
        provider: "Franklin",
    },
    EtfAddress {
        address: address!("f892c7b77d50B8ed19A33fD28e24151600478731"),
        name: "Franklin Templeton EZET_1",
        provider: "Franklin",
    },
    EtfAddress {
        address: address!("0007EaB78C7C2cCa63D70a3A0E3658D9FAF4506F"),
        name: "Grayscale ETHE",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("004cd91081DC9dfF0B9B75274beCcc97B5db3c36"),
        name: "Grayscale ETHE_1",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0380EA7994E22dc7503BAa40BA5ab206F6A55Dab"),
        name: "Grayscale ETHE_10",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1aAAE4c7ff3fc9b3b5c007802B4A1229c1ecf41E"),
        name: "Grayscale ETHE_100",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1B3cC8C6ED708C7cB4dd5AfB97543538E176BBe3"),
        name: "Grayscale ETHE_101",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1b3e055112ce117156a72cAE967A2e5b7C4c9bbF"),
        name: "Grayscale ETHE_102",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1b964d30C2Da31024274e79cbE32aE6CbE7ad198"),
        name: "Grayscale ETHE_103",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1cB95602b86c90731A9145082BF0a55f2dca9124"),
        name: "Grayscale ETHE_104",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1E15580cB4993D473939dAD2B9Caa35dD25E09C7"),
        name: "Grayscale ETHE_105",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1EFEE223012896fe0C66359d30908Efd964cDfAC"),
        name: "Grayscale ETHE_106",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1F4a0A7c953397F6d486AcdB7271890D1e5426B7"),
        name: "Grayscale ETHE_107",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1F51cA435E893FD90e4B4Ceb559C7D97C63ddF10"),
        name: "Grayscale ETHE_108",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1F5bA07332a800BeEd1F90931b7ea378692C5aa0"),
        name: "Grayscale ETHE_109",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0391A9EBE1Fe3d350bed011f76dC12A246569E9D"),
        name: "Grayscale ETHE_11",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1f7E68EB933d18180445ef374C1286232f4ad73F"),
        name: "Grayscale ETHE_110",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1f7f3F97570e2ab1334816Ec0989c2b3a559FA97"),
        name: "Grayscale ETHE_111",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1f9A59E61C7AA9cac90d2B38Cf5AD4964F6F9423"),
        name: "Grayscale ETHE_112",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1Fb0B7f567d4bFF19b3b3ba22538eD7B0934Cbe2"),
        name: "Grayscale ETHE_113",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1fC2bE992CB1CDd714df3Ac9e4bdEF80ED6F0F69"),
        name: "Grayscale ETHE_114",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1Fcb40ff0a6A0bF8e9e473101C56af9a2CAcc187"),
        name: "Grayscale ETHE_115",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1FdD50659FD48c748DF039Dc32420Cd0e8C79CdB"),
        name: "Grayscale ETHE_116",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1Fe729426Df3d04c033cD59771d2Ac4e1e5E046A"),
        name: "Grayscale ETHE_117",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2060A3d40fe510A38cd2861273C5C02262542803"),
        name: "Grayscale ETHE_118",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("20A29F464Eed22b688D9F33B43Be99052Dac13fa"),
        name: "Grayscale ETHE_119",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("03a39109B2bF39D5Da499dDCF6774d9FE1490924"),
        name: "Grayscale ETHE_12",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("20C985e854AD71A9E22829E34c25C83CF992FdDc"),
        name: "Grayscale ETHE_120",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2116ec8b0ba52F1e40c63ba10e40a823bE68D91D"),
        name: "Grayscale ETHE_121",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("212370f0a209C0E76d278A1ED7528CA18107487E"),
        name: "Grayscale ETHE_122",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("21692f8E636C3a79f7eBc8fb9df787f1c5A21a1a"),
        name: "Grayscale ETHE_123",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2194294C661f901BD0375098eb345cE16BA7889F"),
        name: "Grayscale ETHE_124",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("21f2f89FDA0c9081c80679aaec61FCA118410d95"),
        name: "Grayscale ETHE_125",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("220c575a00A9F62B6Aae2Ea2A484971C5Fa3Cdb0"),
        name: "Grayscale ETHE_126",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("225a75e90b76fb1825D82e7Bb67691f0e45aF026"),
        name: "Grayscale ETHE_127",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2298937De1d04b818e74Fc13273caA6476186204"),
        name: "Grayscale ETHE_128",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("22a456f05857acd2A78e6cB1067bBd62c68BF9c1"),
        name: "Grayscale ETHE_129",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("03AcB8C8F020CC9be44ef7639C1e909262381d93"),
        name: "Grayscale ETHE_13",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("23042bC51e20d5d376A9de40b47117751B043B5e"),
        name: "Grayscale ETHE_130",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("23a6f70Cf131F17c9b587A8F4a0B19C81189B29e"),
        name: "Grayscale ETHE_131",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("23A84cA03bB0e7Fd7897426250eEcFE68493a09f"),
        name: "Grayscale ETHE_132",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("23Aa93e55bbbb81cEA3ad02FCb54aFD5304e99Cf"),
        name: "Grayscale ETHE_133",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("23c45a0456B25DAa2eBC25A15c2b604bA4Ff7e4D"),
        name: "Grayscale ETHE_134",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("23C912DcFBFaAf9c502fB478428cB221BEB64ED9"),
        name: "Grayscale ETHE_135",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2426457F8d67DF33AcaB7905C3d13f3F30c17593"),
        name: "Grayscale ETHE_136",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("242B7b0cdacF427C92ECdceC292084de7A0993Ba"),
        name: "Grayscale ETHE_137",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2452A774CD4B7c3FEd1b630A9b4b5f85a9194918"),
        name: "Grayscale ETHE_138",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("245a01D2f879bc75909F0e86032F9236cf3b1DbA"),
        name: "Grayscale ETHE_139",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("042d21E41044af8C7a8643F01aecBc7Eb7908463"),
        name: "Grayscale ETHE_14",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("247cb643F8828Dcb8F5742023A608579667d3eD7"),
        name: "Grayscale ETHE_140",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2486Fcc85CB7D96dc80114341E9151c81d15084E"),
        name: "Grayscale ETHE_141",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("24a52ECE417147f1a7AD7F11ad572900ab8047B6"),
        name: "Grayscale ETHE_142",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("24A9a53931193bcA84e1683474a5180fa8ee71Bd"),
        name: "Grayscale ETHE_143",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("24DEE6705A9E7368522F35119f128de1e178F2cC"),
        name: "Grayscale ETHE_144",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2555EF4fE1005af6d870B4aF4cB39521326FF67E"),
        name: "Grayscale ETHE_145",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2589329Ef7FBB1776d56E3FdbCAd19797FeB8a81"),
        name: "Grayscale ETHE_146",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("25E50cAb2185963446dfBe85CBbBdcE47083bCFB"),
        name: "Grayscale ETHE_147",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("268e811fA91b55FFb6E92907C6515909ff23EEa0"),
        name: "Grayscale ETHE_148",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("26a18533DFa961Bc2FD550184Fe7e256925Cc3D4"),
        name: "Grayscale ETHE_149",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("049373187fb5783CFDdC6c7cEA8Fb6E3D426Df92"),
        name: "Grayscale ETHE_15",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("26aA2057D926137f9a23Fe2EA53f49a349D07a6c"),
        name: "Grayscale ETHE_150",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("26D9ff75372192E206afCdF8737784508311E322"),
        name: "Grayscale ETHE_151",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("278Ad2617A315a184Dec06b7d57d9fa84034cadb"),
        name: "Grayscale ETHE_152",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("27C9Fa9009bea9aD283147D778eb306C8e670759"),
        name: "Grayscale ETHE_153",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("280E4582269826E0432E2bD545113C610Fb590C0"),
        name: "Grayscale ETHE_154",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2825410c632851d9a8590BCf57C799c56DD4BaD1"),
        name: "Grayscale ETHE_155",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2831a789f373506F93Ee4B424a6d5C07510891B7"),
        name: "Grayscale ETHE_156",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2832dDe931aa43385d12A4b542433d5cEA605268"),
        name: "Grayscale ETHE_157",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("288a51ea448d4505178410c1Fbf768d7F5C1E1Cf"),
        name: "Grayscale ETHE_158",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("28ba2AcEc9F711FB89a2B16c76AE842891d212c3"),
        name: "Grayscale ETHE_159",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("04C1c1125338A392251a51bcdFfeCe74669766DF"),
        name: "Grayscale ETHE_16",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("28BB7d15E9cc8b1c07Cc0fb16C7b9C4896A68C1C"),
        name: "Grayscale ETHE_160",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("291b1F8A326A4dBD3c89f142cBeDCDc914cE57ea"),
        name: "Grayscale ETHE_161",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2975bd4909ac4bB879c05712B2774098Af3eAc0A"),
        name: "Grayscale ETHE_162",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("297627e1DDD26b53601C416DBf98bE27DAF89d58"),
        name: "Grayscale ETHE_163",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2A53f7cFF1a204698013412EC5a1b98A9D3deAb1"),
        name: "Grayscale ETHE_164",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2aCadb3aDd5D074017EB03a705156c329eB51b99"),
        name: "Grayscale ETHE_165",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2b892d1A18a1881bEC419F5478FE8c565Ab50928"),
        name: "Grayscale ETHE_166",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2B93c057F1Ec01926FA4feB75310510f807c415D"),
        name: "Grayscale ETHE_167",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2B9a19334c5027717A18C31B8A53038dEc6Bab25"),
        name: "Grayscale ETHE_168",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2bac1984fEa7E034F3AC67dC385Ca2Fa0fCe406A"),
        name: "Grayscale ETHE_169",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("04D9cc35d5bf408A7d442fB45d235667144E4D92"),
        name: "Grayscale ETHE_17",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2bce681Cb725bB848EC82c28F848AB1454073471"),
        name: "Grayscale ETHE_170",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2bED9992D06da80d19d2256759d58873fE5f8006"),
        name: "Grayscale ETHE_171",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2C2D62573a4D304Af961400e57F4959Ac14Cc367"),
        name: "Grayscale ETHE_172",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2D14297b7ebc1c16CE9D09247C2287aFa454046F"),
        name: "Grayscale ETHE_173",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2D609485B0Dbe2721ae07981f9AF3A51fEA02aB9"),
        name: "Grayscale ETHE_174",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2D9A1D603FdF2C5Ed5674fac9Cd05231EAC6136E"),
        name: "Grayscale ETHE_175",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2E0697ef55Ec6727AD0157FB8aF5214D14b660F9"),
        name: "Grayscale ETHE_176",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2e2474172B94BcdC5bADB69bC66A2D80D01930f8"),
        name: "Grayscale ETHE_177",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2ec777B09DFB9B3aECCdEF1ecEf8904cAAf7E04A"),
        name: "Grayscale ETHE_178",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("2f4F99ed52a663439fe23cFe1CbA1E81d13609dA"),
        name: "Grayscale ETHE_179",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("050f910aE80022f5B1dF75B6907584090DAf8e94"),
        name: "Grayscale ETHE_18",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3049B8Da81E2e0D45daD156fD17672c96C23c4c5"),
        name: "Grayscale ETHE_180",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("305f81C7724c3A9B32833E55ac8A11407BCfB5E4"),
        name: "Grayscale ETHE_181",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3088c51b1AaA6D647bEef74C5e2f7E0DF7078E80"),
        name: "Grayscale ETHE_182",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("30A02Abf6421957dB3c25aB4baC048cBf4807b1a"),
        name: "Grayscale ETHE_183",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("30c1e8a2767eD5589C70FA3647A82e4390b2ef6A"),
        name: "Grayscale ETHE_184",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3114ABdDd2025156519fB8DcFD06137767DA8E94"),
        name: "Grayscale ETHE_185",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("31435AeD74efe468918932019550245a971A76A7"),
        name: "Grayscale ETHE_186",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("31e102Be69a7400afb6076c73159Ac86a1F32079"),
        name: "Grayscale ETHE_187",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3208A54432413C3D8f55566Ca9C8C79B4f73aFdD"),
        name: "Grayscale ETHE_188",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("327122cf542F9CcCA02792154F50A46fb430eC6A"),
        name: "Grayscale ETHE_189",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("05434aD86F21df497f857405Fe6714F2894aa66C"),
        name: "Grayscale ETHE_19",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3290dCd945a66b0fe1c21f2252c9F9e229208F05"),
        name: "Grayscale ETHE_190",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("33003EF1d6EF205e280c9529096c28D56958F715"),
        name: "Grayscale ETHE_191",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("334196e498a17aCDDFC0D74e43f6f53088E66e9d"),
        name: "Grayscale ETHE_192",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("33d7bae275F7F92F55bA569113220520C930BfBf"),
        name: "Grayscale ETHE_193",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("33fEf238DC09BDDfdc733B42451F98276E5E5985"),
        name: "Grayscale ETHE_194",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("34138Ddf9aD45B73EBeDe5D4a95dFa5b03AE045e"),
        name: "Grayscale ETHE_195",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("341A069071cde88b3491f7C4934f34Cd51100406"),
        name: "Grayscale ETHE_196",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3442D55E7a7b76d38Dd4f2F599744A208aDe587e"),
        name: "Grayscale ETHE_197",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("347C4C0b7cDFEAbeed74B0d3Ac81A107008e9579"),
        name: "Grayscale ETHE_198",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("34B9A6d48F527538E26Ca68aef8e868150A69d79"),
        name: "Grayscale ETHE_199",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("00c79701f58C8bD9d31Ab54d8ce7344063372049"),
        name: "Grayscale ETHE_2",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("055C8EBC495BC1363260544312CFAa9615f7Fc14"),
        name: "Grayscale ETHE_20",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("34C8Dc4bB3DC940f7b126c4417634E4dA7218383"),
        name: "Grayscale ETHE_200",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("353A2D0453D88F8b88a1F14aFB97d6165f244D42"),
        name: "Grayscale ETHE_201",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("35BdcA281f47f76bDc83d321a611EaBe5648284c"),
        name: "Grayscale ETHE_202",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("362319Ca486b30551Df8d9F4bB763ED179c74B39"),
        name: "Grayscale ETHE_203",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("363E8c2b87368c0dffB5B08c99057a07fB375A9F"),
        name: "Grayscale ETHE_204",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3666f19bc40a5b513DF91DE8461F5f3a29E26d95"),
        name: "Grayscale ETHE_205",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("36e5c4B77138F0C6386eF969225A005C28BCdA63"),
        name: "Grayscale ETHE_206",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("375ab28414204fcefb6E1d9ad4f2197FB29D9374"),
        name: "Grayscale ETHE_207",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3775234F636d6b2F56549FE60adEeB58eCCF504c"),
        name: "Grayscale ETHE_208",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("37F21816c8F1770b6a208530375CB864fB70FDC8"),
        name: "Grayscale ETHE_209",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0569FfdE3ed2f3802cBec6106014532a5DE8b191"),
        name: "Grayscale ETHE_21",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3819bD05a08A486EA3B85C0C82B9B0eEe6cB8465"),
        name: "Grayscale ETHE_210",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3846AEd50a2Ed5959A4ea897C32C97FC70881561"),
        name: "Grayscale ETHE_211",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("38FBA0ab545C09E9B5E55F0eDE29887dEE8e0014"),
        name: "Grayscale ETHE_212",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3903c22A77219539561285CB317ad592083f8a98"),
        name: "Grayscale ETHE_213",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3922Af37DF33aBf27782dF75Da64CC618E6CccF0"),
        name: "Grayscale ETHE_214",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("393930790d3b7202E1BEAdC458A48Ff7Ee4504BD"),
        name: "Grayscale ETHE_215",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3a113a33daB7996937b91FF986280eAF1708Cd36"),
        name: "Grayscale ETHE_216",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3a2E57560dA206AE0535C5fE98F5bF233b304b02"),
        name: "Grayscale ETHE_217",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3A2F53cc8c2A0013CdDB0b95f7676FDc214EF372"),
        name: "Grayscale ETHE_218",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3AD96760650139e59EC67f46C763A808fFB9b3BB"),
        name: "Grayscale ETHE_219",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("059B1b6A0a60412c015CE60Bf4bC63681F96Bf78"),
        name: "Grayscale ETHE_22",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3B102E02Ce180f2f3435d44c6Be597Ca0094DB1d"),
        name: "Grayscale ETHE_220",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3b333b621B2b94e217CFA63d046698a4d99cB399"),
        name: "Grayscale ETHE_221",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3B613F088ED67B239df4ea5468F00ce66B2B65bb"),
        name: "Grayscale ETHE_222",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3B8D07fCb7c236c150177E2791b457270E7C85bF"),
        name: "Grayscale ETHE_223",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3c0C3a1df6EF1320006AaCf1792489503467a238"),
        name: "Grayscale ETHE_224",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3D458cB4144Dab393Ad1D9f90A94Bc00cd49dEA3"),
        name: "Grayscale ETHE_225",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3db3769d4f39A6a7A0948CF8b5D872E29353A9EA"),
        name: "Grayscale ETHE_226",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3DC12115d9D30Be692a169aE75deada74EEAcD69"),
        name: "Grayscale ETHE_227",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3DcD969EE88d1a5B3F5bD90eBC29253128647740"),
        name: "Grayscale ETHE_228",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3E234c9B04ec6B1Aff78B7E9016577C27347DeDe"),
        name: "Grayscale ETHE_229",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0650E9308850A44300De044474dBaDd8fc38026c"),
        name: "Grayscale ETHE_23",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3E639A3240aa81AE91b8eDb391c66745CCd89573"),
        name: "Grayscale ETHE_230",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3Ef2411dE739fFeF2faDbFe33E10C6D13E67Ad6B"),
        name: "Grayscale ETHE_231",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3f3CFC8aB9189C0871e32Aa59388BcbCD5958e59"),
        name: "Grayscale ETHE_232",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3F41d650A3Ce99BAa2832f120BF5f07123Ed5D69"),
        name: "Grayscale ETHE_233",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3F45923b8a5d2c61Ce78aD8BE656532C5948A135"),
        name: "Grayscale ETHE_234",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3Fd4d26F176D1d9F9b6b4330b7F67C02D3a377E7"),
        name: "Grayscale ETHE_235",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("404F7C0F8c11bF2d797918D8E867cD7254c6C0B2"),
        name: "Grayscale ETHE_236",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4069D5D58Ac8A6C8610094bEF5F165Acd92E70DB"),
        name: "Grayscale ETHE_237",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("40A293f290aF3B8985fe0Dc0a0041323c220BE81"),
        name: "Grayscale ETHE_238",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("40D398E2D1471a986A635de2e3C4099b8D368CE3"),
        name: "Grayscale ETHE_239",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("06aB3A15E0E2012d5fDB2A5714f965c511a8aF32"),
        name: "Grayscale ETHE_24",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("40E4cF00f021B5dCeE278f6619BFe966ba8A798d"),
        name: "Grayscale ETHE_240",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("41144f86957101935C62E786b7Fea3a1D71bf111"),
        name: "Grayscale ETHE_241",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("41958b04337651C73350415B1950B2049d69b40B"),
        name: "Grayscale ETHE_242",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("41Ba4da465F2c9de9AdF70FB7A91E510778E9Af7"),
        name: "Grayscale ETHE_243",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("421d841b17A3408A58178EF26e12bC41723E3766"),
        name: "Grayscale ETHE_244",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("427644A3B26406fcf0693a7eD5377c9cD75AE6eb"),
        name: "Grayscale ETHE_245",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("42917f0799F89366C54f5E7F50Cf1BF1046387e8"),
        name: "Grayscale ETHE_246",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("430e4CdD77b8124B82188ebAC0359F5577aaB4E9"),
        name: "Grayscale ETHE_247",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("43435522Fa02173d9d49D6495C236AAb95Db1d3D"),
        name: "Grayscale ETHE_248",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4368931747571cb8c28b4DD1183ef5db5a51A2C8"),
        name: "Grayscale ETHE_249",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("06b8Ad078becC1B07BfD46aF32eff499446801f6"),
        name: "Grayscale ETHE_25",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("44226D4869500dD3b01E3F20b77F7e725Fc81c03"),
        name: "Grayscale ETHE_250",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("44AfAd2e378b2F37c4bA244621cCD74eFFF8f826"),
        name: "Grayscale ETHE_251",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("44De51742Cd194040AEb156A13E40f81383ea92E"),
        name: "Grayscale ETHE_252",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("44e1EaA71B565BeD26ffF68AdCae17D5b5daAA96"),
        name: "Grayscale ETHE_253",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("44e8039CDDFf2C5aE124D47212999ce32D4e4430"),
        name: "Grayscale ETHE_254",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4597306b1ccFA70aF7C2B479c3d7C6246566D5f7"),
        name: "Grayscale ETHE_255",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("45c9a1C0CB059Ef87D1d9A935f9d980951e29d47"),
        name: "Grayscale ETHE_256",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("45d02905C31a6E9F6D26Ef3d998ed7BAFC91192D"),
        name: "Grayscale ETHE_257",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("45E18f9E117F2bCB0eFAD51ae80fC2B7B6FB8344"),
        name: "Grayscale ETHE_258",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("46556c2151bD4602830D48697d7FB964cDBc11b1"),
        name: "Grayscale ETHE_259",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("06f87b41d828FA6AAE07C1aC85F46E827f456977"),
        name: "Grayscale ETHE_26",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4675bE7bFd711281Ba0074D373115FF2DDe31241"),
        name: "Grayscale ETHE_260",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("47895E5009Fd609F0027eA896bb5C38869082223"),
        name: "Grayscale ETHE_261",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("47f4D5c338A6853c89DE9dcE11ba78361b3de18E"),
        name: "Grayscale ETHE_262",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("481C9CefeCA4d50fB31FB32D852b0838B44C2449"),
        name: "Grayscale ETHE_263",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("482D65A58E8722025022AB68dE8F91C6136Dd861"),
        name: "Grayscale ETHE_264",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("48dA64CE8dfDc9300E5d91c4C32115c2BcA6b505"),
        name: "Grayscale ETHE_265",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("48E2Dd5c9bDe84916BC1DbAA976AC1cf2a8D7033"),
        name: "Grayscale ETHE_266",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("48e8EA216e2cc0aC022EAcdED7C0fF7cC981C31c"),
        name: "Grayscale ETHE_267",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("48FAd849Cec1aE0E945D4E0E9abE6253cB83cc40"),
        name: "Grayscale ETHE_268",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("491d2F947369A24915FF2b05324CDFAA1b892e7c"),
        name: "Grayscale ETHE_269",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("072c9C1c0660abA4d08C13e29A97F7Bb175918F6"),
        name: "Grayscale ETHE_27",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4976e9Df99b55D2334A920794d27bE00D9ac8423"),
        name: "Grayscale ETHE_270",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("498441a9ccd6cDcB8374D0Af6024D99311167cD2"),
        name: "Grayscale ETHE_271",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4989d7983057fE34E5Ff0E33DFda49629c9D17f6"),
        name: "Grayscale ETHE_272",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("49B7548Fb65eb35166475CdB61a5C4e893ceCa1F"),
        name: "Grayscale ETHE_273",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("49F2702a89933Dfb14E9A09852CADc291688FE38"),
        name: "Grayscale ETHE_274",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4a063332cCc4C57EbBe62439b11E389Ee83Ccdff"),
        name: "Grayscale ETHE_275",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4A6a321BD349FfFd9f377bE00D3b4122423bAc87"),
        name: "Grayscale ETHE_276",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4A80c83a138fb21f54cb1B55d0FfcC6254bb53dd"),
        name: "Grayscale ETHE_277",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4aaa7c8D86Df45c2b4c2d4487857f203d86A1b0d"),
        name: "Grayscale ETHE_278",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4b1e34C92DA600Ce8da4B9FC5202d17dcA343671"),
        name: "Grayscale ETHE_279",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("075b440D4D92AAf2B66f1365B81a508eCd25a40D"),
        name: "Grayscale ETHE_28",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4b3Ab8E184Ca2B02034A379f1ad8ee0CB85FBBd2"),
        name: "Grayscale ETHE_280",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4B6bC43C0BC9B671227988f8c25422bfF7306D2D"),
        name: "Grayscale ETHE_281",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4BBD78A6DBEA8dC5F5B0cA5117c82E3cC0Eb5875"),
        name: "Grayscale ETHE_282",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4c0179d931E8F23C75bE25912E891e15630aAB12"),
        name: "Grayscale ETHE_283",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4c1128880d330F5E0BB7D702b9b024B958a61D4d"),
        name: "Grayscale ETHE_284",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4C912A73Af93a6D99a456966eA27952426E2109B"),
        name: "Grayscale ETHE_285",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4cAd7E8d1bE7930b51DccD2F9654c78f2e326453"),
        name: "Grayscale ETHE_286",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4Cb5E0034c893f9fbbcc16e660A26a6F7cA4F3ED"),
        name: "Grayscale ETHE_287",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4CD28eb1A30742089f104Ccc4F7FE113BF75cbCC"),
        name: "Grayscale ETHE_288",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4d22E4F9f2BC8a0697279ce83Dd1D23F1136b707"),
        name: "Grayscale ETHE_289",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0778e57edA067231489382E83b968f86880FD1db"),
        name: "Grayscale ETHE_29",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4d31998C8a4d00648d4dB75982E8dfbBEB0Af313"),
        name: "Grayscale ETHE_290",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4d371ca3FFB6e6800aA3216283Dd21672eB647d7"),
        name: "Grayscale ETHE_291",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4D6E89BdF8374DC2E97Aed249b05EE1210e4F9CF"),
        name: "Grayscale ETHE_292",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4daC9B0a16e8C81D692A30c621763df7248DB6f0"),
        name: "Grayscale ETHE_293",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4DAf24CB5ac80F56DF1C342F77fE3F5A364645Ed"),
        name: "Grayscale ETHE_294",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4df294F41736c4767338D4164fDe345fA375d284"),
        name: "Grayscale ETHE_295",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4dF55D7f6795c218DD00430032e630814ca6eBcc"),
        name: "Grayscale ETHE_296",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4E3440271F59De4b072B3D5D9e0b408C4262594A"),
        name: "Grayscale ETHE_297",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4e873Ae18f40B679A2F7d49e495EfA882E8aAA15"),
        name: "Grayscale ETHE_298",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4Ef80540d1195BD17Fc86E42A334d5957A7c3E16"),
        name: "Grayscale ETHE_299",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("012746EE19ad1D0a1648fF1B00d43273877a0ab8"),
        name: "Grayscale ETHE_3",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("07D7a1D64E35e69241a1519Ea6bC1e09F66c99E9"),
        name: "Grayscale ETHE_30",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4F4b4Acc3907511d66442B3EDA559af03cBBED95"),
        name: "Grayscale ETHE_300",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4Fc8846E1d9122e9a58EBb0746da5fBb8DA67024"),
        name: "Grayscale ETHE_301",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("4fe407507956aF73847d703c13F3b33504b8134e"),
        name: "Grayscale ETHE_302",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("500140CE79eF3B75fC2DbD9690AA32E9dA4296D6"),
        name: "Grayscale ETHE_303",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("503a526B4cE2565edB55DD581De2F4Dfe5eF65F0"),
        name: "Grayscale ETHE_304",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("50a37d86b7F5C11A310E280B87f184138BcAaFC5"),
        name: "Grayscale ETHE_305",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("50cdE33Eb1dbd3624529F2968E4443dd8fbC1811"),
        name: "Grayscale ETHE_306",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("51AC57B623555909a6Ef97B88a1880Cb4547e1D3"),
        name: "Grayscale ETHE_307",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("51D164829b4e2938E1ece6D5111290E8b41875Ff"),
        name: "Grayscale ETHE_308",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("51fcA5cD4546e0b2745bBE45C1654DA75928C9bD"),
        name: "Grayscale ETHE_309",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("07e06bA1150946cdF00b9f20e2e52568150E62a2"),
        name: "Grayscale ETHE_31",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("52376658D27cd024Dd18De54912cf355719F0F3E"),
        name: "Grayscale ETHE_310",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("526201784Fb54111a72ABDC50BB4950d75cA561a"),
        name: "Grayscale ETHE_311",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("52676741724fB7E9700135164affbeC7A20C1AA0"),
        name: "Grayscale ETHE_312",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("526bc3216c5419491f52C6c99BCE8406121B5C4c"),
        name: "Grayscale ETHE_313",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5296e103e9072d450970fc18d8736E12A1BC6f9b"),
        name: "Grayscale ETHE_314",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("52B210970d7F054Ac20c346bC56973c35C8EFbfD"),
        name: "Grayscale ETHE_315",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("52EC584ded2958E31A2ECafb76724fd2532Cd9c7"),
        name: "Grayscale ETHE_316",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("532047Bd23F556a66a0B465307258332FC85E7cE"),
        name: "Grayscale ETHE_317",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("534FFEaF23a8672B0a54c245AA1f80300670e246"),
        name: "Grayscale ETHE_318",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("53986753C1dEF5956E17f8dF52cA0709e10356fC"),
        name: "Grayscale ETHE_319",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("081BaAA7BB3F13F55aa1EC2F0911daf532035d5E"),
        name: "Grayscale ETHE_32",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("53ed5F71A8164d5C75E2704Db4ea07189f45AEb8"),
        name: "Grayscale ETHE_320",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("540Ccbf4E9BA934463984Bcb3D28A33b0647f798"),
        name: "Grayscale ETHE_321",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("542E0946e97eA6934b27fb5825C342c477683f5c"),
        name: "Grayscale ETHE_322",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("546b44D5C69C331A5f98D21f0318f762b420af84"),
        name: "Grayscale ETHE_323",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("54b91CA7542Cb7387ec721A6f73DBd9434b3B0D9"),
        name: "Grayscale ETHE_324",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("556D8aBf8e9DA36470Fe6f53e5D84fa62654B4FA"),
        name: "Grayscale ETHE_325",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("55A834B42b33b737b1099b488967BF33a835185d"),
        name: "Grayscale ETHE_326",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("55DE16080D11EEA7dc4bB503884c4d1446480342"),
        name: "Grayscale ETHE_327",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5642f1F205f440899b32c5E0625888Df9F3c3f53"),
        name: "Grayscale ETHE_328",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("569FF84894dD10f944572A5375F09f326127Af1F"),
        name: "Grayscale ETHE_329",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0844E137f4e87Ad3f3738D5B39514024146097E0"),
        name: "Grayscale ETHE_33",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("56AAEeFD0dEb98aB082659B592DCE108755AD0c6"),
        name: "Grayscale ETHE_330",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("56EE86eF8298CD7D69b2184AF6b36322b467592d"),
        name: "Grayscale ETHE_331",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5705F2486Fc28BE7a1eA040bf7571Dd2E774F9F9"),
        name: "Grayscale ETHE_332",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("575C493af071E84e9c383573158F3F53740fa900"),
        name: "Grayscale ETHE_333",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("57b7904E9BE6D1C353022f7262b04F96bE14DffA"),
        name: "Grayscale ETHE_334",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("586cA3c590988B73484C823eef9BdeC23ad1Cae8"),
        name: "Grayscale ETHE_335",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("58baE5c59F73F764148B127873A211eae1111A1b"),
        name: "Grayscale ETHE_336",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("59D2bDf33369266d0b5C0D1C31516e83d571c9dE"),
        name: "Grayscale ETHE_337",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5a4F0A702F1f864bfC3FeF7d312212BFd922EC5b"),
        name: "Grayscale ETHE_338",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5A66112882827010BA6b308062c3F7a722E40d37"),
        name: "Grayscale ETHE_339",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("08582aB2F406fCb715dBDAdAbc134a889d53d50c"),
        name: "Grayscale ETHE_34",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5aCF811f58ae94c8A049fC977CfA4949e43D5264"),
        name: "Grayscale ETHE_340",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5B2c70D606244611396Ed6e6ab8860dbF2A88430"),
        name: "Grayscale ETHE_341",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5B82Eec97845749F29514521F5d3A49b2d733276"),
        name: "Grayscale ETHE_342",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5B9d696B46454e6A9450D0281866A6A6c0740788"),
        name: "Grayscale ETHE_343",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5ceCDa7Ab52609B4bD5459aB3f12B082A0fb21df"),
        name: "Grayscale ETHE_344",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5D48ae24E96ca893E7dE05a277d6A90c35588E5F"),
        name: "Grayscale ETHE_345",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5D81D3226F47536df852428CF2b66DdFB307a2b3"),
        name: "Grayscale ETHE_346",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5d84E5273E5d0260b39De2114F3b4dC81e4BEe2B"),
        name: "Grayscale ETHE_347",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5d8a8f2d93948b11009Bd81828A1452079983A4C"),
        name: "Grayscale ETHE_348",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5dDaeA1D0C93e6489D2C1fF4e28b7e44e2a0F81D"),
        name: "Grayscale ETHE_349",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("085e1B2190Ad385A878d77e134e31274E9833A2F"),
        name: "Grayscale ETHE_35",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5ddE71d006b04272e7850B6A8FFFa567D5d785dD"),
        name: "Grayscale ETHE_350",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5E0C61bd1ad044d3a54d41C43310d9Ac22484868"),
        name: "Grayscale ETHE_351",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5e3a4c31EaDD4554AE901A4491cc62d6c03B8F64"),
        name: "Grayscale ETHE_352",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5E438815628819b1f1618016317bAb36A82c767E"),
        name: "Grayscale ETHE_353",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5ec074901908091Dd04779C120EFf22406497D8F"),
        name: "Grayscale ETHE_354",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5EC2Beb42F59b45E7540f350CD57eB6D6466e083"),
        name: "Grayscale ETHE_355",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5EfCFA606c56a2Def2afCe5B54D607423468997E"),
        name: "Grayscale ETHE_356",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5fB9966db64830a2C10FB298d9A4eF5c98550FDd"),
        name: "Grayscale ETHE_357",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5fc9917Fa8b221DB26F3EEe2A7484d587BDEFB50"),
        name: "Grayscale ETHE_358",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5FCf28A33928beD22fDe2680367f008E89F00FD3"),
        name: "Grayscale ETHE_359",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("086617A910E1DDC1291a26cf65F0ee5746F848d8"),
        name: "Grayscale ETHE_36",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5FcFC77882BdB84905350355fEE16D6a10E282aC"),
        name: "Grayscale ETHE_360",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5ff792Cee953fa098036909a9d95E1F3198586E5"),
        name: "Grayscale ETHE_361",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("600109d669EcC3110A83f72086cA188fDe39795F"),
        name: "Grayscale ETHE_362",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("60184FAe7fE853dA30524B0AD364e3D5d7A53597"),
        name: "Grayscale ETHE_363",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6021DD7F879e0223955F8c68F8EF3194dd1cB527"),
        name: "Grayscale ETHE_364",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("606df8d8b1A5F8fEbe72e68caE9401365Ee0d0b9"),
        name: "Grayscale ETHE_365",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("60f8Cdca7a3164707958C3526bbE4b6C0CE8992e"),
        name: "Grayscale ETHE_366",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("60f97534C05d432D18def0d9920C03Ac4b215E2C"),
        name: "Grayscale ETHE_367",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6121278E5d3bE46E93cFa6ab2a35134a7279cC5a"),
        name: "Grayscale ETHE_368",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6138c242D5b905C70976fD67DF07Bdc58a05B9b0"),
        name: "Grayscale ETHE_369",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("087015b6d9d62c7346f25d01c5e03Acb44EA61d4"),
        name: "Grayscale ETHE_37",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("614c0880A7735597c949e973cF40532860a7149c"),
        name: "Grayscale ETHE_370",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6166de40e3681cAeF80743B3d2aEF36e9f1876aE"),
        name: "Grayscale ETHE_371",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("61AF113c645C0fd0214357eC1C08d0A52E5bfD8f"),
        name: "Grayscale ETHE_372",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("61Bd22068505658F82904E334B3ec303245126B6"),
        name: "Grayscale ETHE_373",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("61d6f8fF3e8c11d89af3BfFDd880bF7333a89078"),
        name: "Grayscale ETHE_374",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("622a687Cfb2C552d4fAa12b984dFF48847DA8FBE"),
        name: "Grayscale ETHE_375",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("625096293C013037F8e951b6CAA469840b27E0e0"),
        name: "Grayscale ETHE_376",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("62FCc6Ba6e187eB39D435098f995cbf0239d3944"),
        name: "Grayscale ETHE_377",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("630FA12c6fB12eeD6f0c6e3030161DB6872e6fB0"),
        name: "Grayscale ETHE_378",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6333a7192ca0dDbAcea55F586092A8A75AB4f2FB"),
        name: "Grayscale ETHE_379",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("089FE46c34c17EF05CE6138ece644404da2039d4"),
        name: "Grayscale ETHE_38",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6349529C8E85408E0C5d939AE5C5A092641DD141"),
        name: "Grayscale ETHE_380",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("63B8C30CbB4E63C9239E25F9387d37E6d987dE63"),
        name: "Grayscale ETHE_381",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("63eD6c4AaFE066789b1c91afAD0DD9917990B79b"),
        name: "Grayscale ETHE_382",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("645471133f8f7aF362a2878796Ed516452d5Cb24"),
        name: "Grayscale ETHE_383",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("64a9305ee172b61e4Ed60cc293B818c47F773498"),
        name: "Grayscale ETHE_384",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("64B101ecE3dC13cf23C44e5496976f14D5821F67"),
        name: "Grayscale ETHE_385",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("65C1c7Df115124D7e268221907B7AAfc2437F75d"),
        name: "Grayscale ETHE_386",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("65CdA63f1D330AB2c43DF76640a9080f0c18d702"),
        name: "Grayscale ETHE_387",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("65fF904B040aa6790bC57E04450e2ae68c2ceD56"),
        name: "Grayscale ETHE_388",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6603C9f466373dCFFE3360e4f054D8F1BB23Af3b"),
        name: "Grayscale ETHE_389",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("08C491e232A8E304e9d59883DBdbe8192B3cF027"),
        name: "Grayscale ETHE_39",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("662c1D33923f1b48b5cdc1F4570423Ec1Ced1228"),
        name: "Grayscale ETHE_390",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6709842C12e713789f5706e0DC207921e0835d50"),
        name: "Grayscale ETHE_391",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("67e2c03a29Bf611BbeF6E09D519945170486ff40"),
        name: "Grayscale ETHE_392",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("68084Eac915E3429320eD6574811a0eA2873427C"),
        name: "Grayscale ETHE_393",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("684bF2573A3fBB92F656787C6EC5b5E7844f29B2"),
        name: "Grayscale ETHE_394",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6864159A8022776c4F61F7c455c86bb9C92b0a2e"),
        name: "Grayscale ETHE_395",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("687E8322e100318c890dfEB21123DC6FCbf42666"),
        name: "Grayscale ETHE_396",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6887dB27b113D72e6158DFD3fE0e69acd7b1FBcf"),
        name: "Grayscale ETHE_397",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("688dD446b595D5702E9dcB545C89261B1C0C7CC7"),
        name: "Grayscale ETHE_398",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("68e4b4a6D98ae3cBc969331BDB11257229165C83"),
        name: "Grayscale ETHE_399",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("02215C9655FA9C14d993Dc6d75B06Aa863C53ec2"),
        name: "Grayscale ETHE_4",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("08CC6C946Cda9D27Dbd2B0a03eCB3011514F8B03"),
        name: "Grayscale ETHE_40",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("692fd667D7C7d38b99397f344d31fA6182f58bdf"),
        name: "Grayscale ETHE_400",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("695483E6f68A939fEDdA945499C9e3eE557C1c3A"),
        name: "Grayscale ETHE_401",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6971f9330d938Bb2AcC64937301e6A472CC2aE56"),
        name: "Grayscale ETHE_402",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("697e3972fc5E51cE91313E159ce19ebffA87dd75"),
        name: "Grayscale ETHE_403",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("698aD35Fd27e5eA9A57C119C7B1770aCa092BBfC"),
        name: "Grayscale ETHE_404",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("699152F1AcE5504Cbee3a962FB83eBdA907D1941"),
        name: "Grayscale ETHE_405",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("69b88cC44CA70008237017A5C8D7161beF0EC043"),
        name: "Grayscale ETHE_406",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6a211b0FDd9eCa4339f02E8da47C348808CA7fC6"),
        name: "Grayscale ETHE_407",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6a2777630E021093A88d431D20dB8FB634E4D88F"),
        name: "Grayscale ETHE_408",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6a455E003D1b8Da4f036eBA4c9B7c913907E7F62"),
        name: "Grayscale ETHE_409",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0936BaB7E471efbd4eFe1d2BE2E37d1Ea70444FB"),
        name: "Grayscale ETHE_41",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6A4e04255371891D03daA2A928b553EAea883DdD"),
        name: "Grayscale ETHE_410",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6A9869dBc3497662b406DaF81a4E030866988319"),
        name: "Grayscale ETHE_411",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6a9A05415F78EeF19DE272e0677f42c954DDECB4"),
        name: "Grayscale ETHE_412",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6af5A0155E102889cEb4A38116194fa9404CC45B"),
        name: "Grayscale ETHE_413",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6B26487e2Ab4Cbe34d88EB5B57311f82Ed4F0e47"),
        name: "Grayscale ETHE_414",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6B7Da8F6844B90f90bfC646816DE6Cd5fD741fD0"),
        name: "Grayscale ETHE_415",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6bEC1049e676D6e71F7578bDC05D8A03d7A1931F"),
        name: "Grayscale ETHE_416",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6c3200e8B067Ec14e737dA32FaE6D902116aB50D"),
        name: "Grayscale ETHE_417",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6c65fDb3B8A07c0eE373eD4968291d5194b1b29D"),
        name: "Grayscale ETHE_418",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6c8d223C95E527BcE5B03CE3741D80d64912B6ac"),
        name: "Grayscale ETHE_419",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("098d384670D9178136bBf89CB2f76CB3eE4DbaC5"),
        name: "Grayscale ETHE_42",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6D1338aE291fa7CbBF74F1B18815005b5d060e0c"),
        name: "Grayscale ETHE_420",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6Da0264F1c8a5eA780AD360fe1264cc6132aBe56"),
        name: "Grayscale ETHE_421",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6Dc0efE64CB54E87f137465fb9E94eC96B5E56c5"),
        name: "Grayscale ETHE_422",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6df177b73AdCD1fa24892ACd367673a19af00E9B"),
        name: "Grayscale ETHE_423",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6e02A9cBcA1C9678a58541256AD532Ee888Ae3aa"),
        name: "Grayscale ETHE_424",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6e473032E85E92A3dc5b57E99b9431918e61BE22"),
        name: "Grayscale ETHE_425",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6Ed11c6B4b64db34300565fB87778638b9114f81"),
        name: "Grayscale ETHE_426",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6f00d284995d3Ad8Ed08B0C9e046b6fF8B5FdFDE"),
        name: "Grayscale ETHE_427",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6F2b800370DBD6F45eCEB8d10d0E89067bf425D3"),
        name: "Grayscale ETHE_428",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("6F64660cE9967D7B9285818C884EC602984b0591"),
        name: "Grayscale ETHE_429",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("099206266A54d4D7a41Fb5eB5B687EBf7ac153B6"),
        name: "Grayscale ETHE_43",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7010aec5Af99FecAA716020CE52752eF07fEEDb9"),
        name: "Grayscale ETHE_430",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("70154EdcD2278cA9acbD30Fc253b8B272e0A76a6"),
        name: "Grayscale ETHE_431",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7047E704228cEF3cFf0295Ca09dA1B5bAB3D7402"),
        name: "Grayscale ETHE_432",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("70A3506B3ab2cA44CE461D702ec91D6B1ADE643c"),
        name: "Grayscale ETHE_433",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7157fd5747a219Ede2180b15865913365A80805F"),
        name: "Grayscale ETHE_434",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("71a3DDF2A2614695A4e85E13BA15afb754a72Fe9"),
        name: "Grayscale ETHE_435",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("71B77Eb874c2E8B530133244E2a28771040762DD"),
        name: "Grayscale ETHE_436",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("720d7dDBDE53962a97f072D069eC11aBBF842B12"),
        name: "Grayscale ETHE_437",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("72412CEC19b8215F48e34Be86D72eEC979AD10F3"),
        name: "Grayscale ETHE_438",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("72848D06C79a85d5632e0B04c9f59578794F1beE"),
        name: "Grayscale ETHE_439",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("09Be86C2f440E333bbB165F9eE810572d88df0b3"),
        name: "Grayscale ETHE_44",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("72a7ddDBe54Ef33f55373B312810553518159BB1"),
        name: "Grayscale ETHE_440",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("72F2702c9baaaa8a24319717B3068c514db87305"),
        name: "Grayscale ETHE_441",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7326972AE7eBa13794Ee2A8b57FaD7057331cE2c"),
        name: "Grayscale ETHE_442",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("733639079F43CA1Ca1F05daAeABF015e0FAF5166"),
        name: "Grayscale ETHE_443",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("736De6Dbd3E15a7BA020202F95582E0062f013A3"),
        name: "Grayscale ETHE_444",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("73eA72aC2059e71Dbaa3A8bb30144Fb2D7A684F9"),
        name: "Grayscale ETHE_445",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7401eA6D95e77894b13775fE3e15b48D053D8E0f"),
        name: "Grayscale ETHE_446",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("745a45AEe9b15EFcf2961617e5E107F7106841bA"),
        name: "Grayscale ETHE_447",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("749bAD48A876B2b9BBA1F5d0DAc7b646D10283f4"),
        name: "Grayscale ETHE_448",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("74CFB89a4b0b0C774f39AADdF80F882518Acf047"),
        name: "Grayscale ETHE_449",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("09EA755F8026604D3F3Cc1f99a0EeA5c343657D6"),
        name: "Grayscale ETHE_45",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("75BA1cE57cE55ae3aB8A83F333449aD3B63De8b3"),
        name: "Grayscale ETHE_450",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("764B0ADfa038721cF27b15e2eB94C22341Fe5A2d"),
        name: "Grayscale ETHE_451",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("767a26Cac94fe8B3C78ae75ad1fE53FC084C82b2"),
        name: "Grayscale ETHE_452",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("77591ae37E1Be16611c330EEbed3A6498a2C4E76"),
        name: "Grayscale ETHE_453",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("77982C9f1B231bcE3F03E712A09462863E73Ee29"),
        name: "Grayscale ETHE_454",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7879a9a2e67BecD33eA97c8597CA249BBeE54fAc"),
        name: "Grayscale ETHE_455",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("78d13179A60c4f452ded10c92a70efd5A547a653"),
        name: "Grayscale ETHE_456",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("793d8C2db11065b359Cc4196c945A39Fe449F1c0"),
        name: "Grayscale ETHE_457",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("798565E80cB60314099C0E0CB1f11baf5a0Ea58C"),
        name: "Grayscale ETHE_458",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("79Ae5DCd42A1Fe24E63EF15296a73Dd6c5e5DD16"),
        name: "Grayscale ETHE_459",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0A172C6c79c87babcA56115A43443aE6c087Cc83"),
        name: "Grayscale ETHE_46",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("79F8c44A5EEa39528F6e03fbD256C0e8D414fcBb"),
        name: "Grayscale ETHE_460",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7A590D53a490663b592de429De498ce105976054"),
        name: "Grayscale ETHE_461",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7AA8cD0e94F81731da1DcEDf437FCbde21bd07b6"),
        name: "Grayscale ETHE_462",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7Ad8DF872E3d0933D47575c70FE22aEDC7aE6eA9"),
        name: "Grayscale ETHE_463",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7b250aE733d29cFeF657cfB881c34c4ee4AE08aC"),
        name: "Grayscale ETHE_464",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7b2E9f8678093ceC16eBfC9e050f0c5696630c7E"),
        name: "Grayscale ETHE_465",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7Ba4A54a91B316018998563145C9C172B32a9190"),
        name: "Grayscale ETHE_466",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7bddAa28Ac15ac13edFfE089B94839B9F9A267aC"),
        name: "Grayscale ETHE_467",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7c11edf59ed7Ddfa72Cdad9Ea413415Df5fd84e0"),
        name: "Grayscale ETHE_468",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7c23c86FE486C3995C842eE6Ae562D276f1AA4ed"),
        name: "Grayscale ETHE_469",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0a24Bc9b2725e94591ED360335903c852807f890"),
        name: "Grayscale ETHE_47",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7C9F709A65C0eE9Ed3f271Be0D20ab9f34607834"),
        name: "Grayscale ETHE_470",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7CE5d5cfec1482A2470682809d7095A90D2c768e"),
        name: "Grayscale ETHE_471",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7d0523795e35c67B14B5EDce151266A5C0Ff2d65"),
        name: "Grayscale ETHE_472",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7d8cBa4e867F85C4ea19E60eDDb40a04dE8F6b7e"),
        name: "Grayscale ETHE_473",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7dd9737a63B7DC02A7719d8b85EFBA52942f35e4"),
        name: "Grayscale ETHE_474",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7E87C3Ea18097BB096F9D7a1A5cf4521207C0fA6"),
        name: "Grayscale ETHE_475",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7EbB43fedb9544F565e0181bdD09De865408f86D"),
        name: "Grayscale ETHE_476",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7f44a10b27d00542aa2428deb3C1f0d5C715c3F5"),
        name: "Grayscale ETHE_477",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7f74AAFd223f2462e52De66f36e98498fd141c6e"),
        name: "Grayscale ETHE_478",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("7FA888ba66819244DEDe8fafA1d2670b8b906204"),
        name: "Grayscale ETHE_479",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0a6700fd5c4B8d5928f7F31Baf3abdD0d53FC940"),
        name: "Grayscale ETHE_48",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("801FAD4e481d610C167c1C45bf5C7A4606c935a1"),
        name: "Grayscale ETHE_480",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8076De48B130d169B1bFd9080E7f83932b2eF24F"),
        name: "Grayscale ETHE_481",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("814f948Dc68e246D28aFDE2077CDF305BfA1dEa6"),
        name: "Grayscale ETHE_482",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("816D8Cec003b00aa87a10D40EdAACadE27619ae3"),
        name: "Grayscale ETHE_483",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("81D41De414eA4a781840D7c45f197506736db104"),
        name: "Grayscale ETHE_484",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("821447965e1A0277393707A4C2a5F5F9B4d6F68e"),
        name: "Grayscale ETHE_485",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("825650C11D8984CFbAb8d131074D93BE36E12F7A"),
        name: "Grayscale ETHE_486",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8307F2728756fE74Fe881bB6555aD7754B3270b0"),
        name: "Grayscale ETHE_487",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("837F4A04694F54B48ff79779a1067d0A61cb0aa6"),
        name: "Grayscale ETHE_488",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("83b4C6E8a68c6b108F9893c141F5671Ea0B32C87"),
        name: "Grayscale ETHE_489",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0a99D8394A28A19D90CDdb8894894db81693a9A1"),
        name: "Grayscale ETHE_49",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8431b405e21DcA8cC947B0073108f0C6922E872b"),
        name: "Grayscale ETHE_490",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8432b7ca18308E1ec9e4eF72619fE0F7De13A69E"),
        name: "Grayscale ETHE_491",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("84528A9c8a5fB6205cEe8068DF033eb7F46b74eD"),
        name: "Grayscale ETHE_492",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8470590Edf758197368b3BC0327cEF0048c2fB1B"),
        name: "Grayscale ETHE_493",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("85A8E39FeD50EFdE8310FaB6cDDc0B68d767F018"),
        name: "Grayscale ETHE_494",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("85E22E411E0Ad55c66BF5718089cD77C5C2b2425"),
        name: "Grayscale ETHE_495",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("861900B0276E2B681791fB0422371b570227927C"),
        name: "Grayscale ETHE_496",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("86815B753bAd06cfEDE42de38B07d27350390472"),
        name: "Grayscale ETHE_497",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("86De4FF1fc04bC47D941aa2721684b86d1066bf3"),
        name: "Grayscale ETHE_498",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("86EF3eD11ad5ed0C9c5D2d1973FeE2c5089427B8"),
        name: "Grayscale ETHE_499",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("02367833F3b379e66F4faF0ef015785b81B187fE"),
        name: "Grayscale ETHE_5",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0B1A9A55D574CeBBdB0E08c90A5CF4Ae3B6E273A"),
        name: "Grayscale ETHE_50",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8740eBf35c08627daC6a8CCB0310Aa920e0E66A1"),
        name: "Grayscale ETHE_500",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("87556707D7f3EDfaBC3564D04c4F6DEE7a2bc2C8"),
        name: "Grayscale ETHE_501",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("88baCDa2Ea0fCc99875244B9a0741e715dA4C18A"),
        name: "Grayscale ETHE_502",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("88ee87F40C77ea2cA43294b4f65Dc4391223b1D8"),
        name: "Grayscale ETHE_503",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8936424b558a95FBDec5938b1dD95A11e40a4532"),
        name: "Grayscale ETHE_504",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8A091A0Ba5521dbb6C017A0461f9Ed069d97Ad5d"),
        name: "Grayscale ETHE_505",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8A6F02B8BA5C575fC668822fAEaDa690D7fD1eFC"),
        name: "Grayscale ETHE_506",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8A9af7e888ef1bB12820480322b042F52754ce5e"),
        name: "Grayscale ETHE_507",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8bdc9f9fb9e2399cfF176b1b8a1230E7606855e5"),
        name: "Grayscale ETHE_508",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8C24Fd5ec2FA8B54a927E98a44B3f420e73Eef23"),
        name: "Grayscale ETHE_509",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0B3FB306E59bf5A2740B452c27717F79756B48e5"),
        name: "Grayscale ETHE_51",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8C7b0BeDEddCf071Ba1C4FA0ADd63c472759c5c4"),
        name: "Grayscale ETHE_510",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8cB040DD5B26AC5d677b5206e92afAA8B3529734"),
        name: "Grayscale ETHE_511",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8d4878c44F18FC8B4C77727C4F3BC9CF3E657057"),
        name: "Grayscale ETHE_512",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8D5cd5cDc222a45CE2bBa319E3C037A38C2D92C0"),
        name: "Grayscale ETHE_513",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8D6897bea342692E4BB57eE3115378c98Da07932"),
        name: "Grayscale ETHE_514",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8d7fb46B650705D3B1DcfeF22FE284EC65Ac2dE1"),
        name: "Grayscale ETHE_515",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8dED4b4Ab7E4eB79eE929D955Bd4652500BaC64E"),
        name: "Grayscale ETHE_516",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8e26A0D4d724CC77C0A988C375B5cEBc0fF5ae62"),
        name: "Grayscale ETHE_517",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8e2B4BCEadc867Eee37281e8793D18996Bdea044"),
        name: "Grayscale ETHE_518",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8E88cb696F99f8b047B0BE56A6Ce743Fc4619F66"),
        name: "Grayscale ETHE_519",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0b8428AbD130eb4c6Ffa9Bc2D89bbbB1F3D09AA4"),
        name: "Grayscale ETHE_52",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8e8f382F5aEFA99C665AE5223685a8b6c308B4e3"),
        name: "Grayscale ETHE_520",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8f1eF2c70A61CAE8Dd54D25BE7B44c2E581C6b33"),
        name: "Grayscale ETHE_521",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8f728cA3561EDf7B015A0b972f8B6Fb04db89e2e"),
        name: "Grayscale ETHE_522",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("8feFB2ac91c4184b79caC44095dED9cEE81d56c4"),
        name: "Grayscale ETHE_523",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("90006E8948045C49B6e6aA50897D8f4f9aD3b091"),
        name: "Grayscale ETHE_524",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("90875d559Db224225aC6d63507fa7b370b70Dd26"),
        name: "Grayscale ETHE_525",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("90d6747a7e78467DE08Beb30FDB8De8f9E4F5C27"),
        name: "Grayscale ETHE_526",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("90dEb2D65335e84b68f79a0706963C5aB9e26c86"),
        name: "Grayscale ETHE_527",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("90eD6b0Cc2AE881A278d55C4831C4d4e708aF586"),
        name: "Grayscale ETHE_528",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("910ca28755b776a7acD02503B3B405F7A48874Ba"),
        name: "Grayscale ETHE_529",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0b94bB7b93E0363Abb06fa8011105C56Ec0AF99c"),
        name: "Grayscale ETHE_53",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("912B06feC618bDc57952839E0f5dc535f051c8E2"),
        name: "Grayscale ETHE_530",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("922a33e294fC10c2bf893AF46706257d30165849"),
        name: "Grayscale ETHE_531",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9239D6A502F305a19a106847f31aE19b6E16B723"),
        name: "Grayscale ETHE_532",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("92462C26Ac4033E896CdeE397E6195cF6653765f"),
        name: "Grayscale ETHE_533",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("92469eD213817dCeB000A2454D2E3D9Eee7C47e2"),
        name: "Grayscale ETHE_534",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("92c7bF3e1546C3B1Ad83F87439Ae7F827AaAb1a8"),
        name: "Grayscale ETHE_535",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("933c0cF95e7Be11D06c3BafAD128C1993B1dF476"),
        name: "Grayscale ETHE_536",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9341078f6eF542aD70ab4E93BE78eeEf02fCF1e6"),
        name: "Grayscale ETHE_537",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("93a654a2813F3aD052929752124B51D68bB9fa8f"),
        name: "Grayscale ETHE_538",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("93E3cDd07B04d3045eec99AB14Ec1e556DfFC9e4"),
        name: "Grayscale ETHE_539",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0b96f4Ea295dE96D7bCf8d29884E62cD0D88522b"),
        name: "Grayscale ETHE_54",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("940D49bEb1E89ddaEFf286e22c7cBed39190cb24"),
        name: "Grayscale ETHE_540",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("943fF4d28614365bcb2A79a58F032F123f37503D"),
        name: "Grayscale ETHE_541",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("95631d64b8825cEC13956b7Afeb8D797f103C372"),
        name: "Grayscale ETHE_542",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("95690beAdCf5Ba9A93F61FD8678785e4948144DF"),
        name: "Grayscale ETHE_543",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("95Bfd1BA0404e1f382448D0D5370B3dD775dB364"),
        name: "Grayscale ETHE_544",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("95cAE6eC900D019265127f03C941BbA668038834"),
        name: "Grayscale ETHE_545",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("961761B37606e9134EbCD14B56d82143377de52F"),
        name: "Grayscale ETHE_546",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("96650ac26678d60c4c299c65017d52D95E96B166"),
        name: "Grayscale ETHE_547",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("96956669C2e9691584e328395c726d9f73730917"),
        name: "Grayscale ETHE_548",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("96DD0d4E10b1e8FaE64771EC14793f716B12E889"),
        name: "Grayscale ETHE_549",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0BecaA21eb09551cA77C90E0E5D7049bf0D96cb0"),
        name: "Grayscale ETHE_55",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("971C785796FedB5Aa2bAe0395D055eFDb8A38058"),
        name: "Grayscale ETHE_550",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("97902B5A8212f11C0bAE8f31665620D05bFdabB1"),
        name: "Grayscale ETHE_551",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("97dD985D6F73AF02Cd0220b5D855CfD4C5A7a067"),
        name: "Grayscale ETHE_552",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("97E935DDf3f5A2C4291E38f106194Bb507DD28f9"),
        name: "Grayscale ETHE_553",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9850a63DA6715A79c08a07E506C5115362128397"),
        name: "Grayscale ETHE_554",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("98692A67c5B8eacce4dfc55c8aaD3879104f6D2f"),
        name: "Grayscale ETHE_555",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9886fb3f45819087fE1ec2173fe308692948Ca55"),
        name: "Grayscale ETHE_556",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("98bed4c85FA10FA1cEe0fB4d7406C6cA8e93b6fA"),
        name: "Grayscale ETHE_557",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("98C879587733db42b32A8f4EB2Db213A28ff3623"),
        name: "Grayscale ETHE_558",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("98Ea569De7C4949c588BF2447DC6b89Cb47ED32A"),
        name: "Grayscale ETHE_559",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0d1ec2169de51e05cFEb9F7bC6e303cb7B55a585"),
        name: "Grayscale ETHE_56",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("992271B5015274cD7cc934860695F7599CD930f8"),
        name: "Grayscale ETHE_560",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("99532EF4c6DC41b32b9759b79780B7dE6D2eBfe0"),
        name: "Grayscale ETHE_561",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("995Ae367358d35E0787e3A2E509D1692DE982307"),
        name: "Grayscale ETHE_562",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("99f85310dA212afB643C934F37ED512e66bf6dE5"),
        name: "Grayscale ETHE_563",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9a345cA67Ee70248aCfb4A64843E7A1e9BeFee96"),
        name: "Grayscale ETHE_564",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9b46991F9AEC29Fb0e93a1f19055F081Bf9C4ed3"),
        name: "Grayscale ETHE_565",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9B6C31B83c2B4AC502443585037bC01d33955F5B"),
        name: "Grayscale ETHE_566",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9C7E1966808A95A3bDa73619f76c35faBc46fE6d"),
        name: "Grayscale ETHE_567",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9cC64AabAa3D3859742c7CB22B0CE4B9e4e77875"),
        name: "Grayscale ETHE_568",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9CDBd99E66966Ec0015F016a19D31c5159E1ec81"),
        name: "Grayscale ETHE_569",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0D2CD9F9573E803DC2C424b35c928297D00b2293"),
        name: "Grayscale ETHE_57",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9Cf6b66279cc4a0D490c26F4Df9E748248ACFCE2"),
        name: "Grayscale ETHE_570",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9d0588846a9616c3744c4A833859859fdb694ff4"),
        name: "Grayscale ETHE_571",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9D9Fb58780FDaDB1f2c211FC72e0Ba2E099811fb"),
        name: "Grayscale ETHE_572",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9DA3cB66538F09dFE14dAD5Bd89009D8CBA8Afbf"),
        name: "Grayscale ETHE_573",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9DBc36696640B023B372Fc6753758F2eb923eA95"),
        name: "Grayscale ETHE_574",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9dcA64C99cb71aB2eFde929bD732DF930b4E81a7"),
        name: "Grayscale ETHE_575",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9DdcD8501e1db8Dae8dcEe323b1537D99d5EF14C"),
        name: "Grayscale ETHE_576",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9e631E208b2B711F42fb43004897e28891Ec7c49"),
        name: "Grayscale ETHE_577",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9e68027f552079a19BFdcf39a0821Bad64020Ae7"),
        name: "Grayscale ETHE_578",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9F38da065041E797E45Fa1C38a04c256D70C5Cc1"),
        name: "Grayscale ETHE_579",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0DB341236151200Ea3D684576F56Ec2F5d23bfCa"),
        name: "Grayscale ETHE_58",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9f8aba64BeAcB170CEbfA0d1366d040051E51215"),
        name: "Grayscale ETHE_580",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("A032437337637eD85bEc0fE2E1EE7a63967B20F8"),
        name: "Grayscale ETHE_581",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a04D81f5c75cc159A72548CAeD8bB77192715bc8"),
        name: "Grayscale ETHE_582",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("A09ad0194Bd492C6F26B7b4Fa7C5a1B1e7D39B20"),
        name: "Grayscale ETHE_583",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a0D4175D637Ce2647C91486DaC78153c49F35C13"),
        name: "Grayscale ETHE_584",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a209ee65A27D7fA73269d959E13d1b44BbE29574"),
        name: "Grayscale ETHE_585",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("A2Ce66F2706c8eCc3B1b8A80BB050B156FFF9b08"),
        name: "Grayscale ETHE_586",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a309fDd59A0071Cb0cFB0965b75D8b30C8D8cAFc"),
        name: "Grayscale ETHE_587",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("A31Ddf9715132a2F78e3B52a7eE841327d7495C7"),
        name: "Grayscale ETHE_588",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a36D12f79169cb1e5677eB3815346CD1de20cd4e"),
        name: "Grayscale ETHE_589",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0Df04749e82E64cF8A7Ba69E1047e64649f841a9"),
        name: "Grayscale ETHE_59",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a43680f780C176216ec4d374Dd1EBE229EcB07aA"),
        name: "Grayscale ETHE_590",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("A43b3a7FC3be6fEa9c605F7cEDC73D4D8f928e4b"),
        name: "Grayscale ETHE_591",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("A4B09BA44C0B69e9b24fb367EF83fbe0BE98D9D9"),
        name: "Grayscale ETHE_592",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a4eb45a4bEe9dE616741919cc166d11C5612c113"),
        name: "Grayscale ETHE_593",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a56B62b30f7f48974dD6899c14a005a7A0203DEF"),
        name: "Grayscale ETHE_594",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a57D20A6C32DD62909fb53b682F39697d2b260Bb"),
        name: "Grayscale ETHE_595",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("A6694389FC8Fd1161AbEeBaB4171BC92BF57D15E"),
        name: "Grayscale ETHE_596",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a6CB61a30b1A2071D1d6f08cE26189b51415Cf9e"),
        name: "Grayscale ETHE_597",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a6DC7bC50EeD59fc7EB15089DFc7d163809D1bad"),
        name: "Grayscale ETHE_598",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("A6E4173EC39b9C7468a23e71a0Da11E6fB23e058"),
        name: "Grayscale ETHE_599",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("02465b4B1eFc21a8eAd4b8E7FD33ac01ba2E224A"),
        name: "Grayscale ETHE_6",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0F3132E5E01240306f9FB0D7e5c4DD2E2ddee224"),
        name: "Grayscale ETHE_60",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a6f3ab8B87793A68957A1C676fc1bdEDd5121481"),
        name: "Grayscale ETHE_600",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("A70D39965feCEA5b9ffaa945C7DC638B1aE5E205"),
        name: "Grayscale ETHE_601",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a7165B9ea2AB66A9b3A5A8A176d715C9A463fBD5"),
        name: "Grayscale ETHE_602",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a71d30BA23EE8CC1894E02ddD733034E1FCECe61"),
        name: "Grayscale ETHE_603",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("A7Df254f08884883f845fb44281e0a914700bd81"),
        name: "Grayscale ETHE_604",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a81B8a4f5809646e4A8d8d7558bbCA0c065fa0c3"),
        name: "Grayscale ETHE_605",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("A85B6DE24F64b0F95F2DEAB7AD897e68765391fa"),
        name: "Grayscale ETHE_606",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("a993C1526Ad7b0d3BBF721d353d509EDAe948A73"),
        name: "Grayscale ETHE_607",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("aA0e58aDb8d1ea1964D0b65c9e83EA84C80A3459"),
        name: "Grayscale ETHE_608",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Aa158387E6281AE605586D0ad26546Ab67eCE10a"),
        name: "Grayscale ETHE_609",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("0fdE25AB1C9d7731CEb303a3f24B2f4df176F977"),
        name: "Grayscale ETHE_61",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("AAe65937bD6C9f5eEE350e10C3f0c1e189488488"),
        name: "Grayscale ETHE_610",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("AB552f41228C5375Ba6b5B791E6d1cb03827c04e"),
        name: "Grayscale ETHE_611",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("aB6a364A21F1d4EbE58bcAe301bcF892b45E36c2"),
        name: "Grayscale ETHE_612",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ab6D4480014bB6968671e9Da152e512fF8C51e8f"),
        name: "Grayscale ETHE_613",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("aB6FE7E0d49316c9e93538AA223599Db06911145"),
        name: "Grayscale ETHE_614",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("AC4E05dD5E7Cf0d68a70E575aA5c58feFd424145"),
        name: "Grayscale ETHE_615",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ac4fb4951A82a7220f803768aA0bf7f7AE588289"),
        name: "Grayscale ETHE_616",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("AD44Aaa5f232b7AC4eC5B9c6B853af1d3B360C90"),
        name: "Grayscale ETHE_617",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("adCece4B9ad1fb062C1F115A9Ab2A50cBF98D96f"),
        name: "Grayscale ETHE_618",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Af2AA93D0cab26c490419BC2E7DAE96cf4e278A7"),
        name: "Grayscale ETHE_619",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("104b1e8e4fDC032e258a655616C481e2207b5474"),
        name: "Grayscale ETHE_62",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("af64B119b0031611B8d8e1b824a11B9CDa36aa4D"),
        name: "Grayscale ETHE_620",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("AfDF3CCA2d9C02E6c9869bBCc687B249b930253D"),
        name: "Grayscale ETHE_621",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B08A93741A2F4f3897F2d9Db6Caf0F6212c7A5D4"),
        name: "Grayscale ETHE_622",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B0d9d83304152EAf4179109dFEe0Cd88fE4A9ef3"),
        name: "Grayscale ETHE_623",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b15eF72F94174c978cBB03E6A1642c6d583B0105"),
        name: "Grayscale ETHE_624",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B15FcB3D829D8dca8B96cA3217B7DB0e8a8A6c8A"),
        name: "Grayscale ETHE_625",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B1744815CF2decb53d4A38080A5Eb5FE9cC9E77C"),
        name: "Grayscale ETHE_626",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b1a01A43418904D67BD8e951373cd25553678459"),
        name: "Grayscale ETHE_627",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b1cAc0Fa8F99fc6fce1827A110112f51D3370730"),
        name: "Grayscale ETHE_628",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B222C09153F04b51bC968B36F35628F3d4841397"),
        name: "Grayscale ETHE_629",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1078876A403922813e4f4ADD2889A5b6e4132fCe"),
        name: "Grayscale ETHE_63",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b2BdF5b0039Ce3F4fe630bcA64717C754797f53D"),
        name: "Grayscale ETHE_630",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B39B01537a2cb3A32677C6465892f1c6637A1ddB"),
        name: "Grayscale ETHE_631",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b4192c96F5597AA59FfEbB7fe2534224C3cA2dB7"),
        name: "Grayscale ETHE_632",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B427C740992A643717Ab2E861e92255c3222587a"),
        name: "Grayscale ETHE_633",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B450652bBF2fFf454cd449C1Aa4F470cC9f35A9d"),
        name: "Grayscale ETHE_634",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B463aAbF19527a7908C0B11C3D39191BdbF70E32"),
        name: "Grayscale ETHE_635",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B4844CBDb1338076b0683606d3de729eA4F7514A"),
        name: "Grayscale ETHE_636",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b48B6674b674a79e8A6D77eD7A273F551221Fa97"),
        name: "Grayscale ETHE_637",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B5227E59ea073DfEdac2289173379241d8de8B1C"),
        name: "Grayscale ETHE_638",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B5761AA4D4f88EEAB67A90955a15017181975565"),
        name: "Grayscale ETHE_639",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1130039C36e63079Ac232bdcCa5011F1abef0763"),
        name: "Grayscale ETHE_64",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B5C4D5016c9f03E9F90f04D7e815DBE47deC5486"),
        name: "Grayscale ETHE_640",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B5EB74CC4Dc944F0dA9566fa44b7412eC312F3fE"),
        name: "Grayscale ETHE_641",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b670243723fe354b2F8782Cfb9ECFFaf95c0F90A"),
        name: "Grayscale ETHE_642",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B68b631E3D734A58bF3a8FcE36886664c7Dc99Ab"),
        name: "Grayscale ETHE_643",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b6A5351E8ed77181d585E97FcCd56FcBaAd9f5B6"),
        name: "Grayscale ETHE_644",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b6c7C2DbB376187f9cF6c1EA19A3C0fcF4428495"),
        name: "Grayscale ETHE_645",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B702775182932110b614Df8dd75c5A0c36DA0274"),
        name: "Grayscale ETHE_646",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b70DA0395f0eDa1D32a739163FeF9dC0E93CE187"),
        name: "Grayscale ETHE_647",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b73C73a993E8a827859fF403aFb4B861Aa4431CD"),
        name: "Grayscale ETHE_648",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B745C5b044843179ea0f3252851bB23eb328710D"),
        name: "Grayscale ETHE_649",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("11fB06E3ed9a12eBbB1345326448aA01D78731D6"),
        name: "Grayscale ETHE_65",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B7D8D7E5F9098e16c2eE15649Fbb95bdfBDEB22D"),
        name: "Grayscale ETHE_650",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b84EBE9526C0F91cB82b99acb1703791078B8028"),
        name: "Grayscale ETHE_651",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B87c8b82Aa9F48E9F4F85C826028901AE69b775E"),
        name: "Grayscale ETHE_652",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B88aF39DA312323Bc3bA10c445BA1aFCBC9397b3"),
        name: "Grayscale ETHE_653",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b977d3Fa2C2f878e4e230b27CBdc35c7a5c0ca5D"),
        name: "Grayscale ETHE_654",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B9786DC15B3E2E4994328834c801266479900328"),
        name: "Grayscale ETHE_655",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b9abCb0CF26A98C2e37823CE78742B6E5A1dcAD8"),
        name: "Grayscale ETHE_656",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("B9bCC7214568633611e91Db91b80EF72a02f8777"),
        name: "Grayscale ETHE_657",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b9f1A414cd3E820968Fd51840B26cC149a4f24Bd"),
        name: "Grayscale ETHE_658",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("b9f69EC82415F366879f97B840E805e8Dfdce3d0"),
        name: "Grayscale ETHE_659",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("124A443D6A09c1Cf5a5238AF6a49E2D03a7D5DaD"),
        name: "Grayscale ETHE_66",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("BA141724e6C5F43CfBf050e0694FE3065F976AAB"),
        name: "Grayscale ETHE_660",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("ba4bd83131fcB7D0B30cA9E9C3742200958635A4"),
        name: "Grayscale ETHE_661",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("BA7886228ADDf36F8e9d477d0bA61366D6e4DDA6"),
        name: "Grayscale ETHE_662",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ba7cf8f7328863d4d429938cE4dab34a2D5b6982"),
        name: "Grayscale ETHE_663",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ba9603360DC9Cc17C94FF72De4914ff6ff450269"),
        name: "Grayscale ETHE_664",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("BAc101A6C81533E718a285a02280d4952DDD5B06"),
        name: "Grayscale ETHE_665",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("baf3d4E644DDD847c28C9c4aea6584448d56bFd6"),
        name: "Grayscale ETHE_666",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("BaF78734222642ec50B6F464ba1D4e41B2Af5b79"),
        name: "Grayscale ETHE_667",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("bB077EBe177c8f8259914f95aC1512842EEa2023"),
        name: "Grayscale ETHE_668",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("bb10b35480EDaA5DD648253F584F15f1185b6C76"),
        name: "Grayscale ETHE_669",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1271892c3724CfF1416dc14C2B4e27a368cFFeb4"),
        name: "Grayscale ETHE_67",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("BbaBC0c69055AC0FcDD92b6b168C04D532D71E0d"),
        name: "Grayscale ETHE_670",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("bBf63e51527F3177e19BaA859cB2eD69C031AEB7"),
        name: "Grayscale ETHE_671",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Bc67bE0A18cb9aa53b4cE821baF35C2F43eeBEd2"),
        name: "Grayscale ETHE_672",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("BCf28d8132f48Aec6e21019be77159Dda5886c96"),
        name: "Grayscale ETHE_673",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Bd0F47deAE5795A26521D1d014630Cb2B35a87e8"),
        name: "Grayscale ETHE_674",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("BDC6384EF4EAd1Ea9BF2A82390f2eDe46bAf1aeF"),
        name: "Grayscale ETHE_675",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("bE3d84dfaCaee119eF5685D82c06BFCdEE876dA2"),
        name: "Grayscale ETHE_676",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("bE9723cDeC785bfa1046A128f3D383b835294cE0"),
        name: "Grayscale ETHE_677",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("bEC4e43fc46e17bB8B543C150B7460b0753f20D9"),
        name: "Grayscale ETHE_678",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("bEFB5BaC4aeFA84C59BD08479aa1926f521E254E"),
        name: "Grayscale ETHE_679",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("12b42BCbf7D017e6Fc811896AC35546F7c00a3bd"),
        name: "Grayscale ETHE_68",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("bf044a0D4cFA289628a4450f4E36E4A5fBeA9d0c"),
        name: "Grayscale ETHE_680",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("bf13709a606f2b77fCe2D3219a586EBa12bD1e76"),
        name: "Grayscale ETHE_681",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Bf43Cbd9B7784A744d5705a2e8365ee96a3120Fc"),
        name: "Grayscale ETHE_682",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("bfAeDe6eB73A94B55860E29a71edC12Fa46891d5"),
        name: "Grayscale ETHE_683",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("bfead5796997751A1D201ad7553aA103FA641E95"),
        name: "Grayscale ETHE_684",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("bFF89978302266e5DFaFCF7A20b2a733721Aa09D"),
        name: "Grayscale ETHE_685",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C0a462d9e3552D16283Cb4f4d0EAdf40a94aA73f"),
        name: "Grayscale ETHE_686",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C0B137404E4ACd6D35840e1a367636ad141ed1A6"),
        name: "Grayscale ETHE_687",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C0ce047837d421bc026d0F43764AAD659E27336C"),
        name: "Grayscale ETHE_688",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c12386613Bcbf62Be78ee80077A721E8CBa3Bc12"),
        name: "Grayscale ETHE_689",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("12EEDD04b57FFB7c0d3DE58D1ea0d16995D5F748"),
        name: "Grayscale ETHE_69",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c19aEbC165869496B4Af8D86D2494dDc8927231a"),
        name: "Grayscale ETHE_690",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C1aE372f35adEDdC8B6F557206a11c725a0f38ef"),
        name: "Grayscale ETHE_691",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C203dC03Ff949B4E2111897850d8D481ae8fCE7E"),
        name: "Grayscale ETHE_692",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C259129b5270D6fedD4EFA7d8736f2F103d97F13"),
        name: "Grayscale ETHE_693",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c2b6AABFEC71Ff4a578b873EB2718a31bc4C4212"),
        name: "Grayscale ETHE_694",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c2bAC1cC7d684918b2eC09A3F7E77AF858aE9B29"),
        name: "Grayscale ETHE_695",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C384dd01cEFccCA8c0E4f3A26158e332c4cC3f34"),
        name: "Grayscale ETHE_696",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C39Af5291b2d9BAa38E49726832bc221BC9eb2DF"),
        name: "Grayscale ETHE_697",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c40464f629449E5B43a3772d4C0FE929Bade8ab9"),
        name: "Grayscale ETHE_698",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c40Cc765114393aEEaae82B74d021B61A0c11409"),
        name: "Grayscale ETHE_699",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("024685DdED24F244e610d6023c3c4cbfB2b0e32D"),
        name: "Grayscale ETHE_7",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("13424CE5cEA47d90Ef9b80d576DC4571C04345c7"),
        name: "Grayscale ETHE_70",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C438BC455ee34FD0469Ce709FA702DB1F0AF476A"),
        name: "Grayscale ETHE_700",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c439E34bF68d0bC4E116D8FE1bAeE89EefBD42D2"),
        name: "Grayscale ETHE_701",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C451f2F0A9348A6bC1C483aD0Ca46D7BC872d628"),
        name: "Grayscale ETHE_702",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c4a99353382616531ED7CEA9cfAFAF7b816F1AF7"),
        name: "Grayscale ETHE_703",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c515c281996118C3eb7e9985D079280c8347c8fe"),
        name: "Grayscale ETHE_704",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C53217a9C7eadbBF52b1515B9eB8f6D1B6FB6860"),
        name: "Grayscale ETHE_705",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C5f33B0751cbCCD6ea28a31A239b9DAb09dE8a1b"),
        name: "Grayscale ETHE_706",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c606c9c0F138A69b7b80491902f3d456e431ca04"),
        name: "Grayscale ETHE_707",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C61Fe4124F587A545B8DEe0072B4289b37e9F478"),
        name: "Grayscale ETHE_708",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C646a017997B450337423EA3b39e4482BBcb57e4"),
        name: "Grayscale ETHE_709",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("134AC9752134362C82256981eb1c4dFfCcd0DF2e"),
        name: "Grayscale ETHE_71",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c685BAD74E7b3cad4eBa1Be9403F7e115295bd42"),
        name: "Grayscale ETHE_710",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c6911AA6C584B82aAa775a9DE9BC3054B4f2ba42"),
        name: "Grayscale ETHE_711",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C698BbD66a41adBce31F514DBFE278B7c43E4bB4"),
        name: "Grayscale ETHE_712",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c71a3d3A5a1eCc158384b59ad5e971D2C0a9Dbcd"),
        name: "Grayscale ETHE_713",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C72687db237D6055A9C013b09d822B563ec8691A"),
        name: "Grayscale ETHE_714",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c741d00DE2479C7216C33682E622C15Be0d94f8c"),
        name: "Grayscale ETHE_715",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c7a7fd11c1575d5A965e4Ebe670481d8B58f27C0"),
        name: "Grayscale ETHE_716",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c7b2b478AcaA7aCebD4245a9A824Af21221cD3F0"),
        name: "Grayscale ETHE_717",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C8395C066a8457D646D991f31ad6d8951d7162B4"),
        name: "Grayscale ETHE_718",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c8CcFBf8BE0164ef82743892d203e89dA86eC5bE"),
        name: "Grayscale ETHE_719",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("139057617F9f88C82647E09424bfC0c745782Bef"),
        name: "Grayscale ETHE_72",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C8D56677d41F87aD7c4DC86023edB6C02dc7a29C"),
        name: "Grayscale ETHE_720",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C8F7A3F4791CA446CbBd4cB75f03299FE0602f99"),
        name: "Grayscale ETHE_721",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C92FCa40dAbcdEC6ba56932d87540B68A34FdFf9"),
        name: "Grayscale ETHE_722",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C958025075f9892360e9FbA962fc864766dAD1BC"),
        name: "Grayscale ETHE_723",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c9d83679aDFE23D07e2cc91C5946861f1f24b346"),
        name: "Grayscale ETHE_724",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("c9FE147123bB185a8d37536294D69f1c403F9894"),
        name: "Grayscale ETHE_725",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ca19401610E2222f19Cb9D47E53e9e29535e87E3"),
        name: "Grayscale ETHE_726",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ca2C2B40c555a74CD444C1250Cc4a2dAd6CFBd6a"),
        name: "Grayscale ETHE_727",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("ca2DD8f07407EE31201E03876F65F1f0e69eF103"),
        name: "Grayscale ETHE_728",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("CA2E89110644D61361e188e322115267142eA3E9"),
        name: "Grayscale ETHE_729",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("13C1FEBd46072f874fA54616b48fd1b1c2E67d56"),
        name: "Grayscale ETHE_73",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cA86E7ea0cebcCF00f89B4eC1FcB70655f528A81"),
        name: "Grayscale ETHE_730",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ca95b7F44c35DBedf586Ae05f165779eeA23Db36"),
        name: "Grayscale ETHE_731",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cafe0a8846dF104Da9b760e484A98f63447971Eb"),
        name: "Grayscale ETHE_732",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("CB54c301ab3fF4aa1bc38C979427360F47369633"),
        name: "Grayscale ETHE_733",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cc2C15A7fEC4958c71a5555807dFf588D8517DD8"),
        name: "Grayscale ETHE_734",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Cc3B4Ca51cA1A372c560136fbA16DD7D32A0967b"),
        name: "Grayscale ETHE_735",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cc87FF23c187A9b320D52D284C7A105f781e4B57"),
        name: "Grayscale ETHE_736",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cC8e4606Db0983Ca93b45c1b1436758EEcD2bD37"),
        name: "Grayscale ETHE_737",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cc979D2D40CCfaA353Fd2E892d3f1AAEfb6975CA"),
        name: "Grayscale ETHE_738",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cC98164e4B9C8EfdEB20C1A2625eF941cd02dC08"),
        name: "Grayscale ETHE_739",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("143221bD51dbA017bCd33D5b65B8C576B7797355"),
        name: "Grayscale ETHE_74",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("CCF4246e36E158258ED4Cb34FdBac6863DA7B526"),
        name: "Grayscale ETHE_740",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Cd10bF12FEC45Af1b2EA93337FDA543fA923f760"),
        name: "Grayscale ETHE_741",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cd2926E14c805d3391BF67223F2B7a2fa48BE175"),
        name: "Grayscale ETHE_742",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("CD93e382eEaBd806023183D2De564459053Bfe3e"),
        name: "Grayscale ETHE_743",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cDa68773011fF8d36CE165091be4959898b06f1c"),
        name: "Grayscale ETHE_744",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("CE4F9695EE844E079Cb102Ff965F6f18Dd5b37B7"),
        name: "Grayscale ETHE_745",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cE6cA7Fc8eceB2D26A874F2f7CE54B21C0C2e59A"),
        name: "Grayscale ETHE_746",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ce79A744E97B379422FED0D1D9bCDE6dBE01e304"),
        name: "Grayscale ETHE_747",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cEB7d894b3C7B1d5327fC41BC18dcB2B4fa2e764"),
        name: "Grayscale ETHE_748",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cec19c7a3Cf460162eC33918c9256436192C5a88"),
        name: "Grayscale ETHE_749",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("146871534fbc39Fd25328daeDD9225D6Ff0F2535"),
        name: "Grayscale ETHE_75",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("CECF531f239BF2d44FD3B0137cEfC1832E993d80"),
        name: "Grayscale ETHE_750",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("CF4a936F951D56fd38E457B7f3b2eb8f4f092727"),
        name: "Grayscale ETHE_751",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cF86E114D4f6706702266c35F25a9F1C9922d3b3"),
        name: "Grayscale ETHE_752",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("CFb5dAf7f7B3cE2c4f8C22bCd1D7C764acA63B41"),
        name: "Grayscale ETHE_753",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("CFc15DDeE4F921A0917D35Bb70174E14f6D45269"),
        name: "Grayscale ETHE_754",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cfce3Ab710e5EC5aD8434b7DAa50E4EbCAD44299"),
        name: "Grayscale ETHE_755",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d05fB26F19C64D0fb0942bA2939e1b5977b4177f"),
        name: "Grayscale ETHE_756",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d07e0c45e63d638aDFe2725C54206895bdBADd14"),
        name: "Grayscale ETHE_757",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D0D50bd6933668D962F8b601b155a5A6a2D2d178"),
        name: "Grayscale ETHE_758",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D1A150ccba297f1E160442aC3DEC8849f8eA5Afc"),
        name: "Grayscale ETHE_759",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("14C728C9aEeAfCe01f1A7b87d02255dD4326f180"),
        name: "Grayscale ETHE_76",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D24f91b642699Bf73FbAe191F3fe3748a2b2e70c"),
        name: "Grayscale ETHE_760",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d2f7164935435E5423193c3d10337A2CdcfA154D"),
        name: "Grayscale ETHE_761",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d3171f648d8972c6CDF364446fD100F863E9e6d6"),
        name: "Grayscale ETHE_762",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D324D5634a6bf87BaA7b25027fbC6101ACDfFA87"),
        name: "Grayscale ETHE_763",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d32c52849aD7241306546113b2073310aD42322E"),
        name: "Grayscale ETHE_764",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D3317f334dB5106feb8d5E13433D4C6AB906A304"),
        name: "Grayscale ETHE_765",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d33fCB93542B761c92f1a0f33E5cEE0ba9655C86"),
        name: "Grayscale ETHE_766",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D35297EdF178d2fa55374F578C381ec379217c86"),
        name: "Grayscale ETHE_767",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D367eE265D151b3A06b51fa21d73671C1123da0a"),
        name: "Grayscale ETHE_768",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d3738b7c4Fc14FdF4D79a7563A71D17BBd2d6326"),
        name: "Grayscale ETHE_769",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("151F202173147bC2c27B92E2341474E08E169714"),
        name: "Grayscale ETHE_77",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D382fD5c8A47866C79b296E3914c7c8AebA40994"),
        name: "Grayscale ETHE_770",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d404316d5BB0641853edaAE62A92CCb6F2d820bc"),
        name: "Grayscale ETHE_771",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d42a5BA156f6F2747652740620ed642B0Aa02fe9"),
        name: "Grayscale ETHE_772",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d49Df8e16AbcAc8846A7e23431320EbeBD80fE0a"),
        name: "Grayscale ETHE_773",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d4bE641AE3e926cf5F754bB3ac18264Cc65086DE"),
        name: "Grayscale ETHE_774",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d510Be34915EF01A74c2Cf86B1A8D7ae47243784"),
        name: "Grayscale ETHE_775",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d53000273d5eb404c8B5A57Ac7985648768a1384"),
        name: "Grayscale ETHE_776",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D5c150b0C0983b5396889eBA8489C7Fc642dbEAd"),
        name: "Grayscale ETHE_777",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d5F1c56e71b89fEe48dFCb6872E1eA422723eCB1"),
        name: "Grayscale ETHE_778",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d68fB33E14bc9bFd95854226262148C85C32dB1d"),
        name: "Grayscale ETHE_779",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1531B0467896ADe3b46A111999da55A83922C7FC"),
        name: "Grayscale ETHE_78",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d6d07c8f0308832551a17D0B6bC049949DD1504a"),
        name: "Grayscale ETHE_780",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D6e1326914d5c332FfAA7a3F7bBa4f8f60AcaACD"),
        name: "Grayscale ETHE_781",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D6ee647f7990E308cB99a767848d7F49cc4629cf"),
        name: "Grayscale ETHE_782",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D737d3Ba9A8735fC050a7c48fb3C10a12d9DE14F"),
        name: "Grayscale ETHE_783",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D7eFd7a45B2affaAb9DEe3713321eEB1e0a9DFBB"),
        name: "Grayscale ETHE_784",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d81E55288737eC99263dAd3a26f6308E39c01CA6"),
        name: "Grayscale ETHE_785",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D83754BDf786687e5e0Cfcc0CB88fadAd764A8b4"),
        name: "Grayscale ETHE_786",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d85Cb52760558dF986E6F594d5d8059bA439556E"),
        name: "Grayscale ETHE_787",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D883fF5eD913dbC5ec523dD11E1D607879373Eb4"),
        name: "Grayscale ETHE_788",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d88f7c2748C3C07C1a58167a2c26dD3fE4F8ffe9"),
        name: "Grayscale ETHE_789",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("15551907810F1f1bEFD97f35bfFC41dC348b433a"),
        name: "Grayscale ETHE_79",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D91BA9fA219Cc9bF05624A472dE472Fe526E74F4"),
        name: "Grayscale ETHE_790",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d91E7cc8C48411f81972c1503a1600ea4e627410"),
        name: "Grayscale ETHE_791",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D939002bC74aF734649555E10654B5B5f5A13FEC"),
        name: "Grayscale ETHE_792",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("d9d99bc4A95ceB64F5B9Dc709d790dd5C4c7516f"),
        name: "Grayscale ETHE_793",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("dA3E74e10f6c789443366A06537c29f7df2105fb"),
        name: "Grayscale ETHE_794",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("DA5967A3E6E4f67E00fd5A75D67FD58CbEFCde9e"),
        name: "Grayscale ETHE_795",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("dA6Faeb52659DB77dfFc80A1cF3980f4CdcC5bD7"),
        name: "Grayscale ETHE_796",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("DBf9B5f4b097f992b18B8203a62d9F3e74997d22"),
        name: "Grayscale ETHE_797",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("dc990C399128275f0C16Eb57b6f13cB28e98297A"),
        name: "Grayscale ETHE_798",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("dcF0e258c3A627d61e9Ec947bdd145661B4d11D7"),
        name: "Grayscale ETHE_799",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("028ad651f143158581Fccb9793B08A58246dA693"),
        name: "Grayscale ETHE_8",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("158d733EFd96495AE265b9C85DDc7B4d4966eAcd"),
        name: "Grayscale ETHE_80",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("DCf57A5b6a8a41231858b242C163a77D1579EdEC"),
        name: "Grayscale ETHE_800",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("DD0611037364EAa4621cC9576843fAdE310C49E1"),
        name: "Grayscale ETHE_801",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("dd3aE84880e8D07f0151fC9010360007Bf93cD3b"),
        name: "Grayscale ETHE_802",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("DD460BFc5274243D92FeA89d19e3f1afE1476c25"),
        name: "Grayscale ETHE_803",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("dD504D37A1c420bA148202500515Cccb8360c7B7"),
        name: "Grayscale ETHE_804",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("dd524A2a0d6914Cb2c04C2A16bf8716aCa51312B"),
        name: "Grayscale ETHE_805",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("DD6eaf823131dF00749a0694C46EC51D8346E94e"),
        name: "Grayscale ETHE_806",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("DdD3eE8ADc6fbA23b5323E667a5a817d33beD8B7"),
        name: "Grayscale ETHE_807",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("DDDf06375DaF0546DE4f7529d3BfBB12804A88c5"),
        name: "Grayscale ETHE_808",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("dDEb7F30944B4EFB72576100fE4aC493e97e3af7"),
        name: "Grayscale ETHE_809",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("15ACA85DbAF8E2b98822A269156aD1D1459F499E"),
        name: "Grayscale ETHE_81",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("DE2aFfA03d4020A44336685E893d82eB8F59BE2e"),
        name: "Grayscale ETHE_810",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("DE34b24bC0f0Ebb4fbB4617de26d9b94f88eCF2d"),
        name: "Grayscale ETHE_811",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("de8CbF72b73fe2409CC107970f4D9Ee189efEe2D"),
        name: "Grayscale ETHE_812",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("deB294797D166D580675Dc5aAEF1A25378Ccb626"),
        name: "Grayscale ETHE_813",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("DEb9Cc22cd17136CCE26f9341b81d2A5c83beCA5"),
        name: "Grayscale ETHE_814",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("DEEeCdB8bD3B451d854459eAb41116Faa81b2fcC"),
        name: "Grayscale ETHE_815",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Df4c374a2a761d2fd6d074fCcBc76967C92a6dBE"),
        name: "Grayscale ETHE_816",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("dF70039aEFcF66e94378f306f97C0f3765891E12"),
        name: "Grayscale ETHE_817",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("DFc3653588183F0d5C79a42776830106486B8a34"),
        name: "Grayscale ETHE_818",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("dfF48a525411d37355d7292C2F5706055507e601"),
        name: "Grayscale ETHE_819",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("15aDE2264508cd4FE63698665a8823a77346BB26"),
        name: "Grayscale ETHE_82",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E06e9f3bDED52930107B34bD326f89e00d68CDD5"),
        name: "Grayscale ETHE_820",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E1024372E8335Fc3601eb6996cf3dae92eDbb904"),
        name: "Grayscale ETHE_821",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e156e01502fDa3A467E713F3B49Ce72c726d06AF"),
        name: "Grayscale ETHE_822",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E16913333978a0f756cAEdD1294307922B760CC5"),
        name: "Grayscale ETHE_823",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E1A9C2f6FC229f6e7094c49DF210761d90ACCd51"),
        name: "Grayscale ETHE_824",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E2467DBd5ef32a03f4650Af1DD665e734CaB58Ae"),
        name: "Grayscale ETHE_825",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E263aE2285b1e697A9e5C76fEa6657693F7b6d49"),
        name: "Grayscale ETHE_826",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e28498437F2818355A9C1bff93Cb3DD590Ff4e2a"),
        name: "Grayscale ETHE_827",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e298Bdc568F348fc7F0AE26f7C6Cd2039359e0B5"),
        name: "Grayscale ETHE_828",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E2bb4c6272D87A7F7E138C8B92A7D0a66fD47BCE"),
        name: "Grayscale ETHE_829",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("16c59455eD84328e787E1b84f04Dc56063AAAEc0"),
        name: "Grayscale ETHE_83",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E2D3E2f61be5Bd2Ff65d931B94B0cf25aC63cE49"),
        name: "Grayscale ETHE_830",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E2defbbD57b9a81ff9dEE266e45492348FC8B2f8"),
        name: "Grayscale ETHE_831",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E355f1d3bAC35fcA6789570dfeA7648ac6a403d5"),
        name: "Grayscale ETHE_832",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e39152e4D1C9FF4F7Ae1E25dd7f0b8283999Bf19"),
        name: "Grayscale ETHE_833",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e4021CE8204fB271C09652f1842fcc5ADD47a9CB"),
        name: "Grayscale ETHE_834",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e467cDaB8ed6d14BaF2742352891FB16A9f76736"),
        name: "Grayscale ETHE_835",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E48B9FB8A44A6eaD33EEC616f98b922bfC8bc270"),
        name: "Grayscale ETHE_836",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e53E7FEe9D6BB139997aCEAc12aA5515768448F4"),
        name: "Grayscale ETHE_837",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E53F97a19f06c10De79862b66026e3aCDfF5607e"),
        name: "Grayscale ETHE_838",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E561CE56084bD11c0033B00209E8A4064B9a0159"),
        name: "Grayscale ETHE_839",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("16E73D37Ff1FEaCED4baC01ebeF8879300Ea2b65"),
        name: "Grayscale ETHE_84",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E693AA3eB806A1D67dA66CA42DfF61010347D956"),
        name: "Grayscale ETHE_840",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e6c43632B0D657eBb7B9352BD18C7Ce79cE1221c"),
        name: "Grayscale ETHE_841",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E6c61D62411a5EDE0213646596B9264b5926d0AF"),
        name: "Grayscale ETHE_842",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e6dD0608F6f9bF589470A7Be991e25E3d76E62EE"),
        name: "Grayscale ETHE_843",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e6E90d59CB21F34a4268c9e18897c5bafb692c38"),
        name: "Grayscale ETHE_844",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E71cB75836f7F91276a1480C7E5ACf8378781bCb"),
        name: "Grayscale ETHE_845",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E71e91c4C151F8421033Ec2627D86fB749C58981"),
        name: "Grayscale ETHE_846",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e7948d408cb475ECb1a09999eE227a69EeD1F566"),
        name: "Grayscale ETHE_847",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E79a28F0089f94977251aaD279c834bd31DeaF30"),
        name: "Grayscale ETHE_848",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E7DF07f59156209ACb4e752a9f6f11844f93b722"),
        name: "Grayscale ETHE_849",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1714c52a6a6Dd686e8757CC28AAC150Ab746699E"),
        name: "Grayscale ETHE_85",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e7Ed02024a70680432bf4Bb2Aa9D2F5CA69B7C7b"),
        name: "Grayscale ETHE_850",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e80F2ddDcA556F9c9986a6F1E4F811B91948ae5b"),
        name: "Grayscale ETHE_851",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e8494fcB661EF9eDe4C0EFF5b49972D5eE48BB43"),
        name: "Grayscale ETHE_852",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e849ad35dA9560A67CeC7236dfa3C3246b939396"),
        name: "Grayscale ETHE_853",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("E86838c5CCEefFD2A05bfB0B58aC87B153efF140"),
        name: "Grayscale ETHE_854",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e88b2F250E5719D015d40a5fD9b636DECa4E6180"),
        name: "Grayscale ETHE_855",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("e8Dc22F57BC0Be62b76F21d1d41Bc0b53e9bde64"),
        name: "Grayscale ETHE_856",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("ea039cf1857Bd0e14919c4EF1A8B332A83110BFF"),
        name: "Grayscale ETHE_857",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ea28D8a2711f39D9F10B50878177360791500Cb6"),
        name: "Grayscale ETHE_858",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ea481D5f0015d67F32854E45fe8F3af7b54c5575"),
        name: "Grayscale ETHE_859",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("174A07844d29CA07638C6c5bEa2E88d3cb011c4e"),
        name: "Grayscale ETHE_86",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("eb28e97A5Af30c62D55d8A81B193408d01FDD9ea"),
        name: "Grayscale ETHE_860",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("eb3d5ddcdf38b898879c8A76A2e8cC3FD12a52Dd"),
        name: "Grayscale ETHE_861",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("EBBBE02E2b41C17870152a27E8BEE518211854d9"),
        name: "Grayscale ETHE_862",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("ebc89c80e20cE10194ff7f4B25124824CDBD5056"),
        name: "Grayscale ETHE_863",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("ebDa0F76B6D941f2339a4A5B502aA8820B99DB34"),
        name: "Grayscale ETHE_864",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("ec50949B71aABbD8A883eE12143F3cA9D0688870"),
        name: "Grayscale ETHE_865",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("eD058B444dFEc7aDabDEEEd0fE7f2E67d8dFf478"),
        name: "Grayscale ETHE_866",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("ede60080bD1407a65823C1001fd56aA9a3a3c4c4"),
        name: "Grayscale ETHE_867",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ee141c878A47B9d8e136d03343dfeEF02985661A"),
        name: "Grayscale ETHE_868",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ee675f59bA9A34Ca382A0067d50998028a1e1293"),
        name: "Grayscale ETHE_869",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1775642e42576c68A8919E4e35C05cAF8c94adEE"),
        name: "Grayscale ETHE_87",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("eE731cF1b54A0E21aB0bD8465e3e396C1C2BcA40"),
        name: "Grayscale ETHE_870",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("EeB4c136D37dEF3050DC41d0021a82F922E1f7E9"),
        name: "Grayscale ETHE_871",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("eeD94941e5b6d503ab6e5d43B9599BAbae798801"),
        name: "Grayscale ETHE_872",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("EF143D0ce31268676b4962E940BDe1B24fFE3BDD"),
        name: "Grayscale ETHE_873",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ef24B7471613d9E74b0CA53D15f65e1a12738643"),
        name: "Grayscale ETHE_874",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("ef50cfeAD7e67d49053c1698C27C9a3b0eB5A24A"),
        name: "Grayscale ETHE_875",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("ef6C7e7409007b3a88d246F4af6aBA7D4264E336"),
        name: "Grayscale ETHE_876",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("eF89827ec0e4Cce06fDD89E2d7252Bb4cEe7A1A9"),
        name: "Grayscale ETHE_877",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f0282099d251b292Fa64DfBc0d5fbDAcab91d9B7"),
        name: "Grayscale ETHE_878",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f0297345C515d7BCE09C7ec29E3454dB91E936Da"),
        name: "Grayscale ETHE_879",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("177D756d34D1763962Cb61446cfC427Ff3E6ee44"),
        name: "Grayscale ETHE_88",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f030AB48A11787DD08222565864271e6Cf206C84"),
        name: "Grayscale ETHE_880",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f0B9DB9A9EED7B55B2e96Aa512F5D99527Af687e"),
        name: "Grayscale ETHE_881",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f1757AD7FB125701B52dD5514cf1F5edD1Db199D"),
        name: "Grayscale ETHE_882",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f1DCFa0837faA2a0ceD8849cB3bF312e163Ce412"),
        name: "Grayscale ETHE_883",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F1fD8009C90a7313B755924502F7AD08bb94DFDB"),
        name: "Grayscale ETHE_884",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F250b4ACd5855362DF708771DDF7280e8e4167b7"),
        name: "Grayscale ETHE_885",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f2573181DDE7d17AF446061e2bB4c8972E8D0171"),
        name: "Grayscale ETHE_886",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f260d8875a8261BA8c211b2857b7c69B4253A53E"),
        name: "Grayscale ETHE_887",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F28ad8F9Cb568D1B932573A76EAAd983A29904Ad"),
        name: "Grayscale ETHE_888",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f28eDd61b07f2B8874b8aBFE96F280ba77A42149"),
        name: "Grayscale ETHE_889",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("17872CBE1c30D707B40b0e5Ab87393B53649082A"),
        name: "Grayscale ETHE_89",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F3348D6994C1D25Ca95a79B996E378F8e4eD22aC"),
        name: "Grayscale ETHE_890",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F35547c48Adc629D61FeA127e419DcF149Bb54d9"),
        name: "Grayscale ETHE_891",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F35F3B032C8bEB33a2Ec88057630d9a269eEc735"),
        name: "Grayscale ETHE_892",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f38356f7c5e47926c0417bf2763b5C44a00336b7"),
        name: "Grayscale ETHE_893",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F39c4573fA757fbc94151776Caaa0AfeE45950D0"),
        name: "Grayscale ETHE_894",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f3C4c3793bDF2655A536Ee0b76eC4AC4B6541B88"),
        name: "Grayscale ETHE_895",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f3C966A09C119ddB5389bb0A2671236c1823a363"),
        name: "Grayscale ETHE_896",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F3De466383196dDc78771F7D112a78EA33D75Ff5"),
        name: "Grayscale ETHE_897",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f3F2184fA4F29FF2a1B53Ce2B0939871183F135c"),
        name: "Grayscale ETHE_898",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F404806324deC457a0C5a8fB49302C8707b48386"),
        name: "Grayscale ETHE_899",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("02EE13fFd5CF5E6731d4CeAB28394074Bb185F61"),
        name: "Grayscale ETHE_9",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("17b153aa3Abe80655B558E52E478F0B3968bFf16"),
        name: "Grayscale ETHE_90",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F47d9B586F9948c7B3fC533eDFB25bf3fBeA3aF8"),
        name: "Grayscale ETHE_900",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F49Bbaec56f80f938700cD07F214be8442957755"),
        name: "Grayscale ETHE_901",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f4A07548a9D9e79D8d8D56E4C6f9297E502d690A"),
        name: "Grayscale ETHE_902",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f4e4B05b504ffCdeddA1F07d07255890a92a643E"),
        name: "Grayscale ETHE_903",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F505b552eAbb108ecde8C96a558c12C4dc4A15FB"),
        name: "Grayscale ETHE_904",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f515c80c77aD5245A6bE51aB5C89526bBbaF9855"),
        name: "Grayscale ETHE_905",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F53ef57b0F32Ed0d151e4e6eeE8C66c919dbd202"),
        name: "Grayscale ETHE_906",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f5552Bc2Cdaf997DDB2d08962ADdEB5C27bABf62"),
        name: "Grayscale ETHE_907",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f5B4386394cB52999d43517bF89AcB4fe902Fa09"),
        name: "Grayscale ETHE_908",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F5e122B93A6e2cF2B5D09B525780eb71B2A1b3D0"),
        name: "Grayscale ETHE_909",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("17e8C689c143e373A9c595Fd02c8Dd641Db9A6D4"),
        name: "Grayscale ETHE_91",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F6e38aAC274EdbB4173643eA718d7002978E755a"),
        name: "Grayscale ETHE_910",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F72f8d7a0337EBdBa0FB85162d3a12e007b51F0D"),
        name: "Grayscale ETHE_911",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F732CCc723213d4bFC346bd346B51F150361cd79"),
        name: "Grayscale ETHE_912",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f77f5b8cAffb6Fdf9013809dF607b7aEd05767eA"),
        name: "Grayscale ETHE_913",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f7ECE7b64e6276924ffcd822ABA139f35afB99a6"),
        name: "Grayscale ETHE_914",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F820502be44549169a4bC136AfafD714d1d25708"),
        name: "Grayscale ETHE_915",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F8626e68A1f544E6fdA851fE089c4B8435223F37"),
        name: "Grayscale ETHE_916",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F8D1E3f35820c6BF262592dFaAe5DbEa24cA404A"),
        name: "Grayscale ETHE_917",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F939Ab194400Da7d12b5Def3f753b1423E6a45e3"),
        name: "Grayscale ETHE_918",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f9Ad8f6CDE350a9731ef7c77FC588Ce88Def56Db"),
        name: "Grayscale ETHE_919",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("180c0c920843a35dDb6BFEEA5dd6436F6Ba7CC61"),
        name: "Grayscale ETHE_92",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f9b993a7136733aD019d5B2497E90a0887fb1049"),
        name: "Grayscale ETHE_920",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f9BadC1EaFfBB01BB132872CFE928AD1121e5438"),
        name: "Grayscale ETHE_921",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("F9Ce8350D3A3E132E9Fd75660fD1F01541d31289"),
        name: "Grayscale ETHE_922",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("f9F160B50C4B54E9FE639C7438251ffff57749Bd"),
        name: "Grayscale ETHE_923",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Fa57EaA79213Ab1bE91F4C52e5a267a5Ca49b244"),
        name: "Grayscale ETHE_924",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("FA923a35AC776c1c4f807fA30E0fe62643F26134"),
        name: "Grayscale ETHE_925",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("FaF7B76670F4265d9d8D57022998c105898064e1"),
        name: "Grayscale ETHE_926",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("fb9cACD26D1C249757A7bFf78A514bca734BEE8B"),
        name: "Grayscale ETHE_927",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("fc0eeB73aE90BD6dB3CcCD036ca4dDfd32020118"),
        name: "Grayscale ETHE_928",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("FC48a70F1121196371020aA61B4f03327Be60301"),
        name: "Grayscale ETHE_929",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("18172CB661F0AC82E1acAAdD3c83Cb1c61736d9c"),
        name: "Grayscale ETHE_93",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("fCa5d74e9faC2aA672D305F5Eab40c0639C1ad51"),
        name: "Grayscale ETHE_930",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("fCaBf30a584ED10b4EB58877AE173452a5De12c9"),
        name: "Grayscale ETHE_931",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("FCB8e0FB3bb60Cb7E63Cb3203Aee3F039f48119A"),
        name: "Grayscale ETHE_932",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Fd0Bcb76f5Fce547c7e783C85205442891f4b743"),
        name: "Grayscale ETHE_933",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("FD3e8b8B6e4DDFdD25d0a79Bc99E7cdbF287199d"),
        name: "Grayscale ETHE_934",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("fD58F0BD175d6695C4faaEA04A5DB85A149bb479"),
        name: "Grayscale ETHE_935",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("fD71c87DA768207dB2B8532cab368AaD8CDE9Cc3"),
        name: "Grayscale ETHE_936",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("fdFb48A407d436530E2732dF52d39c9c63995e42"),
        name: "Grayscale ETHE_937",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("fE1448C64198126eAcbe3E27375a529327bE3D3A"),
        name: "Grayscale ETHE_938",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Fe96B5239D60fe339542c3f0f3d389b93710aDE5"),
        name: "Grayscale ETHE_939",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("188690650Ef51C16FE8959BE63Ddb4dA3c25d7C7"),
        name: "Grayscale ETHE_94",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("FEFaD2b50F3bFc0fbe899f3B6bA489EAF9E7B650"),
        name: "Grayscale ETHE_940",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("fF42E2A81A5aB429eEad7654344a9DEec215Dabf"),
        name: "Grayscale ETHE_941",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("Ff8A34f749B97e19f9821615731Be346E65aeDD3"),
        name: "Grayscale ETHE_942",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("fF9eE960D4a89d19762E6bCdaE96491cEAAE2b80"),
        name: "Grayscale ETHE_943",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("FFd39d27E7cdc53c1a9c74013E6E1C2dF1F27bF1"),
        name: "Grayscale ETHE_944",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("FfF4Fb977fC2F15e7C29527a9f398d6e287D3e03"),
        name: "Grayscale ETHE_945",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("FffAa9F1d590B2A7879279c312d3B4A5f0BF679A"),
        name: "Grayscale ETHE_946",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("18fC5f64d3a758c62Ce03c036705eA887154E550"),
        name: "Grayscale ETHE_95",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("19a1cC5301589Eb80a8D07c0f5475997BbEf1F80"),
        name: "Grayscale ETHE_96",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1A46Ca4Be161E5D4CAD714602a22C9b7FBbA7FB2"),
        name: "Grayscale ETHE_97",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1a8F8a8714f11c3A35B13A34A3eaE15A5f850f47"),
        name: "Grayscale ETHE_98",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1A9635A16A9b2435985399eB64b025Dd8052EC75"),
        name: "Grayscale ETHE_99",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("03058Fa830E90dE326009EB1b6B793B60076d7Dd"),
        name: "Grayscale Mini ETH",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("03211dAae0b65cf396945868D022bB77dE22Efa2"),
        name: "Grayscale Mini ETH_1",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3a2410E77A3Ea976C7DCC9880527762fCEEE6FF3"),
        name: "Grayscale Mini ETH_10",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("452962d1d9F7f0DEd2e73E79c859D0140181A9F7"),
        name: "Grayscale Mini ETH_11",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5B4ccB047b982Dc0Eba47c5cF35f80A1AAa25544"),
        name: "Grayscale Mini ETH_12",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5f07D515B5897af3BEDbE42d74d350A41508973e"),
        name: "Grayscale Mini ETH_13",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("5F2c071093F4F03C718240F2Ac5DF3222909aCb8"),
        name: "Grayscale Mini ETH_14",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("660B4f8b8Ac68fD6F6E2caeb19Bc5529d4c05Bd4"),
        name: "Grayscale Mini ETH_15",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("78c42af600F67483474B1FEc5681e9B6938B9b4B"),
        name: "Grayscale Mini ETH_16",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("88a709A920AeBC96706735C0b27B5460e2D61a1c"),
        name: "Grayscale Mini ETH_17",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9919098d320FA16EBe00E74aFEc41C054b3995e7"),
        name: "Grayscale Mini ETH_18",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("9d8B6e761FDead614Bd1FBBB9F03D558a526C676"),
        name: "Grayscale Mini ETH_19",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("037CA5ca8b5acFEb335B4aA389F08C325a62CD2d"),
        name: "Grayscale Mini ETH_2",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("ab3E9f1133c597F03768510E7e65004A04c9d427"),
        name: "Grayscale Mini ETH_20",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("BD46378faB9C17f50fD45C14980155dba9c814c5"),
        name: "Grayscale Mini ETH_21",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("BE8628F850838E8683E53734aED211C8Ad7be95b"),
        name: "Grayscale Mini ETH_22",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("BFD45E3030A9Cb9801954a7adFF074164A71605a"),
        name: "Grayscale Mini ETH_23",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C073f502e033185D211B2FD339706CE44E6F1054"),
        name: "Grayscale Mini ETH_24",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("C7E9079f03a07D93E101Cd4079B69283D9f47d29"),
        name: "Grayscale Mini ETH_25",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("CA8a031cDb5bbd4559d526a723093582F80aA3C2"),
        name: "Grayscale Mini ETH_26",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cACe62287B7794a8fa7B4dAF45f0D037434c54db"),
        name: "Grayscale Mini ETH_27",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("cf3dC64a2F99Cd77148F0485A91933dadc4FaB6c"),
        name: "Grayscale Mini ETH_28",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("D616e186C4DB1b46a47B5CBF368C331dD2fB709e"),
        name: "Grayscale Mini ETH_29",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("09F928cB05359507866b97451E71d55Fbdb43A3C"),
        name: "Grayscale Mini ETH_3",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("eaA76C2c18161a31487C6205Eb85671D87d7a0cC"),
        name: "Grayscale Mini ETH_30",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("1Ad1eb01Ac0409dDd3B3c17ce1D4C83F3236A3CB"),
        name: "Grayscale Mini ETH_4",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("21316bBeCD9Ac31dabBFF2A6a885Dd232807dfe8"),
        name: "Grayscale Mini ETH_5",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("29AA2a311c184DF32d1D73d30EB3b54FF31583C8"),
        name: "Grayscale Mini ETH_6",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("327eb8aa24e09BE3fB7aF5844657870D513687C5"),
        name: "Grayscale Mini ETH_7",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("3707487518d9485A98f44D7B4b678DaAdB8360Da"),
        name: "Grayscale Mini ETH_8",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("38eC364A13b6AF2bC3faA002D2A7Ad005743243d"),
        name: "Grayscale Mini ETH_9",
        provider: "Grayscale",
    },
    EtfAddress {
        address: address!("00c2c0fE37cA5B3b7A0BF0179AC5505AeDb6cfd2"),
        name: "Invesco QETH",
        provider: "Invesco",
    },
    EtfAddress {
        address: address!("400D68de9f7769C106d1108471d1d6C0CfF78548"),
        name: "Invesco QETH_1",
        provider: "Invesco",
    },
    EtfAddress {
        address: address!("aa16Ff2000885E9A2E725E84179BBB62427C6067"),
        name: "Invesco QETH_2",
        provider: "Invesco",
    },
    EtfAddress {
        address: address!("c47b4a69aA1B7689983420443011f112B064A727"),
        name: "Invesco QETH_3",
        provider: "Invesco",
    },
    EtfAddress {
        address: address!("57F0566BDca5e8094285DEfB817C4E598F6d51f2"),
        name: "VanEck ETHV",
        provider: "VanEck",
    },
    EtfAddress {
        address: address!("AD10A0Ec7A7FdD54B9d13fa8e2Ee1d5f4E94627A"),
        name: "VanEck ETHV_1",
        provider: "VanEck",
    },
];

/// Total number of ETF addresses
pub const ETF_ADDRESS_COUNT: usize = 1151;

/// Lazy static HashSet for quick lookups
pub static ETF_ADDRESS_SET: Lazy<HashSet<Address>> =
    Lazy::new(|| ETF_ADDRESSES.iter().map(|entry| entry.address).collect());

/// Lazy static HashMap for address to ETF mapping
pub static ETF_BY_ADDRESS: Lazy<HashMap<Address, &'static EtfAddress>> = Lazy::new(|| {
    ETF_ADDRESSES
        .iter()
        .map(|entry| (entry.address, entry))
        .collect()
});

/// Group addresses by provider
pub static ADDRESSES_BY_PROVIDER: Lazy<HashMap<&'static str, Vec<Address>>> = Lazy::new(|| {
    let mut map: HashMap<&'static str, Vec<Address>> = HashMap::new();
    for entry in ETF_ADDRESSES {
        map.entry(entry.provider)
            .or_insert_with(Vec::new)
            .push(entry.address);
    }
    map
});

/// Check if an address belongs to an ETF
pub fn is_etf_address(address: Address) -> bool {
    ETF_ADDRESS_SET.contains(&address)
}

/// Get ETF info by address
pub fn get_etf_by_address(address: Address) -> Option<&'static EtfAddress> {
    ETF_BY_ADDRESS.get(&address).copied()
}

/// Get all addresses for a specific provider
pub fn get_provider_addresses(provider: &str) -> Option<&'static Vec<Address>> {
    ADDRESSES_BY_PROVIDER.get(provider)
}

/// Provider counts:
/// - Grayscale: 978 addresses
/// - BlackRock: 141 addresses
/// - Fidelity: 10 addresses
/// - 21Shares: 8 addresses
/// - Bitwise: 6 addresses
/// - Invesco: 4 addresses
/// - Franklin: 2 addresses
/// - VanEck: 2 addresses
/// Provider names with counts
pub fn provider_stats() -> Vec<(&'static str, usize)> {
    let mut stats: Vec<_> = ADDRESSES_BY_PROVIDER
        .iter()
        .map(|(name, addrs)| (*name, addrs.len()))
        .collect();
    stats.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    stats
}
