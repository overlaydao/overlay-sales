use crate::config::ACCOUNT_ADDRESS_SIZE;
use crate::context;
use anyhow::{bail, ensure, Context};
use chrono::{TimeZone, Utc};
use concordium_base::common::Deserial;
use concordium_contracts_common::{
    schema::{FunctionV2, Type, VersionedModuleSchema},
    AccountAddress, Address, Amount, ContractAddress, Cursor, OwnedParameter, OwnedReceiveName,
};
use concordium_rust_sdk::{
    smart_contracts::common::{
        from_bytes,
        schema::{ContractV3, ModuleV1, ModuleV2, ModuleV3},
        Timestamp,
    },
    types::hashes::HashBytes,
    types::{
        hashes::ModuleReferenceMarker,
        smart_contracts::{ModuleReference, ModuleSource, WasmModule, WasmVersion},
    },
    v2::{BlockIdentifier, Client, Endpoint},
};
use concordium_smart_contract_engine::{
    v0::{HasChainMetadata, HasReceiveContext},
    v1::{
        self, trie::MutableState, InitInvocation, InitResult, ProcessedImports, ReceiveResult,
        ReturnValue,
    },
    InterpreterEnergy,
};
use concordium_wasm::artifact::{Artifact, CompiledFunction};
use ptree::{print_tree_with, PrintConfig, TreeBuilder};
use std::{
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::context::ReceiveContextV1Opt;

pub fn init_logger() {
    use simplelog::*;

    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            std::fs::File::create(format!(
                "./logs/{}.log",
                chrono::Local::now().format("%y%m%d_%H:%M")
            ))
            .unwrap(),
        ),
    ])
    .unwrap();
}

pub fn test_init_context() -> context::InitContextOpt {
    let dt = chrono::DateTime::parse_from_rfc3339("2023-05-18T00:00:00+09:00").unwrap();
    let ts = Timestamp::from_timestamp_millis(dt.timestamp_millis() as u64);
    let addr = account_address_bytes_from_str("3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1")
        .unwrap();
    context::InitContextOpt::new(ts, Some(AccountAddress(addr)), None)
}

pub fn get_wasm_module_from_file<P: AsRef<Path> + ToString>(
    module_file: P,
) -> anyhow::Result<WasmModule> {
    let versioned_module: Vec<u8> = std::fs::read(&module_file).context(format!(
        "Could not read module file.{}",
        module_file.to_string()
    ))?;
    let mut cursor = std::io::Cursor::new(&versioned_module[..]);

    let version = WasmVersion::deserial(&mut cursor)?;
    let wasm_length = {
        let mut buf = [0u8; 4];
        cursor
            .read_exact(&mut buf)
            .context("Could not parse supplied module.")?;
        u32::from_be_bytes(buf)
    };
    cursor.set_position(4);

    let source = ModuleSource::deserial(&mut cursor)?;
    ensure!(
        source.size() == wasm_length as u64,
        "[Parse Error]The specified length does not match the size of the provided data."
    );
    let wasm_module = WasmModule { version, source };

    Ok(wasm_module)
}

pub async fn get_wasm_module_from_node(
    endpoint: Endpoint,
    module_str: &str,
) -> anyhow::Result<WasmModule> {
    let mut client = Client::new(endpoint)
        .await
        .context("Cannot connect to the node.")?;
    let mod_ref: ModuleReference =
        HashBytes::<ModuleReferenceMarker>::from_str(module_str).unwrap();
    let res = client
        .get_module_source(&mod_ref, &BlockIdentifier::LastFinal)
        .await?;
    let wasm_module: WasmModule = res.response;

    Ok(wasm_module)
}

pub fn get_schema(src: &WasmModule) -> anyhow::Result<VersionedModuleSchema> {
    // let file_path = Path::new("./test/schema_operator.bin");
    // let contents = std::fs::read(file_path).expect("Should have been able to read the file");
    // let module_schema: VersionedModuleSchema = match from_bytes(&contents) {
    //     Ok(o) => o,
    //     Err(e) => bail!("no!!!!!"),
    // };

    Ok(concordium_smart_contract_engine::utils::get_embedded_schema_v1(src.source.as_ref())?)
}

pub fn get_artifact(
    src: &WasmModule,
) -> anyhow::Result<Artifact<v1::ProcessedImports, CompiledFunction>> {
    let artifact: Artifact<v1::ProcessedImports, CompiledFunction> =
        concordium_wasm::utils::instantiate_with_metering::<v1::ProcessedImports, _>(
            &v1::ConcordiumAllowedImports {
                support_upgrade: true,
            },
            src.source.as_ref(),
        )?;
    Ok(artifact)
}

pub fn get_object_from_json(path: PathBuf) -> anyhow::Result<serde_json::Value> {
    // let mut state_cursor = Cursor::new(parameter_bytes);
    // match types.to_json(&mut state_cursor) {
    //     Ok(schema) => {
    //         println!("{:?}", schema);
    //         let json = serde_json::to_string_pretty(&schema).unwrap();
    //         println!("{}", json);
    //     },
    //     Err(e) => bail!("x"),
    // }

    let file = std::fs::read(path).context("Could not read file.")?;
    let parameter_json = serde_json::from_slice(&file).context("Could not parse the JSON.")?;
    Ok(parameter_json)
}

pub fn account_address_bytes_from_str(v: &str) -> anyhow::Result<[u8; ACCOUNT_ADDRESS_SIZE]> {
    let mut buf = [0xff; 1 + ACCOUNT_ADDRESS_SIZE + 4];
    let len = bs58::decode(v).with_check(Some(1)).into(&mut buf)?;

    if len != 1 + ACCOUNT_ADDRESS_SIZE {
        bail!("invalid byte length");
    }

    let mut address_bytes = [0u8; ACCOUNT_ADDRESS_SIZE];
    address_bytes.copy_from_slice(&buf[1..1 + ACCOUNT_ADDRESS_SIZE]);
    Ok(address_bytes)
}

pub fn account_address_string_from_byte(bytes: [u8; 32]) -> anyhow::Result<String> {
    let mut encoded = String::with_capacity(50);
    let mut decoded: Vec<u8> = [1].iter().chain(bytes.iter()).map(|v| *v).collect();
    let decoded: [u8; 33] = decoded.try_into().unwrap();
    bs58::encode(decoded).with_check().into(&mut encoded)?;
    Ok(encoded)
}

pub fn display_state(state: &v1::trie::PersistentState) -> anyhow::Result<()> {
    let mut loader = v1::trie::Loader::new([]);

    let mut tree_builder = TreeBuilder::new("StateRoot".into());
    state.display_tree(&mut tree_builder, &mut loader);
    let tree = tree_builder.build();

    log::debug!("{:#?}", tree);

    // let config = PrintConfig::default();
    // print_tree_with(&tree, &config).context("Could not print the state as a tree.")
    Ok(())
}

pub fn print_error(rv: ReturnValue, schema_error: Option<&Type>) -> anyhow::Result<()> {
    if let Some(schema) = schema_error {
        let out = schema
            .to_json_string_pretty(&rv)
            .map_err(|_| anyhow::anyhow!("Could not output error value in JSON"))?;
        log::error!("Error: {}", out);
        Ok::<_, anyhow::Error>(())
    } else {
        log::info!(
            "No schema for the error value. The raw error value is {:?}.",
            rv
        );
        Ok(())
    }
}

pub fn print_return_value(
    rv: ReturnValue,
    schema_return_value: Option<&Type>,
) -> anyhow::Result<()> {
    if let Some(schema) = schema_return_value {
        let out = schema
            .to_json_string_pretty(&rv)
            .map_err(|_| anyhow::anyhow!("Could not output return value in JSON"))?;
        log::info!("Return value: {}", out);
        Ok::<_, anyhow::Error>(())
    } else {
        log::info!(
            "No schema for the return value. The raw return value is {:?}.",
            rv
        );
        Ok(())
    }
}

pub fn print_state(
    mut state: v1::trie::MutableState,
    loader: &mut v1::trie::Loader<&[u8]>,
    should_display_state: bool,
    out_bin_file: &str,
) -> anyhow::Result<()> {
    let mut collector = v1::trie::SizeCollector::default();
    let frozen = state.freeze(loader, &mut collector);
    log::debug!(
        "The contract will produce {}B of additional state that will be charged for.",
        collector.collect()
    );

    let mut out_file = std::fs::File::create(out_bin_file)
        .context("Could not create file to write state into.")?;
    frozen
        .serialize(loader, &mut out_file)
        .context("Could not write the state.")?;
    log::info!("Resulting state written to {}.", out_bin_file);

    if should_display_state {
        display_state(&frozen)?;
    }
    Ok(())
}

pub fn get_schemas_for_init<'a>(
    vschema: &'a VersionedModuleSchema,
    contract_name: &str,
) -> anyhow::Result<(
    Option<&'a Type>,
    Option<&'a Type>,
    Option<&'a Type>,
    Option<&'a Type>,
)> {
    let (schema_parameter, schema_return_value, schema_error, schema_event) =
        if let VersionedModuleSchema::V3(module_schema) = vschema {
            match module_schema.contracts.get(contract_name) {
                Some(contract_schema) => {
                    if let Some(func_schema) = contract_schema.init.as_ref() {
                        (
                            func_schema.parameter(),
                            func_schema.return_value(),
                            func_schema.error(),
                            contract_schema.event(),
                        )
                    } else {
                        (None, None, None, None)
                        // anyhow::bail!("[Schema Error] No entrypoint in the contract!");
                    }
                },
                None => anyhow::bail!("[Schema Error] No contract name in the schema!"),
            }
        } else {
            anyhow::bail!("[Schema Error] Currently only support Schema Version3!");
        };

    Ok((
        schema_parameter,
        schema_return_value,
        schema_error,
        schema_event,
    ))
}

pub fn get_schemas_for_receive<'a>(
    vschema: &'a VersionedModuleSchema,
    contract_name: &str,
    func_name: &str,
) -> anyhow::Result<(
    Option<&'a Type>,
    Option<&'a Type>,
    Option<&'a Type>,
    Option<&'a Type>,
)> {
    let (schema_parameter, schema_return_value, schema_error, schema_event) =
        if let VersionedModuleSchema::V3(module_schema) = vschema {
            match module_schema.contracts.get(contract_name) {
                Some(contract_schema) => {
                    if let Some(func_schema) = contract_schema.receive.get(func_name) {
                        (
                            func_schema.parameter(),
                            func_schema.return_value(),
                            func_schema.error(),
                            contract_schema.event(),
                        )
                    } else {
                        anyhow::bail!("[Schema Error] No entrypoint in the contract!");
                    }
                },
                None => anyhow::bail!("[Schema Error] No contract name in the schema!"),
            }
        } else {
            anyhow::bail!("[Schema Error] Currently only support Schema Version3!");
        };

    Ok((
        schema_parameter,
        schema_return_value,
        schema_error,
        schema_event,
    ))
}

// pub fn deserializer(
//     contract_name: &str,
//     func_name: &str,
//     state_bytes: String,
//     schema: String,
// ) -> Result<String, String> {
//     let module_schema: ModuleV3 = match from_bytes(&hex::decode(schema).unwrap()) {
//         Ok(o) => o,
//         Err(e) => return Err(format!("unable to parse schema: {:#?}", e)),
//     };

//     let contract_schema: &ContractV3 = module_schema
//         .contracts
//         .get(contract_name)
//         .ok_or_else(|| "Unable to get contract schema: not included in module schema")
//         .unwrap();

//     let mut state_cursor = Cursor::new(hex::decode(state_bytes).unwrap());

//     let state_schema: &FunctionV2 = contract_schema.receive.get(func_name).unwrap();
//     let types: &Type = state_schema.return_value().unwrap();

//     match types.to_json(&mut state_cursor) {
//         Ok(schema) => Ok(schema.to_string()),
//         Err(e) => Err(format!("Unable to parse state to json: {:?}", e)),
//     }
// }
