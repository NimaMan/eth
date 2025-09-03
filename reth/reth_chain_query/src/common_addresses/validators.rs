//! Validator and MEV builder fee recipient addresses
//! 
//! This file is auto-generated from Python address files.
//! Do not edit manually - regenerate using scripts/convert_addresses_to_rust.py

use alloy_primitives::{address, Address};
use std::collections::{HashMap, HashSet};
use once_cell::sync::Lazy;

/// Validator/Builder fee recipient entry
#[derive(Debug, Clone)]
pub struct FeeRecipient {
    pub address: Address,
    pub name: &'static str,
}

/// All fee recipient addresses (validators and MEV builders)
pub const FEE_RECIPIENT_LIST: &[FeeRecipient] = &[
    FeeRecipient {
        address: address!("000000000000d3B2C76221467d2f8c8f1dE832A2"),
        name: "Builder Smith",
    },
    FeeRecipient {
        address: address!("00066282d9FAc206F8F0fd0b935958ae55E13333"),
        name: "MEV Builder: 0x000...333",
    },
    FeeRecipient {
        address: address!("000E633ddeF00DA46aBd5044779257a64ead9bce"),
        name: "MEV Builder: 0x000...bce",
    },
    FeeRecipient {
        address: address!("036C9c0aaE7a8268F332bA968dac5963c6aDAca5"),
        name: "MEV Builder: 0x036...ca5",
    },
    FeeRecipient {
        address: address!("089780A88f35B58144Aa8a9BE654207A1aFe7959"),
        name: "MEV Builder: 0x089...959",
    },
    FeeRecipient {
        address: address!("0Aa8EBb6aD5A8e499E550ae2C461197624c6e667"),
        name: "MEV Builder: 0x0Aa...667",
    },
    FeeRecipient {
        address: address!("0b1Ddf6D1DA69532Ad4198470679b0b49176c68f"),
        name: "MEV Builder: 0x0b1...68f",
    },
    FeeRecipient {
        address: address!("11a8961fbD55e67Fe4Ab99c7ca47616Dcf3D0010"),
        name: "MEV Builder: 0x11a...010",
    },
    FeeRecipient {
        address: address!("1324c0fB6F45f3bF1AAA1fCdC08f17431F53DeD7"),
        name: "MEV Builder: 0x13...eD7",
    },
    FeeRecipient {
        address: address!("14F1a856A821B3CE5F59d3bA813D614546125C15"),
        name: "MEV Builder: 0x14F...C15",
    },
    FeeRecipient {
        address: address!("185A5012067d8F0f6ab2de1C78B96f15Aa552043"),
        name: "MEV Builder: 0x185...043",
    },
    FeeRecipient {
        address: address!("195d0F5F00833bcE2F40920DE0DB1D92a8808886"),
        name: "MEV Builder: 0x195...886",
    },
    FeeRecipient {
        address: address!("199D5ED7F45F4eE35960cF22EAde2076e95B253F"),
        name: "bloXroute: Regulated Builder",
    },
    FeeRecipient {
        address: address!("1b887Aa026f7A90a0f7173C2f0722d263c7CeC45"),
        name: "MEV Builder: 0x1b8...C45",
    },
    FeeRecipient {
        address: address!("1d0124FeE8Dbe21884Ab97adCCBF5C55d768886e"),
        name: "MEV Builder: 0x1d0...86e",
    },
    FeeRecipient {
        address: address!("1e54945FBf1872e34D76B7d72151B861704Df8B2"),
        name: "MEV Builder: 0x1e54...8B2",
    },
    FeeRecipient {
        address: address!("1f9090aaE28b8a3dCeaDf281B0F12828e676c326"),
        name: "rsync-builder.eth",
    },
    FeeRecipient {
        address: address!("2194331af2cF9dE9Adb36cf09654faD65cafb58b"),
        name: "MEV Builder: 0x219...58b",
    },
    FeeRecipient {
        address: address!("229b8325bb9Ac04602898B7e8989998710235d5f"),
        name: "MEV Builder: 0x22...d5f",
    },
    FeeRecipient {
        address: address!("24b1D27B0f6B5A2Aa052Acf59817a8D9e7A8600A"),
        name: "Titanbuilder: 0x24b...00A",
    },
    FeeRecipient {
        address: address!("25B71878850D008ec4237C55f0A59198BCC72b43"),
        name: "MEV Builder: 0x25B…b43",
    },
    FeeRecipient {
        address: address!("25D88437dF70730122b73Ef35462435d187C466f"),
        name: "MEV Builder: 0x25D...66f",
    },
    FeeRecipient {
        address: address!("29F94b27Fe0B410e4546b6021E4044ff985dc252"),
        name: "MEV Builder: 0x29F...252",
    },
    FeeRecipient {
        address: address!("333333f332a06ECB5D20D35da44ba07986D6E203"),
        name: "MEV Builder: 0x333...203",
    },
    FeeRecipient {
        address: address!("3B6c26116749a6F9D194172d56299377E61bB0aE"),
        name: "MEV Builder: 0x3B6...0aE",
    },
    FeeRecipient {
        address: address!("3Bee5122E2a2FbE11287aAfb0cB918e22aBB5436"),
        name: "MEV Builder: 0x3B...436",
    },
    FeeRecipient {
        address: address!("3Ca601b21D62790308298E3274Fd852669Fdfc08"),
        name: "MEV Builder: 0x3Ca...c08",
    },
    FeeRecipient {
        address: address!("3E3753491f224571dd8d7E925B73cc685ab0aae4"),
        name: "MEV Builder: 0x3E3...ae4",
    },
    FeeRecipient {
        address: address!("3b64216AD1a58f61538b4fA1B27327675Ab7ED67"),
        name: "Boba Builder",
    },
    FeeRecipient {
        address: address!("3b7fAEc3181114A99c243608BC822c5436441FfF"),
        name: "MEV Builder: 0x3b...FfF",
    },
    FeeRecipient {
        address: address!("3c496DF419762533607f30BB2143aFF77bEBc36A"),
        name: "MEV Builder: 0x3c...36A",
    },
    FeeRecipient {
        address: address!("418211EFaf54e6A9b376f6Bfd9E0AE304E064CBb"),
        name: "MEV Builder: 0x418...CBb",
    },
    FeeRecipient {
        address: address!("4675C7e5BaAFBFFbca748158bEcBA61ef3b0a263"),
        name: "MEV Builder: 0x467...263",
    },
    FeeRecipient {
        address: address!("473780deAF4a2Ac070BBbA936B0cdefe7F267dFc"),
        name: "MEV Builder: 0x473...dFc",
    },
    FeeRecipient {
        address: address!("47fE0AEe392D59Ccaa7Cc3B162F629eCb0f2671F"),
        name: "MEV Builder: 0x47f...71F",
    },
    FeeRecipient {
        address: address!("4838B106FCe9647Bdf1E7877BF73cE8B0BAD5f97"),
        name: "Titan Builder",
    },
    FeeRecipient {
        address: address!("4A55474EACb48CEFe25D7656Db1976AA7AE70E3C"),
        name: "MEV Builder: 0x4A5...E3C",
    },
    FeeRecipient {
        address: address!("5124fcC2B3F99F571AD67D075643C743F38f1C34"),
        name: "Faith Builder",
    },
    FeeRecipient {
        address: address!("5416f0dd6C29bFF6a4f32BaFB5c5dA6365472973"),
        name: "MEV Builder: 0x541...973",
    },
    FeeRecipient {
        address: address!("5638cbdC72bd8554055883D309CFc70357190CF3"),
        name: "MEV Builder: 0x563...cf3",
    },
    FeeRecipient {
        address: address!("57865ba267D48671A41431F471933aEC32a7c7d1"),
        name: "Manifold Finance: Builder",
    },
    FeeRecipient {
        address: address!("57af10eD3469b2351AE60175d3C9B3740E1Bb649"),
        name: "MEV Builder: 0x57...649",
    },
    FeeRecipient {
        address: address!("5A266F52802e846ddf93B87e520404B4fe778411"),
        name: "MEV Builder: 0x5A2...411",
    },
    FeeRecipient {
        address: address!("5F525f637759FCa7C9d1C0C4f9d479D6E8D8ceF5"),
        name: "MEV Builder: 0x5F...eF5",
    },
    FeeRecipient {
        address: address!("5F927395213ee6b95dE97bDdCb1b2B1C0F16844F"),
        name: "Manta-builder",
    },
    FeeRecipient {
        address: address!("5c8D0eeD35a9e632BB8c0AbE4662B6aB3326850b"),
        name: "MEV Builder: 0x5c8...50b",
    },
    FeeRecipient {
        address: address!("690B9A9E9aa1C9dB991C7721a92d351Db4FaC990"),
        name: "builder0x69",
    },
    FeeRecipient {
        address: address!("6aF43cC73c4a871274767887e8E39Eeb540582A3"),
        name: "MEV Builder: 0x6a...2A3",
    },
    FeeRecipient {
        address: address!("70B6c88f608AC228Fd767d05094967eb91d02583"),
        name: "MEV Builder: 0x70b...583",
    },
    FeeRecipient {
        address: address!("7316b4E0f0D4B19b4aC13895224cD522D785e51D"),
        name: "lightspeedbuilder 1",
    },
    FeeRecipient {
        address: address!("77777A6C097a1cE65C61A96a49bd1100F660eC94"),
        name: "MEV Builder: 0x777...C94",
    },
    FeeRecipient {
        address: address!("795e17B08f45cd06E833138a2236Fa8C7aA0b3AC"),
        name: "MEV Builder: 0x795...3AC",
    },
    FeeRecipient {
        address: address!("7AdE2D98420d1735EA2Ad3C17Ef46bF11500Fc4f"),
        name: "MEV Builder: 0x7Ad...c4f",
    },
    FeeRecipient {
        address: address!("7aDc0e867EBc337E2d20c44DB181c067fA08637b"),
        name: "blockbeelder",
    },
    FeeRecipient {
        address: address!("7dA0aEf1B75035cbf364a690411BCCa7E7859dF8"),
        name: "MEV Builder: 0x7dA...dF8",
    },
    FeeRecipient {
        address: address!("7e2a2FA2a064F693f0a55C5639476d913Ff12D05"),
        name: "MEV Builder: 0x7e2...D05",
    },
    FeeRecipient {
        address: address!("88c6C46EBf353A52Bdbab708c23D0c81dAA8134A"),
        name: "MEV Builder: 0x88c...34A",
    },
    FeeRecipient {
        address: address!("8D5998A27b3CdF33479B65B18F075E20a7aa05b9"),
        name: "MEV Builder: 0x8D...5b9",
    },
    FeeRecipient {
        address: address!("8E57bC446f76B2054089CC5c8fA6F0F5B72fC59a"),
        name: "Titanbuilder: 0x8E5...59a",
    },
    FeeRecipient {
        address: address!("95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5"),
        name: "beaverbuild",
    },
    FeeRecipient {
        address: address!("965Df5Ff6116C395187E288e5C87fb96CfB8141c"),
        name: "bloXroute: Builder 1",
    },
    FeeRecipient {
        address: address!("9D8e2dc5615c674F329d18786D52AF10a65Af08b"),
        name: "MEV Builder: 0x9D8...08b",
    },
    FeeRecipient {
        address: address!("9FE3bC4A1A4116c6Dc1fFD61226E262c3f2bc561"),
        name: "Titanbuilder: 0x9FE...561",
    },
    FeeRecipient {
        address: address!("A7FdCa7AA0B69927a34ec48DdcFe3d4C66fF0d94"),
        name: "MEV Builder: 0xA7F...d94",
    },
    FeeRecipient {
        address: address!("AAB27b150451726EC7738aa1d0A94505c8729bd1"),
        name: "Eden Network: Builder",
    },
    FeeRecipient {
        address: address!("B279d48442aAfCF8F2af6d9E7d5d9C23f63b4e16"),
        name: "MEV Builder: 0xB27...e16",
    },
    FeeRecipient {
        address: address!("BaF6dC2E647aeb6F510f9e318856A1BCd66C5e19"),
        name: "MEV Builder: 0xBaF...e19",
    },
    FeeRecipient {
        address: address!("C4b7a6008d8e2C2E1b5F8B743E71d2c0495cd777"),
        name: "MEV Builder: 0xC4b...777",
    },
    FeeRecipient {
        address: address!("C6108744c9D5db8b30f8004053a16D5683cD3489"),
        name: "MEV Builder: 0xC61...489",
    },
    FeeRecipient {
        address: address!("CE0BaBc8398144Aa98D9210d595E3A9714910748"),
        name: "payload",
    },
    FeeRecipient {
        address: address!("DAFEA492D9c6733ae3d56b7Ed1ADB60692c98Bc5"),
        name: "Flashbots: Builder",
    },
    FeeRecipient {
        address: address!("DccA982701a264e8d629A6E8CFBa9C1a27912623"),
        name: "MEV Builder: 0xDcc...623",
    },
    FeeRecipient {
        address: address!("E821377b30C63a873Be0eb7fE0c6f31911285d38"),
        name: "MEV Builder: 0xE82...d38",
    },
    FeeRecipient {
        address: address!("EeEE8Db5fC7d505e99970945a9220Ab7992050E3"),
        name: "MEV Builder: 0xEe...0E3",
    },
    FeeRecipient {
        address: address!("F2f5C73fa04406b1995e397B55c24aB1f3eA726C"),
        name: "bloXroute: Max Profit Builder",
    },
    FeeRecipient {
        address: address!("FeebabE6b0418eC13b30aAdF129F5DcDd4f70CeA"),
        name: "eth-builder",
    },
    FeeRecipient {
        address: address!("aC7EA48093B61f2E217b9d077d69D9d55CA1B106"),
        name: "MEV Builder: 0xaC7...106",
    },
    FeeRecipient {
        address: address!("ae08c571e771F360c35f5715E36407ECc89D91ed"),
        name: "MEV Builder: 0xae...1ed",
    },
    FeeRecipient {
        address: address!("b4c9E4617a16Be36B92689b9e07e9F64757c1792"),
        name: "MEV Builder: 0xb4c...792",
    },
    FeeRecipient {
        address: address!("b646D87963Da1FB9D192Ddba775f24f33e857128"),
        name: "MEV Builder: 0xb64…128",
    },
    FeeRecipient {
        address: address!("b64a30399f7F6b0C154c2E7Af0a3ec7B0A5b131a"),
        name: "Flashbots: Old Builder",
    },
    FeeRecipient {
        address: address!("bEED9A1A750945966cfc8800Abd4Cf7eECD22500"),
        name: "MEV Bot: 0xbEE...500",
    },
    FeeRecipient {
        address: address!("bd3Afb0bB76683eCb4225F9DBc91f998713C3b01"),
        name: "BuildAI.net",
    },
    FeeRecipient {
        address: address!("c1612dc56C3E7e00D86c668dF03904B7E59616C5"),
        name: "MEV Builder: 0xc16...6C5",
    },
    FeeRecipient {
        address: address!("c83dad6e38BF7F2d79f2a51dd3C4bE3f530965D6"),
        name: "Flashbots: SGX Builder",
    },
    FeeRecipient {
        address: address!("c9D945721ed37c6451E457b3C7F1e0ceC42417fb"),
        name: "antbuilder",
    },
    FeeRecipient {
        address: address!("cDA9D71bdfAe59b89Cee131eD3079f8AC4c77062"),
        name: "MEV Builder: 0xcDA...062",
    },
    FeeRecipient {
        address: address!("cDBF58a9A9b54a2C43800c50C7192946dE858321"),
        name: "MEV Builder: 0xcDB...321",
    },
    FeeRecipient {
        address: address!("d11D7D2cb0aFF72A61Df37fD016EE1bd9F180633"),
        name: "MEV Builder: 0xd11...633",
    },
    FeeRecipient {
        address: address!("d1A0b5843F384f92a6759015c742fc12d1d579a1"),
        name: "MEV Builder: 0xd1...9a1",
    },
    FeeRecipient {
        address: address!("d2090025857B9C7B24387741f120538E928A3a59"),
        name: "lightspeedbuilder 2",
    },
    FeeRecipient {
        address: address!("dA795b000C29e6F47C2b2A5F3A35c5647695e301"),
        name: "MEV Builder: 0xda7...301",
    },
    FeeRecipient {
        address: address!("f0Ef0B3D1CE0a2C303e76200213B3AD5dE61a4B7"),
        name: "Titanbuilder:0xf0E...4B7",
    },
    FeeRecipient {
        address: address!("f573d99385C05c23B24ed33De616ad16a43a0919"),
        name: "bloXroute: Ethical Builder",
    }
];

/// Total number of fee recipients
pub const FEE_RECIPIENT_COUNT: usize = 94;

/// Lazy static HashSet for quick lookups
pub static FEE_RECIPIENTS: Lazy<HashSet<Address>> = Lazy::new(|| {
    FEE_RECIPIENT_LIST.iter().map(|entry| entry.address).collect()
});

/// Lazy static HashMap for address to name mapping
pub static FEE_RECIPIENT_BY_ADDRESS: Lazy<HashMap<Address, &'static str>> = Lazy::new(|| {
    FEE_RECIPIENT_LIST
        .iter()
        .map(|entry| (entry.address, entry.name))
        .collect()
});

/// Check if an address is a known fee recipient (validator/builder)
pub fn is_fee_recipient(address: Address) -> bool {
    FEE_RECIPIENTS.contains(&address)
}

/// Get fee recipient name by address
pub fn get_fee_recipient_name(address: Address) -> Option<&'static str> {
    FEE_RECIPIENT_BY_ADDRESS.get(&address).copied()
}

/// Check if this is a bribe payment (ETH transfer to fee recipient)
pub fn is_bribe(to_address: Address) -> bool {
    is_fee_recipient(to_address)
}

/// Get builder/validator statistics
pub fn fee_recipient_stats() -> Vec<(&'static str, usize)> {
    let mut builders = 0usize;
    let mut validators = 0usize;
    let mut flashbots = 0usize;
    let mut titan = 0usize;
    let mut rsync = 0usize;
    let mut bloxroute = 0usize;
    
    for entry in FEE_RECIPIENT_LIST {
        let name_lower = entry.name.to_lowercase();
        if name_lower.contains("flashbots") {
            flashbots += 1;
        } else if name_lower.contains("titan") {
            titan += 1;
        } else if name_lower.contains("rsync") {
            rsync += 1;
        } else if name_lower.contains("bloxroute") {
            bloxroute += 1;
        }
        
        if name_lower.contains("builder") {
            builders += 1;
        } else {
            validators += 1;
        }
    }
    
    vec![
        ("Total Fee Recipients", FEE_RECIPIENT_COUNT),
        ("Builders", builders),
        ("Validators", validators),
        ("Flashbots", flashbots),
        ("Titan", titan),
        ("bloXroute", bloxroute),
        ("rsync", rsync),
    ]
}
