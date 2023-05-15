#![allow(unused)]

pub mod cmd;
mod config;
mod types;

use crate::config::*;
use crate::types::*;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, FixedOffset, Local, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use concordium_rust_sdk::types::transactions::EncodedPayload;
use concordium_rust_sdk::{
    common::types::{KeyPair, Signature, TransactionTime},
    eddsa_ed25519,
    id::types::CredentialDataWithSigning,
    smart_contracts::common::Amount,
    types::{
        hashes::{HashBytes, ModuleReferenceMarker},
        queries::ConsensusInfo,
        smart_contracts::{
            concordium_contracts_common::AccountAddress as AA,
            concordium_contracts_common::ContractAddress as CA, ModuleReference, OwnedContractName,
            OwnedParameter, OwnedReceiveName,
        },
        transactions::{
            send, BlockItem, ExactSizeTransactionSigner, InitContractPayload, UpdateContractPayload,
        },
        AccountInfo, WalletAccount,
    },
    v2::{BlockIdentifier, Client, Endpoint},
};
use concordium_std::{
    from_bytes, to_bytes, AccountAddress, ContractAddress, OwnedEntrypointName, PublicKeyEd25519,
    SignatureEd25519, Timestamp,
};
use ed25519_dalek::{Keypair, PublicKey, Signature as EdSig, Signer, Verifier};
use rand::{rngs::OsRng, AsByteSliceMut};
use sale_utils::types::*;
use sha2::{Digest, Sha256};
use std::{
    collections::{hash_map, BTreeSet, HashMap},
    convert::TryInto,
    ffi::OsStr,
    fmt::{self},
    fs::{create_dir_all, read_dir, read_to_string, File},
    hash,
    io::{self, BufWriter, Cursor, Write},
    path::{Component, Path, PathBuf},
    str::FromStr,
};

/// Simple program to greet a person(About Text)
#[derive(Parser, Debug)]
#[command(name = "OvlSigner", author = "newsnow", version, long_about = None)]
// #[command(next_line_help = true)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Custom config file if needed
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Flag
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum MessageType {
    None,
    AddKey,
    RemoveKey,
    Invoke,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// node v2 information
    Nodeinfo {
        #[arg(long = "node", default_value = NODE_ENDPOINT_V2)]
        endpoint: Endpoint,
    },
    /// current timestamp plus h
    Timestamp {
        #[arg(short = 'o', long, default_value = "24")]
        hour: i64,
    },
    /// key generation
    Keygen {
        #[arg(short, default_value = "keys", value_name = "filename")]
        filename: String,
    },
    /// sign
    Sign {
        #[arg(
            short,
            long = "signkey",
            default_value = "./keys/keys.json",
            value_name = "PATH"
        )]
        keys: PathBuf,
        #[arg(short, long, value_name = "PATH")]
        payload: Option<PathBuf>,
        #[arg(short, long, value_enum, default_value_t=MessageType::AddKey)]
        mode: MessageType,
    },
    /// check message
    Confirm {
        encoded_msg: String,
        #[arg(short, long, value_enum, default_value_t=MessageType::None)]
        mode: MessageType,
    },
    /// init contract
    Init { contract: String },
    /// update contract
    UpdateKey,
    /// update contract
    UpdateInvoke,
    /// update contract for test
    UpdateKeyTest {
        #[arg(short, default_value = "add")]
        mode: String,
        // ex1: Option<String>,
        // ex2: Vec<String>,
    },
}

async fn broadcast(client: &mut Client, item: BlockItem<EncodedPayload>) -> anyhow::Result<()> {
    let transaction_hash = client.send_block_item(&item).await?;
    println!("Transaction {} submitted.", transaction_hash);

    let (bh, bs) = client.wait_until_finalized(&transaction_hash).await?;
    println!("Transaction finalized in block {}.", bh);
    println!("The outcome is {:#?}", bs);

    Ok(())
}

pub fn filepath_exp() -> Result<()> {
    // let file_str = "src/bin/path_is.rs";
    // let dir_str = "target";
    // let path_file = Path::new(file_str);
    // let path_dir = Path::new(dir_str);
    // println!("{} is file: {}", file_str, path_file.is_file());
    // println!("{} is directory: {}", dir_str, path_dir.is_dir());
    // println!("{} is absolute: {}", dir_str, path_dir.is_absolute());
    // println!("{} is relative: {}", dir_str, path_dir.is_relative());
    // println!("{} has root: {}", dir_str, path_dir.has_root());
    // println!("{} exists: {}", file_str, path_file.exists());
    // println!(
    //     "{} starts with 'src': {}",
    //     file_str,
    //     path_file.starts_with("src")
    // );
    // println!(
    //     "{} ends with 'path_is.rs': {}",
    //     file_str,
    //     path_file.ends_with("path_is.rs")
    // );

    let data_path = Path::new("data/msg_rm_key.json");
    if validate_file_path(&data_path) {
        bail!("Invalid json filepath.")
    }
    let json = std::fs::read_to_string(data_path).unwrap();
    println!("{:?}", json);

    Ok(())
}

pub fn datetime_exp() -> Result<()> {
    let date_time: NaiveDateTime = NaiveDate::from_ymd_opt(2017, 11, 12)
        .unwrap()
        .and_hms_opt(17, 33, 44)
        .unwrap();
    println!(
        "Number of seconds between 1970-01-01 00:00:00 and {} is {}.",
        date_time,
        date_time.timestamp()
    );

    // let d = NaiveDate::from_ymd_opt(2017, 11, 12).unwrap();
    // let t = NaiveTime::from_hms_milli_opt(17, 33, 44, 000).unwrap();
    // let dt: NaiveDateTime = d.and_time(t);
    // println!("{}", dt.timestamp_millis());

    // let no_timezone = NaiveDateTime::parse_from_str("2017-11-12 17:33:44", "%Y-%m-%d %H:%M:%S")?;
    // println!("{}", no_timezone.timestamp_millis());

    let rfc3339 = DateTime::parse_from_rfc3339("2017-11-12T17:33:44+00:00")?;
    println!("{}", rfc3339.timestamp_millis());
    println!("{}", rfc3339.format("%a %b %e %T %Y"));

    let rfc3339 = DateTime::parse_from_rfc3339("2017-11-12T17:33:44+09:00")?;
    let rfc3339 = rfc3339 + Duration::hours(9);
    println!("{}", rfc3339.timestamp_millis());
    println!("{}", rfc3339.format("%a %b %e %T %Y"));

    // let dt: Result<DateTime<FixedOffset>, _> =
    //     DateTime::parse_from_rfc3339("2018-12-07T19:31:28+09:00");
    // println!("DateTime::parse_from_rfc3339: {:?}", dt);

    // let dt: Result<DateTime<FixedOffset>, _> =
    //     DateTime::parse_from_str("2018/12/07 19:31:28 +0900", "%Y/%m/%d %H:%M:%S %z");
    // println!("DateTime::parse_from_str: {:?}", dt);

    // let dt: Result<NaiveDateTime, _> =
    //     NaiveDateTime::parse_from_str("2018/12/07 19:31:28", "%Y/%m/%d %H:%M:%S");
    // println!("NaiveDateTime::parse_from_str: {:?}", dt);

    Ok(())
}

pub fn get_keypair_from_wallet_keys(keys: &WalletAccount) -> anyhow::Result<&KeyPair> {
    // let x = keys.access_structure();
    let keypair: &KeyPair = keys
        .keys
        .keys
        .first_key_value()
        .unwrap()
        .1
        .keys
        .first_key_value()
        .unwrap()
        .1;
    Ok(keypair)
}

pub fn validate_file_path(path: &Path) -> bool {
    !path.exists()
        || path
            .components()
            .into_iter()
            .any(|x| x == Component::ParentDir)
        || path.is_absolute()
        || path.extension() != Some(OsStr::new("json"))
}

pub fn vec_to_arr<T, const N: usize>(v: Vec<T>) -> [T; N] {
    v.try_into()
        .unwrap_or_else(|v: Vec<T>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
}

pub fn timestamp_from_str(ts: &str) -> Result<Timestamp> {
    let time = DateTime::parse_from_rfc3339(ts)?;
    // let time = timelimit + Duration::hours(9);
    Ok(Timestamp::from_timestamp_millis(
        time.timestamp_millis() as u64
    ))
}

pub fn account_address_from_str(v: &str) -> Result<AccountAddress> {
    let mut buf = [0xff; 1 + ACCOUNT_ADDRESS_SIZE + 4];
    let len = bs58::decode(v).with_check(Some(1)).into(&mut buf)?;

    if len != 1 + ACCOUNT_ADDRESS_SIZE {
        bail!("invalid byte length");
    }

    let mut address_bytes = [0u8; ACCOUNT_ADDRESS_SIZE];
    address_bytes.copy_from_slice(&buf[1..1 + ACCOUNT_ADDRESS_SIZE]);
    Ok(AccountAddress(address_bytes))
}

// pub fn account_address_from_byte(bytes: [u8; 32]) -> Result<String> {
//     let mut encoded = String::with_capacity(50);
//     let mut decoded: Vec<u8> = [1].iter().chain(bytes.iter()).map(|v| *v).collect();
//     let decoded: [u8; 33] = decoded.try_into().unwrap();
//     bs58::encode(decoded).with_check().into(&mut encoded)?;
//     Ok(encoded)
// }

pub async fn update_add_key_exp(mode: &str) -> Result<()> {
    let keys3: WalletAccount = WalletAccount::from_json_file("./keys/keys3.json")
        .context("Could not read the keys file.")?;
    let keypair3: &KeyPair = keys3
        .keys
        .keys
        .first_key_value()
        .unwrap()
        .1
        .keys
        .first_key_value()
        .unwrap()
        .1;

    let mut new_ops: Vec<operators::OperatorWithKeyParam> = Vec::new();
    new_ops.push(operators::OperatorWithKeyParam {
        account: AccountAddress(keys3.address.0),
        public_key: PublicKeyEd25519(keypair3.public.to_bytes()),
    });

    // -------------------------------------------------------

    let keys1: WalletAccount = WalletAccount::from_json_file("./keys/keys.json")
        .context("Could not read the keys file.")?;
    let keypair1: &KeyPair = keys1
        .keys
        .keys
        .first_key_value()
        .unwrap()
        .1
        .keys
        .first_key_value()
        .unwrap()
        .1;

    let keys2: WalletAccount = WalletAccount::from_json_file("./keys/keys2.json")
        .context("Could not read the keys file.")?;
    let keypair2: &KeyPair = keys2
        .keys
        .keys
        .first_key_value()
        .unwrap()
        .1
        .keys
        .first_key_value()
        .unwrap()
        .1;

    // -------------------------------------------------------
    let index = INDEX_OPERATOR;
    let hour = (chrono::Utc::now().timestamp_millis() + 60 * 60 * 24 * 1000) as u64;

    let (ep_name, action) = match mode {
        "add" => ("addOperatorKeys", PermitAction::AddKey),
        "remove" => ("removeOperatorKeys", PermitAction::RemoveKey),
        _ => bail!("Invalid Mode"),
    };

    let message = PermitMessageWithParameter {
        contract_address: ContractAddress { index, subindex: 0 },
        entry_point: OwnedEntrypointName::new_unchecked(ep_name.into()),
        action,
        timestamp: Timestamp::from_timestamp_millis(hour),
        parameter: to_bytes(&new_ops),
    };
    let message_bytes = to_bytes(&message);
    let message_hash = Sha256::digest(&message_bytes);

    let sig1: Signature = keypair1.sign(&message_hash);
    let sig2: Signature = keypair2.sign(&message_hash);
    // let siged1: [u8; 64] = vec_to_arr(sig1.sig);
    // let siged2: [u8; 64] = vec_to_arr(sig2.sig);

    let sc1: String = hex::encode(sig1.as_ref());
    let siged1: [u8; 64] = vec_to_arr(hex::decode(&sc1).unwrap());

    let sc2: String = hex::encode(sig2.as_ref());
    let siged2: [u8; 64] = vec_to_arr(hex::decode(&sc2).unwrap());

    println!("{sc1:?}");
    println!("{siged1:?}");
    println!("{sc2:?}");
    println!("{siged2:?}");

    let mut signatures = BTreeSet::new();
    signatures.insert((AccountAddress(keys1.address.0), SignatureEd25519(siged1)));
    signatures.insert((AccountAddress(keys2.address.0), SignatureEd25519(siged2)));

    // -------------------------------------------------------

    let p = operators::ParamsWithSignatures {
        signatures,
        message,
    };

    let param = OwnedParameter::try_from(to_bytes(&p)).unwrap();

    // -------------------------------------------------------
    let mut client = Client::new(Endpoint::from_static(NODE_ENDPOINT_V2))
        .await
        .context("Cannot connect.")?;
    let consensus_info: ConsensusInfo = client.get_consensus_info().await?;

    let acc_info: AccountInfo = client
        .get_account_info(&keys1.address.into(), &BlockIdentifier::Best)
        .await
        .context("Cannot connect.")?
        .response;

    let nonce = acc_info.account_nonce;
    let expiry: TransactionTime =
        TransactionTime::from_seconds((chrono::Utc::now().timestamp() + 300) as u64);

    // -------------------------------------------------------
    let amount = Amount::zero();
    let receive_name =
        OwnedReceiveName::new_unchecked(format!("{}.{}", CONTRACT_OPERATOR, ep_name).to_string());

    let payload = UpdateContractPayload {
        amount,
        address: CA::new(INDEX_OPERATOR, 0),
        receive_name,
        message: param,
    };

    let tx = send::update_contract(
        &keys1,
        keys1.address,
        nonce,
        expiry,
        payload,
        10000u64.into(),
    );
    let item = BlockItem::AccountTransaction(tx);
    // let transaction_hash = item.hash();

    let transaction_hash = client.send_block_item(&item).await?;
    println!(
        "Transaction {} submitted (nonce = {}).",
        transaction_hash, nonce,
    );
    let (bh, bs) = client.wait_until_finalized(&transaction_hash).await?;
    println!("Transaction finalized in block {}.", bh);
    println!("The outcome is {:#?}", bs);
    Ok(())
}

// fn prompt(name: &str) -> String {
//     print!("{}", name);

//     let mut line = String::new();
//     std::io::stdout().flush().unwrap();
//     std::io::stdin()
//         .read_line(&mut line)
//         .expect("Error: Could not read a line");

//     return line.trim().to_string();
// }

// fn readline() -> anyhow::Result<String, String> {
//     write!(std::io::stdout(), "> ").map_err(|e| e.to_string())?;
//     std::io::stdout().flush().map_err(|e| e.to_string())?;
//     let mut buffer = String::new();
//     std::io::stdin()
//         .read_line(&mut buffer)
//         .map_err(|e| e.to_string())?;
//     Ok(buffer)
// }
// fn respond(line: &str) -> Result<bool, String> {
//     Ok(false)
// }
// fn test() -> () {
//     loop {
//         let input = prompt("> ");

//         // let line = readline().unwrap();
//         // let line = line.trim();
//         // if line.is_empty() {
//         //     continue;
//         // }
//         //   match respond(line) {
//         //     Ok(quit) => {
//         //         if quit {
//         //             break;
//         //         }
//         //     }
//         //     Err(err) => {
//         //         write!(std::io::stdout(), "{err}").map_err(|e| e.to_string())?;
//         //         std::io::stdout().flush().map_err(|e| e.to_string())?;
//         //     }
//         // }

//         if input == "now" {
//             let unixtime = SystemTime::now()
//                 .duration_since(SystemTime::UNIX_EPOCH)
//                 .unwrap();
//             print!("Current Unix time is {:?}\n", unixtime);
//         } else if input == "exit" {
//             break;
//         };
//     }
// }
