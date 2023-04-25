#[cfg(any(feature = "wasm-test", test))]
mod sctest;
mod state;
mod view;

use concordium_cis2::{
    AdditionalData, OnReceivingCis2Params, Receiver, TokenIdUnit, Transfer, TransferParams,
};
use concordium_std::{collections::BTreeMap, *};
use sale_utils::{PUBLIC_RIDO_FEE, PUBLIC_RIDO_FEE_BBB, PUBLIC_RIDO_FEE_OVL};
use state::{State, *};

#[derive(Debug, Serialize, SchemaType)]
pub struct InitParams {
    /// Contract owner
    pub(crate) operator: Receiver,
    /// cis2 contract for usdc token
    pub usdc_contract: ContractAddress,
    /// Account of the administrator of the entity running the IDO
    pub proj_admin: AccountAddress,
    /// Address of Overlay for receiving sale fee
    pub addr_ovl: Address,
    /// Address of Overlay for buy back burn
    pub addr_bbb: Address,
    /// IDO schedule(The process is split into some phases)
    pub open_at: BTreeMap<Timestamp, Prior>,
    /// Sale End Time
    pub close_at: Timestamp,
    /// User(sale particicants) can withdraw assets according to the vesting period
    pub vesting_period: BTreeMap<Duration, AllowedPercentage>,
    /// Swap price of the project token
    pub price_per_token: MicroUsdc,
    /// Amount of project tokens contained in a unit
    pub token_per_unit: ContractTokenAmount,
    /// Hardcap
    pub max_units: UnitsAmount,
    /// Softcap
    pub min_units: UnitsAmount,
}

/// # Init Function
/// everyone can init this module, but need to be initialized by ovl_team
/// since contract_id is needed to record into project contract.
#[init(contract = "pub_rido_usdc", parameter = "InitParams")]
fn contract_init<S: HasStateApi>(
    ctx: &impl HasInitContext,
    state_builder: &mut StateBuilder<S>,
) -> InitResult<State<S>> {
    let params: InitParams = ctx.parameter_cursor().get()?;

    let schedule = SaleSchedule::new(
        ctx.metadata().slot_time(),
        params.open_at,
        params.close_at,
        params.vesting_period,
    )?;

    // 1usdc == 1_000_000 microusdc
    let saleinfo = SaleInfo::new(
        params.price_per_token,
        params.token_per_unit,
        params.max_units,
        params.min_units,
    )?;

    Ok(State::new(
        state_builder,
        params.operator,
        params.usdc_contract,
        params.proj_admin,
        params.addr_ovl,
        params.addr_bbb,
        schedule,
        saleinfo,
    ))
}

// ==============================================
// For ovl team
// ==========================================

/// To change the status to something arbitrary, but is not normally used.
///
/// Caller: contract owner only
/// Reject if:
/// - The sender is not the contract instance owner.
/// - Fails to parse parameter
#[receive(
    contract = "pub_rido_usdc",
    name = "setStatus",
    parameter = "SaleStatus",
    error = "ContractError",
    mutable
)]
fn contract_set_status<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure!(
        ctx.sender().matches_account(&ctx.owner()),
        ContractError::Unauthorized
    );
    let status: SaleStatus = ctx.parameter_cursor().get()?;
    host.state_mut().status = status;

    Ok(())
}

/// Set status to fix for next stage(claim).
/// Note: if not reached softcap, the sale will be cancelled.
///
/// Caller: contract instance owner only
/// Reject if:
/// - The sender is not the contract owner.
/// - Called before the end of the sale
#[receive(
    contract = "pub_rido_usdc",
    name = "setFixed",
    error = "ContractError",
    mutable
)]
fn contract_set_fixed<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure!(
        ctx.sender().matches_account(&ctx.owner()),
        ContractError::Unauthorized
    );

    let mut state = host.state_mut();

    // This func does not work until the sale is closed.
    ensure!(
        state.schedule.is_sale_closed(ctx.metadata().slot_time()),
        CustomContractError::InvalidSchedule.into()
    );

    if state.saleinfo.is_reached_sc() {
        state.status = SaleStatus::Fixed;
    } else {
        state.status = SaleStatus::Suspend;
    }

    Ok(())
}

// #[TODO] WhitelistingParams can be shared with other sales.

/// Parameter type for the contract function `whitelisting`.
/// Currently user can be both account and contract.
/// [#TODO] But need to consider when user can be contract.
#[derive(Debug, Serialize, SchemaType)]
struct WhitelistingParams {
    /// the whitelist
    wl: Vec<AllowedUserParams>,
    /// If true, it means no further registration
    ready: bool,
}

#[derive(Debug, Serialize, SchemaType)]
struct AllowedUserParams {
    /// Users address to be whitelisted
    user: Address,
    /// Priority for participation in the sale
    prior: Prior,
}

/// Whitelist users who can participate in the sale
/// Note: All user can be allocated just one unit.
///
/// Caller: contract instance owner only
/// Reject if:
/// - Fails to parse parameter
/// - The sender is not the contract owner.
/// - Status is not Prepare
#[receive(
    contract = "pub_rido_usdc",
    name = "whitelisting",
    parameter = "Vec<AllowedUserParams>",
    error = "ContractError",
    mutable
)]
fn contract_whitelisting<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure!(
        ctx.sender().matches_account(&ctx.owner()),
        ContractError::Unauthorized
    );

    let mut state = host.state_mut();
    ensure_eq!(
        state.status,
        SaleStatus::Prepare,
        CustomContractError::AlreadySaleStarted.into()
    );

    let params: WhitelistingParams = ctx.parameter_cursor().get()?;

    // all can purchase only 1 unit;
    for AllowedUserParams { user, prior } in params.wl {
        if let Address::Account(_) = user {
            // if the user exists, just ignore.
            state.whitelisting(&user, prior);
        } else {
            // #[TODO] Only support AccountAddress for now.
            bail!(CustomContractError::AccountOnly.into())
        };
    }

    if params.ready {
        state.status = SaleStatus::Ready;
    }

    Ok(())
}

/// To claim sale fee for overlay team.
/// Note: 5% for now.
///
/// Caller: contract instance owner only
/// Reject if:
/// - The sender is not the contract owner
/// - Status is not Fixed
/// - Project admin has not yet registered the project token
/// - Project admin has not yet registered the TGE
#[receive(
    contract = "pub_rido_usdc",
    name = "ovlClaim",
    error = "ContractError",
    mutable
)]
fn contract_ovl_claim<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure!(
        ctx.sender().matches_account(&ctx.owner()),
        ContractError::Unauthorized
    );

    let mut state = host.state_mut();

    ensure!(
        state.status == SaleStatus::Fixed,
        CustomContractError::SaleNotFixed.into()
    );

    ensure!(
        state.project_token.is_some(),
        CustomContractError::NotSetProjectToken.into()
    );
    ensure!(
        state.schedule.vesting_start.is_some(),
        CustomContractError::NotSetTge.into()
    );

    let vesting_start = state.schedule.vesting_start.unwrap();

    let now = ctx.metadata().slot_time();
    let total_units = cmp::min(state.saleinfo.max_units, state.saleinfo.applied_units);

    let (amount, inc): (ContractTokenAmount, u8) = state.calc_vesting_amount(
        now,
        vesting_start,
        total_units as u64,
        PUBLIC_RIDO_FEE_OVL,
        state.ovl_claimed_inc,
    )?;

    if inc > state.ovl_claimed_inc {
        state.ovl_claimed_inc = inc;
    }

    if amount.0 > 0 {
        let to = match state.addr_ovl {
            Address::Account(account_addr) => Receiver::from_account(account_addr),
            Address::Contract(contract_addr) => Receiver::from_contract(
                contract_addr,
                OwnedEntrypointName::new_unchecked("callback".to_owned()),
            ),
        };

        let transfer = Transfer {
            from: Address::from(ctx.self_address()),
            to,
            token_id: TokenIdUnit(),
            amount,
            data: AdditionalData::empty(),
        };
        let project_token = state.project_token.unwrap();
        let _ = host.invoke_contract(
            &project_token,
            &TransferParams::from(vec![transfer]),
            EntrypointName::new_unchecked("transfer"),
            Amount::zero(),
        )?;
    }

    Ok(())
}

/// To claim sale fee for Buy Back Burn.
/// Note: 5% for now.
///
/// Caller: contract instance owner only
/// Reject if:
/// - The sender is not the contract owner
/// - Status is not Fixed
/// - Project admin has not yet registered the project token
/// - Project admin has not yet registered the TGE
#[receive(
    contract = "pub_rido_usdc",
    name = "bbbClaim",
    error = "ContractError",
    mutable
)]
fn contract_bbb_claim<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure!(
        ctx.sender().matches_account(&ctx.owner()),
        ContractError::Unauthorized
    );

    let mut state = host.state_mut();

    ensure!(
        state.status == SaleStatus::Fixed,
        CustomContractError::SaleNotFixed.into()
    );

    ensure!(
        state.project_token.is_some(),
        CustomContractError::NotSetProjectToken.into()
    );
    ensure!(
        state.schedule.vesting_start.is_some(),
        CustomContractError::NotSetTge.into()
    );

    let vesting_start = state.schedule.vesting_start.unwrap();

    let now = ctx.metadata().slot_time();
    let total_units = cmp::min(state.saleinfo.max_units, state.saleinfo.applied_units);

    let (amount, inc): (ContractTokenAmount, u8) = state.calc_vesting_amount(
        now,
        vesting_start,
        total_units as u64,
        PUBLIC_RIDO_FEE_BBB,
        state.bbb_claimed_inc,
    )?;

    if inc > state.bbb_claimed_inc {
        state.bbb_claimed_inc = inc;
    }

    if amount.0 > 0 {
        let to = match state.addr_bbb {
            Address::Account(account_addr) => Receiver::from_account(account_addr),
            Address::Contract(contract_addr) => Receiver::from_contract(
                contract_addr,
                OwnedEntrypointName::new_unchecked("callback".to_owned()),
            ),
        };

        let transfer = Transfer {
            from: Address::from(ctx.self_address()),
            to,
            token_id: TokenIdUnit(),
            amount,
            data: AdditionalData::empty(),
        };
        let project_token = state.project_token.unwrap();
        let _ = host.invoke_contract(
            &project_token,
            &TransferParams::from(vec![transfer]),
            EntrypointName::new_unchecked("transfer"),
            Amount::zero(),
        )?;
    }

    Ok(())
}

/// Change TGE(vesting period)
/// Note: should not be called except in case of emergency.
///
/// Caller: contract instance owner only
/// Reject if:
/// - Fails to parse parameter
/// - The sender is not the contract owner.
#[receive(
    contract = "pub_rido_usdc",
    name = "changeTGE",
    parameter = "Timestamp",
    error = "ContractError",
    mutable
)]
fn contract_change_tge<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    //#[TODO] need multiple sig?
    ensure!(
        ctx.sender().matches_account(&ctx.owner()),
        ContractError::Unauthorized
    );

    let ts: Timestamp = ctx.parameter_cursor().get()?;
    host.state_mut().schedule.vesting_start = Some(ts);

    Ok(())
}

/// Change project token contract
/// Note: should not be called except in case of emergency.
///
/// Caller: contract instance owner only
/// Reject if:
/// - Fails to parse parameter
/// - The sender is not the contract owner.
#[receive(
    contract = "pub_rido_usdc",
    name = "changePjtoken",
    parameter = "ContractAddress",
    error = "ContractError",
    mutable
)]
fn contract_change_pjtoken<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    //#[TODO] need multiple sig?
    ensure!(
        ctx.sender().matches_account(&ctx.owner()),
        ContractError::Unauthorized
    );

    let addr: ContractAddress = ctx.parameter_cursor().get()?;
    host.state_mut().project_token = Some(addr);

    Ok(())
}

/// Some transferable functions (createPool, projectClaim, deposit, quit, userClaim)
/// cannot be executed when the contract is paused.
///
/// Caller: contract instance owner only
/// Reject if:
/// - The sender is not the contract owner.
#[receive(
    contract = "pub_rido_usdc",
    name = "setPaused",
    error = "ContractError",
    mutable
)]
fn contract_set_paused<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure!(
        ctx.sender().matches_account(&ctx.owner()),
        ContractError::Unauthorized
    );
    host.state_mut().paused = true;
    Ok(())
}

/// The contract is unpaused.
///
/// Caller: contract instance owner only
/// Reject if:
/// - The sender is not the contract owner.
#[receive(
    contract = "pub_rido_usdc",
    name = "setUnpaused",
    error = "ContractError",
    mutable
)]
fn contract_set_unpaused<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure!(
        ctx.sender().matches_account(&ctx.owner()),
        ContractError::Unauthorized
    );
    host.state_mut().paused = false;
    Ok(())
}

// ==============================================
// For project admin
// ==========================================

/// Set project token contract.
///
/// Caller: #[TODO] not decided yet
/// Reject if:
/// - Fails to parse parameter
/// - Already set the contract address
/// - The sender is not the project admin ?
#[receive(
    contract = "pub_rido_usdc",
    name = "setPjtoken",
    parameter = "ContractAddress",
    error = "ContractError",
    mutable
)]
fn contract_set_pjtoken<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    // currently under discussion who should call this func
    ensure!(
        ctx.sender().matches_account(&host.state().proj_admin),
        ContractError::Unauthorized
    );
    let addr: ContractAddress = ctx.parameter_cursor().get()?;

    let mut state = host.state_mut();

    ensure!(
        state.project_token.is_none(),
        CustomContractError::Inappropriate.into()
    );

    state.project_token = Some(addr);

    Ok(())
}

/// Set TGE, which means it determines the beginning of the vesting period.
///
/// Caller: Project Admin only
/// Reject if:
/// - Fails to parse parameter
/// - The sender is not the project admin
/// - Already set the TGE
#[receive(
    contract = "pub_rido_usdc",
    name = "setTGE",
    parameter = "Timestamp",
    error = "ContractError",
    mutable
)]
fn contract_set_tge<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    ensure!(
        ctx.sender().matches_account(&host.state().proj_admin),
        ContractError::Unauthorized
    );
    let ts: Timestamp = ctx.parameter_cursor().get()?;

    let mut state = host.state_mut();

    ensure!(
        state.schedule.vesting_start.is_none(),
        CustomContractError::Inappropriate.into()
    );

    state.schedule.vesting_start = Some(ts);

    Ok(())
}

/// Project Administrator should this function once project token are generated.
/// The amount to be deposited must be the same as the amount sold at the sale
/// Note: This contract is supposed to be called from a CIS2 contract
///
/// Caller: Project Token Contract only
/// Invoker: Project Admin only
/// Reject if:
/// - Contract is paused
/// - Fails to parse parameter
/// - Status is not Fixed
/// - The sender is not the project token contract
/// - The quantity to be deposited differs from the quantity sold in the sale.
#[receive(
    contract = "pub_rido_usdc",
    name = "createPool",
    parameter = "OnReceivingCis2Params<ContractTokenId, ContractTokenAmount>",
    error = "ContractError"
)]
fn contract_create_pool<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    let state = host.state();
    ensure!(!state.paused, CustomContractError::ContractPaused.into());
    ensure_eq!(
        state.status,
        SaleStatus::Fixed,
        CustomContractError::SaleNotFixed.into()
    );

    let sender = if let Address::Contract(contract) = ctx.sender() {
        contract
    } else {
        bail!(CustomContractError::ContractOnly.into())
    };

    ensure!(
        sender == state.project_token.unwrap_or(ContractAddress::new(0, 0))
            && ctx.invoker() == state.proj_admin,
        ContractError::Unauthorized
    );

    let params: OnReceivingCis2Params<ContractTokenId, ContractTokenAmount> =
        ctx.parameter_cursor().get()?;

    //#[TODO] Check this func is only called after the sale is over.
    // if not need project_refund func
    let amount = state.saleinfo.amount_of_pjtoken()?;
    ensure!(
        amount == params.amount,
        CustomContractError::NotMatchAmount.into()
    );

    Ok(())
}

/// Project admin can claim USDC sold at the sale.
/// Note: No sale fee is charged to the project.
///
/// Caller: Anyone on the whitelist
/// Reject if:
/// - Contract is paused
/// - Status is not Fixed
/// - The sender is not the project admin
/// - Fails to invoke transfer from this contract to the admin
#[receive(
    contract = "pub_rido_usdc",
    name = "projectClaim",
    error = "ContractError",
    mutable
)]
fn contract_project_claim<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    let state = host.state();
    ensure!(!state.paused, CustomContractError::ContractPaused.into());

    ensure_eq!(
        state.status,
        SaleStatus::Fixed,
        CustomContractError::SaleNotFixed.into()
    );

    ensure!(
        ctx.sender().matches_account(&state.proj_admin),
        ContractError::Unauthorized
    );

    // Transfer the whole balance to the project admin.
    // #[TODO] need to check balanceOf?
    let total_tokens: ContractTokenAmount = state.saleinfo.amount_of_pjtoken()?;
    // this price is based on microusdc
    let total_saled = state.saleinfo.price_per_token as u64 * total_tokens.0;
    // let total_saled = state.price_per_token as u64 * total_tokens / 10_u64.pow(MICRO_USDC_DECIMALS);

    // let exchange_token = state.usdc_contract;
    let exchange_token = host.state().usdc_contract;
    let transfer = Transfer {
        from: Address::from(ctx.self_address()),
        to: Receiver::from_account(state.proj_admin),
        token_id: TokenIdUnit(),
        amount: ContractTokenAmount::from(total_saled),
        data: AdditionalData::empty(),
    };

    let ret = host.invoke_contract(
        &exchange_token,
        &TransferParams::from(vec![transfer]),
        EntrypointName::new_unchecked("transfer"),
        Amount::from_micro_ccd(0u64),
    );

    match ret {
        Ok((_, _)) => Ok(()),
        Err(e) => match e {
            _ => bail!(e.into()),
        },
    }
}

// ==============================================
// For users
// ==========================================

/// Sale participant call USCD contract for transfer USDC to this contract
/// to fix the right to purchase tokens
/// by deposit their USDC to this contract.
///
/// Caller: Anyone(Not limited to users on the whitelist)
/// Reject if:
/// - Contract is paused
/// - Status is not Ready
/// - User does not have valid priority
/// - User have already deposited
/// - Hardcap has already been reached
/// - Sended USDC not match Sale Amount
///
/// Note: host.invoke_transfer() can only transfer USDC to the AccountAddress.
/// If needed, host.invoke_contract() can trasfer USDC to the Contract, but need entrypoint!
#[receive(
    contract = "pub_rido_usdc",
    name = "userDeposit",
    parameter = "OnReceivingCis2Params<ContractTokenId, ContractTokenAmount>",
    error = "ContractError",
    mutable
)]
fn contract_user_deposit<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    let state = host.state_mut();

    ensure!(!state.paused, CustomContractError::ContractPaused.into());

    ensure!(
        state.status == SaleStatus::Ready,
        CustomContractError::SaleNotReady.into()
    );

    let sender = if let Address::Contract(contract) = ctx.sender() {
        contract
    } else {
        bail!(CustomContractError::ContractOnly.into())
    };
    ensure!(sender == state.usdc_contract, ContractError::Unauthorized);

    // get current priority
    let current_priority = state
        .schedule
        .check_sale_priority(ctx.metadata().slot_time());

    ensure!(
        current_priority.is_some(),
        CustomContractError::InvalidSchedule.into()
    );
    let current_priority = current_priority.unwrap();

    let room = state.saleinfo.check_room_to_apply();
    ensure!(room > 0, CustomContractError::AlreadySaleClosed.into());

    let invoker = Address::from(ctx.invoker());
    let user_state = state.get_user_any(&invoker)?;

    // check already deposited
    ensure!(
        user_state.win_units == 0,
        CustomContractError::AlreadyDeposited.into()
    );

    // check priority the user have
    if user_state.prior > current_priority {
        bail!(ContractError::Unauthorized)
    }

    // update userstate
    let win_units: u8 = user_state.tgt_units;
    ensure!(
        room >= win_units as u32,
        CustomContractError::AlreadySaleClosed.into()
    );

    let params: OnReceivingCis2Params<ContractTokenId, ContractTokenAmount> =
        ctx.parameter_cursor().get()?;

    let calced_price: ContractTokenAmount =
        state.saleinfo.calc_price_per_unit()? * win_units as u64;

    ensure!(
        params.amount == calced_price,
        CustomContractError::NotMatchAmount.into()
    );
    let _ = state.deposit(&invoker, params.amount, win_units)?;

    Ok(())
}

/// Sale participants can claim project token when the vesting period arrives.
/// Note: If a user claims many times within a certain period of time,
/// they will just get 0 back.
///
/// Caller: Anyone on the whitelist
/// Reject if:
/// - Contract is paused
/// - Status is not Fixed
/// - Project admin has not yet registered the project token
/// - Project admin has not yet registered the TGE
/// - The sender is not on the whitelist
#[receive(
    contract = "pub_rido_usdc",
    name = "userClaim",
    error = "ContractError",
    mutable
)]
fn contract_user_claim<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    let state = host.state_mut();

    ensure!(!state.paused, CustomContractError::ContractPaused.into());
    ensure_eq!(
        state.status,
        SaleStatus::Fixed,
        CustomContractError::SaleNotFixed.into()
    );

    ensure!(
        state.project_token.is_some(),
        CustomContractError::NotSetProjectToken.into()
    );
    ensure!(
        state.schedule.vesting_start.is_some(),
        CustomContractError::NotSetTge.into()
    );
    let vesting_start = state.schedule.vesting_start.unwrap();

    let user = ctx.sender();
    let user_state = state.get_user(&user)?;

    let now = ctx.metadata().slot_time();

    let (amount, inc): (ContractTokenAmount, u8) = state.calc_vesting_amount(
        now,
        vesting_start,
        user_state.win_units as u64,
        100 - PUBLIC_RIDO_FEE,
        user_state.claimed_inc,
    )?;

    if inc > user_state.claimed_inc {
        state.increment_user_claimed(&user, inc)?;
    }

    if amount.0 > 0 {
        let to = match user {
            Address::Account(account_address) => Receiver::from_account(account_address),
            Address::Contract(contract_address) => Receiver::from_contract(
                contract_address,
                OwnedEntrypointName::new_unchecked("callback".to_owned()),
            ),
        };

        let transfer = Transfer {
            from: Address::from(ctx.self_address()),
            to,
            token_id: TokenIdUnit(),
            amount: ContractTokenAmount::from(amount),
            data: AdditionalData::empty(),
        };

        let project_token = state.project_token.unwrap();

        let _ = host.invoke_contract(
            &project_token,
            &TransferParams::from(vec![transfer]),
            EntrypointName::new_unchecked("transfer"),
            Amount::zero(),
        )?;
    }

    Ok(())
}

/// Sale participants call this function to quit the sale and
/// to be refunded their usdc.
/// Note: Not available for now, means no one can quit once deposit their fund.
///
/// Caller: No one
/// Reject if:
/// - Always(currently)
/// - Contract is paused
/// - Status is not Ready
/// - Not on sale
/// - The sender is not on the whitelist
/// - The sender has not deposited.
/// - The sender is ContractAddress.
///
/// Note: host.invoke_transfer() can only transfer USDC to the AccountAddress.
/// If needed, host.invoke_contract() can trasfer USDC to the Contract, but need entrypoint!
#[receive(
    contract = "pub_rido_usdc",
    name = "userQuit",
    error = "ContractError",
    mutable
)]
fn contract_user_quit<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    let state = host.state_mut();
    ensure!(!state.paused, CustomContractError::ContractPaused.into());

    if state.status != SaleStatus::Suspend {
        // currently no one can quit
        ensure!(false, CustomContractError::DisabledForNow.into());

        ensure_eq!(
            state.status,
            SaleStatus::Ready,
            CustomContractError::SaleNotReady.into()
        );

        ensure!(
            state.schedule.is_on_sale(ctx.metadata().slot_time()),
            CustomContractError::InvalidSchedule.into()
        );
    }

    let sender = ctx.sender();
    let user = state.get_user(&sender)?;

    ensure!(user.win_units > 0, CustomContractError::NotDeposited.into());

    let user_addr = if let Address::Account(addr) = sender {
        addr
    } else {
        // [#TODO] If need to transfer to Contract, consider invoke_contract.
        // But in that case, the contract need to implement specific entrypoint.
        bail!(CustomContractError::AccountOnly.into())
    };

    state.remove_participant(&sender, user.win_units);

    let exchange_token = state.usdc_contract;
    let transfer = Transfer {
        from: Address::from(ctx.self_address()),
        to: Receiver::from_account(user_addr),
        token_id: TokenIdUnit(),
        amount: ContractTokenAmount::from(user.deposit_usdc),
        data: AdditionalData::empty(),
    };

    let ret = host.invoke_contract(
        &exchange_token,
        &TransferParams::from(vec![transfer]),
        EntrypointName::new_unchecked("transfer"),
        Amount::from_micro_ccd(0u64),
    );

    match ret {
        Ok((_, _)) => Ok(()),
        Err(e) => match e {
            _ => bail!(e.into()),
        },
    }
}
