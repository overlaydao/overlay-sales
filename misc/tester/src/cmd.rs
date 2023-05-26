use anyhow::Context;
use chrono::{DateTime, Local, Utc};
use concordium_rust_sdk::{
    smart_contracts::common::{from_bytes, schema::VersionedModuleSchema},
    types::hashes::HashBytes,
    types::{
        hashes::ModuleReferenceMarker,
        smart_contracts::{ModuleReference, WasmModule},
    },
    v2::{BlockIdentifier, Client, Endpoint},
};
use std::str::FromStr;

pub fn timestamp(h: i64) -> anyhow::Result<()> {
    let utc_datetime: DateTime<Utc> = Utc::now();
    println!("{} <= UTC", utc_datetime.to_rfc3339());

    let local_datetime: DateTime<Local> = Local::now();
    println!("{} <= Local", local_datetime.to_rfc3339());
    // println!("custom format: {}", local_datetime.format("%a %b %e %T %Y"));

    println!("current timestamp: {:?}", utc_datetime.timestamp_millis());

    if h < 0 {
        anyhow::bail!("Invalid hour");
    }
    let s: i64 = utc_datetime.timestamp_millis() + h * 60 * 60 * 1000;
    println!("{h:?} hour later: {s:?}");

    Ok(())
}

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
