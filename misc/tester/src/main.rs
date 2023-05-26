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

use tester::*;

pub const NODE_ENDPOINT_V2: &str = "http://153.126.181.131:20001";
pub const CONTRACT_OPERATOR: &str = "ovl_operator";
pub const MODREF_OPERATOR: &str =
    "0e2e594df9b11dbc4728195ab4a1d1437fbfc310acf51273b59306330209119d";
pub const INDEX_OPERATOR: u64 = 4513;
pub const CONTRACT_PUB_RIDO_USDC: &str = "pub_rido_usdc";
pub const MODREF_PUB_RIDO_USDC: &str =
    "a03c62c603482ec786f2614b15586f14d135e1cc0cdadf13400c4271d9f40c91";
pub const INDEX_PUB_RIDO_USDC: u64 = 4534;

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

            // ============================
            // // Wasm Module from node
            // let endpoint = Endpoint::from_static(NODE_ENDPOINT_V2);
            // let wasm_module: WasmModule =
            //     utils::get_wasm_module_from_node(endpoint, MODREF_OPERATOR).await?;

            let pkg = "ovl-sale-usdc-public";
            let module_file = format!(
                "../../{}/target/concordium/wasm32-unknown-unknown/release/{}.wasm.v1",
                pkg,
                pkg.to_lowercase().replace('-', "_")
            );
            let wasm_module: WasmModule = utils::get_wasm_module_from_file(module_file)?;

            // Schema
            let schema_usdc: VersionedModuleSchema = utils::get_schema(&wasm_module)?;

            let artifact = utils::get_artifact(&wasm_module)?;
            let arc_art = std::sync::Arc::new(artifact);

            // =======================================================================
            // Init
            // =======================================================================

            let init_env = utils::InitEnvironment {
                contract_name: CONTRACT_PUB_RIDO_USDC,
                context_file: "./data/init_context.json",
                param_file: Some("./data/init_pub_usdc.json"),
                state_out_file: Some("./data/state.bin"),
            };

            let amount = Amount::zero();
            let energy = InterpreterEnergy::from(1_000_000);

            init_env.do_call(wasm_module.source.as_ref(), &schema_usdc, amount, energy)?;

            // =======================================================================
            // Receive
            // =======================================================================

            let amount = Amount::zero();
            let energy = InterpreterEnergy::from(1_000_000);

            let env1 = utils::ReceiveEnvironment {
                contract_name: CONTRACT_PUB_RIDO_USDC,
                entry_point: "setStatus",
                context_file: "./data/upd_context.json",
                param_file: Some("./data/set_status.json"),
                state_in_file: "./data/state.bin",
                state_out_file: Some("./data/state2.bin"),
            };

            let env2 = utils::ReceiveEnvironment {
                contract_name: CONTRACT_PUB_RIDO_USDC,
                entry_point: "view",
                context_file: "./data/upd_context.json",
                param_file: None,
                state_in_file: "./data/state2.bin",
                state_out_file: Some("./data/state3.bin"),
            };

            let envs = vec![env1, env2];

            for env in envs {
                env.do_call(&schema_usdc, &arc_art, amount, energy)?;
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
