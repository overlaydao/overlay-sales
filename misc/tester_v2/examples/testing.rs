#![allow(unused)]
use anyhow::{Context, Result};
use clap::Parser;
use concordium_base::smart_contracts::WasmModule;
use concordium_contracts_common::{
    schema::VersionedModuleSchema, AccountAddress, Amount, OwnedParameter, OwnedReceiveName,
    Timestamp,
};
use concordium_smart_contract_engine::{
    v1::{self, ReturnValue},
    InterpreterEnergy,
};
use concordium_smart_contract_testing::*;
use concordium_wasm::{artifact::Artifact, validate::ValidateImportExport};
use serde::Deserialize;
use sha2::digest::typenum::Mod;
use std::{collections::HashMap, fs::File, str::FromStr, vec};
use std::{io::Read, path::PathBuf};

use config::*;
use tester_v2::{chain::ModuleInfo, *};

const EXAMPLE_ID: &str = "1";

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    utils::init_logger();

    let team_ovl = AccountAddress::from_str("3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1")?;
    let usdc_owner =
        AccountAddress::from_str("3uxeCZwa3SxbksPWHwXWxCsaPucZdzNaXsRbkztqUUYRo1MnvF")?;
    let proj_admin =
        AccountAddress::from_str("4HuP25JqvP77bYaedygbXCD9YYwuFwWKFT5gkvHY5EC5Lxij3n")?;
    let user_1 = AccountAddress::from_str("2xBpaHottqhwFZURMZW4uZfWhg5fNFPhozzS1hYYbAHzJ5CCyn")?;
    let user_2 = AccountAddress::from_str("2yxPwev4mVd8yUYUTKWXFR68qBDgqd2mdEg9WErdW6eqRHL9JA")?;
    let user_3 = AccountAddress::from_str("3BeTZDN3FVLyvJinyMMbYr37o5aXThKfVkXXPxUhe6pLz1CMFD")?;

    let mut chain = chain::generate_chain(
        vec![
            (team_ovl, AMOUNT_INIT),
            (usdc_owner, AMOUNT_INIT),
            (proj_admin, AMOUNT_INIT),
            (user_1, AMOUNT_INIT),
            (user_2, AMOUNT_INIT),
            (user_3, AMOUNT_INIT),
        ],
        Some("2023-05-28T06:00:00Z"),
    );

    let mut contracts: HashMap<u64, chain::InstanceInfo> = HashMap::new();

    // -------------------------------------------------------
    // Init
    // -------------------------------------------------------
    let module0: ModuleInfo = chain::deploy_module("ovl_sale_usdc_public", &team_ovl, &mut chain)?;
    let module1: ModuleInfo = chain::deploy_module("cis2_bridgeable", &usdc_owner, &mut chain)?;
    let module2: ModuleInfo = chain::deploy_module("cis2_ovl", &proj_admin, &mut chain)?;

    let id_rido_usdc_pub = 0;
    let instance: chain::InstanceInfo = module0.initialize(
        chain::InitEnvironment {
            id: id_rido_usdc_pub,
            contract_name: CONTRACT_PUB_RIDO_USDC,
            data_dir: format!("./p/{}/rido/", EXAMPLE_ID),
            owner: team_ovl,
            param_file: Some("p_init_pub_usdc.json"),
        },
        &mut chain,
    )?;
    contracts.insert(id_rido_usdc_pub, instance);

    let id_usdc = 1;
    let instance: chain::InstanceInfo = module1.initialize(
        chain::InitEnvironment {
            id: id_usdc,
            contract_name: CONTRACT_USDC,
            data_dir: format!("./p/{}/usdc/", EXAMPLE_ID),
            owner: usdc_owner,
            param_file: Some("p_init.json"),
        },
        &mut chain,
    )?;
    contracts.insert(id_usdc, instance);

    let id_pjtoken = 2;
    let instance: chain::InstanceInfo = module2.initialize(
        chain::InitEnvironment {
            id: id_pjtoken,
            contract_name: CONTRACT_PROJECT_TOKEN,
            data_dir: format!("./p/{}/pjtoken/", EXAMPLE_ID),
            owner: proj_admin,
            param_file: Some("p_init.json"),
        },
        &mut chain,
    )?;
    contracts.insert(id_pjtoken, instance);

    // -------------------------------------------------------
    // Receive
    // -------------------------------------------------------
    let envs = vec![
        chain::UpdateEnvironment {
            contract_index: id_usdc,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: usdc_owner,
            entry_point: "grantRole",
            param_file: Some("p_grant_role.json"),
        },
        chain::UpdateEnvironment {
            contract_index: id_usdc,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: usdc_owner,
            entry_point: "deposit",
            param_file: Some("p_deposit.json"),
        },
        // chain::UpdateEnvironment {
        //     contract_index: id_usdc,
        //     slot_time: "2023-05-28T06:00:00Z",
        //     invoker: user_1,
        //     entry_point: "balanceOf",
        //     param_file: Some("p_balanceof_user1.json"),
        // },
        chain::UpdateEnvironment {
            contract_index: id_usdc,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: usdc_owner,
            entry_point: "deposit",
            param_file: Some("p_deposit2.json"),
        },
        chain::UpdateEnvironment {
            contract_index: id_usdc,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: usdc_owner,
            entry_point: "deposit",
            param_file: Some("p_deposit3.json"),
        },
        chain::UpdateEnvironment {
            contract_index: id_pjtoken,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: proj_admin,
            entry_point: "mint",
            param_file: Some("p_mint.json"),
        },
        // chain::UpdateEnvironment {
        //     contract_index: id_pjtoken,
        //     slot_time: "2023-05-28T06:00:00Z",
        //     invoker: proj_admin,
        //     entry_point: "balanceOf",
        //     param_file: Some("p_balanceof_pj.json"),
        // },
        // -----------------------------------
        // sale
        chain::UpdateEnvironment {
            contract_index: id_rido_usdc_pub,
            invoker: team_ovl,
            entry_point: "whitelisting",
            param_file: Some("p_whitelisted.json"),
            slot_time: "2023-05-28T06:00:00Z",
        },
        chain::UpdateEnvironment {
            //user top
            contract_index: id_usdc,
            slot_time: "2023-06-01T12:00:00Z",
            invoker: user_1,
            entry_point: "transfer", // invoke userDeposit
            param_file: Some("p_transfer_contract.json"),
        },
        chain::UpdateEnvironment {
            //user second
            contract_index: id_usdc,
            slot_time: "2023-06-02T12:00:00Z",
            invoker: user_2,
            entry_point: "transfer", // invoke userDeposit
            param_file: Some("p_transfer_contract2.json"),
        },
        chain::UpdateEnvironment {
            //user any
            contract_index: id_usdc,
            slot_time: "2023-06-04T12:00:00Z",
            invoker: user_3,
            entry_point: "transfer", // invoke userDeposit
            param_file: Some("p_transfer_contract3.json"),
        },
        chain::UpdateEnvironment {
            contract_index: id_rido_usdc_pub,
            slot_time: "2023-06-05T12:01:00Z",
            invoker: team_ovl,
            entry_point: "setFixed",
            param_file: None,
        },
        // -----------------------------------
        // view
        chain::UpdateEnvironment {
            contract_index: id_rido_usdc_pub,
            slot_time: "2023-07-01T06:00:00Z",
            invoker: team_ovl,
            entry_point: "view",
            param_file: None,
        },
        chain::UpdateEnvironment {
            contract_index: id_rido_usdc_pub,
            slot_time: "2023-07-01T06:00:00Z",
            invoker: user_1,
            entry_point: "viewWinUnits",
            param_file: None,
        },
        chain::UpdateEnvironment {
            contract_index: id_rido_usdc_pub,
            slot_time: "2023-07-01T06:00:00Z",
            invoker: team_ovl,
            entry_point: "viewParticipants",
            param_file: None,
        },
    ];

    for env in envs {
        let r = contracts
            .get_mut(&env.contract_index)
            .unwrap()
            .update(env, &mut chain)?;
    }

    // let x = chain.contract_balance(instance1.it.contract_address);
    // println!("{:?}", x);

    Ok(())
}
