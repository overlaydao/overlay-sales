#![allow(unused)]
use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::{fs::File, io::Read, path::PathBuf, str::FromStr};

use concordium_base::smart_contracts::WasmModule;
use concordium_contracts_common::{
    schema::VersionedModuleSchema, AccountAddress, Amount, OwnedParameter, OwnedReceiveName,
};
use concordium_rust_sdk::{
    smart_contracts::common::Timestamp,
    v2::{BlockIdentifier, Client, Endpoint},
};
use concordium_smart_contract_engine::{
    v1::{self, ReturnValue},
    InterpreterEnergy,
};
use concordium_smart_contract_testing::*;
use concordium_wasm::artifact::Artifact;

use config::*;
use tester::*;
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    // utils::init_logger();

    Ok(())
}
