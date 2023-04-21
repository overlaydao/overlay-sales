#![allow(unused)]
mod state;

use concordium_cis2::{
    AdditionalData, OnReceivingCis2Params, Receiver, TokenIdUnit, Transfer, TransferParams,
};
use concordium_std::{collections::BTreeMap, *};
use sale_utils::{PUBLIC_RIDO_FEE, PUBLIC_RIDO_FEE_BBB, PUBLIC_RIDO_FEE_OVL};
use state::{State, *};

#[derive(Debug, Serialize, SchemaType)]
pub struct InitParams {
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
