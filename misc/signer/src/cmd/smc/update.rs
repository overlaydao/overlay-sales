use crate::{
    account_address_from_str, types::*, vec_to_arr, CONTRACT_OPERATOR, INDEX_OPERATOR,
    MODREF_OPERATOR, NODE_ENDPOINT_V2,
};
use anyhow::{bail, Context};
use concordium_rust_sdk::{
    common::types::{KeyPair, TransactionTime},
    smart_contracts::common::Amount,
    types::{
        hashes::{HashBytes, ModuleReferenceMarker},
        smart_contracts::{
            concordium_contracts_common::ContractAddress as CA, OwnedContractName, OwnedParameter,
            OwnedReceiveName,
        },
        transactions::{
            send, BlockItem, EncodedPayload, InitContractPayload, UpdateContractPayload,
        },
        AccountInfo, Nonce, WalletAccount,
    },
    v2::{BlockIdentifier, Client, Endpoint},
};
use concordium_std::{from_bytes, to_bytes, AccountAddress, PublicKeyEd25519, SignatureEd25519};
use sale_utils::types::{PermitAction, PermitMessageWithParameter};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    path::Path,
    str::FromStr,
};

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

pub async fn invoke() -> anyhow::Result<()> {
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

    // -------------------------------------------------------
    // sender
    let keys1: WalletAccount = WalletAccount::from_json_file("./keys/keys.json")
        .context("Could not read the keys file.")?;

    let mut client = Client::new(Endpoint::from_static(NODE_ENDPOINT_V2))
        .await
        .context("Cannot connect.")?;

    let acc_info: AccountInfo = client
        .get_account_info(&keys1.address.into(), &BlockIdentifier::Best)
        .await
        .context("Cannot connect.")?
        .response;

    // Parameter for Tx
    let index = INDEX_OPERATOR;
    let nonce = acc_info.account_nonce;
    let amount = Amount::zero();

    let params = operators::ParamsWithSignatures {
        signatures,
        message,
    };

    let item = create_update_tx_item(
        &keys1,
        nonce,
        index,
        CONTRACT_OPERATOR,
        &msg.method,
        to_bytes(&params),
        amount,
    );

    crate::broadcast(&mut client, item);
    Ok(())
}

pub async fn update_keys() -> anyhow::Result<()> {
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

    // -------------------------------------------------------
    // sender
    let keys1: WalletAccount = WalletAccount::from_json_file("./keys/keys.json")
        .context("Could not read the keys file.")?;

    let mut client = Client::new(Endpoint::from_static(NODE_ENDPOINT_V2))
        .await
        .context("Cannot connect.")?;

    let acc_info: AccountInfo = client
        .get_account_info(&keys1.address.into(), &BlockIdentifier::Best)
        .await
        .context("Cannot connect.")?
        .response;

    // Parameter for Tx
    let index = INDEX_OPERATOR;
    let nonce = acc_info.account_nonce;
    let amount = Amount::zero();

    let params = operators::ParamsWithSignatures {
        signatures,
        message,
    };

    let item = create_update_tx_item(
        &keys1,
        nonce,
        index,
        CONTRACT_OPERATOR,
        &msg.method,
        to_bytes(&params),
        amount,
    );

    crate::broadcast(&mut client, item);

    Ok(())
}

// =========================================================

fn create_update_tx_item(
    sender: &WalletAccount,
    nonce: Nonce,
    index: u64,
    contract: &str,
    method: &str,
    params: Vec<u8>,
    amount: Amount,
) -> BlockItem<EncodedPayload> {
    let expiry: TransactionTime =
        TransactionTime::from_seconds((chrono::Utc::now().timestamp() + 300) as u64);

    let address = CA::new(index, 0);
    let receive_name =
        OwnedReceiveName::new_unchecked(format!("{}.{}", contract, method).to_string());
    let message = OwnedParameter::try_from(params).unwrap();

    let payload = UpdateContractPayload {
        amount,
        address,
        receive_name,
        message,
    };

    let tx = send::update_contract(
        sender,
        sender.address,
        nonce,
        expiry,
        payload,
        30000u64.into(),
    );

    BlockItem::AccountTransaction(tx)
}

// pub async fn update_keys_exp() -> anyhow::Result<()> {
//     // let data_path = Path::new("./data/sigs_add_key.json");
//     let data_path = Path::new("./data/sigs_invoke.json");

//     let json = std::fs::read_to_string(data_path).unwrap();
//     let msg: UpdateKeys = serde_json::from_str(&json)?;

//     // message
//     // let message_encoded = "a11100000000000000000000000000000f006164644f70657261746f724b6579730000dc410d88010000440001000000b178059cc8633b0fee7846235071681f47b7272436dbacde903e09ea3720dc8c1c73d4c58e7b23c4594018a3cee35fb7cc1209fac4bfe2b98b53a56338b42e39";
//     let message_encoded = msg.message;
//     let message_byte = hex::decode(&message_encoded).unwrap();
//     let message = from_bytes::<PermitMessageWithParameter>(&message_byte).unwrap();

//     // signatures for update
//     let mut signatures = BTreeSet::new();
//     for v in msg.sigs {
//         let addr = account_address_from_str(&v.address).unwrap();
//         let sig: [u8; 64] = vec_to_arr(hex::decode(v.signature).unwrap());
//         signatures.insert((addr, SignatureEd25519(sig)));
//     }

//     // Parameter for Tx
//     let amount = Amount::zero();
//     let address = CA::new(INDEX_OPERATOR, 0);
//     let receive_name = OwnedReceiveName::new_unchecked(
//         format!("{}.{}", CONTRACT_OPERATOR, msg.method).to_string(),
//     );

//     let params = operators::ParamsWithSignatures {
//         signatures,
//         message,
//     };
//     let params_byte = OwnedParameter::try_from(to_bytes(&params)).unwrap();

//     // -------------------------------------------------------

//     // sender
//     let keys1: WalletAccount = WalletAccount::from_json_file("./keys/keys.json")
//         .context("Could not read the keys file.")?;

//     let mut client = Client::new(Endpoint::from_static(NODE_ENDPOINT_V2))
//         .await
//         .context("Cannot connect.")?;
//     // let consensus_info: ConsensusInfo = client.get_consensus_info().await?;

//     let acc_info: AccountInfo = client
//         .get_account_info(&keys1.address.into(), &BlockIdentifier::Best)
//         .await
//         .context("Cannot connect.")?
//         .response;

//     let nonce = acc_info.account_nonce;
//     let expiry: TransactionTime =
//         TransactionTime::from_seconds((chrono::Utc::now().timestamp() + 300) as u64);

//     // -------------------------------------------------------
//     // Tx
//     let payload = UpdateContractPayload {
//         amount,
//         address,
//         receive_name,
//         message: params_byte,
//     };

//     let tx = send::update_contract(
//         &keys1,
//         keys1.address,
//         nonce,
//         expiry,
//         payload,
//         10000u64.into(),
//     );
//     let item = BlockItem::AccountTransaction(tx);

//     // -------------------------------------------------------
//     // Broadcast
//     let transaction_hash = client.send_block_item(&item).await?;
//     println!(
//         "Transaction {} submitted (nonce = {}).",
//         transaction_hash, nonce,
//     );
//     let (bh, bs) = client.wait_until_finalized(&transaction_hash).await?;
//     println!("Transaction finalized in block {}.", bh);
//     println!("The outcome is {:#?}", bs);

//     Ok(())
// }
