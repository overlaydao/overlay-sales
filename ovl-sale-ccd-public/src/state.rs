use collections::BTreeMap;
use concordium_std::{SchemaType, Serialize, *};
pub(crate) use sale_utils::{
    error::{ContractError, ContractResult, CustomContractError},
    types::*,
};

#[derive(Debug, Serial, DeserialWithState, StateClone)]
#[concordium(state_parameter = "S")]
pub struct State<S: HasStateApi> {
    // base info
    pub(crate) proj_admin: AccountAddress,
    pub(crate) status: SaleStatus,
    pub(crate) paused: bool,
    pub(crate) addr_ovl: Address,
    pub(crate) addr_bbb: Address,
    pub(crate) ovl_claimed_inc: u8,
    pub(crate) bbb_claimed_inc: u8,
    // dependencies
    pub(crate) project_token: Option<ContractAddress>,
    // schedule
    pub(crate) schedule: SaleSchedule,
    // saleinfo
    pub(crate) saleinfo: SaleInfo,
    // users
    pub(crate) participants: StateMap<Address, UserState, S>,
}

impl<S: HasStateApi> State<S> {
    pub(crate) fn new(
        state_builder: &mut StateBuilder<S>,
        proj_admin: AccountAddress,
        addr_ovl: Address,
        addr_bbb: Address,
        schedule: SaleSchedule,
        saleinfo: SaleInfo,
    ) -> Self {
        State {
            proj_admin,
            paused: false,
            status: SaleStatus::Prepare,
            addr_ovl,
            addr_bbb,
            ovl_claimed_inc: 0,
            bbb_claimed_inc: 0,
            project_token: None,
            schedule,
            saleinfo,
            participants: state_builder.new_map(),
        }
    }

    pub(crate) fn calc_vesting_amount(
        &mut self,
        now: Timestamp,
        vesting_start: Timestamp,
        total_units: u64,
        shared: u8,
        cur_inc: u8,
    ) -> ContractResult<(ContractTokenAmount, u8)> {
        let mut amount: u128 = 0;
        let mut inc: u8 = 0;

        for (duration, per) in self.schedule.vesting_period.iter() {
            let ts = match vesting_start.checked_add(*duration) {
                Some(v) => v,
                None => bail!(CustomContractError::InvalidSchedule.into()),
            };

            if now < ts {
                break;
            }

            if cur_inc > inc {
                inc += 1;
                continue;
            }

            let total_amount: u128 = (self.saleinfo.token_per_unit.0 as u128)
                .checked_mul(u128::from(total_units))
                .ok_or(ContractError::from(CustomContractError::OverflowError))?;

            let total_claimable: u128 = total_amount
                .checked_mul(u128::from(shared))
                .ok_or(ContractError::from(CustomContractError::OverflowError))?
                / 100;

            let allocation: u128 = total_claimable
                .checked_mul(u128::from(*per as u128))
                .ok_or(ContractError::from(CustomContractError::OverflowError))?
                / 100;

            amount += allocation;
            inc += 1;
        }

        if amount > u64::MAX as u128 {
            bail!(ContractError::from(CustomContractError::OverflowError))
        } else {
            Ok((ContractTokenAmount::from(amount as u64), inc))
        }
    }

    pub(crate) fn whitelist(&mut self, user: &Address, prior: Prior, tgt_units: u8) {
        // #[Caution] if the user exists, the state is overwritten.
        let _ = self
            .participants
            .entry(*user)
            .or_insert_with(|| UserState::new(prior, Amount::zero(), tgt_units));
    }

    pub(crate) fn get_user_any(
        &mut self,
        user: &Address,
        tgt_units: u8,
    ) -> ContractResult<UserState> {
        let user = self
            .participants
            .entry(*user)
            .or_insert_with(|| UserState::new(Prior::ANY, Amount::zero(), tgt_units));
        let user = user.get_ref();
        Ok(user.clone())
    }

    // fn modify_whitelist(
    //     &mut self,
    //     user: &Address,
    //     prior: Prior,
    //     tgt_units: u8,
    // ) -> ContractResult<()> {
    //     let mut user = self
    //         .participants
    //         .get_mut(&user)
    //         .ok_or(ContractError::Custom(CustomContractError::InvalidInput))?;
    //     user.prior = prior;
    //     Ok(())
    // }

    pub(crate) fn check_listed(&mut self, user: &Address) -> bool {
        self.participants.entry(*user).is_occupied()
    }

    pub(crate) fn get_user(&mut self, user: &Address) -> ContractResult<UserState> {
        let user = self
            .participants
            .get(user)
            .ok_or(ContractError::Unauthorized)?;
        Ok(user.clone())
    }

    pub(crate) fn deposit(
        &mut self,
        user: &Address,
        amount: Amount,
        win_units: u8,
    ) -> ContractResult<()> {
        let mut user = self
            .participants
            .get_mut(user)
            .ok_or(ContractError::Unauthorized)?;
        user.deposit_ccd = amount;
        user.win_units = win_units;

        self.saleinfo.applied_units += win_units as UnitsAmount;
        Ok(())
    }

    pub(crate) fn increment_user_claimed(&mut self, user: &Address, n: u8) -> ContractResult<()> {
        let mut user = self
            .participants
            .get_mut(user)
            .ok_or(ContractError::Unauthorized)?;
        user.claimed_inc = n;
        Ok(())
    }

    pub(crate) fn remove_participant(&mut self, user: &Address, tgt_units: u8) {
        self.participants.remove(user);
        self.saleinfo.applied_units -= tgt_units as UnitsAmount;
    }
}

#[derive(Debug, Serialize, SchemaType, Clone)]
pub struct SaleSchedule {
    pub(crate) open_at: BTreeMap<Timestamp, Prior>,
    pub(crate) close_at: Timestamp,
    pub(crate) vesting_start: Option<Timestamp>,
    pub(crate) vesting_period: BTreeMap<Duration, AllowedPercentage>,
}

impl SaleSchedule {
    pub fn new(
        now: Timestamp,
        open_at: BTreeMap<Timestamp, Prior>,
        close_at: Timestamp,
        vesting_period: BTreeMap<Duration, AllowedPercentage>,
    ) -> Result<Self, CustomContractError> {
        ensure!(!open_at.is_empty(), CustomContractError::InvalidSchedule);

        ensure!(
            now < *open_at.first_key_value().unwrap().0,
            CustomContractError::InvalidSchedule
        );

        ensure!(
            *open_at.last_key_value().unwrap().0 < close_at,
            CustomContractError::InvalidSchedule
        );

        // check vesting_period
        let mut total_per = 0;
        for (_, per) in vesting_period.iter() {
            total_per += *per;
        }
        ensure!(total_per == 100, CustomContractError::Inappropriate);

        Ok(SaleSchedule {
            open_at,
            close_at,
            vesting_start: None,
            vesting_period,
        })
    }

    pub(crate) fn is_sale_opened(&self, now: Timestamp) -> bool {
        if now < *self.open_at.first_key_value().unwrap().0 {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_sale_closed(&self, now: Timestamp) -> bool {
        if self.close_at < now {
            true
        } else {
            false
        }
    }

    pub(crate) fn is_on_sale(&self, now: Timestamp) -> bool {
        if *self.open_at.first_key_value().unwrap().0 <= now && now < self.close_at {
            true
        } else {
            false
        }
    }

    pub(crate) fn check_sale_priority(&self, now: Timestamp) -> Option<Prior> {
        if !self.is_on_sale(now) {
            return None;
        }

        let mut current = Prior::TOP;
        for (ts, priority) in self.open_at.iter() {
            if priority == &current {
                continue;
            }
            if now < *ts {
                return Some(current);
            } else {
                current = priority.clone();
            }
        }
        return Some(current);
    }
}

#[derive(Debug, Serialize, SchemaType, Clone)]
pub struct SaleInfo {
    pub(crate) price_per_token: MicroCcd,
    pub(crate) token_per_unit: ContractTokenAmount,
    pub(crate) max_units: UnitsAmount,
    pub(crate) min_units: UnitsAmount,
    pub(crate) applied_units: UnitsAmount,
}

impl SaleInfo {
    pub(crate) fn new(
        price_per_token: MicroCcd,
        token_per_unit: ContractTokenAmount,
        max_units: UnitsAmount,
        min_units: UnitsAmount,
    ) -> Result<Self, CustomContractError> {
        ensure!(min_units < max_units, CustomContractError::Inappropriate);

        Ok(SaleInfo {
            price_per_token,
            token_per_unit,
            max_units,
            min_units,
            applied_units: 0,
        })
    }

    pub(crate) fn check_room_to_apply(&self) -> UnitsAmount {
        if self.applied_units < self.max_units {
            self.max_units - self.applied_units
        } else {
            0
        }
    }

    pub(crate) fn is_reached_sc(&self) -> bool {
        if self.min_units <= self.applied_units {
            true
        } else {
            false
        }
    }

    pub(crate) fn amount_of_pjtoken(&self) -> ContractTokenAmount {
        self.token_per_unit * self.applied_units as u64
        // self.token_per_unit * self.max_units as u64
    }

    pub(crate) fn calc_price_per_unit(&self) -> MicroCcd {
        self.price_per_token * self.token_per_unit.0
    }
}

#[derive(Debug, Serialize, SchemaType, Clone, PartialEq, Eq)]
pub struct UserState {
    pub(crate) prior: Prior,
    pub(crate) deposit_ccd: Amount,
    pub(crate) tgt_units: u8,
    pub(crate) win_units: u8,
    pub(crate) claimed_inc: u8,
}

impl UserState {
    pub fn new(prior: Prior, deposit_ccd: Amount, tgt_units: u8) -> Self {
        UserState {
            prior,
            deposit_ccd,
            tgt_units,
            win_units: 0,
            claimed_inc: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sctest::init_parameter;
    use crate::test_infrastructure::*;
    use sale_utils::{PUBLIC_RIDO_FEE, PUBLIC_RIDO_FEE_BBB, PUBLIC_RIDO_FEE_OVL};

    const OVL_TEAM_ACC: AccountAddress = AccountAddress([0u8; 32]);
    const OVL_TEAM_ADDR: Address = Address::Account(OVL_TEAM_ACC);
    const PJ_ADMIN_ACC: AccountAddress = AccountAddress([1u8; 32]);
    const PJ_ADMIN_ADDR: Address = Address::Account(PJ_ADMIN_ACC);

    const USER1_ACC: AccountAddress = AccountAddress([10u8; 32]);
    const USER1_ADDR: Address = Address::Account(USER1_ACC);
    const USER2_ACC: AccountAddress = AccountAddress([11u8; 32]);
    const USER2_ADDR: Address = Address::Account(USER2_ACC);
    const USER3_ACC: AccountAddress = AccountAddress([12u8; 32]);
    const USER3_ADDR: Address = Address::Account(USER3_ACC);

    #[test]
    fn test_invalid_schedule() {
        let open_at = BTreeMap::from([
            (Timestamp::from_timestamp_millis(10), Prior::TOP),
            (Timestamp::from_timestamp_millis(20), Prior::SECOND),
        ]);
        let close_at = Timestamp::from_timestamp_millis(30);
        let vesting_period = BTreeMap::from([
            (Duration::from_days(30), 30),
            (Duration::from_days(60), 40),
            (Duration::from_days(90), 30),
        ]);
        let schedule = SaleSchedule::new(
            Timestamp::from_timestamp_millis(15),
            open_at,
            close_at,
            vesting_period,
        );

        // let schedule = match schedule {
        //     Ok(v) => v,
        //     Err(e) => bail!(e.into()),
        // };

        assert_eq!(
            schedule.expect_err_report("error"),
            CustomContractError::InvalidSchedule
        );
    }

    #[test]
    fn test_state() {
        // initialize
        let mut state_builder = TestStateBuilder::new();
        let params = init_parameter(BTreeMap::new());
        let schedule = SaleSchedule::new(
            Timestamp::from_timestamp_millis(1),
            params.open_at,
            params.close_at,
            params.vesting_period,
        )
        .unwrap_abort();

        let saleinfo = SaleInfo::new(
            params.price_per_token,
            params.token_per_unit,
            params.max_units,
            params.min_units,
        )
        .unwrap_abort();
        let mut state = State::new(
            &mut state_builder,
            params.proj_admin,
            params.addr_ovl,
            params.addr_bbb,
            schedule,
            saleinfo,
        );

        state
            .schedule
            .check_sale_priority(Timestamp::from_timestamp_millis(30));

        // whitelisted
        let users = vec![
            (&USER1_ADDR, Prior::TOP),
            (&USER2_ADDR, Prior::SECOND),
            (&USER2_ADDR, Prior::TOP),
        ];
        for v in users.into_iter() {
            state.whitelist(v.0, v.1, 1);
        }

        assert_eq!(
            state.get_user(&USER1_ADDR).unwrap(),
            UserState {
                prior: Prior::TOP,
                deposit_ccd: Amount::zero(),
                tgt_units: 1,
                win_units: 0,
                claimed_inc: 0
            },
            "something wrong with user1 before deposit!"
        );

        // deposit = allocation fixed
        state.deposit(&USER1_ADDR, Amount::from_ccd(100), 1);

        // vesting
        assert!(state.check_listed(&USER1_ADDR), "user1 should exist!");
        assert!(!state.check_listed(&USER3_ADDR), "user3 should not on list");
        assert_eq!(
            state.get_user(&USER1_ADDR).unwrap(),
            UserState {
                prior: Prior::TOP,
                deposit_ccd: Amount::from_micro_ccd(100_000_000),
                tgt_units: 1,
                win_units: 1,
                claimed_inc: 0
            },
            "something wrong with user1 after deposit!"
        );
    }

    #[test]
    fn test_vesting_first() {
        let first_per = 25;
        let max = 5_000_000;
        let applied = 6_000_000;
        let token_per_unit = 200_000_000;

        let mut state_builder = TestStateBuilder::new();
        let params = init_parameter(BTreeMap::new());
        let schedule = SaleSchedule::new(
            Timestamp::from_timestamp_millis(1),
            BTreeMap::from([
                (Timestamp::from_timestamp_millis(10), Prior::TOP),
                (Timestamp::from_timestamp_millis(20), Prior::SECOND),
            ]),
            Timestamp::from_timestamp_millis(30),
            BTreeMap::from([
                (Duration::from_millis(10), first_per),
                (Duration::from_millis(20), 40),
                (Duration::from_millis(30), 35),
            ]),
        )
        .unwrap();
        let saleinfo = SaleInfo::new(15_000_000, token_per_unit.into(), max, 100).unwrap();

        let mut state = State::new(
            &mut state_builder,
            params.proj_admin,
            params.addr_ovl,
            params.addr_bbb,
            schedule,
            saleinfo,
        );
        let cur_inc = 0;
        let total_units = cmp::min(state.saleinfo.max_units, applied);
        let ret = state
            .calc_vesting_amount(
                Timestamp::from_timestamp_millis(61),
                Timestamp::from_timestamp_millis(50),
                total_units as u64,
                PUBLIC_RIDO_FEE_OVL,
                cur_inc,
            )
            .unwrap();

        claim_eq!(
            ret.0 .0,
            total_units as u64 * token_per_unit * PUBLIC_RIDO_FEE_OVL as u64 / 100
                * first_per as u64
                / 100,
            "Something wrong with vesting calcuration!"
        );
        claim_eq!(ret.1, 1, "Something wrong with claimed_inc!");
    }

    #[test]
    fn test_vesting_too_early() {
        let first_per = 25;
        let max = 5_000_000;
        let applied = 6_000_000;
        let token_per_unit = 200_000_000;

        let mut state_builder = TestStateBuilder::new();
        let params = init_parameter(BTreeMap::new());
        let schedule = SaleSchedule::new(
            Timestamp::from_timestamp_millis(1),
            BTreeMap::from([
                (Timestamp::from_timestamp_millis(10), Prior::TOP),
                (Timestamp::from_timestamp_millis(20), Prior::SECOND),
            ]),
            Timestamp::from_timestamp_millis(30),
            BTreeMap::from([
                (Duration::from_millis(10), first_per),
                (Duration::from_millis(20), 40),
                (Duration::from_millis(30), 35),
            ]),
        )
        .unwrap();
        let saleinfo = SaleInfo::new(15_000_000, token_per_unit.into(), max, 100).unwrap();

        let mut state = State::new(
            &mut state_builder,
            params.proj_admin,
            params.addr_ovl,
            params.addr_bbb,
            schedule,
            saleinfo,
        );
        let cur_inc = 0;
        let total_units = cmp::min(state.saleinfo.max_units, applied);
        let ret = state
            .calc_vesting_amount(
                Timestamp::from_timestamp_millis(40),
                Timestamp::from_timestamp_millis(50),
                total_units as u64,
                PUBLIC_RIDO_FEE_OVL,
                cur_inc,
            )
            .unwrap();
        claim_eq!(ret.0 .0, 0, "Something wrong with vesting calcuration!");
        claim_eq!(ret.1, 0, "Something wrong with claimed_inc!");
    }

    #[test]
    fn test_vesting_second_all_at_once() {
        let first_per = 25;
        let second_per = 40;
        let max = 5_000_000;
        let applied = 6_000_000;
        let token_per_unit = 200_000_000;

        let mut state_builder = TestStateBuilder::new();
        let params = init_parameter(BTreeMap::new());
        let schedule = SaleSchedule::new(
            Timestamp::from_timestamp_millis(1),
            BTreeMap::from([
                (Timestamp::from_timestamp_millis(10), Prior::TOP),
                (Timestamp::from_timestamp_millis(20), Prior::SECOND),
            ]),
            Timestamp::from_timestamp_millis(30),
            BTreeMap::from([
                (Duration::from_millis(10), first_per),
                (Duration::from_millis(20), second_per),
                (Duration::from_millis(30), 35),
            ]),
        )
        .unwrap();
        let saleinfo = SaleInfo::new(15_000_000, token_per_unit.into(), max, 100).unwrap();

        let mut state = State::new(
            &mut state_builder,
            params.proj_admin,
            params.addr_ovl,
            params.addr_bbb,
            schedule,
            saleinfo,
        );
        let cur_inc = 0;
        let total_units = cmp::min(state.saleinfo.max_units, applied);
        let ret = state
            .calc_vesting_amount(
                Timestamp::from_timestamp_millis(70),
                Timestamp::from_timestamp_millis(50),
                total_units as u64,
                PUBLIC_RIDO_FEE_OVL,
                cur_inc,
            )
            .unwrap();

        claim_eq!(
            ret.0 .0,
            total_units as u64 * token_per_unit * PUBLIC_RIDO_FEE_OVL as u64 / 100
                * (first_per + second_per) as u64
                / 100,
            "Something wrong with vesting calcuration!"
        );
        claim_eq!(ret.1, 2, "Something wrong with claimed_inc!");
    }

    #[test]
    fn test_vesting_second_separate() {
        let first_per = 25;
        let second_per = 40;
        let max = 5_000_000;
        let applied = 6_000_000;
        let token_per_unit = 200_000_000;

        let mut state_builder = TestStateBuilder::new();
        let params = init_parameter(BTreeMap::new());
        let schedule = SaleSchedule::new(
            Timestamp::from_timestamp_millis(1),
            BTreeMap::from([
                (Timestamp::from_timestamp_millis(10), Prior::TOP),
                (Timestamp::from_timestamp_millis(20), Prior::SECOND),
            ]),
            Timestamp::from_timestamp_millis(30),
            BTreeMap::from([
                (Duration::from_millis(10), first_per),
                (Duration::from_millis(20), second_per),
                (Duration::from_millis(30), 35),
            ]),
        )
        .unwrap();
        let saleinfo = SaleInfo::new(15_000_000, token_per_unit.into(), max, 100).unwrap();

        let mut state = State::new(
            &mut state_builder,
            params.proj_admin,
            params.addr_ovl,
            params.addr_bbb,
            schedule,
            saleinfo,
        );
        let cur_inc = 1;
        let total_units = cmp::min(state.saleinfo.max_units, applied);
        let ret = state
            .calc_vesting_amount(
                Timestamp::from_timestamp_millis(70),
                Timestamp::from_timestamp_millis(50),
                total_units as u64,
                PUBLIC_RIDO_FEE_OVL,
                cur_inc,
            )
            .unwrap();

        claim_eq!(
            ret.0 .0,
            total_units as u64 * token_per_unit * PUBLIC_RIDO_FEE_OVL as u64 / 100
                * second_per as u64
                / 100,
            "Something wrong with vesting calcuration!"
        );
        claim_eq!(ret.1, 2, "Something wrong with claimed_inc!");
    }

    #[test]
    fn test_vesting_overflow() {
        let first_per = 25;
        let max = 5_000_000;
        let applied = 6_000_000;
        let token_per_unit = 200_000_000_000_000_000;

        let mut state_builder = TestStateBuilder::new();
        let params = init_parameter(BTreeMap::new());
        let schedule = SaleSchedule::new(
            Timestamp::from_timestamp_millis(1),
            BTreeMap::from([
                (Timestamp::from_timestamp_millis(10), Prior::TOP),
                (Timestamp::from_timestamp_millis(20), Prior::SECOND),
            ]),
            Timestamp::from_timestamp_millis(30),
            BTreeMap::from([
                (Duration::from_millis(10), first_per),
                (Duration::from_millis(20), 40),
                (Duration::from_millis(30), 35),
            ]),
        )
        .unwrap();
        let saleinfo = SaleInfo::new(5_000_000, token_per_unit.into(), max, 100).unwrap();

        let mut state = State::new(
            &mut state_builder,
            params.proj_admin,
            params.addr_ovl,
            params.addr_bbb,
            schedule,
            saleinfo,
        );
        let cur_inc = 0;
        let total_units = cmp::min(state.saleinfo.max_units, applied);
        let ret = state.calc_vesting_amount(
            Timestamp::from_timestamp_millis(61),
            Timestamp::from_timestamp_millis(50),
            total_units as u64,
            PUBLIC_RIDO_FEE_OVL,
            cur_inc,
        );

        claim_eq!(
            ret,
            Err(ContractError::from(CustomContractError::OverflowError)),
            "Should overflow!"
        );
    }

    #[test]
    fn test_vesting_check_u64() {
        let first_per = 25;
        let max = 100;
        let applied = 100;
        let token_per_unit = 1_844_674_407_370_955_161;

        let mut state_builder = TestStateBuilder::new();
        let params = init_parameter(BTreeMap::new());
        let schedule = SaleSchedule::new(
            Timestamp::from_timestamp_millis(1),
            BTreeMap::from([
                (Timestamp::from_timestamp_millis(10), Prior::TOP),
                (Timestamp::from_timestamp_millis(20), Prior::SECOND),
            ]),
            Timestamp::from_timestamp_millis(30),
            BTreeMap::from([
                (Duration::from_millis(10), first_per),
                (Duration::from_millis(20), 40),
                (Duration::from_millis(30), 35),
            ]),
        )
        .unwrap();
        let saleinfo = SaleInfo::new(15_000_000, token_per_unit.into(), max, 10).unwrap();

        let mut state = State::new(
            &mut state_builder,
            params.proj_admin,
            params.addr_ovl,
            params.addr_bbb,
            schedule,
            saleinfo,
        );
        let cur_inc = 0;
        let total_units = cmp::min(state.saleinfo.max_units, applied);
        let ret = state
            .calc_vesting_amount(
                Timestamp::from_timestamp_millis(61),
                Timestamp::from_timestamp_millis(50),
                total_units as u64,
                PUBLIC_RIDO_FEE_OVL,
                cur_inc,
            )
            .unwrap();

        // overflow
        // let ans = total_units as u64 * token_per_unit * PUBLIC_RIDO_FEE_OVL as u64 / 100
        //     * first_per as u64
        //     / 100;

        claim_eq!(
            ret.0 .0,
            2305843009213693951,
            "Something wrong with vesting calcuration!"
        );
        claim_eq!(ret.1, 1, "Something wrong with claimed_inc!");
    }
}
