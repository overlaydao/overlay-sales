use crate::{
    account_address_from_str, config::INDEX_PUB_RIDO_USDC, get_keypair_from_wallet_keys, types::*,
    vec_to_arr, MessageType, INDEX_OPERATOR,
};
use anyhow::bail;
use chrono::{DateTime, Duration};
use concordium_rust_sdk::{
    common::types::{KeyPair, Signature},
    types::{transactions::TransactionSigner, WalletAccount},
};
use concordium_std::{
    from_bytes, to_bytes, ContractAddress, OwnedEntrypointName, PublicKeyEd25519, Timestamp,
};
use sale_utils::types::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    io::Write,
    path::{Component, Path},
    time::SystemTime,
};

pub fn confirm_signed_message(encoded_msg: String, mode: MessageType) -> anyhow::Result<()> {
    let message_bytes = hex::decode(&encoded_msg).unwrap();

    // check message payload for sure
    let check_message = from_bytes::<PermitMessageWithParameter>(&message_bytes).unwrap();
    println!("{:#?}", check_message);

    match mode {
        MessageType::AddKey | MessageType::RemoveKey => {
            let operators: Vec<operators::OperatorWithKeyParam> =
                from_bytes(&check_message.parameter).unwrap();
            println!("Operators: {:#?}", operators);
        },
        MessageType::Invoke => {
            let status: SaleStatus = from_bytes(&check_message.parameter).unwrap();
            println!("Sale Status: {:#?}", status);
        },
        MessageType::None => {},
    }

    Ok(())
}

pub fn sign(keys_path: &Path, data_path: &Path, mode: MessageType) -> anyhow::Result<()> {
    if crate::validate_file_path(keys_path) {
        bail!("Invalid filepath.")
    }

    if crate::validate_file_path(data_path) {
        bail!("Invalid json filepath.")
    }

    let keys: WalletAccount = WalletAccount::from_json_file(keys_path)?;
    let keypair = get_keypair_from_wallet_keys(&keys)?;

    let json = std::fs::read_to_string(data_path).unwrap();

    // #[Todo]
    let message: PermitMessageWithParameter = match mode {
        MessageType::AddKey | MessageType::RemoveKey => message_for_update_operators(json)?,
        MessageType::Invoke => message_for_invoke(json)?,
        _ => bail!("no message for ..."),
    };

    let message_encoded = hex::encode(&to_bytes(&message));
    println!("message_encoded: {:?}", message_encoded);

    // sign
    let message_bytes = hex::decode(&message_encoded).unwrap();

    let message_hash = Sha256::digest(&message_bytes);
    let signature: Signature = keypair.sign(&message_hash);
    println!("sign: {:?}", hex::encode(signature.as_ref()));

    Ok(())
}

// ===============================================================

#[derive(Serialize, Deserialize, Debug)]
struct MsgParamsForInvoke {
    index: u64,
    timestamp: String,
    method: String,
    status: Vec<u8>,
}

fn message_for_invoke(json: String) -> anyhow::Result<PermitMessageWithParameter> {
    let msg: MsgParamsForInvoke = serde_json::from_str(&json)?;

    let params: SaleStatus = from_bytes(&msg.status).unwrap();

    let action = match msg.method.as_str() {
        "setStatus" => PermitAction::Invoke(
            ContractAddress::new(INDEX_PUB_RIDO_USDC, 0),
            OwnedEntrypointName::new_unchecked(msg.method.clone()),
        ),
        _ => bail!("Invalid Mode"),
    };

    // signers should be sure of what they are signing.
    let message: PermitMessageWithParameter = create_permit_message(
        msg.index,
        msg.method,
        action,
        msg.timestamp,
        to_bytes(&params),
    )?;

    Ok(message)
}

// ---------------------------------

#[derive(Serialize, Deserialize, Debug)]
struct Operator {
    address: String,
    pubkey: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct MsgParamsForUpdateKeys {
    index: u64,
    timestamp: String,
    method: String,
    operators: Vec<Operator>,
}

fn message_for_update_operators(json: String) -> anyhow::Result<PermitMessageWithParameter> {
    let msg: MsgParamsForUpdateKeys = serde_json::from_str(&json)?;

    let mut params: Vec<operators::OperatorWithKeyParam> = Vec::new();
    for v in msg.operators {
        let addr = account_address_from_str(&v.address).unwrap();
        let pubkey: [u8; 32] = vec_to_arr(hex::decode(v.pubkey).unwrap());
        params.push(operators::OperatorWithKeyParam {
            account: addr,
            public_key: PublicKeyEd25519(pubkey),
        });
    }

    let action = match msg.method.as_str() {
        "addOperatorKeys" => PermitAction::AddKey,
        "removeOperatorKeys" => PermitAction::RemoveKey,
        _ => bail!("Invalid Mode"),
    };

    // signers should be sure of what they are signing.
    let message: PermitMessageWithParameter = create_permit_message(
        msg.index,
        msg.method,
        action,
        msg.timestamp,
        to_bytes(&params),
    )?;

    Ok(message)
}

// ---------------------------------

fn create_permit_message(
    index: u64,
    method: String,
    action: PermitAction,
    ts: String,
    params: Vec<u8>,
) -> anyhow::Result<PermitMessageWithParameter> {
    let timelimit = DateTime::parse_from_rfc3339(ts.as_str())?;
    // let timelimit = timelimit + Duration::hours(9);

    let message = PermitMessageWithParameter {
        contract_address: ContractAddress { index, subindex: 0 },
        entry_point: OwnedEntrypointName::new_unchecked(method),
        action,
        timestamp: Timestamp::from_timestamp_millis(timelimit.timestamp_millis() as u64),
        parameter: params,
    };

    Ok(message)
}

// pub fn sign_exp(
//     config_path: &Path,
//     _mode: &str,
//     _index: u64,
//     _timestamp: &str,
// ) -> anyhow::Result<()> {
//     if crate::validate_file_path(config_path) {
//         bail!("Invalid filepath.")
//     }

//     let keys: WalletAccount = WalletAccount::from_json_file(config_path)?;
//     let keypair = get_keypair_from_wallet_keys(&keys)?;

//     let json = std::fs::read_to_string("./data/msg.json").unwrap();
//     let msg: MsgParamsForUpdateKeys = serde_json::from_str(&json)?;

//     let mut new_op: Vec<OperatorWithKeyParam> = Vec::new();
//     for v in msg.operators {
//         let addr = account_address_from_str(&v.address).unwrap();
//         let pubkey: [u8; 32] = vec_to_arr(hex::decode(v.pubkey).unwrap());
//         new_op.push(OperatorWithKeyParam {
//             account: addr,
//             public_key: PublicKeyEd25519(pubkey),
//         });
//     }
//     let parameter = to_bytes(&new_op);

//     // -------------------------------------------------------
//     let action = match msg.method.as_str() {
//         "addOperatorKeys" => PermitAction::AddKey,
//         "removeOperatorKeys" => PermitAction::RemoveKey,
//         _ => bail!("Invalid Mode"),
//     };

//     let timelimit = DateTime::parse_from_rfc3339(msg.timestamp.as_str())?;
//     // let timelimit = timelimit + Duration::hours(9);
//     let timestamp = Timestamp::from_timestamp_millis(timelimit.timestamp_millis() as u64);

//     // signers should be sure of what they are signing.
//     let message = PermitMessageWithParameter {
//         contract_address: ContractAddress {
//             index: msg.index,
//             subindex: 0,
//         },
//         entry_point: OwnedEntrypointName::new_unchecked(msg.method),
//         action,
//         timestamp,
//         parameter,
//     };

//     let message_bytes = to_bytes(&message);
//     let encoded = hex::encode(&message_bytes);
//     println!("message_encoded: {:?}", encoded);

//     // -------------------------------------------------------

//     let message_bytes = hex::decode(&encoded).unwrap();

//     let message_hash = Sha256::digest(&message_bytes);
//     // let message_hash = Sha256::new().chain_update(&message_bytes).finalize();

//     // sign
//     let sig: Signature = keypair.sign(&message_hash);
//     println!("sign: {:?}", hex::encode(sig.as_ref()));

//     Ok(())
// }
