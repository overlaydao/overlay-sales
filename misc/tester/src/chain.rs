use crate::config::*;
use crate::utils;
use anyhow::Context;
use concordium_base::smart_contracts::WasmModule;
use concordium_contracts_common::schema::VersionedModuleSchema;
use concordium_contracts_common::Timestamp;
use concordium_smart_contract_testing::*;

pub fn generate_chain(accounts: Vec<(AccountAddress, Amount)>, slot_time: Option<&str>) -> Chain {
    let mut chain = if let Some(t) = slot_time {
        let dt = chrono::DateTime::parse_from_rfc3339(t).unwrap();
        let ts: SlotTime = Timestamp::from_timestamp_millis(dt.timestamp_millis() as u64);
        Chain::new_with_time(ts)
    } else {
        Chain::new()
    };

    for acc in accounts {
        chain.create_account(Account::new(acc.0, acc.1));
    }

    chain
}

pub fn deploy_module(
    contract_name: &'static str,
    pkg: &str,
    chain: &mut Chain,
) -> anyhow::Result<ModuleInfo> {
    let module_file = format!("{}{}.wasm.v1", TARGET_DIR, pkg);

    let wasm_module: WasmModule = module_load_v1(&module_file).expect("Module exists and is valid");
    let schema: VersionedModuleSchema = utils::get_schema(&wasm_module)?;
    let module: ModuleDeploySuccess = chain
        .module_deploy_v1(Signer::with_one_key(), ACC_ADDR_OWNER, wasm_module)
        .expect("Deploying valid module should succeed");

    Ok(ModuleInfo {
        contract_name,
        module,
        schema,
    })
}

// ------------------------
pub struct InitEnvironment<'a> {
    pub id: u64,
    pub data_dir: String,
    pub owner: AccountAddress,
    pub param_file: Option<&'a str>,
}

pub struct UpdateEnvironment<'a> {
    pub invoker: AccountAddress,
    pub entry_point: &'a str,
    pub param_file: Option<&'a str>,
}

pub struct InstanceInfo {
    pub data_dir: String,
    pub it: ContractInitSuccess,
}

// context
pub struct ModuleInfo {
    pub contract_name: &'static str,
    pub module: ModuleDeploySuccess,
    pub schema: VersionedModuleSchema,
}

impl ModuleInfo {
    pub fn initialize(
        &self,
        env: InitEnvironment,
        chain: &mut Chain,
    ) -> anyhow::Result<InstanceInfo> {
        let parameter = {
            let mut init_param = Vec::new();
            if let Some(param_file) = env.param_file {
                let f = format!("{}{}", env.data_dir, param_file);
                let parameter_json = utils::get_object_from_json(f.into())?;
                let schema_parameter = &self.schema.get_init_param_schema(self.contract_name);
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

        let func_name: String = format!("init_{}", self.contract_name);
        let it = chain
            .contract_init(
                Signer::with_one_key(),
                env.owner,
                Energy::from(10000),
                InitContractPayload {
                    amount: Amount::zero(),
                    mod_ref: self.module.module_reference,
                    init_name: OwnedContractName::new_unchecked(func_name),
                    param: parameter,
                },
            )
            .expect("Initialization should always succeed");

        assert_eq!(
            it.contract_address.index, env.id,
            "An error may occur when updating the contract because an incorrect id is assigned."
        );

        Ok(InstanceInfo {
            data_dir: env.data_dir,
            it,
        })
    }

    pub fn update(
        &self,
        env: UpdateEnvironment,
        instance: &InstanceInfo,
        chain: &mut Chain,
    ) -> anyhow::Result<ContractInvokeSuccess> {
        let entry_point = "whitelisting";
        let receive_name =
            OwnedReceiveName::new_unchecked(format!("{}.{}", self.contract_name, entry_point));

        let param_file = Some("p_whitelisted.json");
        let parameter = {
            let mut params = Vec::new();
            if let Some(file) = param_file {
                let f = format!("{}{}", instance.data_dir, file);
                let parameter_json = utils::get_object_from_json(f.into())?;
                let schema_parameter = &self
                    .schema
                    .get_receive_param_schema(self.contract_name, entry_point)?;
                log::debug!("param > {:?}", parameter_json);
                log::debug!("schema > {:?}", schema_parameter);
                schema_parameter
                    .serial_value_into(&parameter_json, &mut params)
                    .context("Could not generate parameter bytes using schema and JSON.")?;
            }
            OwnedParameter::try_from(params).unwrap()
        };

        let update = chain.contract_update(
            Signer::with_one_key(),
            ACC_ADDR_OWNER,
            Address::Account(ACC_ADDR_OWNER),
            Energy::from(10000),
            UpdateContractPayload {
                amount: AMOUNT_ZERO,
                address: instance.it.contract_address,
                receive_name,
                message: parameter,
            },
        )?;

        Ok(update)
    }
}
