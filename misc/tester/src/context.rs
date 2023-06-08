use crate::utils;
use anyhow::anyhow;
use concordium_base::{smart_contracts::WasmModule, transactions::cost::A};
use concordium_contracts_common::schema::VersionedModuleSchema;
use concordium_rust_sdk::{
    smart_contracts::common::Timestamp,
    types::smart_contracts::concordium_contracts_common::{
        to_bytes, AccountAddress, Address, Amount, ContractAddress, EntrypointName,
        OwnedEntrypointName, OwnedPolicy, Serial, SlotTime,
    },
};
use concordium_smart_contract_engine::{
    v0,
    v1::{self, ProcessedImports},
    ExecResult, InterpreterEnergy,
};
use concordium_wasm::artifact::{Artifact, CompiledFunction};
use serde::Deserialize;
use std::{collections::HashMap, str::FromStr};

pub struct ModuleInfo {
    pub contract_name: &'static str,
    pub owner: AccountAddress,
    pub data_dir: String,
    pub schema: VersionedModuleSchema,
    pub artifact: std::sync::Arc<Artifact<ProcessedImports, CompiledFunction>>,
}

pub struct ChainContext {
    pub modules: HashMap<u64, ModuleInfo>,
}

pub struct BalanceContext {
    pub balances: HashMap<Address, Amount>,
}

impl BalanceContext {
    pub fn faucet(&mut self, to: &str, amount: u64) -> anyhow::Result<()> {
        let to = Address::from(AccountAddress::from_str(to)?);
        let amount = Amount::from_micro_ccd(amount);

        let mut to_balance = self.balances.entry(to).or_insert_with(|| Amount::zero());
        *to_balance += amount;
        Ok(())
    }

    pub fn get_contract_balance(&mut self, index: u64) -> anyhow::Result<Option<Amount>> {
        let addr = Address::from(ContractAddress::new(index, 0));
        // let balance = self.balances.entry(addr).or_insert_with(|| Amount::zero());
        let balance = self.balances.get(&addr);
        if let Some(a) = balance {
            Ok(Some(*a))
        } else {
            Ok(Some(Amount::zero()))
        }
    }

    pub fn transfer(&mut self, from: &Address, to: &Address, amount: Amount) -> anyhow::Result<()> {
        if amount.micro_ccd == 0u64 {
            return Ok(());
        };

        let mut from_balance = self.balances.get_mut(from).unwrap();
        if *from_balance < amount {
            anyhow::bail!("no module registerd in chain context!");
        }
        *from_balance -= amount;

        let mut to_balance = self.balances.entry(*to).or_insert_with(|| Amount::zero());
        *to_balance += amount;

        Ok(())
    }
}

impl ChainContext {
    pub fn add_instance(
        &mut self,
        index: u64,
        contract_name: &'static str,
        module_file: String,
        owner: AccountAddress,
        data_dir: String,
        env: crate::env::init::InitEnvironment,
        amount: Amount,
        energy: InterpreterEnergy,
    ) -> anyhow::Result<()> {
        let wasm_module: WasmModule = utils::get_wasm_module_from_file(module_file)?;
        let vschema: VersionedModuleSchema = utils::get_schema(&wasm_module)?;
        let artifact = utils::get_artifact(&wasm_module)?;
        let arc_art = std::sync::Arc::new(artifact);
        // wasm_module.source.as_ref()
        // types::usdc::test(&schema_usdc, CONTRACT_USDC, "deposit");

        let mod_info = ModuleInfo {
            contract_name,
            owner,
            data_dir,
            schema: vschema,
            artifact: arc_art,
        };
        env.do_call(&mod_info, amount, energy)?;
        self.modules.insert(index, mod_info);
        Ok(())
    }

    pub fn add_module(&mut self, index: u64, module: ModuleInfo) {
        self.modules.insert(index, module);
    }

    pub fn get_contract_name(&self, index: u64) -> anyhow::Result<&'static str> {
        let mods = self.modules.get(&index);
        if mods.is_none() {
            anyhow::bail!("no module registerd in chain context!");
        }
        Ok(mods.unwrap().contract_name)
    }
}

/// A chain metadata with an optional field.
/// Used when simulating contracts to allow the user to only specify the
/// necessary context fields.
/// The default value is `None` for all `Option` fields.
#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChainMetadataOpt {
    slot_time: Option<SlotTime>,
}

impl Default for ChainMetadataOpt {
    fn default() -> Self {
        let t = chrono::Utc::now().timestamp_millis();
        Self {
            slot_time: Some(Timestamp::from_timestamp_millis(t as u64)),
        }
    }
}

impl ChainMetadataOpt {
    pub fn new(ts: Timestamp) -> Self {
        Self {
            slot_time: Some(ts),
        }
    }
}

impl v0::HasChainMetadata for ChainMetadataOpt {
    fn slot_time(&self) -> ExecResult<SlotTime> {
        unwrap_ctx_field(self.slot_time, "metadata.slotTime")
    }
}

/// An init context with optional fields.
/// Used when simulating contracts to allow the user to only specify the
/// context fields used by the contract.
/// The default value is `None` for all `Option` fields and the default of
/// `ChainMetadataOpt` for `metadata`.
#[derive(serde::Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct InitContextOpt {
    #[serde(default)]
    pub metadata: ChainMetadataOpt,
    pub init_origin: Option<AccountAddress>,
    #[serde(default, deserialize_with = "deserialize_policy_bytes_from_json")]
    pub sender_policies: Option<Vec<u8>>,
}

impl InitContextOpt {
    pub fn new(
        ts: Timestamp,
        init_origin: Option<AccountAddress>,
        sender_policies: Option<Vec<u8>>,
    ) -> Self {
        Self {
            metadata: ChainMetadataOpt::new(ts),
            init_origin,
            sender_policies,
        }
    }
}

impl v0::HasInitContext for InitContextOpt {
    type MetadataType = ChainMetadataOpt;

    fn metadata(&self) -> &Self::MetadataType {
        &self.metadata
    }

    fn init_origin(&self) -> ExecResult<&AccountAddress> {
        unwrap_ctx_field(self.init_origin.as_ref(), "initOrigin")
    }

    fn sender_policies(&self) -> ExecResult<&[u8]> {
        unwrap_ctx_field(
            self.sender_policies.as_ref().map(Vec::as_ref),
            "senderPolicies",
        )
    }
}

/// Serde deserializer for Option<Address>.
/// Introduced to avoid breaking changes when the serde implementation for
/// Address was changed to match the node.
fn deserialize_optional_address<'de, D: serde::de::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Address>, D::Error> {
    /// Newtype for address for deriving a differen serde implementation.
    #[derive(serde::Deserialize)]
    #[serde(tag = "type", content = "address", rename_all = "lowercase")]
    enum AddressWrapper {
        Account(AccountAddress),
        Contract(ContractAddress),
    }

    let option =
        Option::<AddressWrapper>::deserialize(deserializer)?.map(|wrapped| match wrapped {
            AddressWrapper::Account(address) => Address::Account(address),
            AddressWrapper::Contract(address) => Address::Contract(address),
        });
    Ok(option)
}

/// A receive context with optional fields.
/// Used when simulating contracts to allow the user to only specify the
/// context fields used by the contract.
/// The default value is `None` for all `Option` fields and the default of
/// `ChainMetadataOpt` for `metadata`.
#[derive(serde::Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReceiveContextOpt {
    #[serde(default)]
    metadata: ChainMetadataOpt,
    invoker: Option<AccountAddress>,
    self_address: Option<ContractAddress>,
    // This is pub because it is overwritten when `--balance` is used.
    pub self_balance: Option<Amount>,
    #[serde(deserialize_with = "deserialize_optional_address")]
    sender: Option<Address>,
    owner: Option<AccountAddress>,
    #[serde(default, deserialize_with = "deserialize_policy_bytes_from_json")]
    sender_policies: Option<Vec<u8>>,
}

impl ReceiveContextOpt {
    pub fn set_owner(&mut self, acc: AccountAddress) -> () {
        self.owner = Some(acc);
    }

    pub fn set_sender(&mut self, addr: Address) -> () {
        self.sender = Some(addr);
    }

    pub fn set_self_address(&mut self, index: u64) -> () {
        self.self_address = Some(ContractAddress::new(index, 0));
    }
}

impl v0::HasReceiveContext for ReceiveContextOpt {
    type MetadataType = ChainMetadataOpt;

    fn metadata(&self) -> &Self::MetadataType {
        &self.metadata
    }

    fn invoker(&self) -> ExecResult<&AccountAddress> {
        unwrap_ctx_field(self.invoker.as_ref(), "invoker")
    }

    fn self_address(&self) -> ExecResult<&ContractAddress> {
        unwrap_ctx_field(self.self_address.as_ref(), "selfAddress")
    }

    fn self_balance(&self) -> ExecResult<Amount> {
        unwrap_ctx_field(self.self_balance, "selfBalance")
    }

    fn sender(&self) -> ExecResult<&Address> {
        unwrap_ctx_field(self.sender.as_ref(), "sender")
    }

    fn owner(&self) -> ExecResult<&AccountAddress> {
        unwrap_ctx_field(self.owner.as_ref(), "owner")
    }

    fn sender_policies(&self) -> ExecResult<&[u8]> {
        unwrap_ctx_field(
            self.sender_policies.as_ref().map(Vec::as_ref),
            "senderPolicies",
        )
    }
}

// Error handling when unwrapping
fn unwrap_ctx_field<A>(opt: Option<A>, name: &str) -> ExecResult<A> {
    match opt {
        Some(v) => Ok(v),
        None => Err(anyhow!(
            "Missing field '{}' in the context. Make sure to provide a context file with all the \
             fields the contract uses.",
            name,
        )),
    }
}

/// A receive context with optional fields.
/// Used when simulating contracts to allow the user to only specify the
/// context fields used by the contract.
/// The default value is `None` for all `Option` fields and the default of
/// `ChainMetadataOpt` for `metadata`.
#[derive(serde::Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReceiveContextV1Opt {
    #[serde(flatten)]
    pub common: ReceiveContextOpt,
    entrypoint: Option<OwnedEntrypointName>,
}

impl ReceiveContextV1Opt {
    pub fn new(ts: Timestamp, index: u64, owner: Option<AccountAddress>, invoker: &str) -> Self {
        let self_address = Some(ContractAddress::new(index, 0));

        let (invoker, sender) = if invoker == "" {
            (None, None)
        } else {
            let invoker = AccountAddress::from_str(invoker).unwrap();
            (Some(invoker), Some(Address::from(invoker)))
        };

        Self {
            common: ReceiveContextOpt {
                metadata: ChainMetadataOpt::new(ts),
                self_address,
                owner,
                sender,
                invoker,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl v0::HasReceiveContext for ReceiveContextV1Opt {
    type MetadataType = ChainMetadataOpt;

    fn metadata(&self) -> &Self::MetadataType {
        &self.common.metadata
    }

    fn invoker(&self) -> ExecResult<&AccountAddress> {
        unwrap_ctx_field(self.common.invoker.as_ref(), "invoker")
    }

    fn self_address(&self) -> ExecResult<&ContractAddress> {
        unwrap_ctx_field(self.common.self_address.as_ref(), "selfAddress")
    }

    fn self_balance(&self) -> ExecResult<Amount> {
        unwrap_ctx_field(self.common.self_balance, "selfBalance")
    }

    fn sender(&self) -> ExecResult<&Address> {
        unwrap_ctx_field(self.common.sender.as_ref(), "sender")
    }

    fn owner(&self) -> ExecResult<&AccountAddress> {
        unwrap_ctx_field(self.common.owner.as_ref(), "owner")
    }

    fn sender_policies(&self) -> ExecResult<&[u8]> {
        unwrap_ctx_field(
            self.common.sender_policies.as_ref().map(Vec::as_ref),
            "senderPolicies",
        )
    }
}

impl v1::HasReceiveContext for ReceiveContextV1Opt {
    fn entrypoint(&self) -> ExecResult<EntrypointName> {
        let ep = unwrap_ctx_field(self.entrypoint.as_ref(), "entrypoint")?;
        Ok(ep.as_entrypoint_name())
    }
}

fn deserialize_policy_bytes_from_json<'de, D: serde::de::Deserializer<'de>>(
    des: D,
) -> Result<Option<Vec<u8>>, D::Error> {
    let policies = Option::<Vec<OwnedPolicy>>::deserialize(des)?;
    // It might be better to define a serialization instance in the future.
    // Its a bit finicky since this is not the usual serialization, it prepends
    // length of data so that data can be skipped and loaded lazily inside the
    // contract.
    if let Some(policies) = policies {
        let mut out = Vec::new();
        let len = policies.len() as u16;
        len.serial(&mut out).expect("Cannot fail writing to vec.");
        for policy in policies.iter() {
            let bytes = to_bytes(policy);
            let internal_len = bytes.len() as u16;
            internal_len
                .serial(&mut out)
                .expect("Cannot fail writing to vec.");
            out.extend_from_slice(&bytes);
        }
        Ok(Some(out))
    } else {
        Ok(None)
    }
}
