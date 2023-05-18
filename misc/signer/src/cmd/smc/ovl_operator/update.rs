use crate::{account_address_from_str, types::*, vec_to_arr};
use concordium_std::{from_bytes, to_bytes, SignatureEd25519};
use sale_utils::types::PermitMessageWithParameter;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

#[derive(Serialize, Deserialize, Debug)]
struct Sigs {
    address: String,
    signature: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct UpdateKeys {
    message: String,
    method: String,
    sigs: Vec<Sigs>,
}

pub async fn invoke() -> anyhow::Result<(String, Vec<u8>)> {
    let data_path = Path::new("./data/sigs_invoke.json");

    let json = std::fs::read_to_string(data_path).unwrap();
    let msg: UpdateKeys = serde_json::from_str(&json)?;

    // message
    let message_encoded = msg.message;
    let message_byte = hex::decode(&message_encoded).unwrap();
    let message = from_bytes::<PermitMessageWithParameter>(&message_byte).unwrap();

    // signatures for update
    let mut signatures = BTreeSet::new();
    for v in msg.sigs {
        let addr = account_address_from_str(&v.address).unwrap();
        let sig: [u8; 64] = vec_to_arr(hex::decode(v.signature).unwrap());
        signatures.insert((addr, SignatureEd25519(sig)));
    }

    let params = operators::ParamsWithSignatures {
        signatures,
        message,
    };

    Ok((msg.method, to_bytes(&params)))
}

pub async fn update_keys() -> anyhow::Result<(String, Vec<u8>)> {
    let data_path = Path::new("./data/sigs_add_key.json");

    let json = std::fs::read_to_string(data_path).unwrap();
    let msg: UpdateKeys = serde_json::from_str(&json)?;

    // message
    let message_encoded = msg.message;
    let message_byte = hex::decode(&message_encoded).unwrap();
    let message = from_bytes::<PermitMessageWithParameter>(&message_byte).unwrap();

    // signatures for update
    let mut signatures = BTreeSet::new();
    for v in msg.sigs {
        let addr = account_address_from_str(&v.address).unwrap();
        let sig: [u8; 64] = vec_to_arr(hex::decode(v.signature).unwrap());
        signatures.insert((addr, SignatureEd25519(sig)));
    }

    let params = operators::ParamsWithSignatures {
        signatures,
        message,
    };

    Ok((msg.method, to_bytes(&params)))
}
