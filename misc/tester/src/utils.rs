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
        log::debug!("Return value: {}", out);
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
    log::info!(
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

// =========================================================

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InitEnvironment {
    pub slot_time: &'static str,
    pub context_file: Option<&'static str>,
    pub param_file: Option<&'static str>,
    pub state_out_file: &'static str,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ReceiveEnvironment {
    pub contract_index: u64,
    pub slot_time: &'static str,
    pub invoker: &'static str,
    pub entry_point: &'static str,
    pub param_file: Option<&'static str>,
    pub context_file: Option<&'static str>,
    pub state_in_file: &'static str,
    pub state_out_file: &'static str,
}

impl Default for ReceiveEnvironment {
    fn default() -> Self {
        Self {
            contract_index: 0,
            slot_time: "",
            invoker: "",
            entry_point: "",
            param_file: None,
            context_file: None,
            state_in_file: "state.bin",
            state_out_file: "state.bin",
        }
    }
}

#[derive(Debug)]
pub struct InvokeEnvironment<'a> {
    pub contract_index: u64,
    pub entry_point: String,
    pub parameter: OwnedParameter,
    pub state_in_file: &'a str,
    pub state_out_file: &'a str,
}

impl InitEnvironment {
    pub fn do_call(
        &self,
        mods: &context::ModuleInfo,
        // source: &Vec<u8>,
        // arc_art: &std::sync::Arc<Artifact<ProcessedImports, CompiledFunction>>,
        // artifact: &Artifact<ProcessedImports, CompiledFunction>,
        // schema: &VersionedModuleSchema,
        amount: Amount,
        energy: InterpreterEnergy,
    ) -> anyhow::Result<()> {
        let func_name: String = format!("init_{}", mods.contract_name);
        log::info!("================= Init::{:?} =================", func_name);

        // Context
        let init_ctx: context::InitContextOpt = if let Some(context_file) = self.context_file {
            // #[Todo] should not overwrite owner property even if the file provided.
            let f = format!("{}{}", mods.data_dir, context_file);
            let ctx_content = std::fs::read(f).context("Could not read init context file.")?;
            serde_json::from_slice(&ctx_content).context("Could not parse init context.")?
        } else {
            let dt = chrono::DateTime::parse_from_rfc3339(self.slot_time).unwrap();
            let ts = Timestamp::from_timestamp_millis(dt.timestamp_millis() as u64);
            context::InitContextOpt::new(ts, Some(mods.owner), None)
        };

        log::info!(
            "Current Time: {:?}",
            Utc.timestamp_millis_opt(init_ctx.metadata.slot_time()?.timestamp_millis() as i64)
                .unwrap()
        );

        // Parameter
        let parameter = {
            let mut init_param = Vec::new();
            if let Some(param_file) = self.param_file {
                let f = format!("{}{}", mods.data_dir, param_file);
                let parameter_json = get_object_from_json(f.into())?;
                let schema_parameter = &mods.schema.get_init_param_schema(mods.contract_name)?;
                schema_parameter
                    .serial_value_into(&parameter_json, &mut init_param)
                    .context("Could not generate parameter bytes using schema and JSON.")?;
            }
            OwnedParameter::try_from(init_param).unwrap()
        };

        let mut loader = v1::trie::Loader::new(&[][..]);

        // Call Init
        let res = v1::invoke_init(
            std::sync::Arc::clone(&mods.artifact),
            init_ctx,
            InitInvocation {
                amount,
                init_name: &func_name,
                parameter: parameter.as_ref(),
                energy,
            },
            false,
            loader,
        )
        .context("Initialization failed due to a runtime error.")?;

        check_init_result(
            res,
            &mut loader,
            &mods.schema,
            mods.contract_name,
            &energy,
            format!("{}{}", mods.data_dir, self.state_out_file).as_str(),
        )?;

        Ok(())
    }
}

impl ReceiveEnvironment {
    pub fn do_call(
        &self,
        chain: &context::ChainContext,
        // arc_art: &std::sync::Arc<Artifact<ProcessedImports, CompiledFunction>>,
        // schema: &VersionedModuleSchema,
        amount: Amount,
        energy: InterpreterEnergy,
    ) -> anyhow::Result<()> {
        // Chain - Module
        let mods = chain.modules.get(&self.contract_index).unwrap();

        let func_name =
            OwnedReceiveName::new_unchecked(format!("{}.{}", mods.contract_name, self.entry_point));
        log::info!(
            "================= Receive::{:?} =================",
            func_name
        );

        // Context
        let mut receive_context: context::ReceiveContextV1Opt =
            if let Some(context_file) = self.context_file {
                // #[Todo] should not overwrite owner property even if the file provided.
                let f = format!("{}{}", mods.data_dir, context_file);
                let ctx_content = std::fs::read(f).context("Could not read init context file.")?;
                serde_json::from_slice(&ctx_content).context("Could not parse init context.")?
            } else {
                let dt = chrono::DateTime::parse_from_rfc3339(self.slot_time).unwrap();
                let ts = Timestamp::from_timestamp_millis(dt.timestamp_millis() as u64);
                context::ReceiveContextV1Opt::new(
                    ts,
                    self.contract_index,
                    Some(mods.owner),
                    self.invoker,
                )
            };
        // println!("{:#?}", receive_context);

        log::info!(
            "\nCurrent Time: {:?}\nSender: {:?}",
            Utc.timestamp_millis_opt(
                receive_context.metadata().slot_time()?.timestamp_millis() as i64
            )
            .unwrap(),
            receive_context.sender()
        );

        // State
        let current_state: v1::trie::PersistentState = {
            let f = format!("{}{}", mods.data_dir, self.state_in_file);
            let state_bin = std::fs::File::open(f).context("Could not read state file.")?;
            let mut reader = std::io::BufReader::new(state_bin);

            v1::trie::PersistentState::deserialize(&mut reader)
                .context("Could not deserialize the provided state.")?
        };

        let mut loader = v1::trie::Loader::new(&[][..]);
        let mut mutable_state = current_state.thaw();
        let instance_state = v1::InstanceState::new(loader, mutable_state.get_inner(&mut loader));

        // Parameter
        let parameter = {
            let mut params = Vec::new();
            if let Some(file) = self.param_file {
                let f = format!("{}{}", mods.data_dir, file);
                let parameter_json = get_object_from_json(f.into())?;
                let schema_parameter = &mods
                    .schema
                    .get_receive_param_schema(mods.contract_name, self.entry_point)?;
                log::debug!("param > {:?}", parameter_json);
                log::debug!("schema > {:?}", schema_parameter);
                schema_parameter
                    .serial_value_into(&parameter_json, &mut params)
                    .context("Could not generate parameter bytes using schema and JSON.")?;
            }
            OwnedParameter::try_from(params).unwrap()
        };

        let receive_invocation = v1::ReceiveInvocation {
            amount,
            receive_name: func_name.as_receive_name(),
            parameter: parameter.as_ref(),
            energy,
        };

        let receive_params = v1::ReceiveParams {
            max_parameter_size: u16::MAX as usize,
            limit_logs_and_return_values: false,
            support_queries: true,
        };

        // Call
        let res = v1::invoke_receive::<
            _,
            _,
            _,
            _,
            context::ReceiveContextV1Opt,
            context::ReceiveContextV1Opt,
        >(
            std::sync::Arc::clone(&mods.artifact),
            receive_context.clone(),
            receive_invocation,
            instance_state,
            receive_params,
        )
        .context("Calling receive failed.")?;

        // Result
        if let v1::ReceiveResult::Interrupt { .. } = res {
            receive_context
                .common
                .set_sender(Address::Contract(ContractAddress::new(
                    self.contract_index,
                    0,
                )));
        };

        check_receive_result(
            res,
            chain,
            &mut loader,
            mutable_state,
            &mods.schema,
            mods.contract_name,
            self.entry_point,
            &energy,
            &format!("{}{}", mods.data_dir, self.state_out_file),
            receive_context,
        )?;

        Ok(())
    }
}

impl<'a> InvokeEnvironment<'a> {
    pub fn do_invoke(
        &self,
        chain: &context::ChainContext,
        mut receive_context: context::ReceiveContextV1Opt,
        amount: Amount,
        energy: InterpreterEnergy,
    ) -> anyhow::Result<()> {
        // Chain - Module
        let mods = chain.modules.get(&self.contract_index).unwrap();

        let func_name =
            OwnedReceiveName::new_unchecked(format!("{}.{}", mods.contract_name, self.entry_point));
        log::info!(
            "================= Invoke::{:?} =================",
            func_name
        );

        // State
        let current_state: v1::trie::PersistentState = {
            let f = format!("{}{}", mods.data_dir, self.state_in_file);
            let state_bin = std::fs::File::open(f).context("Could not read state file.")?;
            let mut reader = std::io::BufReader::new(state_bin);

            v1::trie::PersistentState::deserialize(&mut reader)
                .context("Could not deserialize the provided state.")?
        };

        let mut loader = v1::trie::Loader::new(&[][..]);
        let mut mutable_state = current_state.thaw();
        let instance_state = v1::InstanceState::new(loader, mutable_state.get_inner(&mut loader));

        let receive_invocation = v1::ReceiveInvocation {
            amount,
            receive_name: func_name.as_receive_name(),
            parameter: self.parameter.as_ref(),
            energy,
        };

        let receive_params = v1::ReceiveParams {
            max_parameter_size: u16::MAX as usize,
            limit_logs_and_return_values: false,
            support_queries: true,
        };

        // Call
        let res = v1::invoke_receive::<
            _,
            _,
            _,
            _,
            context::ReceiveContextV1Opt,
            context::ReceiveContextV1Opt,
        >(
            std::sync::Arc::clone(&mods.artifact),
            receive_context.clone(),
            receive_invocation,
            instance_state,
            receive_params,
        )
        .context("Calling receive failed.")?;

        // Result
        if let v1::ReceiveResult::Interrupt { .. } = res {
            receive_context
                .common
                .set_sender(Address::Contract(ContractAddress::new(
                    self.contract_index,
                    0,
                )));
        };
        check_receive_result(
            res,
            chain,
            &mut loader,
            mutable_state,
            &mods.schema,
            mods.contract_name,
            self.entry_point.as_str(),
            &energy,
            &format!("{}{}", mods.data_dir, self.state_out_file),
            receive_context,
        )?;

        Ok(())
    }
}

pub fn check_init_result(
    res: InitResult,
    loader: &mut v1::trie::Loader<&[u8]>,
    vschema: &VersionedModuleSchema,
    contract_name: &str,
    energy: &InterpreterEnergy,
    state_out_file: &str,
) -> anyhow::Result<()> {
    let (_, schema_return_value, schema_error, schema_event) =
        get_schemas_for_init(vschema, contract_name)?;

    match res {
        v1::InitResult::Success {
            logs,
            state,
            remaining_energy,
            return_value,
        } => {
            log::info!("Init call <succeeded>.");
            // print_logs(logs);
            // println!("{:?}", state);
            print_state(state, loader, true, state_out_file)?;
            print_return_value(return_value, schema_return_value)?;
            log::info!(
                "Interpreter energy spent is {}",
                energy.subtract(remaining_energy.energy)
            )
        },
        v1::InitResult::Reject {
            remaining_energy,
            reason,
            return_value,
        } => {
            log::info!("Init call rejected with reason {}.", reason);
            log::info!("The following error value was returned:");
            print_error(return_value, schema_error)?;
            log::info!(
                "Interpreter energy spent is {}",
                energy.subtract(remaining_energy.energy)
            )
        },
        v1::InitResult::Trap {
            remaining_energy,
            error,
        } => {
            return Err(error.context(format!(
                "Execution triggered a runtime error after spending {} interpreter energy.",
                energy.subtract(remaining_energy.energy)
            )));
        },
        v1::InitResult::OutOfEnergy => {
            log::info!("Init call terminated with out of energy.")
        },
    }

    Ok(())
}

pub fn check_receive_result(
    res: ReceiveResult<CompiledFunction, ReceiveContextV1Opt>,
    chain: &context::ChainContext,
    loader: &mut v1::trie::Loader<&[u8]>,
    mutable_state: MutableState,
    vschema: &VersionedModuleSchema,
    contract_name: &str,
    entrypoint: &str,
    energy: &InterpreterEnergy,
    state_out_file: &str,
    receive_context: ReceiveContextV1Opt,
) -> anyhow::Result<()> {
    let (_, schema_return_value, schema_error, schema_event) =
        get_schemas_for_receive(vschema, contract_name, entrypoint)?;

    match res {
        v1::ReceiveResult::Success {
            logs,
            state_changed,
            remaining_energy,
            return_value,
        } => {
            log::info!("Receive function <succeeded>.");
            // print_logs(logs);
            if !state_changed {
                log::info!("The state of the contract did not change.");
            }
            print_state(mutable_state, loader, true, state_out_file)?;
            print_return_value(return_value, schema_return_value)?;
            log::info!(
                "Interpreter energy spent is {}",
                energy.subtract(remaining_energy)
            )
        },
        v1::ReceiveResult::Reject {
            remaining_energy,
            reason,
            return_value,
        } => {
            log::info!("Receive call rejected with reason {}", reason);
            log::info!("The following error value was returned:");
            print_error(return_value, schema_error)?;
            log::info!(
                "Interpreter energy spent is {}",
                energy.subtract(remaining_energy)
            )
        },
        v1::ReceiveResult::OutOfEnergy => {
            log::info!("Receive call terminated with: out of energy.")
        },
        v1::ReceiveResult::Interrupt {
            remaining_energy,
            state_changed,
            logs,
            config: _,
            interrupt,
        } => {
            log::info!("Receive function <interrupted>.");
            // print_logs(logs);
            if state_changed {
                print_state(mutable_state, loader, true, &state_out_file)?;
            } else {
                log::info!("The state of the contract did not change.");
            }
            match interrupt {
                v1::Interrupt::Transfer { to, amount } => log::info!(
                    "Receive call invoked a transfer of {} CCD to {}.",
                    amount,
                    to
                ),
                v1::Interrupt::Call {
                    address,
                    parameter,
                    name,
                    amount,
                } => {
                    log::info!(
                        "Receive call invoked contract at ({}, {}), calling method {} with \
                     amount {} and parameter {:?}.",
                        address.index,
                        address.subindex,
                        name,
                        amount,
                        parameter
                    );
                    let energy = InterpreterEnergy::from(1_000_000);
                    let x = InvokeEnvironment {
                        contract_index: address.index,
                        entry_point: String::from(name),
                        parameter: OwnedParameter::new_unchecked(parameter),
                        state_in_file: "state.bin",
                        state_out_file: "state.bin",
                    };
                    x.do_invoke(chain, receive_context, amount, energy);
                },

                v1::Interrupt::Upgrade { module_ref } => log::info!(
                    "Receive call requested to upgrade the contract to module reference \
                     {}.",
                    hex::encode(module_ref.as_ref())
                ),

                v1::Interrupt::QueryAccountBalance { address } => {
                    log::info!("Receive call requested balance of the account {}.", address)
                },

                v1::Interrupt::QueryContractBalance { address } => log::info!(
                    "Receive call requested balance of the contract {}.",
                    address
                ),
                v1::Interrupt::QueryExchangeRates => {
                    log::info!("Receive call requested exchange rates.")
                },
            }
            log::info!(
                "Interpreter energy spent is {}",
                energy.subtract(remaining_energy)
            )
        },
        v1::ReceiveResult::Trap {
            remaining_energy,
            error,
        } => {
            return Err(error.context(format!(
                "Execution triggered a runtime error after spending {} interpreter energy.",
                energy.subtract(remaining_energy)
            )));
        },
    }
    Ok(())
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