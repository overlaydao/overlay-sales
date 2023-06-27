use crate::config::ACCOUNT_ADDRESS_SIZE;

use anyhow::{bail, ensure, Context};
use chrono::{TimeZone, Utc};
use concordium_base::{
    common::Deserial,
    smart_contracts::{ModuleReference, ModuleSource, WasmModule, WasmVersion},
};
use concordium_contracts_common::{
    from_bytes,
    hashes::{HashBytes, ModuleReferenceMarker},
    schema::{ContractV3, FunctionV2, ModuleV1, ModuleV2, ModuleV3, Type, VersionedModuleSchema},
    AccountAddress, Address, Amount, ContractAddress, Cursor, EntrypointName, OwnedEntrypointName,
    OwnedParameter, OwnedReceiveName, Timestamp,
};
// use concordium_rust_sdk::v2::{BlockIdentifier, Client, Endpoint};
use concordium_smart_contract_engine::{
    v0::{self, HasChainMetadata, HasReceiveContext},
    v1::{
        self, trie::MutableState, InitInvocation, InitResult, ProcessedImports, ReceiveResult,
        ReturnValue,
    },
    InterpreterEnergy,
};
use concordium_smart_contract_testing::ContractEvent;
use concordium_wasm::artifact::{Artifact, CompiledFunction};
use ptree::{print_tree_with, PrintConfig, TreeBuilder};
use std::fs::{create_dir_all, read_to_string, File};
use std::{
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
};

pub fn init_logger() {
    use simplelog::*;

    let output_dir = Path::new("logs");
    create_dir_all(&output_dir);

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

fn check_diff_files(f1: &mut File, f2: &mut File) -> bool {
    let mut buff1: &mut [u8] = &mut [0; 1024];
    let mut buff2: &mut [u8] = &mut [0; 1024];

    loop {
        match f1.read(buff1) {
            Err(_) => return false,
            Ok(f1_read_len) => match f2.read(buff2) {
                Err(_) => return false,
                Ok(f2_read_len) => {
                    if f1_read_len != f2_read_len {
                        return false;
                    }
                    if f1_read_len == 0 {
                        return true;
                    }
                    if &buff1[0..f1_read_len] != &buff2[0..f2_read_len] {
                        return false;
                    }
                },
            },
        }
    }
}

/// Takes two string filepaths and returns true if the two files are identical and exist.
pub fn is_same(f1: &str, f2: &str) -> bool {
    let mut fh1 = File::open(f1);
    let mut fh2 = File::open(f2);

    fh1.as_mut()
        .and_then(|file1| {
            fh2.as_mut()
                .and_then(|file2| Ok(check_diff_files(file1, file2)))
        })
        .unwrap_or(false)
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

pub fn print_logs(logs: &Vec<ContractEvent>, schema_event: Option<&Type>) {
    for (i, item) in logs.iter().enumerate() {
        match schema_event {
            Some(schema) => {
                let out = schema
                    .to_json_string_pretty(item.as_ref())
                    .map_err(|_| anyhow::anyhow!("Could not output event value in JSON"));
                match out {
                    Ok(event_json) => {
                        log::info!("The JSON representation of event {} is:\n{}", i, event_json);
                    },
                    Err(error) => {
                        log::error!(
                            "Event schema had an error. {:?}. The raw value of event {} \
                                 is:\n{:?}",
                            error,
                            i,
                            item
                        );
                    },
                }
            },
            None => {
                log::error!("The raw value of event {} is:\n{:?}", i, item);
            },
        }
    }
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

// pub fn get_schemas_for_receive<'a, S: Into<OwnedEntrypointName>>(
//     vschema: &'a VersionedModuleSchema,
//     contract_name: &str,
//     func_name: S,
// ) -> anyhow::Result<(
//     Option<&'a Type>,
//     Option<&'a Type>,
//     Option<&'a Type>,
//     Option<&'a Type>,
// )> {
//     let (schema_parameter, schema_return_value, schema_error, schema_event) =
//         if let VersionedModuleSchema::V3(module_schema) = vschema {
//             match module_schema.contracts.get(contract_name) {
//                 Some(contract_schema) => {
//                     if let Some(func_schema) =
//                         contract_schema.receive.get(&func_name.into().to_string())
//                     {
//                         (
//                             func_schema.parameter(),
//                             func_schema.return_value(),
//                             func_schema.error(),
//                             contract_schema.event(),
//                         )
//                     } else {
//                         anyhow::bail!("[Schema Error] No entrypoint in the contract!");
//                     }
//                 },
//                 None => anyhow::bail!("[Schema Error] No contract name in the schema!"),
//             }
//         } else {
//             anyhow::bail!("[Schema Error] Currently only support Schema Version3!");
//         };

//     Ok((
//         schema_parameter,
//         schema_return_value,
//         schema_error,
//         schema_event,
//     ))
// }

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