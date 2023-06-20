#![allow(unused)]
use anyhow::{bail, Context, Result};
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

    let exid = "1001";
    let contract_name = "piggy_auction";

    // let index_aution = 1;
    // let name_aution = "auction";
    // chain.add_instance(
    //     index_aution,
    //     name_aution,
    //     format!(
    //         "../../../contract_test/piggy-auction/target/concordium/wasm32-unknown-unknown/release/{}.wasm.v1",
    //         contract_name,
    //     ),
    //     AccountAddress::from_str(team_ovl)?,
    //     format!("./p/{}/a/", exid),
    //     env::init::InitEnvironment {
    //         slot_time: "2023-05-28T06:00:00Z",
    //         context_file: None,
    //         param_file: Some("init.json"),
    //         state_out_file: "state.bin",
    //     },
    //     Amount::zero(),
    //     energy,
    // );

    let index_piggy = 2;
    let name_piggy = "piggy_auction";
    chain.add_instance(
        index_piggy,
        name_piggy,
        format!(
            "../../../contract_test/piggy-auction/target/concordium/wasm32-unknown-unknown/release/{}.wasm.v1",
            contract_name,
        ),
        AccountAddress::from_str(team_ovl)?,
        format!("./p/{}/p/", exid),
        env::init::InitEnvironment {
            slot_time: "2023-05-28T06:00:00Z",
            context_file: None,
            param_file: Some("init.json"),
            state_out_file: "state.bin",
        },
        Amount::zero(),
        energy,
    )?;

    let index_weather = 3;
    let name_weather = "weather";
    chain.add_instance(
        index_weather,
        name_weather,
        format!(
            "../../../contract_test/piggy-auction/target/concordium/wasm32-unknown-unknown/release/{}.wasm.v1",
            contract_name,
        ),
        AccountAddress::from_str(team_ovl)?,
        format!("./p/{}/w/", exid),
        env::init::InitEnvironment {
            slot_time: "2023-05-28T06:00:00Z",
            context_file: None,
            param_file: None,
            state_out_file: "state.bin",
        },
        Amount::zero(),
        energy,
    )?;

    // ====================================================================================
    // Receive
    // ====================================================================================

    let envs = [
        // env::receive::ReceiveEnvironment {
        //     contract_index: index_piggy,
        //     slot_time: "2023-05-28T06:00:00Z",
        //     invoker: team_ovl,
        //     entry_point: "change",
        //     param_file: Some("change.json"),
        //     ..Default::default()
        // },
        // env::receive::ReceiveEnvironment {
        //     contract_index: index_piggy,
        //     slot_time: "2023-05-28T06:00:00Z",
        //     invoker: team_ovl,
        //     entry_point: "view",
        //     param_file: None,
        //     ..Default::default()
        // },
        // env::receive::ReceiveEnvironment {
        //     contract_index: index_weather,
        //     slot_time: "2023-05-28T06:00:00Z",
        //     invoker: team_ovl,
        //     entry_point: "view",
        //     param_file: None,
        //     ..Default::default()
        // },
        // env::receive::ReceiveEnvironment {
        //     contract_index: index_weather,
        //     slot_time: "2023-05-28T06:00:00Z",
        //     invoker: team_ovl,
        //     entry_point: "get",
        //     param_file: Some("get_none.json"),
        //     ..Default::default()
        // },
        // env::receive::ReceiveEnvironment {
        //     contract_index: index_weather,
        //     slot_time: "2023-05-28T06:00:00Z",
        //     invoker: team_ovl,
        //     entry_point: "set",
        //     param_file: Some("set.json"),
        //     ..Default::default()
        // },
        env::receive::ReceiveEnvironment {
            contract_index: index_weather,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: team_ovl,
            entry_point: "get",
            param_file: Some("get_some.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: index_piggy,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: team_ovl,
            entry_point: "view",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: index_weather,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: team_ovl,
            entry_point: "view",
            param_file: None,
            ..Default::default()
        },
    ];

    for env in envs {
        env.do_call(&chain, &mut balances, env.amount, energy)?;
    }

    Ok(())
}
