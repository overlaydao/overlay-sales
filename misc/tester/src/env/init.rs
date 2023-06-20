use crate::context;
use crate::context::ReceiveContextV1Opt;
use crate::utils::*;
use anyhow::{bail, ensure, Context};
use chrono::{TimeZone, Utc};
use concordium_contracts_common::{
    constants, schema::VersionedModuleSchema, Amount, OwnedParameter, Timestamp,
};
use concordium_smart_contract_engine::{
    v0::HasChainMetadata,
    v1::{self, InitInvocation, InitResult},
    InterpreterEnergy,
};
use concordium_std::UnwrapAbort;

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InitEnvironment {
    pub slot_time: &'static str,
    pub context_file: Option<&'static str>,
    pub param_file: Option<&'static str>,
    pub state_out_file: &'static str,
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
        log::info!("===== Init::{:?} =====", func_name);

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
                let schema_parameter = &mods.schema.get_init_param_schema(mods.contract_name);

                if schema_parameter.is_err() {
                    log::error!("no schema provided for init function!");
                } else {
                    let schema_parameter = schema_parameter.as_ref().unwrap();
                    schema_parameter
                        .serial_value_into(&parameter_json, &mut init_param)
                        .context("Could not generate parameter bytes using schema and JSON.")?;
                }
            }
            OwnedParameter::try_from(init_param).unwrap()
        };

        if parameter.as_ref().len() > constants::MAX_PARAMETER_LEN {
            bail!("exceed parameter limit!");
        }

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

fn check_init_result(
    res: InitResult,
    loader: &mut v1::trie::Loader<&[u8]>,
    vschema: &VersionedModuleSchema,
    contract_name: &str,
    energy: &InterpreterEnergy,
    state_out_file: &str,
) -> anyhow::Result<()> {
    let (_, schema_return_value, schema_error, schema_event) =
        get_schemas_for_init(vschema, contract_name).unwrap();

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
            log::debug!(
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
            log::debug!(
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
