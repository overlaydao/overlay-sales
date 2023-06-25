#![allow(unused)]
pub mod chain;
pub mod config;
pub mod types;
pub mod utils;

use anyhow::{bail, Context, Result};
use chrono::DateTime;
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use concordium_rust_sdk::types::transactions::EncodedPayload;
use concordium_rust_sdk::types::BlockItemSummary;

use concordium_rust_sdk::{
    common::types::{KeyPair, Signature, TransactionTime},
    smart_contracts::common::Amount,
    types::{
        queries::ConsensusInfo,
        smart_contracts::{
            concordium_contracts_common::ContractAddress as CA, OwnedParameter, OwnedReceiveName,
        },
        transactions::{send, BlockItem, UpdateContractPayload},
        AccountInfo, WalletAccount,
    },
    v2::{Client, Endpoint},
};
use concordium_std::{
    from_bytes, to_bytes, AccountAddress, ContractAddress, OwnedEntrypointName, PublicKeyEd25519,
    SignatureEd25519, Timestamp,
};
use std::{
    convert::TryInto,
    ffi::OsStr,
    fs::read_dir,
    path::{Component, Path, PathBuf},
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
    // #[command(subcommand)]
    // pub command: Option<Commands>,
}

// #[derive(Subcommand, Debug)]
// pub enum Commands {
//     /// current timestamp plus h
//     Timestamp {
//         #[arg(short = 'o', long, default_value = "24")]
//         hour: i64,
//     },
// }
