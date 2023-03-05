use std::str::FromStr;

use bdk::{
    bitcoin::{secp256k1::Secp256k1, util::bip32::DerivationPath, Network},
    keys::{
        bip39::{Language, Mnemonic, MnemonicType},
        DerivableKey,
        DescriptorKey::{self, Secret},
        ExtendedKey, GeneratableKey, GeneratedKey,
    },
    miniscript::miniscript::Segwitv0,
};

pub fn get_descriptors() -> (String, String) {
    // a new secp context is created
    let secp = Secp256k1::new();

    let password = Some("random password".to_string());

    // generate fresh mmemonic and then a private key
    let mmemonic: GeneratedKey<_, Segwitv0> =
        Mnemonic::generate((MnemonicType::Words12, Language::English)).unwrap();
    let mmemonic = mmemonic.into_key();
    let xpubkey: ExtendedKey = (mmemonic, password).into_extended_key().unwrap();
    let xprivkey = xpubkey.into_xprv(Network::Regtest).unwrap();

    // Create derived privkey from the above master privkey
    // We use the following derivation paths for receive and change keys
    // receive: "m/84h/1h/0h/0"
    // change: "m/84h/1h/0h/1"
    let mut keys = Vec::new();

    for path in ["m/84h/1h/0h/0", "m/84h/1h/0h/1"] {
        let derivation_path = DerivationPath::from_str(path).unwrap();
        let derived_xprivkey = &xprivkey.derive_priv(&secp, &derivation_path).unwrap();
        let origin = (xprivkey.fingerprint(&secp), derivation_path);
        let derived_xprv_desc_key: DescriptorKey<Segwitv0> = derived_xprivkey
            .into_descriptor_key(Some(origin), DerivationPath::default())
            .unwrap();

        if let Secret(key, _, _) = derived_xprv_desc_key {
            let mut desc = "wpkh(".to_string();
            desc.push_str(&key.to_string());
            desc.push_str(")");
            keys.push(desc)
        }
    }

    // Return the keys as a tuple
    (keys[0].clone(), keys[1].clone())
}
