//! # A Concordium V1 smart contract
#[cfg(any(feature = "wasm-test", test))]
mod sctest;
use concordium_std::{collections::BTreeSet, *};
pub use sale_utils::{
    error::{ContractError, ContractResult, CustomContractError},
    types::*,
};

// -------------------------------------------------------------

#[derive(Serial, DeserialWithState, StateClone, Debug)]
#[concordium(state_parameter = "S")]
struct State<S> {
    /// A registry to link an account to an public key.
    /// To call this contract, it must be approved by at least 2 accounts registered here.
    pub(crate) operators: StateMap<AccountAddress, PublicKeyEd25519, S>,
}

impl<S: HasStateApi> State<S> {
    pub(crate) fn new(
        state_builder: &mut StateBuilder<S>,
        operators: Vec<OperatorWithKeyParam>,
    ) -> Self {
        let mut state = State {
            operators: state_builder.new_map(),
        };

        for OperatorWithKeyParam {
            account,
            public_key,
        } in operators.into_iter()
        {
            state.operators.entry(account).or_insert_with(|| public_key);
        }

        state
    }

    fn check_auth(
        &self,
        action: PermitAction,
        message: &PermitMessageWithParameter,
        signatures: &BTreeSet<(AccountAddress, SignatureEd25519)>,
        ctx: &impl HasReceiveContext,
        crypto_primitives: &impl HasCryptoPrimitives,
    ) -> ContractResult<()> {
        // #[Todo] need appropriate errors
        ensure_eq!(
            message.action,
            action,
            CustomContractError::WrongAction.into()
        );

        // Check that the message was intended for this contract.
        ensure_eq!(
            message.contract_address,
            ctx.self_address(),
            CustomContractError::WrongContract.into()
        );

        // Check that calling function is appropriate.
        ensure_eq!(
            message.entry_point.as_entrypoint_name(),
            ctx.named_entrypoint().as_entrypoint_name(),
            CustomContractError::WrongEntrypoint.into()
        );

        // Check signature is not expired.
        ensure!(
            message.timestamp > ctx.metadata().slot_time(),
            CustomContractError::Expired.into()
        );

        // Calculate the message hash.
        let message_hash = crypto_primitives.hash_sha2_256(&to_bytes(message)).0;

        let mut legit = 0;
        for (addr, sig) in signatures.iter() {
            let pubkey = self
                .operators
                .get(&addr)
                .ok_or(CustomContractError::NoPublicKey)?;

            if crypto_primitives.verify_ed25519_signature(*pubkey, *sig, &message_hash) {
                legit += 1;
            }
        }

        // #[TODO] use constant or parameter
        ensure!(legit > 1, ContractError::Unauthorized);

        Ok(())
    }

    fn add_operator(
        &mut self,
        account: &AccountAddress,
        pubkey: &PublicKeyEd25519,
    ) -> ContractResult<()> {
        // Register the public key.
        let old_pubkey = self.operators.insert(*account, *pubkey);

        // Return an error if the owner tries to update/re-write a new public key for an
        // already registered account.
        ensure!(
            old_pubkey.is_none(),
            CustomContractError::AccountDuplicated.into()
        );
        Ok(())
    }

    fn remove_operator(&mut self, account: &AccountAddress) -> ContractResult<()> {
        self.operators.remove(account);
        Ok(())
    }
}

// --------------------------------------------------------------

/// Part of the parameter type for the contract entrypoint `addOperatorKeys`.
/// Takes the account and the public key that should be linked.
#[derive(Debug, Serialize, SchemaType)]
pub struct OperatorWithKeyParam {
    /// Account that a public key will be registered to.
    account: AccountAddress,
    /// The public key that should be linked to the above account.
    public_key: PublicKeyEd25519,
}

#[derive(Debug, Serialize, SchemaType)]
pub struct InitParams {
    /// Contract operators
    #[concordium(size_length = 1)]
    pub(crate) operators: Vec<OperatorWithKeyParam>,
}

/// Init function that creates a new smart contract.
#[init(contract = "ovl_operator", parameter = "InitParams")]
fn contract_init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    let params: InitParams = ctx.parameter_cursor().get()?;
    ensure!(
        params.operators.len() > 1,
        CustomContractError::Inappropriate.into()
    );
    Ok(State::new(state_builder, params.operators))
}

// ============================================================
// Entrypoint for self contract
// ============================================================

/// The parameter type for the contract function `addOperatorKeys`.
#[derive(Debug, Serialize, SchemaType)]
pub struct UpdatePublicKeyParams {
    // /// Contract operators
    // #[concordium(size_length = 1)]
    // operators: Vec<OperatorWithKeyParam>,
    /// Signatures of those who approve calling the contract.
    signatures: BTreeSet<(AccountAddress, SignatureEd25519)>,
    /// Message that was signed.
    message: PermitMessageWithParameter,
}

/// Register a public key for a given account. The corresponding private
/// key can sign messages that can be submitted to the function.
/// Once registered, the public key cannot be updated.
///
/// It rejects if:
/// - Fails to parse parameter.
/// - Not enough keys.
/// - Contract or Entrypoint is not compatible with signed message.
/// - Signature is expired.
/// - A public key is already registered to the account.
#[receive(
    contract = "ovl_operator",
    name = "addOperatorKeys",
    error = "ContractError",
    parameter = "UpdatePublicKeyParams",
    crypto_primitives,
    mutable
)]
fn contract_add_operators<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    crypto_primitives: &impl HasCryptoPrimitives,
) -> ContractResult<()> {
    // Parse the parameter.
    let params: UpdatePublicKeyParams = ctx.parameter_cursor().get()?;

    host.state().check_auth(
        PermitAction::AddKey,
        &params.message,
        &params.signatures,
        ctx,
        crypto_primitives,
    )?;

    let operators: Vec<OperatorWithKeyParam> = from_bytes(&params.message.parameter).unwrap();

    // execute a specific operation of this entrypoint
    for param in operators {
        // Register the public key.
        host.state_mut()
            .add_operator(&param.account, &param.public_key)?;
    }

    Ok(())
}

/// Register a public key for a given account. The corresponding private
/// key can sign messages that can be submitted to the function.
/// Once registered, the public key cannot be updated.
///
/// It rejects if:
/// - Fails to parse parameter.
/// - Not enough keys.
/// - Contract or Entrypoint is not compatible with signed message.
/// - Signature is expired.
#[receive(
    contract = "ovl_operator",
    name = "removeOperatorKeys",
    error = "ContractError",
    parameter = "UpdatePublicKeyParams",
    crypto_primitives,
    mutable
)]
fn contract_remove_operators<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    crypto_primitives: &impl HasCryptoPrimitives,
) -> ContractResult<()> {
    // Parse the parameter.
    let params: UpdatePublicKeyParams = ctx.parameter_cursor().get()?;

    host.state().check_auth(
        PermitAction::RemoveKey,
        &params.message,
        &params.signatures,
        ctx,
        crypto_primitives,
    )?;

    let operators: Vec<OperatorWithKeyParam> = from_bytes(&params.message.parameter).unwrap();

    // execute a specific operation of this entrypoint
    for param in operators {
        // Delete the public key.
        host.state_mut().remove_operator(&param.account)?;
    }

    Ok(())
}

// --------------------------------------

#[derive(Debug, Serialize, SchemaType)]
struct UpgradeParams {
    /// The new module reference.
    module: ModuleReference,
    /// Optional entrypoint to call in the new module after upgrade.
    migrate: Option<(OwnedEntrypointName, OwnedParameter)>,
    /// Signatures of those who approve upgrading the contract.
    signatures: BTreeSet<(AccountAddress, SignatureEd25519)>,
    /// Message that was signed.
    message: PermitMessageWithParameter,
}

/// Upgrade contract.
/// Even the contract owner cannot be executed at one's own discretion.
/// Note: should not be called except in case of emergency.
///
/// Caller: contract instance owner only
/// Reject if:
/// - The sender is not the contract owner.
/// - Fails to parse parameter
/// - The message was intended for a different contract.
/// - The message was intended for a different `entry_point`.
/// - The message is expired.
/// - No multiple valid signatures.

#[receive(
    contract = "ovl_operator",
    name = "upgrade",
    mutable,
    crypto_primitives,
    parameter = "UpgradeParams"
)]
fn contract_upgrade<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    crypto_primitives: &impl HasCryptoPrimitives,
) -> ContractResult<()> {
    // Authorize the sender.
    ensure!(
        ctx.sender().matches_account(&ctx.owner()),
        ContractError::Unauthorized
    );

    // Parse the parameter.
    let params: UpgradeParams = ctx.parameter_cursor().get()?;

    host.state().check_auth(
        PermitAction::Upgrade,
        &params.message,
        &params.signatures,
        ctx,
        crypto_primitives,
    )?;

    // #[todo] Need to make sure params.module is approved by operators.

    // Trigger the upgrade.
    host.upgrade(params.module)?;
    // Call a migration function if provided.
    if let Some((func, parameter)) = params.migrate {
        host.invoke_contract_raw(
            &ctx.self_address(),
            parameter.as_parameter(),
            func.as_entrypoint_name(),
            Amount::zero(),
        )?;
    }
    Ok(())
}

// ============================================================
// Entrypoint for invoking sale contract
// ============================================================

#[derive(PartialEq, Eq, Debug)]
struct RawReturnValue(Option<Vec<u8>>);

impl Serial for RawReturnValue {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match &self.0 {
            Some(rv) => out.write_all(rv),
            None => Ok(()),
        }
    }
}

#[derive(Debug, Serialize, SchemaType)]
pub struct InvokeParams {
    /// Signatures of those who approve invoking the contract.
    signatures: BTreeSet<(AccountAddress, SignatureEd25519)>,
    /// Message that was signed.
    message: PermitMessageWithParameter,
}

/// The fallback method, which redirects the invocations to the sale contract.
#[receive(
    contract = "ovl_operator",
    parameter = "InvokeParams",
    fallback,
    mutable,
    crypto_primitives
)]
fn contract_invoke_sale<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    crypto_primitives: &impl HasCryptoPrimitives,
) -> ContractResult<RawReturnValue> {
    let mut cursor = ctx.parameter_cursor();
    let params: InvokeParams = cursor.get()?;

    // Check that the message was intended for this contract.
    ensure_eq!(
        params.message.contract_address,
        ctx.self_address(),
        CustomContractError::WrongContract.into()
    );

    ensure_eq!(
        params.message.entry_point,
        ctx.named_entrypoint(),
        CustomContractError::WrongEntrypoint.into()
    );

    // Check signature is not expired.
    ensure!(
        params.message.timestamp > ctx.metadata().slot_time(),
        CustomContractError::Expired.into()
    );

    // Calculate the message hash.
    let message_hash = crypto_primitives
        .hash_sha2_256(&to_bytes(&params.message))
        .0;

    let mut legit = 0;
    for (addr, sig) in params.signatures {
        let pubkey = host
            .state_mut()
            .operators
            .entry(addr)
            .occupied_or(CustomContractError::NoPublicKey)?;

        if crypto_primitives.verify_ed25519_signature(*pubkey, sig, &message_hash) {
            legit += 1;
        }
    }

    ensure!(legit > 1, ContractError::Unauthorized);

    let (contract, entrypoint) = match params.message.action {
        PermitAction::Invoke(contract, entrypoint) => (contract, entrypoint),
        _ => bail!(CustomContractError::Inappropriate.into()),
    };

    ensure_eq!(
        entrypoint,
        ctx.named_entrypoint(),
        CustomContractError::WrongContract.into()
    );

    let return_value = host
        .invoke_contract_raw(
            &contract,
            Parameter::new_unchecked(&params.message.parameter),
            entrypoint.as_entrypoint_name(),
            Amount::zero(),
        )?
        .1;

    match return_value {
        Some(mut rv) => {
            let mut rv_buffer = vec![0; rv.size() as usize];
            rv.read_exact(&mut rv_buffer)?;
            Ok(RawReturnValue(Some(rv_buffer)))
        },
        None => Ok(RawReturnValue(None)),
    }
}

type ViewOperatorsResponse = Vec<(AccountAddress, PublicKeyEd25519)>;

#[receive(
    contract = "ovl_operator",
    name = "viewOperators",
    return_value = "ViewOperatorsResponse"
)]
fn contract_view_participants<S: HasStateApi>(
    _ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ReceiveResult<ViewOperatorsResponse> {
    let state = host.state();

    let mut ret: Vec<(AccountAddress, PublicKeyEd25519)> = Vec::new();
    for (addr, key) in state.operators.iter() {
        ret.push((*addr, *key));
    }

    Ok(ret)
}
