#![allow(unused)]
mod state;

use concordium_std::{collections::BTreeMap, *};
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
