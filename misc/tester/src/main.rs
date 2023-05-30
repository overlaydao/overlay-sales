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

    match cli.command {
        Some(Commands::Timestamp { hour }) => {
            println!("{:?}", hour);
            Ok(())
        },
        None => {
            utils::init_logger();

            let amount = Amount::zero();
            let energy = InterpreterEnergy::from(1_000_000);

            // ====================================================================================
            // Init
            // ====================================================================================

            // USDC
            let pkg = "cis2-bridgeable";
            let module_file = format!(
                "../../../eth-ccd-bridge/concordium_contracts/{}/target/concordium/wasm32-unknown-unknown/release/{}.wasm.v1",
                pkg,
                pkg.to_lowercase().replace('-', "_")
            );
            let wasm_module: WasmModule = utils::get_wasm_module_from_file(module_file)?;
            println!("module reference: {:?}", wasm_module.get_module_ref());

            let schema_usdc: VersionedModuleSchema = utils::get_schema(&wasm_module)?;
            // types::usdc::test(&schema_usdc, CONTRACT_USDC, "deposit");
            let artifact_usdc = utils::get_artifact(&wasm_module)?;
            let arc_art_usdc = std::sync::Arc::new(artifact_usdc);

            let init_env_usdc = utils::InitEnvironment {
                contract_name: CONTRACT_USDC,
                context_file: "./data/usdc/ctx_init.json",
                param_file: Some("./data/usdc/p_init.json"),
                state_out_file: Some("./data/usdc/state.bin"),
            };
            // init_env_usdc.do_call(wasm_module.source.as_ref(), &schema_usdc, amount, energy)?;
            init_env_usdc.do_call(&arc_art_usdc, &schema_usdc, amount, energy)?;

            // ====================================================================================
            // Updates
            // ====================================================================================

            // Receive ---------------------------------------
            let envs = vec![
                utils::ReceiveEnvironment {
                    contract_name: CONTRACT_USDC,
                    entry_point: "grantRole",
                    context_file: "./data/usdc/ctx_upd.json",
                    param_file: Some("./data/usdc/p_grant_role.json"),
                    state_in_file: "./data/usdc/state.bin",
                    state_out_file: Some("./data/usdc/state2.bin"),
                },
                utils::ReceiveEnvironment {
                    contract_name: CONTRACT_USDC,
                    entry_point: "deposit",
                    context_file: "./data/usdc/ctx_upd.json",
                    param_file: Some("./data/usdc/p_deposit.json"),
                    state_in_file: "./data/usdc/state2.bin",
                    state_out_file: Some("./data/usdc/state3.bin"),
                },
                utils::ReceiveEnvironment {
                    contract_name: CONTRACT_USDC,
                    entry_point: "transfer",
                    context_file: "./data/usdc/ctx_upd.json",
                    param_file: Some("./data/usdc/p_transfer_contract.json"),
                    state_in_file: "./data/usdc/state3.bin",
                    state_out_file: Some("./data/usdc/state4.bin"),
                },
                utils::ReceiveEnvironment {
                    contract_name: CONTRACT_USDC,
                    entry_point: "balanceOf",
                    context_file: "./data/usdc/ctx_upd.json",
                    param_file: Some("./data/usdc/p_balanceof.json"),
                    state_in_file: "./data/usdc/state4.bin",
                    state_out_file: Some("./data/usdc/state5.bin"),
                },
            ];

            for env in envs {
                env.do_call(&arc_art_usdc, &schema_usdc, amount, energy)?;
            }

            // ====================================================================================
            // RIDO_USDC_PUBLIC
            // ====================================================================================
            // let pkg = "ovl-sale-usdc-public";
            // let module_file = format!(
            //     "../../{}/target/concordium/wasm32-unknown-unknown/release/{}.wasm.v1",
            //     pkg,
            //     pkg.to_lowercase().replace('-', "_")
            // );
            // let wasm_module: WasmModule = utils::get_wasm_module_from_file(module_file)?;

            // // Schema
            // let schema_rido_usdc: VersionedModuleSchema = utils::get_schema(&wasm_module)?;

            // let artifact = utils::get_artifact(&wasm_module)?;
            // let arc_art = std::sync::Arc::new(artifact);

            // let amount = Amount::zero();
            // let energy = InterpreterEnergy::from(1_000_000);

            // // Init - rido_usdc -----------------------------
            // let init_env = utils::InitEnvironment {
            //     contract_name: CONTRACT_PUB_RIDO_USDC,
            //     context_file: "./data/rido_usdc/ctx_init.json",
            //     param_file: Some("./data/rido_usdc/p_init_pub_usdc.json"),
            //     state_out_file: Some("./data/rido_usdc/state.bin"),
            // };

            // init_env.do_call(
            //     wasm_module.source.as_ref(),
            //     &schema_rido_usdc,
            //     amount,
            //     energy,
            // )?;

            // // Receive --------------------------------
            // let env1 = utils::ReceiveEnvironment {
            //     contract_name: CONTRACT_PUB_RIDO_USDC,
            //     entry_point: "setStatus",
            //     context_file: "./data/rido_usdc/ctx_upd.json",
            //     param_file: Some("./data/rido_usdc/p_set_status.json"),
            //     state_in_file: "./data/rido_usdc/state.bin",
            //     state_out_file: Some("./data/rido_usdc/state2.bin"),
            // };

            // let env2 = utils::ReceiveEnvironment {
            //     contract_name: CONTRACT_PUB_RIDO_USDC,
            //     entry_point: "view",
            //     context_file: "./data/rido_usdc/ctx_upd.json",
            //     param_file: None,
            //     state_in_file: "./data/rido_usdc/state2.bin",
            //     state_out_file: Some("./data/rido_usdc/state3.bin"),
            // };

            // let envs = vec![env1, env2];

            // for env in envs {
            //     env.do_call(&arc_art, &schema_rido_usdc, amount, energy)?;
            // }

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
        },
    }
}
