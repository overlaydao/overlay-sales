mod overlay_team;
mod participant;
mod project_admin;

use super::state::{State, *};
use concordium_cis2::Receiver;
use concordium_std::test_infrastructure::*;
use concordium_std::{collections::BTreeMap, *};

const USDC: ContractAddress = ContractAddress {
    index: 1,
    subindex: 0,
};
const SELF_ADDRESS: ContractAddress = ContractAddress {
    index: 10,
    subindex: 0,
};
const PJ_TOKEN: ContractAddress = ContractAddress {
    index: 11,
    subindex: 0,
};
const OVL_TEAM_ACC: AccountAddress = AccountAddress([0u8; 32]);
const PJ_ADMIN_ACC: AccountAddress = AccountAddress([1u8; 32]);
const ADDR_OVL: Address = Address::Account(AccountAddress([2u8; 32]));
const ADDR_BBB: Address = Address::Contract(ContractAddress {
    index: 100,
    subindex: 0,
});

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

fn def_sale_schedule() -> SaleSchedule {
    SaleSchedule {
        open_at: BTreeMap::from([
            (Timestamp::from_timestamp_millis(10), Prior::TOP),
            (Timestamp::from_timestamp_millis(20), Prior::SECOND),
        ]),
        close_at: Timestamp::from_timestamp_millis(30),
        vesting_start: None,
        vesting_period: BTreeMap::from([
            (Duration::from_days(1), 25),
            (Duration::from_days(2), 40),
            (Duration::from_days(3), 35),
        ]),
    }
}

fn def_sale_info(applied_units: u32) -> SaleInfo {
    SaleInfo {
        price_per_token: 5_000_000,
        token_per_unit: ContractTokenAmount::from(200),
        max_units: 100,
        min_units: 1,
        applied_units,
    }
}

fn def_operator() -> Receiver {
    Receiver::Contract(
        ContractAddress {
            index: 88,
            subindex: 0,
        },
        OwnedEntrypointName::new_unchecked("callback".to_owned()),
    )
}
