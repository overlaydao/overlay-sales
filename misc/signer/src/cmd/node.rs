use std::{path::Path, str::FromStr};

use anyhow::{bail, Context};
use concordium_rust_sdk::{
    smart_contracts::common::{
        from_bytes,
        schema::{ModuleV1, ModuleV2, ModuleV3, VersionedModuleSchema},
    },
    types::hashes::HashBytes,
    types::{
        hashes::ModuleReferenceMarker,
        smart_contracts::{ModuleReference, WasmModule},
    },
    v2::{BlockIdentifier, Client, Endpoint},
};

use crate::{
    config::{NODE_ENDPOINT_V2_TEST, NODE_ENDPOINT_V2_TEST_NOTLS},
    CONTRACT_OPERATOR, MODREF_OPERATOR,
};

pub async fn nodeinfo(endpoint: Endpoint) -> anyhow::Result<()> {
    // let endpoint = Endpoint::from_static(NODE_ENDPOINT_V2_TEST_NOTLS);
    // let endpoint = Endpoint::from_static(NODE_ENDPOINT_V2_TEST);
    let mut client = Client::new(endpoint)
        .await
        .context("Cannot connect to the node.")?;

    let node_info = client.get_node_info().await;
    println!("{:#?}", node_info);

    Ok(())
}

pub async fn get_module(endpoint: Endpoint, module: &str) -> anyhow::Result<VersionedModuleSchema> {
    let mut client = Client::new(endpoint)
        .await
        .context("Cannot connect to the node.")?;

    let mod_ref: ModuleReference = HashBytes::<ModuleReferenceMarker>::from_str(module).unwrap();

    let res = client
        .get_module_source(&mod_ref, &BlockIdentifier::LastFinal)
        .await?;

    let module: WasmModule = res.response;
    println!("Module version: {}", module.version);
    // println!("Module reference: {}", module.get_module_ref().to_string());

    let module_bytes: &Vec<u8> = module.source.as_ref();
    let module_str: String = hex::encode(module_bytes);

    let padding_for_remove = 0;
    let custom_section_string = "concordium-schema";
    let index = module_str
        .find(&hex::encode(custom_section_string))
        .unwrap();
    let schema = &module_bytes[index / 2 + custom_section_string.len() + padding_for_remove..];

    let module_schema: VersionedModuleSchema = from_bytes::<VersionedModuleSchema>(schema)?;

    Ok(module_schema)
}
