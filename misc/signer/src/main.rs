#![allow(unused)]
use anyhow::{bail, Context, Result};
use clap::Parser;
use signer::*;
use std::path::{Path, PathBuf};

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
            cmd::smc::init::initialize(contract).await?;
            Ok(())
        },
        Some(Commands::UpdateKey) => {
            cmd::smc::update::update_keys().await?;
            Ok(())
        },
        Some(Commands::UpdateInvoke) => {
            cmd::smc::update::invoke().await?;
            Ok(())
        },
        Some(Commands::UpdateKeyTest { mode }) => {
            update_add_key_exp(&mode).await?;
            Ok(())
        },
        None => {
            // let path = Path::new("data");
            // let mut files = Vec::new();
            // traverse(path, &mut |e| files.push(e)).unwrap();
            // for file in files {
            //     println!("{:?}", file);
            // }

            Ok(())
        },
    }
}
