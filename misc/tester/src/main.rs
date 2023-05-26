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
            let source = wasm_module.source.as_ref();

            // Schema
            let schema_usdc: VersionedModuleSchema = utils::get_schema(&wasm_module)?;

            let artifact = utils::get_artifact(&wasm_module)?;
            let arc_art = std::sync::Arc::new(artifact);

            // =======================================================================
            // Init
            // =======================================================================
            println!("================= Init Function =================");

            let contract_name = CONTRACT_PUB_RIDO_USDC;
            let func_name = format!("init_{}", contract_name);

            // Context
            // let context_file = "./data/init_context.json";
            // let ctx_content =
            //     std::fs::read(context_file).context("Could not read init context file.")?;
            // let init_ctx: context::InitContextOpt =
            //     serde_json::from_slice(&ctx_content).context("Could not parse init context.")?;

            let dt = chrono::DateTime::parse_from_rfc3339("2023-05-18T00:00:00+09:00")?;
            let ts = Timestamp::from_timestamp_millis(dt.timestamp_millis() as u64);
            let addr = utils::account_address_bytes_from_str(
                "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1",
            )?;
            let init_ctx = context::InitContextOpt::new(ts, Some(AccountAddress(addr)), None);

            // -------------------
            // Parameter
            let schema_parameter = &schema_usdc.get_init_param_schema(contract_name)?;

            let param_file = "./data/init_pub_usdc.json";
            let parameter_json = utils::get_object_from_json(param_file.into())?;

            let mut init_param = Vec::new();
            schema_parameter
                .serial_value_into(&parameter_json, &mut init_param)
                .context("Could not generate parameter bytes using schema and JSON.")?;

            let amount = Amount::zero();
            let energy = InterpreterEnergy::from(1_000_000);
            let parameter = OwnedParameter::try_from(init_param).unwrap();
            let source_ctx = v1::InvokeFromSourceCtx {
                source,
                amount,
                parameter: parameter.as_ref(),
                energy,
                support_upgrade: true,
            };

            let mut loader = v1::trie::Loader::new(&[][..]);

            // Call Init
            let res = v1::invoke_init_with_metering_from_source(
                source_ctx, init_ctx, &func_name, loader, false,
            )
            .context("Initialization failed due to a runtime error.")?;

            let out_init_state_bin = Some("./data/state.bin");
            utils::check_init_result(
                res,
                &mut loader,
                &schema_usdc,
                contract_name,
                &energy,
                &out_init_state_bin,
            )?;

            // =======================================================================
            // Receive
            // =======================================================================

            let env1 = utils::ReceiveEnvironment {
                contract_name: CONTRACT_PUB_RIDO_USDC,
                entry_point: "setStatus",
                context_file: "./data/upd_context.json",
                state_in_file: "./data/state.bin",
                state_out_file: Some("./data/state2.bin"),
                param_file: Some("./data/set_status.json"),
            };

            let env2 = utils::ReceiveEnvironment {
                contract_name: CONTRACT_PUB_RIDO_USDC,
                entry_point: "view",
                context_file: "./data/upd_context.json",
                state_in_file: "./data/state2.bin",
                state_out_file: Some("./data/state3.bin"),
                param_file: None,
            };

            let envs = vec![env1, env2];

            for env in envs {
                env.do_call(&schema_usdc, &arc_art)?;
            }

            // =======================================================================
            // Receive
            // =======================================================================
            // println!("================= Receive Function =================");

            // let contract_name = CONTRACT_PUB_RIDO_USDC;
            // let entry_point = "view";
            // let func_name =
            //     OwnedReceiveName::new_unchecked(format!("{}.{}", contract_name, entry_point));

            // // Context And State
            // let context_file = "./data/upd_context.json";
            // let state_file = "./data/state2.bin";
            // let out_upd_bin: Option<PathBuf> = Some(PathBuf::from("./data/state3.bin"));

            // let ctx_content =
            //     std::fs::read(context_file).context("Could not read init context file.")?;
            // let upd_ctx: context::ReceiveContextV1Opt =
            //     serde_json::from_slice(&ctx_content).context("Could not parse init context.")?;

            // let state_bin = File::open(state_file).context("Could not read state file.")?;
            // let mut reader = std::io::BufReader::new(state_bin);
            // let current_state = v1::trie::PersistentState::deserialize(&mut reader)
            //     .context("Could not deserialize the provided state.")?;

            // let mut mutable_state = current_state.thaw();
            // let mut loader = v1::trie::Loader::new(&[][..]);
            // let inner = mutable_state.get_inner(&mut loader);
            // let instance_state = v1::InstanceState::new(loader, inner);

            // let res = v1::invoke_receive::<
            //     _,
            //     _,
            //     _,
            //     _,
            //     context::ReceiveContextV1Opt,
            //     context::ReceiveContextV1Opt,
            // >(
            //     std::sync::Arc::clone(&arc_art),
            //     upd_ctx,
            //     v1::ReceiveInvocation {
            //         amount,
            //         receive_name: func_name.as_receive_name(),
            //         parameter: parameter.as_ref(),
            //         energy,
            //     },
            //     instance_state,
            //     v1::ReceiveParams {
            //         max_parameter_size: u16::MAX as usize,
            //         limit_logs_and_return_values: false,
            //         support_queries: true,
            //     },
            // )
            // .context("Calling receive failed.")?;

            // utils::check_receive_result(
            //     res,
            //     &mut loader,
            //     mutable_state,
            //     &schema_usdc,
            //     contract_name,
            //     entry_point,
            //     &energy,
            //     &out_upd_bin,
            // )?;

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
