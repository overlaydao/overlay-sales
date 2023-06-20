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
use concordium_wasm::artifact::Artifact;
use serde::Deserialize;
use std::{fs::File, str::FromStr};
use std::{io::Read, path::PathBuf};

use config::*;
use tester::*;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    utils::init_logger();

    let energy = InterpreterEnergy::from(1_000_000);

    // Chain Context
    let mut modules = std::collections::HashMap::new();
    let mut chain = context::ChainContext { modules };
    let mut balances = std::collections::HashMap::new();
    let mut balances = context::BalanceContext { balances };
    // ====================================================================================
    // Prepare for chain context - Instantiate
    // ====================================================================================
    let team_ovl = "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1";
    let proj_admin = "4HuP25JqvP77bYaedygbXCD9YYwuFwWKFT5gkvHY5EC5Lxij3n";
    let user_1 = "2xBpaHottqhwFZURMZW4uZfWhg5fNFPhozzS1hYYbAHzJ5CCyn";
    let user_2 = "2yxPwev4mVd8yUYUTKWXFR68qBDgqd2mdEg9WErdW6eqRHL9JA";
    let user_3 = "3BeTZDN3FVLyvJinyMMbYr37o5aXThKfVkXXPxUhe6pLz1CMFD";
    let user_4 = "3uxeCZwa3SxbksPWHwXWxCsaPucZdzNaXsRbkztqUUYRo1MnvF";

    balances.faucet(user_1, 100000);
    balances.faucet(user_2, 100000);
    balances.faucet(user_3, 100000);
    balances.faucet(user_4, 100000);

    let exid = "11";

    // RIDO_USDC_PUBLIC
    let pkg = "ovl-sale-ccd-public";
    let module_file = format!(
        "../../{}/target/concordium/wasm32-unknown-unknown/release/{}.wasm.v1",
        pkg,
        pkg.to_lowercase().replace('-', "_")
    );
    chain.add_instance(
        INDEX_PUB_RIDO_CCD,
        CONTRACT_PUB_RIDO_CCD,
        module_file,
        AccountAddress::from_str(team_ovl)?,
        format!("./p/{}/rido_ccd/", exid),
        env::init::InitEnvironment {
            slot_time: "2023-05-28T06:00:00Z",
            context_file: None,
            param_file: Some("init.json"),
            state_out_file: "state.bin",
        },
        Amount::zero(),
        energy,
    );

    // RROJECT_TOKEN
    let pkg = "cis2-ovl";
    let module_file = format!(
        "../../../{}/target/concordium/wasm32-unknown-unknown/release/{}.wasm.v1",
        pkg,
        pkg.to_lowercase().replace('-', "_")
    );
    chain.add_instance(
        INDEX_PROJECT_TOKEN,
        CONTRACT_PROJECT_TOKEN,
        module_file,
        AccountAddress::from_str(proj_admin)?,
        format!("./p/{}/pjtoken/", exid),
        env::init::InitEnvironment {
            slot_time: "2023-05-28T06:00:00Z",
            context_file: None,
            param_file: Some("p_init.json"),
            state_out_file: "state.bin",
        },
        Amount::zero(),
        energy,
    );

    // ====================================================================================
    // Prepare
    // ====================================================================================

    // ====================================================================================
    // Receive
    // ====================================================================================
    // before sale
    let envs = [
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_CCD,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: team_ovl,
            entry_point: "view",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_CCD,
            slot_time: "2023-05-30T06:00:00Z",
            invoker: team_ovl,
            entry_point: "whitelisting",
            param_file: Some("wl.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_CCD,
            slot_time: "2023-06-01T12:00:00Z",
            invoker: user_1,
            entry_point: "userDeposit",
            param_file: None,
            amount: Amount::from_micro_ccd(100000),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_CCD,
            slot_time: "2023-06-02T12:00:00Z",
            invoker: user_2,
            entry_point: "userDeposit",
            param_file: None,
            amount: Amount::from_micro_ccd(100000),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_CCD,
            slot_time: "2023-06-05T11:59:59Z",
            invoker: user_3,
            entry_point: "userDeposit",
            param_file: None,
            amount: Amount::from_micro_ccd(100000),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_CCD,
            slot_time: "2023-06-05T12:00:00Z",
            invoker: user_4,
            entry_point: "userDeposit",
            param_file: None,
            amount: Amount::from_micro_ccd(100000),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_CCD,
            slot_time: "2023-06-01T06:00:00Z",
            invoker: team_ovl,
            entry_point: "viewParticipants",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_CCD,
            slot_time: "2023-06-05T12:00:01Z",
            invoker: team_ovl,
            entry_point: "setFixed",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_CCD,
            slot_time: "2023-06-10T12:00:00Z",
            invoker: proj_admin,
            entry_point: "projectClaim",
            param_file: None,
            ..Default::default()
        },
    ];

    for env in envs {
        env.do_call(&chain, &mut balances, env.amount, energy)?;
    }

    Ok(())
}
