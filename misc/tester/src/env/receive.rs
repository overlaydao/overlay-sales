use crate::context::ReceiveContextV1Opt;
use crate::context::{self, ModuleInfo};
use crate::utils::*;
use anyhow::{bail, ensure, Context};
use chrono::{TimeZone, Utc};
use concordium_contracts_common::{
    constants, schema::VersionedModuleSchema, Address, Amount, ContractAddress, OwnedParameter,
    OwnedReceiveName, Timestamp,
};
use concordium_smart_contract_engine::{
    v0::{HasChainMetadata, HasReceiveContext},
    v1::{self, trie::MutableState, ReceiveResult},
    InterpreterEnergy,
};
use concordium_wasm::artifact::CompiledFunction;

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
    pub amount: Amount,
}

#[derive(Debug)]
pub struct InvokeEnvironment<'a> {
    pub contract_index: u64,
    pub entry_point: String,
    pub parameter: OwnedParameter,
    pub state_in_file: &'a str,
    pub state_out_file: &'a str,
    pub amount: Amount,
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
            amount: Amount::zero(),
        }
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
        log::info!("=============== Receive::{:?} ===============", func_name);

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
        log::debug!("{:?}", receive_context);

        // log::debug!(
        //     "\nCurrent Time: {:?}\nSender: {:?}",
        //     Utc.timestamp_millis_opt(
        //         receive_context.metadata().slot_time()?.timestamp_millis() as i64
        //     )
        //     .unwrap(),
        //     receive_context.sender(),
        // );

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

        if parameter.as_ref().len() > constants::MAX_PARAMETER_LEN {
            bail!("exceed parameter limit!");
        }

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
            mods,
            &mut loader,
            mutable_state,
            self.entry_point,
            &energy,
            receive_context,
            None,
        )?;

        Ok(())
    }
}

impl<'a> InvokeEnvironment<'a> {
    pub fn do_invoke(
        &self,
        chain: &context::ChainContext,
        mut receive_context: context::ReceiveContextV1Opt,
        data_dir: &str,
        amount: Amount,
        energy: InterpreterEnergy,
    ) -> anyhow::Result<()> {
        // Chain - Module
        let mods = chain.modules.get(&self.contract_index).unwrap();

        let func_name =
            OwnedReceiveName::new_unchecked(format!("{}.{}", mods.contract_name, self.entry_point));
        log::info!(">>>>> [Invoke::{:?}] <<<<<", func_name);

        receive_context.common.set_self_address(self.contract_index);
        receive_context.common.set_owner(mods.owner);

        log::debug!("{:?}", receive_context);

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
            mods,
            &mut loader,
            mutable_state,
            self.entry_point.as_str(),
            &energy,
            receive_context,
            Some(data_dir),
        )?;

        Ok(())
    }
}

// --------------------------------------------------
//
fn check_receive_result(
    res: ReceiveResult<CompiledFunction, ReceiveContextV1Opt>,
    chain: &context::ChainContext,
    mods: &ModuleInfo,
    loader: &mut v1::trie::Loader<&[u8]>,
    mutable_state: MutableState,
    entrypoint: &str,
    energy: &InterpreterEnergy,
    mut receive_context: ReceiveContextV1Opt,
    invoked_from: Option<&str>,
) -> anyhow::Result<()> {
    let vschema: &VersionedModuleSchema = &mods.schema;
    let (_, schema_return_value, schema_error, schema_event) =
        get_schemas_for_receive(vschema, mods.contract_name, entrypoint)?;

    match res {
        v1::ReceiveResult::Success {
            logs,
            state_changed,
            remaining_energy,
            return_value,
        } => {
            log::info!("Receive function <succeeded>.");
            // print_logs(logs);

            if let Some(dir) = invoked_from {
                log::info!(
                    "Commit {:?} State since invoked function has been succeeded.",
                    dir
                );
                let f1: &str = &format!("{}{}", dir, "_state.bin");
                let f2: &str = &format!("{}{}", dir, "state.bin");
                std::fs::copy(f1, f2)?;
            }

            if !state_changed {
                log::debug!("The state of the contract did not change.");
            }
            let state_out_file: &str = &format!("{}{}", mods.data_dir, "state.bin");
            print_state(mutable_state, loader, true, state_out_file)?;
            print_return_value(return_value, schema_return_value)?;
            log::debug!(
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
            log::debug!(
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
                let state_out_file: &str = &format!("{}{}", mods.data_dir, "_state.bin");
                print_state(mutable_state, loader, true, state_out_file)?;
            } else {
                log::debug!("The state of the contract did not change.");
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
                        amount,
                    };
                    x.do_invoke(chain, receive_context, mods.data_dir, amount, energy);
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
