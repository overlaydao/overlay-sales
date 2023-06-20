#![allow(unused)]
use anyhow::{Context, Result};
use clap::Parser;
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
use serde::Deserialize;
use std::{fs::File, str::FromStr};
use std::{io::Read, path::PathBuf};

use config::*;
use tester::*;

const EXAMPLE_ID: &str = "1";

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    utils::init_logger();

    let mut chain = chain::generate_chain(
        vec![(ACC_ADDR_OWNER, AMOUNT_INIT), (ACC_ADDR_OTHER, AMOUNT_INIT)],
        Some("2023-05-28T06:00:00Z"),
    );

    // -------------------------------------------------------
    // Init
    // -------------------------------------------------------
    let contract_name = CONTRACT_PUB_RIDO_USDC;
    let env = chain::InitEnvironment {
        id: 0,
        data_dir: format!("./p/{}/rido/", EXAMPLE_ID),
        owner: ACC_ADDR_OWNER,
        param_file: Some("p_init_pub_usdc.json"),
    };

    let module1 = chain::deploy_module(contract_name, "ovl_sale_usdc_public", &mut chain)?;
    let instance1: chain::InstanceInfo = module1.initialize(env, &mut chain)?;

    // -------------------------------------------------------
    // Receive
    // -------------------------------------------------------
    let env = chain::UpdateEnvironment {
        invoker: ACC_ADDR_OWNER,
        entry_point: "whitelisting",
        param_file: Some("p_init_pub_usdc.json"),
    };

    let r = module1.update(env, &instance1, &mut chain)?;
    println!("{:?}", r);

    let x = chain.contract_balance(instance1.it.contract_address);
    println!("{:?}", x);

    Ok(())
}
