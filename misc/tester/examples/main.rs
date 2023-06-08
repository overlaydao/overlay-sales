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

    let amount = Amount::zero();
    let energy = InterpreterEnergy::from(1_000_000);

    // Chain Context
    let mut modules = std::collections::HashMap::new();
    let mut chain = context::ChainContext { modules };
    let mut balances = std::collections::HashMap::new();
    let mut balances = context::BalanceContext { balances };
    // ====================================================================================
    // Prepare for chain context - Instantiate
    // ====================================================================================
    // USDC
    let pkg = "cis2-bridgeable";
    let module_file = format!(
                "../../../eth-ccd-bridge/concordium_contracts/{}/target/concordium/wasm32-unknown-unknown/release/{}.wasm.v1",
                pkg,
                pkg.to_lowercase().replace('-', "_")
            );
    chain.add_instance(
        3496,
        CONTRACT_USDC,
        module_file,
        AccountAddress::from_str("3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1")?,
        "./p/0/usdc/".to_string(),
        env::init::InitEnvironment {
            slot_time: "2023-05-28T06:00:00Z",
            context_file: None,
            param_file: Some("p_init.json"),
            state_out_file: "state.bin",
        },
        amount,
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
        10,
        CONTRACT_PUB_RIDO_USDC,
        module_file,
        AccountAddress::from_str("3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1")?,
        "./p/0/rido_usdc/".to_string(),
        env::init::InitEnvironment {
            slot_time: "2023-05-28T06:00:00Z",
            context_file: None,
            param_file: Some("p_init_pub_usdc.json"),
            state_out_file: "state.bin",
        },
        amount,
        energy,
    );

    // ====================================================================================
    // Receive
    // ====================================================================================

    let envs = vec![
        env::receive::ReceiveEnvironment {
            contract_index: 3496,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1",
            entry_point: "grantRole",
            param_file: Some("p_grant_role.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: 3496,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1",
            entry_point: "deposit",
            param_file: Some("p_deposit.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: 3496,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1",
            entry_point: "deposit",
            param_file: Some("p_deposit.json"),
            ..Default::default()
        },
    ];

    for env in envs {
        env.do_call(&chain, &mut balances, amount, energy)?;
    }

    let envs = vec![
        env::receive::ReceiveEnvironment {
            contract_index: 10,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1",
            entry_point: "setStatus",
            param_file: Some("p_set_status.json"),
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: 3496,
            slot_time: "2023-05-28T06:11:00Z",
            invoker: "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1",
            entry_point: "transfer", // invoke userDeposit
            param_file: Some("p_transfer_contract.json"),
            ..Default::default()
        },
    ];

    for env in envs {
        env.do_call(&chain, &mut balances, amount, energy)?;
    }

    let envs = vec![
        env::receive::ReceiveEnvironment {
            contract_index: 10,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1",
            entry_point: "view",
            param_file: None,
            ..Default::default()
        },
        env::receive::ReceiveEnvironment {
            contract_index: 3496,
            slot_time: "2023-05-28T06:00:00Z",
            invoker: "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1",
            entry_point: "balanceOf",
            param_file: Some("p_balanceof.json"),
            ..Default::default()
        },
    ];

    for env in envs {
        env.do_call(&chain, &mut balances, amount, energy)?;
    }

    // =======================================================================

    // let path = Path::new("test");
    // let mut files = Vec::new();
    // traverse(path, &mut |e| files.push(e)).unwrap();
    // for file in files {
    //     println!("{:?}", file);
    // }

    // ----------------------------------------------------------------
    // Init in real node
    // ------------------------

    // // Wasm Module from node
    // let endpoint = Endpoint::from_static(NODE_ENDPOINT_V2);
    // let wasm_module: WasmModule =
    //     utils::get_wasm_module_from_node(endpoint, MODREF_OPERATOR).await?;

    // let endpoint = Endpoint::from_static(NODE_ENDPOINT_V2);
    // let schema_ops = cmd::node::get_module(endpoint.clone(), MODREF_OPERATOR).await?;
    // let types_ops = &schema_ops.get_init_param_schema(CONTRACT_OPERATOR)?;

    // let schema_usdc = cmd::node::get_module(endpoint, MODREF_PUB_RIDO_USDC).await?;
    // let types_usdc = &schema_usdc.get_init_param_schema(CONTRACT_PUB_RIDO_USDC)?;

    // let mut parameter_bytes = Vec::new();
    // let parameter_json = get_object_from_json("./test/init_pub_usdc.json".into())?;
    // types_usdc
    //     .serial_value_into(&parameter_json, &mut parameter_bytes)
    //     .context("Could not generate parameter bytes using schema and JSON.")?;

    // let summary =
    //     cmd::smc::initialize(MODREF_OPERATOR, CONTRACT_OPERATOR, parameter_bytes).await?;

    // println!("{:?}", summary);

    Ok(())
}
