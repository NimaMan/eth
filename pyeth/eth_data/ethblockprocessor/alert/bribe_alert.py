
from ethblockprocessor.data_models.txn_models import InternalTransaction, DetailedTransaction
from ethblockprocessor.data_models.alert_models import BribeAlertData


class BribeAlert:
    def __init__(self, bribe_threshold: float = 0.1):
        self.fee_recipients_set = fee_recipients_set
        self.bribe_threshold = bribe_threshold

    def get_alerts(self, txn: DetailedTransaction):
        internal_txns = txn.internal_transactions
        if internal_txns:
            for txn in internal_txns:
                if txn.to.lower() in self.fee_recipients_set:
                    if txn.value > self.bribe_threshold:
                        alert_data = self.create_alert(txn)
                        self.send_alert(alert_data)
                        return [alert_data]
        return []
    
    def create_alert(self, txn: DetailedTransaction):
        alert_data = BribeAlertData(
            block_number=txn.block_number,
            transaction_hash=txn.hash,
            alert_type="Bribe",
            details={
                "from_address": txn.from_address,
                "value": txn.value,
                "bribe_amount": txn.value,
            }
        )
        return alert_data

    def send_alert(self, alert_data: BribeAlertData):
        print(f"Bribe alert sent: {alert_data}")



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

