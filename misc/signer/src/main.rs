#![allow(unused)]
use anyhow::{bail, Context, Result};
use clap::Parser;
use concordium_rust_sdk::smart_contracts::common::Cursor;
use signer::*;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tonic::transport::Endpoint;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Timestamp { hour }) => {
            cmd::timestamp(hour)?;
            Ok(())
        },
        Some(Commands::Nodeinfo { endpoint }) => {
            cmd::node::nodeinfo(endpoint).await?;
            // cmd::node::get_module(endpoint).await?;
            Ok(())
        },
        Some(Commands::Keygen { filename }) => {
            let keys = cmd::keygen::gen_keys()?;
            cmd::keygen::output_json(keys, &filename)
        },
        Some(Commands::Confirm { encoded_msg, mode }) => {
            cmd::sign::confirm_signed_message(encoded_msg, mode)
        },
        Some(Commands::Sign {
            keys,
            payload,
            mode,
        }) => {
            let data_path = if let Some(path) = payload.as_deref() {
                path
            } else {
                match mode {
                    MessageType::AddKey => Path::new("./data/msg_add_key.json"),
                    MessageType::RemoveKey => Path::new("./data/msg_rm_key.json"),
                    MessageType::Invoke => Path::new("./data/msg_invoke.json"),
                    _ => bail!("need payload args. ex) -p data/msg_for_sign.json"),
                }
            };
            cmd::sign::sign(&keys, data_path, mode)?;
            Ok(())
        },
        Some(Commands::Init { contract }) => {
            let (modref, contract_name, init_params) = match contract.as_str() {
                "ops" => {
                    let params_bytes = cmd::smc::ovl_operator::init::create_init_operators_exp()?;
                    (MODREF_OPERATOR, CONTRACT_OPERATOR, params_bytes)
                },
                "usdc" => {
                    let params_bytes =
                        cmd::smc::pub_rido_usdc::init::create_init_pub_rido_usdc_exp()?;
                    (MODREF_PUB_RIDO_USDC, CONTRACT_PUB_RIDO_USDC, params_bytes)
                },
                _ => {
                    bail!("there is no such contract!")
                },
            };

            cmd::smc::initialize(modref, contract_name, init_params).await?;

            Ok(())
        },
        Some(Commands::UpdateKey) => {
            let (method, update_params) = cmd::smc::ovl_operator::update::update_keys().await?;
            cmd::smc::update(INDEX_OPERATOR, CONTRACT_OPERATOR, &method, update_params).await?;
            Ok(())
        },
        Some(Commands::UpdateInvoke) => {
            let (method, update_params) = cmd::smc::ovl_operator::update::invoke().await?;
            cmd::smc::update(INDEX_OPERATOR, CONTRACT_OPERATOR, &method, update_params).await?;
            Ok(())
        },
        Some(Commands::UpdateKeyTest { mode }) => {
            update_add_key_exp(&mode).await?;
            Ok(())
        },
        None => {
            let endpoint = Endpoint::from_static(NODE_ENDPOINT_V2);
            let schema_ops = cmd::node::get_module(endpoint.clone(), MODREF_OPERATOR).await?;
            let types_ops = &schema_ops.get_init_param_schema(CONTRACT_OPERATOR)?;
            let schema_usdc = cmd::node::get_module(endpoint, MODREF_PUB_RIDO_USDC).await?;
            let types_usdc = &schema_usdc.get_init_param_schema(CONTRACT_PUB_RIDO_USDC)?;

            let mut parameter_bytes = Vec::new();
            let parameter_json = get_object_from_json("./test/init_pub_usdc.json".into())?;
            types_usdc
                .serial_value_into(&parameter_json, &mut parameter_bytes)
                .context("Could not generate parameter bytes using schema and JSON.")?;

            // let summary = cmd::smc::initialize(
            //     MODREF_PUB_RIDO_USDC,
            //     CONTRACT_PUB_RIDO_USDC,
            //     parameter_bytes,
            // )
            // .await?;
            // println!("{:?}", summary);

            Ok(())
        },
    }
}
