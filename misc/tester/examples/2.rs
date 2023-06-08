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
    let usdc_owner = "3uxeCZwa3SxbksPWHwXWxCsaPucZdzNaXsRbkztqUUYRo1MnvF";
    let team_ovl = "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1";
    let proj_admin = "4HuP25JqvP77bYaedygbXCD9YYwuFwWKFT5gkvHY5EC5Lxij3n";
    let user_1 = "2xBpaHottqhwFZURMZW4uZfWhg5fNFPhozzS1hYYbAHzJ5CCyn";
    let user_2 = "2yxPwev4mVd8yUYUTKWXFR68qBDgqd2mdEg9WErdW6eqRHL9JA";
    let user_3 = "3BeTZDN3FVLyvJinyMMbYr37o5aXThKfVkXXPxUhe6pLz1CMFD";

    let exid = "2";

    // USDC
    let pkg = "cis2-bridgeable";
    let module_file = format!(
                "../../../eth-ccd-bridge/concordium_contracts/{}/target/concordium/wasm32-unknown-unknown/release/{}.wasm.v1",
                pkg,
                pkg.to_lowercase().replace('-', "_")
            );
    chain.add_instance(
        INDEX_USDC,
        CONTRACT_USDC,
        module_file,
        AccountAddress::from_str(usdc_owner)?,
        format!("./p/{}/usdc/", exid),
        env::init::InitEnvironment {
            slot_time: "2023-05-28T06:00:00Z",
            context_file: None,
            param_file: Some("p_init.json"),
            state_out_file: "state.bin",
        },
        Amount::zero(),
        energy,
    );

    // RIDO_USDC_PUBLIC
    let pkg = "ovl-sale-usdc-public";
    let module_file = format!(
        "../../{}/target/concordium/wasm32-unknown-unknown/release/{}.wasm.v1",
        pkg,
        pkg.to_lowercase().replace('-', "_")
    );
    chain.add_instance(
        INDEX_PUB_RIDO_USDC,
        CONTRACT_PUB_RIDO_USDC,
        module_file,
        AccountAddress::from_str(team_ovl)?,
        format!("./p/{}/rido/", exid),
        env::init::InitEnvironment {
            slot_time: "2023-05-28T06:00:00Z",
            context_file: None,
            param_file: Some("p_init_pub_usdc.json"),
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
    let envs = vec![
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_USDC,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: usdc_owner,
            entry_point: "grantRole",
            param_file: Some("p_grant_role.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_USDC,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: usdc_owner,
            entry_point: "deposit",
            param_file: Some("p_deposit.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_USDC,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: usdc_owner,
            entry_point: "deposit",
            param_file: Some("p_deposit2.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_USDC,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: usdc_owner,
            entry_point: "deposit",
            param_file: Some("p_deposit3.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PROJECT_TOKEN,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: proj_admin,
            entry_point: "mint",
            param_file: Some("p_mint.json"),
            ..Default::default()
        },
        // ====================================================================================
        // Receive
        // ====================================================================================
        // before sale
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-05-30T06:00:00Z",
            invoker: team_ovl,
            entry_point: "whitelisting",
            param_file: Some("p_whitelisted.json"),
            ..Default::default()
        },
        // during sale
        env::receive::ReceiveEnvironment {
            //user top
            contract_index: INDEX_USDC,
            slot_time: "2023-06-01T12:00:00Z",
            invoker: user_1,
            entry_point: "transfer", // invoke userDeposit
            param_file: Some("p_transfer_contract.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            //user second
            contract_index: INDEX_USDC,
            slot_time: "2023-06-02T12:00:00Z",
            invoker: user_2,
            entry_point: "transfer", // invoke userDeposit
            param_file: Some("p_transfer_contract2.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            //user any
            contract_index: INDEX_USDC,
            slot_time: "2023-06-04T12:00:00Z",
            invoker: user_3,
            entry_point: "transfer", // invoke userDeposit
            param_file: Some("p_transfer_contract3.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-06-05T12:01:00Z",
            invoker: team_ovl,
            entry_point: "setFixed",
            param_file: None,
            ..Default::default()
        },
        // ---------------------------------------
        // Project Claim
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_USDC,
            slot_time: "2023-06-05T06:00:00Z",
            invoker: team_ovl,
            entry_point: "balanceOf",
            param_file: Some("p_balanceof_contract.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-06-10T12:00:00Z",
            invoker: proj_admin,
            entry_point: "projectClaim",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_USDC,
            slot_time: "2023-06-25T12:10:00Z",
            invoker: proj_admin,
            entry_point: "balanceOf",
            param_file: Some("p_balanceof_pj.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-06-15T12:01:00Z",
            invoker: proj_admin,
            entry_point: "setPjtoken",
            param_file: Some("p_set_pjtoken.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-07-01T06:00:00Z",
            invoker: team_ovl,
            entry_point: "view",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-06-15T12:00:00Z",
            invoker: proj_admin,
            entry_point: "setTGE",
            param_file: Some("p_set_tge.json"),
            ..Default::default()
        },
        // ---------------------------------------
        // Vesting
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PROJECT_TOKEN,
            slot_time: "2023-06-20T06:00:00Z",
            invoker: team_ovl,
            entry_point: "balanceOf",
            param_file: Some("p_balanceof_ovl.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PROJECT_TOKEN,
            slot_time: "2023-06-21T12:00:00Z",
            invoker: proj_admin,
            entry_point: "transfer", // invoke createPool
            param_file: Some("p_transfer_create_pool.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PROJECT_TOKEN,
            slot_time: "2023-06-21T12:00:00Z",
            invoker: team_ovl,
            entry_point: "balanceOf",
            param_file: Some("p_balanceof_contract.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-06-24T12:00:00Z",
            invoker: team_ovl,
            entry_point: "ovlClaim",
            param_file: None,
            ..Default::default()
        },
        //
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-06-24T12:00:00Z",
            invoker: user_1,
            entry_point: "userClaim",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PROJECT_TOKEN,
            slot_time: "2023-06-24T12:10:00Z",
            invoker: team_ovl,
            entry_point: "balanceOf",
            param_file: Some("p_balanceof_user1.json"),
            ..Default::default()
        },
        //
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-06-25T12:00:00Z",
            invoker: user_1,
            entry_point: "userClaim",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PROJECT_TOKEN,
            slot_time: "2023-06-25T12:10:00Z",
            invoker: team_ovl,
            entry_point: "balanceOf",
            param_file: Some("p_balanceof_user1.json"),
            ..Default::default()
        },
        //
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-06-26T12:00:00Z",
            invoker: user_1,
            entry_point: "userClaim",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PROJECT_TOKEN,
            slot_time: "2023-06-26T12:10:00Z",
            invoker: team_ovl,
            entry_point: "balanceOf",
            param_file: Some("p_balanceof_user1.json"),
            ..Default::default()
        },
        //
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-06-28T12:00:00Z",
            invoker: user_1,
            entry_point: "userClaim",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PROJECT_TOKEN,
            slot_time: "2023-06-28T12:10:00Z",
            invoker: team_ovl,
            entry_point: "balanceOf",
            param_file: Some("p_balanceof_user1.json"),
            ..Default::default()
        },
    ];

    let envs2 = vec![
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PROJECT_TOKEN,
            slot_time: "2023-07-01T12:10:00Z",
            invoker: team_ovl,
            entry_point: "balanceOf",
            param_file: Some("p_balanceof_ovl.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-07-01T06:00:00Z",
            invoker: team_ovl,
            entry_point: "view",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-07-01T06:00:00Z",
            invoker: user_1,
            entry_point: "viewWinUnits",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-07-01T06:00:00Z",
            invoker: team_ovl,
            entry_point: "setStatus",
            param_file: Some("p_set_status.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_USDC,
            slot_time: "2023-07-01T06:00:00Z",
            invoker: team_ovl,
            entry_point: "balanceOf",
            param_file: Some("p_balanceof_contract.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-07-01T06:00:00Z",
            invoker: user_1,
            entry_point: "userQuit",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_PUB_RIDO_USDC,
            slot_time: "2023-07-01T06:00:00Z",
            invoker: team_ovl,
            entry_point: "viewParticipants",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: INDEX_USDC,
            slot_time: "2023-07-01T06:00:00Z",
            invoker: team_ovl,
            entry_point: "balanceOf",
            param_file: Some("p_balanceof_ovl.json"),
            ..Default::default()
        },
    ];

    let amount = Amount::from_micro_ccd(0);

    for env in envs {
        env.do_call(&chain, &mut balances, amount, energy)?;
    }

    for env in envs2 {
        env.do_call(&chain, &mut balances, amount, energy)?;
    }

    // =======================================================================

    // let current_state: v1::trie::PersistentState = {
    //     let f = format!("{}{}", "./p/1/rido/", "state.bin");
    //     let state_bin = std::fs::File::open(f).context("Could not read state file.")?;
    //     let mut reader = std::io::BufReader::new(state_bin);

    //     v1::trie::PersistentState::deserialize(&mut reader)
    //         .context("Could not deserialize the provided state.")?
    // };

    // let mut loader = v1::trie::Loader::new(&[][..]);
    // let mut mutable_state = current_state.thaw();
    // let mut collector = v1::trie::SizeCollector::default();
    // let frozen = mutable_state.freeze(&mut loader, &mut collector);

    // let mut tree_builder = ptree::TreeBuilder::new("StateRoot".into());
    // frozen.display_tree(&mut tree_builder, &mut loader);
    // let tree = tree_builder.build();
    // println!("{:#?}", tree);

    // let path = Path::new("test");
    // let mut files = Vec::new();
    // traverse(path, &mut |e| files.push(e)).unwrap();
    // for file in files {
    //     println!("{:?}", file);
    // }

    Ok(())
}
