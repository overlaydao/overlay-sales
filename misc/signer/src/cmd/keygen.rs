use crate::types::*;
use concordium_rust_sdk::types::smart_contracts::concordium_contracts_common::AccountAddress;
use ed25519_dalek::Keypair;
use rand::rngs::OsRng;
use std::{
    collections::HashMap,
    fs::{create_dir_all, File},
    io::Write,
    path::Path,
};

pub fn gen_keys() -> anyhow::Result<AccountKeys> {
    // dummy address
    let address = AccountAddress([0; 32]).to_string();

    let mut csprng = OsRng {};
    let keypair: Keypair = Keypair::generate(&mut csprng);

    let mut keys = HashMap::new();
    keys.insert(
        0,
        KeypairString {
            sign_key: hex::encode(keypair.secret.as_bytes()),
            verify_key: hex::encode(keypair.public.as_bytes()),
        },
    );

    let mut account_keys = HashMap::new();
    account_keys.insert(0, KeyContent { keys, threshold: 1 });

    Ok(AccountKeys {
        account_keys: Keys {
            keys: account_keys,
            threshold: 1,
        },
        address,
    })
}

pub fn output_json(keys: AccountKeys, filename: &str) -> anyhow::Result<()> {
    let index = chrono::Utc::now().timestamp();

    let json = serde_json::to_string_pretty(&keys).unwrap();

    let output_dir = Path::new("out");
    create_dir_all(&output_dir)?;
    let file_path = output_dir.join(format!("{}{}.json", filename, index));

    let file = File::create(&file_path)?;
    write!(&file, "{}", json)?;

    println!("File created: {:?}", file_path.canonicalize()?);
    println!("[Caution!] Note that the file format is compatible with WalletAccount, but the address is dummy.");

    Ok(())
}
