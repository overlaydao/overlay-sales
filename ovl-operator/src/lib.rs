//! # A Concordium V1 smart contract
#![allow(unused)]
use concordium_std::{
    collections::{BTreeMap, BTreeSet},
    *,
};
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
    Ok(State::new(state_builder, params.operators))
}

// ============================================================
// Common
// ============================================================
/// Part of the parameter type for PermitMessage.
#[derive(Debug, Serialize, SchemaType, Clone)]
enum PermitAction {
    AddKey,
    RemoveKey,
    Upgrade,
    Invoke(
        /// The invoking address.
        ContractAddress,
        /// The function to call on the invoking contract.
        OwnedEntrypointName,
    ),
}

/// Part of the parameter type for calling this contract.
/// Specifies the message that is signed.
#[derive(SchemaType, Serialize, Debug)]
struct PermitMessage {
    /// The contract_address that the signature is intended for.
    contract_address: ContractAddress,
    /// The entry_point that the signature is intended for.
    entry_point: OwnedEntrypointName,
    /// Enum to identify the action.
    action: PermitAction,
    /// A timestamp to make signatures expire.
    timestamp: Timestamp,
}

/// Part of the parameter type for calling this contract.
/// Specifies the message that is signed.
#[derive(SchemaType, Serialize, Debug)]
struct PermitMessageWithParameter {
    /// The contract_address that the signature is intended for.
    contract_address: ContractAddress,
    /// The entry_point that the signature is intended for.
    entry_point: OwnedEntrypointName,
    /// Enum to identify the action.
    action: PermitAction,
    /// A timestamp to make signatures expire.
    timestamp: Timestamp,
    /// The serialized parameter that should be forwarded to callee entrypoint.
    #[concordium(size_length = 2)]
    parameter: Vec<u8>,
}

// ============================================================
// Entrypoint for self contract
// ============================================================

/// The parameter type for the contract function `addOperatorKeys`.
#[derive(Debug, Serialize, SchemaType)]
pub struct AddPublicKeyParams {
    /// Contract operators
    #[concordium(size_length = 1)]
    operators: Vec<OperatorWithKeyParam>,
    /// Signatures of those who approve calling the contract.
    signatures: BTreeSet<(AccountAddress, SignatureEd25519)>,
    /// Message that was signed.
    message: PermitMessage,
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
    parameter = "AddPublicKeyParams",
    crypto_primitives,
    mutable
)]
fn contract_add_operators<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
    crypto_primitives: &impl HasCryptoPrimitives,
) -> ContractResult<()> {
    // Parse the parameter.
    let params: AddPublicKeyParams = ctx.parameter_cursor().get()?;

    // Check that the message was intended for this contract.
    ensure_eq!(
        params.message.contract_address,
        ctx.self_address(),
        CustomContractError::WrongContract.into()
    );

    // Check that calling function is appropriate.
    ensure_eq!(
        params.message.entry_point.as_entrypoint_name(),
        ctx.named_entrypoint().as_entrypoint_name(),
        CustomContractError::WrongContract.into()
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

    // #[TODO] use constant or parameter
    ensure!(legit > 1, ContractError::Unauthorized);

    // execute a specific operation of this entrypoint
    for param in params.operators {
        // Register the public key.
        let old_public_key = host
            .state_mut()
            .operators
            .insert(param.account, param.public_key);

        // Return an error if the owner tries to update/re-write a new public key for an
        // already registered account.
        ensure!(
            old_public_key.is_none(),
            CustomContractError::AccountDuplicated.into()
        );
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
    message: PermitMessage,
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

    // Check that the message was intended for this contract.
    ensure_eq!(
        params.message.contract_address,
        ctx.self_address(),
        CustomContractError::WrongContract.into()
    );

    ensure_eq!(
        params.message.entry_point.as_entrypoint_name(),
        ctx.named_entrypoint().as_entrypoint_name(),
        CustomContractError::WrongContract.into()
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
        CustomContractError::WrongContract.into()
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

// =============================================================

#[concordium_cfg_test]
mod tests {
    use super::*;
    use test_infrastructure::*;

    const ACCOUNT0: AccountAddress = AccountAddress([0u8; 32]);
    const ACCOUNT1: AccountAddress = AccountAddress([1u8; 32]);
    const ACCOUNT2: AccountAddress = AccountAddress([2u8; 32]);

    const SELF_ADDRESS: ContractAddress = ContractAddress {
        index: 10,
        subindex: 0,
    };

    const KEY1: PublicKeyEd25519 = PublicKeyEd25519([
        28, 115, 212, 197, 142, 123, 35, 196, 89, 64, 24, 163, 206, 227, 95, 183, 204, 18, 9, 250,
        196, 191, 226, 185, 139, 83, 165, 99, 56, 180, 46, 57,
    ]);

    const KEY2: PublicKeyEd25519 = PublicKeyEd25519([
        105, 229, 243, 235, 166, 114, 145, 226, 213, 241, 2, 3, 243, 211, 212, 201, 84, 45, 75, 2,
        204, 209, 86, 162, 41, 240, 250, 255, 243, 232, 27, 167,
    ]);

    const SIGNATURE1: SignatureEd25519 = SignatureEd25519([
        208, 226, 169, 210, 105, 81, 90, 239, 139, 19, 105, 67, 87, 156, 232, 198, 5, 116, 105, 32,
        200, 252, 173, 118, 124, 24, 12, 119, 133, 41, 47, 25, 16, 127, 129, 83, 91, 72, 238, 117,
        119, 167, 67, 153, 169, 227, 159, 134, 78, 14, 204, 115, 185, 40, 168, 136, 94, 67, 33, 35,
        121, 141, 206, 6,
    ]);

    const SIGNATURE2: SignatureEd25519 = SignatureEd25519([
        121, 71, 126, 175, 214, 63, 25, 78, 197, 17, 194, 249, 155, 198, 245, 13, 255, 223, 188,
        46, 0, 147, 114, 103, 42, 156, 240, 141, 178, 168, 67, 88, 153, 178, 204, 236, 158, 1, 134,
        176, 154, 122, 18, 226, 252, 19, 74, 132, 211, 105, 227, 54, 250, 17, 59, 180, 18, 173, 84,
        101, 33, 244, 217, 13,
    ]);

    #[derive(Debug, Serialize)]
    struct WhitelistingParams {
        /// the whitelist
        wl: Vec<AllowedUserParams>,
        /// If true, it means no further registration
        ready: bool,
    }

    #[derive(Debug, Serialize)]
    struct AllowedUserParams {
        /// Users address to be whitelisted
        user: Address,
        /// Priority for participation in the sale
        prior: Prior,
    }

    fn init_context(
        sender: AccountAddress,
        slot_time: SlotTime,
        parameter_bytes: &[u8],
    ) -> TestInitContext {
        let mut ctx = TestInitContext::empty();
        ctx.set_init_origin(sender);
        ctx.set_metadata_slot_time(slot_time);
        ctx.set_parameter(parameter_bytes);
        ctx
    }

    fn receive_context(
        owner: AccountAddress,
        invoker: AccountAddress,
        sender: Address,
        slot_time: SlotTime,
        parameter_bytes: &[u8],
    ) -> TestReceiveContext {
        let mut ctx = TestReceiveContext::empty();
        ctx.set_self_address(SELF_ADDRESS);
        ctx.set_metadata_slot_time(slot_time);
        ctx.set_owner(owner);
        ctx.set_invoker(invoker);
        ctx.set_sender(sender);
        ctx.set_parameter(parameter_bytes);
        ctx
    }

    /// Test initialization succeeds.
    #[concordium_test]
    fn test_init() {
        let mut builder = TestStateBuilder::new();

        let ops1 = OperatorWithKeyParam {
            account: ACCOUNT1,
            public_key: KEY1,
        };
        let ops2 = OperatorWithKeyParam {
            account: ACCOUNT2,
            public_key: KEY2,
        };
        let ops3 = OperatorWithKeyParam {
            account: ACCOUNT1,
            public_key: KEY2,
        };

        let params = InitParams {
            operators: vec![ops1, ops2, ops3],
        };
        let params_byte = to_bytes(&params);

        let ctx = init_context(ACCOUNT0, Timestamp::from_timestamp_millis(1), &params_byte);
        let result = contract_init(&ctx, &mut builder).unwrap();
    }

    #[concordium_test]
    fn test_invoke() {
        let acc1 = AccountAddress([11u8; 32]);
        let acc2 = AccountAddress([12u8; 32]);

        let ops1 = OperatorWithKeyParam {
            account: ACCOUNT1,
            public_key: KEY1,
        };
        let ops2 = OperatorWithKeyParam {
            account: ACCOUNT2,
            public_key: KEY2,
        };
        let ops3 = OperatorWithKeyParam {
            account: ACCOUNT1,
            public_key: KEY2,
        };

        let mut state_builder = TestStateBuilder::new();

        let mut state = State {
            operators: state_builder.new_map(),
        };

        for v in &vec![ops1, ops2, ops3] {
            state.operators.insert(v.account, v.public_key);
        }

        let mut host = TestHost::new(state, state_builder);
        host.setup_mock_entrypoint(
            ContractAddress {
                index: 9,
                subindex: 0,
            },
            OwnedEntrypointName::new_unchecked("whitelisting".into()),
            MockFn::new_v1(move |parameter, _amount, _balance, _state| {
                let param_bytes = parameter.as_ref();
                Ok((false, ()))
            }),
        );

        let mut signatures = BTreeSet::new();
        signatures.insert((ACCOUNT1, SIGNATURE1));
        signatures.insert((ACCOUNT2, SIGNATURE2));

        let whitelist = vec![
            AllowedUserParams {
                user: Address::Account(AccountAddress([10u8; 32])),
                prior: Prior::TOP,
            },
            AllowedUserParams {
                user: Address::Account(AccountAddress([11u8; 32])),
                prior: Prior::TOP,
            },
            AllowedUserParams {
                user: Address::Account(AccountAddress([12u8; 32])),
                prior: Prior::SECOND,
            },
            AllowedUserParams {
                user: Address::Account(AccountAddress([13u8; 32])),
                prior: Prior::SECOND,
            },
            AllowedUserParams {
                user: Address::Account(AccountAddress([14u8; 32])),
                prior: Prior::ANY,
            },
        ];

        let invoke_params = WhitelistingParams {
            wl: whitelist,
            ready: true,
        };

        // Deposit from user1
        let params = InvokeParams {
            signatures,
            message: PermitMessageWithParameter {
                contract_address: ContractAddress {
                    index: 10,
                    subindex: 0,
                },
                entry_point: OwnedEntrypointName::new_unchecked("whitelisting".into()),
                action: PermitAction::Invoke(
                    ContractAddress {
                        index: 9,
                        subindex: 0,
                    },
                    OwnedEntrypointName::new_unchecked("whitelisting".into()),
                ),
                timestamp: Timestamp::from_timestamp_millis(100),
                parameter: to_bytes(&invoke_params),
            },
        };
        let params_bytes = to_bytes(&params);
        let mut ctx = receive_context(
            ACCOUNT0,
            ACCOUNT0,
            Address::from(ACCOUNT0),
            Timestamp::from_timestamp_millis(15),
            &params_bytes,
        );
        ctx.set_named_entrypoint(OwnedEntrypointName::new_unchecked("whitelisting".into()));

        let crypto_primitives = TestCryptoPrimitives::new();
        let result: ContractResult<_> = contract_invoke_sale(&ctx, &mut host, &crypto_primitives);
        println!("{:?}", result);
        claim!(result.is_ok(), "Results in rejection with user1 deposit");
    }
}
