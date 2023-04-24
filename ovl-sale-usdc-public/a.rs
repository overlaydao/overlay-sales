#![feature(prelude_import)]
#![allow(unused)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
mod state {
    use collections::BTreeMap;
    use concordium_std::{SchemaType, Serialize, *};
    pub use sale_utils::{
        error::{ContractError, ContractResult, CustomContractError},
        types::*,
    };
    /// All participants can purchase only 1 unit.
    pub const TARGET_UNITS: u8 = 1;
    /// The contract state
    #[concordium(state_parameter = "S")]
    pub struct State<S: HasStateApi> {
        /// cis2 contract for usdc token
        pub(crate) usdc_contract: ContractAddress,
        /// Account of the administrator of the entity running the IDO
        pub(crate) proj_admin: AccountAddress,
        /// Enum for sale status
        pub(crate) status: SaleStatus,
        /// If `true`, some functions will stop working
        pub(crate) paused: bool,
        /// Address of Overlay for receiving sale fee
        pub(crate) addr_ovl: Address,
        /// Address of Overlay for buy back burn
        pub(crate) addr_bbb: Address,
        /// Number of how many fee received
        pub(crate) ovl_claimed_inc: u8,
        /// Number of how many fee for BBB received
        pub(crate) bbb_claimed_inc: u8,
        /// Project token contract address for RIDO
        pub(crate) project_token: Option<ContractAddress>,
        /// Sale schedule
        pub(crate) schedule: SaleSchedule,
        /// Information about sale
        pub(crate) saleinfo: SaleInfo,
        /// Sale participants
        pub(crate) participants: StateMap<Address, UserState, S>,
    }
    #[automatically_derived]
    impl<S: ::core::fmt::Debug + HasStateApi> ::core::fmt::Debug for State<S> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let names: &'static _ = &[
                "usdc_contract",
                "proj_admin",
                "status",
                "paused",
                "addr_ovl",
                "addr_bbb",
                "ovl_claimed_inc",
                "bbb_claimed_inc",
                "project_token",
                "schedule",
                "saleinfo",
                "participants",
            ];
            let values: &[&dyn ::core::fmt::Debug] = &[
                &self.usdc_contract,
                &self.proj_admin,
                &self.status,
                &self.paused,
                &self.addr_ovl,
                &self.addr_bbb,
                &self.ovl_claimed_inc,
                &self.bbb_claimed_inc,
                &self.project_token,
                &self.schedule,
                &self.saleinfo,
                &&self.participants,
            ];
            ::core::fmt::Formatter::debug_struct_fields_finish(f, "State", names, values)
        }
    }
    #[automatically_derived]
    impl<S: HasStateApi> concordium_std::Serial for State<S> {
        fn serial<W: concordium_std::Write>(&self, out: &mut W) -> Result<(), W::Err> {
            concordium_std::Serial::serial(&self.usdc_contract, out)?;
            concordium_std::Serial::serial(&self.proj_admin, out)?;
            concordium_std::Serial::serial(&self.status, out)?;
            concordium_std::Serial::serial(&self.paused, out)?;
            concordium_std::Serial::serial(&self.addr_ovl, out)?;
            concordium_std::Serial::serial(&self.addr_bbb, out)?;
            concordium_std::Serial::serial(&self.ovl_claimed_inc, out)?;
            concordium_std::Serial::serial(&self.bbb_claimed_inc, out)?;
            concordium_std::Serial::serial(&self.project_token, out)?;
            concordium_std::Serial::serial(&self.schedule, out)?;
            concordium_std::Serial::serial(&self.saleinfo, out)?;
            concordium_std::Serial::serial(&self.participants, out)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl<S: HasStateApi> DeserialWithState<S> for State<S>
    where
        S: HasStateApi,
    {
        fn deserial_with_state<__R: Read>(
            _______________________________state: &S,
            ________________source: &mut __R,
        ) -> ParseResult<Self> {
            let usdc_contract =
                <ContractAddress as concordium_std::DeserialWithState<S>>::deserial_with_state(
                    _______________________________state,
                    ________________source,
                )?;
            let proj_admin =
                <AccountAddress as concordium_std::DeserialWithState<S>>::deserial_with_state(
                    _______________________________state,
                    ________________source,
                )?;
            let status = <SaleStatus as concordium_std::DeserialWithState<S>>::deserial_with_state(
                _______________________________state,
                ________________source,
            )?;
            let paused = <bool as concordium_std::DeserialWithState<S>>::deserial_with_state(
                _______________________________state,
                ________________source,
            )?;
            let addr_ovl = <Address as concordium_std::DeserialWithState<S>>::deserial_with_state(
                _______________________________state,
                ________________source,
            )?;
            let addr_bbb = <Address as concordium_std::DeserialWithState<S>>::deserial_with_state(
                _______________________________state,
                ________________source,
            )?;
            let ovl_claimed_inc =
                <u8 as concordium_std::DeserialWithState<S>>::deserial_with_state(
                    _______________________________state,
                    ________________source,
                )?;
            let bbb_claimed_inc =
                <u8 as concordium_std::DeserialWithState<S>>::deserial_with_state(
                    _______________________________state,
                    ________________source,
                )?;
            let project_token = <Option<ContractAddress> as concordium_std::DeserialWithState<
                S,
            >>::deserial_with_state(
                _______________________________state, ________________source
            )?;
            let schedule =
                <SaleSchedule as concordium_std::DeserialWithState<S>>::deserial_with_state(
                    _______________________________state,
                    ________________source,
                )?;
            let saleinfo = <SaleInfo as concordium_std::DeserialWithState<S>>::deserial_with_state(
                _______________________________state,
                ________________source,
            )?;
            let participants = < StateMap < Address , UserState , S > as concordium_std :: DeserialWithState < S > > :: deserial_with_state (_______________________________state , ________________source) ? ;
            Ok(State {
                usdc_contract,
                proj_admin,
                status,
                paused,
                addr_ovl,
                addr_bbb,
                ovl_claimed_inc,
                bbb_claimed_inc,
                project_token,
                schedule,
                saleinfo,
                participants,
            })
        }
    }
    #[automatically_derived]
    unsafe impl<S: HasStateApi> concordium_std::StateClone<S> for State<S>
    where
        S: concordium_std::HasStateApi,
    {
        unsafe fn clone_state(&self, cloned_state_api: &S) -> Self {
            let usdc_contract =
                concordium_std::StateClone::clone_state(&self.usdc_contract, cloned_state_api);
            let proj_admin =
                concordium_std::StateClone::clone_state(&self.proj_admin, cloned_state_api);
            let status = concordium_std::StateClone::clone_state(&self.status, cloned_state_api);
            let paused = concordium_std::StateClone::clone_state(&self.paused, cloned_state_api);
            let addr_ovl =
                concordium_std::StateClone::clone_state(&self.addr_ovl, cloned_state_api);
            let addr_bbb =
                concordium_std::StateClone::clone_state(&self.addr_bbb, cloned_state_api);
            let ovl_claimed_inc =
                concordium_std::StateClone::clone_state(&self.ovl_claimed_inc, cloned_state_api);
            let bbb_claimed_inc =
                concordium_std::StateClone::clone_state(&self.bbb_claimed_inc, cloned_state_api);
            let project_token =
                concordium_std::StateClone::clone_state(&self.project_token, cloned_state_api);
            let schedule =
                concordium_std::StateClone::clone_state(&self.schedule, cloned_state_api);
            let saleinfo =
                concordium_std::StateClone::clone_state(&self.saleinfo, cloned_state_api);
            let participants =
                concordium_std::StateClone::clone_state(&self.participants, cloned_state_api);
            Self {
                usdc_contract,
                proj_admin,
                status,
                paused,
                addr_ovl,
                addr_bbb,
                ovl_claimed_inc,
                bbb_claimed_inc,
                project_token,
                schedule,
                saleinfo,
                participants,
            }
        }
    }
    impl<S: HasStateApi> State<S> {
        pub(crate) fn new(
            state_builder: &mut StateBuilder<S>,
            usdc_contract: ContractAddress,
            proj_admin: AccountAddress,
            addr_ovl: Address,
            addr_bbb: Address,
            schedule: SaleSchedule,
            saleinfo: SaleInfo,
        ) -> Self {
            State {
                usdc_contract,
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
                    None => {
                        return Err(CustomContractError::InvalidSchedule.into());
                    }
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
                {
                    return Err(ContractError::from(CustomContractError::OverflowError));
                }
            } else {
                Ok((ContractTokenAmount::from(amount as u64), inc))
            }
        }
        pub(crate) fn whitelisting(&mut self, user: &Address, prior: Prior) {
            self.participants.entry(*user).or_insert_with(|| {
                UserState::new(prior, ContractTokenAmount::from(0), TARGET_UNITS)
            });
        }
        pub(crate) fn get_user_any(&mut self, user: &Address) -> ContractResult<UserState> {
            let user = self.participants.entry(*user).or_insert_with(|| {
                UserState::new(Prior::ANY, ContractTokenAmount::from(0), TARGET_UNITS)
            });
            let user = user.get_ref();
            Ok(user.clone())
        }
        #[allow(dead_code)]
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
            amount: ContractTokenAmount,
            win_units: u8,
        ) -> ContractResult<()> {
            let mut user = self
                .participants
                .get_mut(user)
                .ok_or(ContractError::Unauthorized)?;
            user.deposit_usdc = amount;
            user.win_units = win_units;
            self.saleinfo.applied_units += win_units as UnitsAmount;
            Ok(())
        }
        pub(crate) fn increment_user_claimed(
            &mut self,
            user: &Address,
            n: u8,
        ) -> ContractResult<()> {
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
    /// Sale Schedule
    pub struct SaleSchedule {
        /// IDO schedule(The process is split into some phases)
        pub(crate) open_at: BTreeMap<Timestamp, Prior>,
        /// Sale End Time
        pub(crate) close_at: Timestamp,
        /// Actual vesting_period is calculated based on this start time
        pub(crate) vesting_start: Option<Timestamp>,
        /// User(sale particicants) can withdraw assets according to the vesting period
        pub(crate) vesting_period: BTreeMap<Duration, AllowedPercentage>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for SaleSchedule {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field4_finish(
                f,
                "SaleSchedule",
                "open_at",
                &self.open_at,
                "close_at",
                &self.close_at,
                "vesting_start",
                &self.vesting_start,
                "vesting_period",
                &&self.vesting_period,
            )
        }
    }
    #[automatically_derived]
    impl concordium_std::Deserial for SaleSchedule {
        fn deserial<__R: concordium_std::Read>(
            ________________source: &mut __R,
        ) -> concordium_std::ParseResult<Self> {
            let open_at = <BTreeMap<Timestamp, Prior> as concordium_std::Deserial>::deserial(
                ________________source,
            )?;
            let close_at =
                <Timestamp as concordium_std::Deserial>::deserial(________________source)?;
            let vesting_start =
                <Option<Timestamp> as concordium_std::Deserial>::deserial(________________source)?;
            let vesting_period =
                <BTreeMap<Duration, AllowedPercentage> as concordium_std::Deserial>::deserial(
                    ________________source,
                )?;
            Ok(SaleSchedule {
                open_at,
                close_at,
                vesting_start,
                vesting_period,
            })
        }
    }
    #[automatically_derived]
    impl concordium_std::Serial for SaleSchedule {
        fn serial<W: concordium_std::Write>(&self, out: &mut W) -> Result<(), W::Err> {
            concordium_std::Serial::serial(&self.open_at, out)?;
            concordium_std::Serial::serial(&self.close_at, out)?;
            concordium_std::Serial::serial(&self.vesting_start, out)?;
            concordium_std::Serial::serial(&self.vesting_period, out)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for SaleSchedule {
        #[inline]
        fn clone(&self) -> SaleSchedule {
            SaleSchedule {
                open_at: ::core::clone::Clone::clone(&self.open_at),
                close_at: ::core::clone::Clone::clone(&self.close_at),
                vesting_start: ::core::clone::Clone::clone(&self.vesting_start),
                vesting_period: ::core::clone::Clone::clone(&self.vesting_period),
            }
        }
    }
    impl SaleSchedule {
        pub fn new(
            now: Timestamp,
            open_at: BTreeMap<Timestamp, Prior>,
            close_at: Timestamp,
            vesting_period: BTreeMap<Duration, AllowedPercentage>,
        ) -> Result<Self, CustomContractError> {
            {
                if !!open_at.is_empty() {
                    {
                        return Err(CustomContractError::InvalidSchedule);
                    };
                }
            };
            {
                if !(now < *open_at.first_key_value().unwrap().0) {
                    {
                        return Err(CustomContractError::InvalidSchedule);
                    };
                }
            };
            {
                if !(*open_at.last_key_value().unwrap().0 < close_at) {
                    {
                        return Err(CustomContractError::InvalidSchedule);
                    };
                }
            };
            let mut total_per = 0;
            for (_, per) in vesting_period.iter() {
                total_per += *per;
            }
            {
                if !(total_per == 100) {
                    {
                        return Err(CustomContractError::Inappropriate);
                    };
                }
            };
            Ok(SaleSchedule {
                open_at,
                close_at,
                vesting_start: None,
                vesting_period,
            })
        }
        #[allow(dead_code)]
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
    /// Information about sale
    pub struct SaleInfo {
        /// Price in usdc per a project token
        pub(crate) price_per_token: MicroUsdc,
        /// Amount of tokens contained in a unit
        pub(crate) token_per_unit: ContractTokenAmount,
        /// Maximum quantity to be issued in this sale
        pub(crate) max_units: UnitsAmount,
        /// Minimum quantity to be issued for this sale
        pub(crate) min_units: UnitsAmount,
        /// Amount of sales completed at that point
        pub(crate) applied_units: UnitsAmount,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for SaleInfo {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field5_finish(
                f,
                "SaleInfo",
                "price_per_token",
                &self.price_per_token,
                "token_per_unit",
                &self.token_per_unit,
                "max_units",
                &self.max_units,
                "min_units",
                &self.min_units,
                "applied_units",
                &&self.applied_units,
            )
        }
    }
    #[automatically_derived]
    impl concordium_std::Deserial for SaleInfo {
        fn deserial<__R: concordium_std::Read>(
            ________________source: &mut __R,
        ) -> concordium_std::ParseResult<Self> {
            let price_per_token =
                <MicroUsdc as concordium_std::Deserial>::deserial(________________source)?;
            let token_per_unit = <ContractTokenAmount as concordium_std::Deserial>::deserial(
                ________________source,
            )?;
            let max_units =
                <UnitsAmount as concordium_std::Deserial>::deserial(________________source)?;
            let min_units =
                <UnitsAmount as concordium_std::Deserial>::deserial(________________source)?;
            let applied_units =
                <UnitsAmount as concordium_std::Deserial>::deserial(________________source)?;
            Ok(SaleInfo {
                price_per_token,
                token_per_unit,
                max_units,
                min_units,
                applied_units,
            })
        }
    }
    #[automatically_derived]
    impl concordium_std::Serial for SaleInfo {
        fn serial<W: concordium_std::Write>(&self, out: &mut W) -> Result<(), W::Err> {
            concordium_std::Serial::serial(&self.price_per_token, out)?;
            concordium_std::Serial::serial(&self.token_per_unit, out)?;
            concordium_std::Serial::serial(&self.max_units, out)?;
            concordium_std::Serial::serial(&self.min_units, out)?;
            concordium_std::Serial::serial(&self.applied_units, out)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for SaleInfo {
        #[inline]
        fn clone(&self) -> SaleInfo {
            SaleInfo {
                price_per_token: ::core::clone::Clone::clone(&self.price_per_token),
                token_per_unit: ::core::clone::Clone::clone(&self.token_per_unit),
                max_units: ::core::clone::Clone::clone(&self.max_units),
                min_units: ::core::clone::Clone::clone(&self.min_units),
                applied_units: ::core::clone::Clone::clone(&self.applied_units),
            }
        }
    }
    impl SaleInfo {
        pub(crate) fn new(
            price_per_token: MicroUsdc,
            token_per_unit: ContractTokenAmount,
            max_units: UnitsAmount,
            min_units: UnitsAmount,
        ) -> Result<Self, CustomContractError> {
            {
                if !(min_units < max_units) {
                    {
                        return Err(CustomContractError::Inappropriate);
                    };
                }
            };
            if price_per_token.checked_mul(token_per_unit.0).is_none() {
                {
                    return Err(CustomContractError::OverflowError);
                };
            }
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
        pub(crate) fn amount_of_pjtoken(&self) -> Result<ContractTokenAmount, CustomContractError> {
            let token_amount = self.token_per_unit.0.checked_mul(self.applied_units as u64);
            if token_amount.is_none() {
                {
                    return Err(CustomContractError::OverflowError);
                };
            }
            Ok(ContractTokenAmount::from(token_amount.unwrap()))
        }
        pub(crate) fn calc_price_per_unit(
            &self,
        ) -> Result<ContractTokenAmount, CustomContractError> {
            let price = self.price_per_token.checked_mul(self.token_per_unit.0);
            if price.is_none() {
                {
                    return Err(CustomContractError::OverflowError);
                };
            }
            let price = price.unwrap();
            Ok(ContractTokenAmount::from(price))
        }
    }
    /// About sale participants
    pub struct UserState {
        /// Priority to participate in the sale
        pub(crate) prior: Prior,
        /// If deposited, their right to receive tokens will be confirmed.
        pub(crate) deposit_usdc: ContractTokenAmount,
        /// Number of unit desired(or available) to be purchased
        pub(crate) tgt_units: u8,
        /// Number actually determined to be purchased
        pub(crate) win_units: u8,
        /// Number of tokens received during the vesting period(neither Amount or number of claim)
        pub(crate) claimed_inc: u8,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for UserState {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field5_finish(
                f,
                "UserState",
                "prior",
                &self.prior,
                "deposit_usdc",
                &self.deposit_usdc,
                "tgt_units",
                &self.tgt_units,
                "win_units",
                &self.win_units,
                "claimed_inc",
                &&self.claimed_inc,
            )
        }
    }
    #[automatically_derived]
    impl concordium_std::Deserial for UserState {
        fn deserial<__R: concordium_std::Read>(
            ________________source: &mut __R,
        ) -> concordium_std::ParseResult<Self> {
            let prior = <Prior as concordium_std::Deserial>::deserial(________________source)?;
            let deposit_usdc = <ContractTokenAmount as concordium_std::Deserial>::deserial(
                ________________source,
            )?;
            let tgt_units = <u8 as concordium_std::Deserial>::deserial(________________source)?;
            let win_units = <u8 as concordium_std::Deserial>::deserial(________________source)?;
            let claimed_inc = <u8 as concordium_std::Deserial>::deserial(________________source)?;
            Ok(UserState {
                prior,
                deposit_usdc,
                tgt_units,
                win_units,
                claimed_inc,
            })
        }
    }
    #[automatically_derived]
    impl concordium_std::Serial for UserState {
        fn serial<W: concordium_std::Write>(&self, out: &mut W) -> Result<(), W::Err> {
            concordium_std::Serial::serial(&self.prior, out)?;
            concordium_std::Serial::serial(&self.deposit_usdc, out)?;
            concordium_std::Serial::serial(&self.tgt_units, out)?;
            concordium_std::Serial::serial(&self.win_units, out)?;
            concordium_std::Serial::serial(&self.claimed_inc, out)?;
            Ok(())
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for UserState {
        #[inline]
        fn clone(&self) -> UserState {
            UserState {
                prior: ::core::clone::Clone::clone(&self.prior),
                deposit_usdc: ::core::clone::Clone::clone(&self.deposit_usdc),
                tgt_units: ::core::clone::Clone::clone(&self.tgt_units),
                win_units: ::core::clone::Clone::clone(&self.win_units),
                claimed_inc: ::core::clone::Clone::clone(&self.claimed_inc),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for UserState {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for UserState {
        #[inline]
        fn eq(&self, other: &UserState) -> bool {
            self.prior == other.prior
                && self.deposit_usdc == other.deposit_usdc
                && self.tgt_units == other.tgt_units
                && self.win_units == other.win_units
                && self.claimed_inc == other.claimed_inc
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralEq for UserState {}
    #[automatically_derived]
    impl ::core::cmp::Eq for UserState {
        #[inline]
        #[doc(hidden)]
        #[no_coverage]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<Prior>;
            let _: ::core::cmp::AssertParamIsEq<ContractTokenAmount>;
            let _: ::core::cmp::AssertParamIsEq<u8>;
        }
    }
    impl UserState {
        pub fn new(prior: Prior, deposit_usdc: ContractTokenAmount, tgt_units: u8) -> Self {
            UserState {
                prior,
                deposit_usdc,
                tgt_units,
                win_units: 0,
                claimed_inc: 0,
            }
        }
    }
}
use concordium_cis2::{
    AdditionalData, OnReceivingCis2Params, Receiver, TokenIdUnit, Transfer, TransferParams,
};
use concordium_std::{collections::BTreeMap, *};
use sale_utils::{PUBLIC_RIDO_FEE, PUBLIC_RIDO_FEE_BBB, PUBLIC_RIDO_FEE_OVL};
use state::{State, *};
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
#[automatically_derived]
impl ::core::fmt::Debug for InitParams {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let names: &'static _ = &[
            "usdc_contract",
            "proj_admin",
            "addr_ovl",
            "addr_bbb",
            "open_at",
            "close_at",
            "vesting_period",
            "price_per_token",
            "token_per_unit",
            "max_units",
            "min_units",
        ];
        let values: &[&dyn ::core::fmt::Debug] = &[
            &self.usdc_contract,
            &self.proj_admin,
            &self.addr_ovl,
            &self.addr_bbb,
            &self.open_at,
            &self.close_at,
            &self.vesting_period,
            &self.price_per_token,
            &self.token_per_unit,
            &self.max_units,
            &&self.min_units,
        ];
        ::core::fmt::Formatter::debug_struct_fields_finish(f, "InitParams", names, values)
    }
}
#[automatically_derived]
impl concordium_std::Deserial for InitParams {
    fn deserial<__R: concordium_std::Read>(
        ________________source: &mut __R,
    ) -> concordium_std::ParseResult<Self> {
        let usdc_contract =
            <ContractAddress as concordium_std::Deserial>::deserial(________________source)?;
        let proj_admin =
            <AccountAddress as concordium_std::Deserial>::deserial(________________source)?;
        let addr_ovl = <Address as concordium_std::Deserial>::deserial(________________source)?;
        let addr_bbb = <Address as concordium_std::Deserial>::deserial(________________source)?;
        let open_at = <BTreeMap<Timestamp, Prior> as concordium_std::Deserial>::deserial(
            ________________source,
        )?;
        let close_at = <Timestamp as concordium_std::Deserial>::deserial(________________source)?;
        let vesting_period =
            <BTreeMap<Duration, AllowedPercentage> as concordium_std::Deserial>::deserial(
                ________________source,
            )?;
        let price_per_token =
            <MicroUsdc as concordium_std::Deserial>::deserial(________________source)?;
        let token_per_unit =
            <ContractTokenAmount as concordium_std::Deserial>::deserial(________________source)?;
        let max_units =
            <UnitsAmount as concordium_std::Deserial>::deserial(________________source)?;
        let min_units =
            <UnitsAmount as concordium_std::Deserial>::deserial(________________source)?;
        Ok(InitParams {
            usdc_contract,
            proj_admin,
            addr_ovl,
            addr_bbb,
            open_at,
            close_at,
            vesting_period,
            price_per_token,
            token_per_unit,
            max_units,
            min_units,
        })
    }
}
#[automatically_derived]
impl concordium_std::Serial for InitParams {
    fn serial<W: concordium_std::Write>(&self, out: &mut W) -> Result<(), W::Err> {
        concordium_std::Serial::serial(&self.usdc_contract, out)?;
        concordium_std::Serial::serial(&self.proj_admin, out)?;
        concordium_std::Serial::serial(&self.addr_ovl, out)?;
        concordium_std::Serial::serial(&self.addr_bbb, out)?;
        concordium_std::Serial::serial(&self.open_at, out)?;
        concordium_std::Serial::serial(&self.close_at, out)?;
        concordium_std::Serial::serial(&self.vesting_period, out)?;
        concordium_std::Serial::serial(&self.price_per_token, out)?;
        concordium_std::Serial::serial(&self.token_per_unit, out)?;
        concordium_std::Serial::serial(&self.max_units, out)?;
        concordium_std::Serial::serial(&self.min_units, out)?;
        Ok(())
    }
}
#[export_name = "init_pub_rido_usdc"]
pub extern "C" fn export_contract_init(amount: concordium_std::Amount) -> i32 {
    use concordium_std::{trap, ExternContext, ExternInitContext, StateBuilder, ExternReturnValue};
    if amount.micro_ccd != 0 {
        return concordium_std::Reject::from(concordium_std::NotPayableError)
            .error_code
            .get();
    }
    let ctx = ExternContext::<ExternInitContext>::open(());
    let mut state_api = ExternStateApi::open();
    let mut state_builder = StateBuilder::open(state_api.clone());
    match contract_init(&ctx, &mut state_builder) {
        Ok(state) => {
            let mut root_entry = state_api.create_entry(&[]).unwrap_abort();
            state.serial(&mut root_entry).unwrap_abort();
            0
        }
        Err(reject) => {
            let code = Reject::from(reject).error_code.get();
            if code < 0 {
                code
            } else {
                trap()
            }
        }
    }
}
/// # Init Function
/// everyone can init this module, but need to be initialized by ovl_team
/// since contract_id is needed to record into project contract.
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
#[export_name = "pub_rido_usdc.setStatus"]
pub extern "C" fn export_contract_set_status(amount: concordium_std::Amount) -> i32 {
    use concordium_std::{SeekFrom, StateBuilder, Logger, ExternHost, trap};
    if amount.micro_ccd != 0 {
        return concordium_std::Reject::from(concordium_std::NotPayableError)
            .error_code
            .get();
    }
    let ctx = ExternContext::<ExternReceiveContext>::open(());
    let state_api = ExternStateApi::open();
    if let Ok(state) = DeserialWithState::deserial_with_state(
        &state_api,
        &mut state_api.lookup_entry(&[]).unwrap_abort(),
    ) {
        let mut state_builder = StateBuilder::open(state_api);
        let mut host = ExternHost {
            state,
            state_builder,
        };
        match contract_set_status(&ctx, &mut host) {
            Ok(rv) => {
                if rv.serial(&mut ExternReturnValue::open()).is_err() {
                    trap()
                }
                let mut root_entry_end = host
                    .state_builder
                    .into_inner()
                    .lookup_entry(&[])
                    .unwrap_abort();
                host.state.serial(&mut root_entry_end).unwrap_abort();
                let new_state_size = root_entry_end.size().unwrap_abort();
                root_entry_end.truncate(new_state_size).unwrap_abort();
                0
            }
            Err(reject) => {
                let reject = Reject::from(reject);
                let code = reject.error_code.get();
                if code < 0 {
                    if let Some(rv) = reject.return_value {
                        if ExternReturnValue::open().write_all(&rv).is_err() {
                            trap()
                        }
                    }
                    code
                } else {
                    trap()
                }
            }
        }
    } else {
        trap()
    }
}
/// To change the status to something arbitrary, but is not normally used.
///
/// Caller: contract owner only
/// Reject if:
/// - The sender is not the contract instance owner.
/// - Fails to parse parameter
fn contract_set_status<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    {
        if !ctx.sender().matches_account(&ctx.owner()) {
            {
                return Err(ContractError::Unauthorized);
            };
        }
    };
    let status: SaleStatus = ctx.parameter_cursor().get()?;
    host.state_mut().status = status;
    Ok(())
}
#[export_name = "pub_rido_usdc.setFixed"]
pub extern "C" fn export_contract_set_fixed(amount: concordium_std::Amount) -> i32 {
    use concordium_std::{SeekFrom, StateBuilder, Logger, ExternHost, trap};
    if amount.micro_ccd != 0 {
        return concordium_std::Reject::from(concordium_std::NotPayableError)
            .error_code
            .get();
    }
    let ctx = ExternContext::<ExternReceiveContext>::open(());
    let state_api = ExternStateApi::open();
    if let Ok(state) = DeserialWithState::deserial_with_state(
        &state_api,
        &mut state_api.lookup_entry(&[]).unwrap_abort(),
    ) {
        let mut state_builder = StateBuilder::open(state_api);
        let mut host = ExternHost {
            state,
            state_builder,
        };
        match contract_set_fixed(&ctx, &mut host) {
            Ok(rv) => {
                if rv.serial(&mut ExternReturnValue::open()).is_err() {
                    trap()
                }
                let mut root_entry_end = host
                    .state_builder
                    .into_inner()
                    .lookup_entry(&[])
                    .unwrap_abort();
                host.state.serial(&mut root_entry_end).unwrap_abort();
                let new_state_size = root_entry_end.size().unwrap_abort();
                root_entry_end.truncate(new_state_size).unwrap_abort();
                0
            }
            Err(reject) => {
                let reject = Reject::from(reject);
                let code = reject.error_code.get();
                if code < 0 {
                    if let Some(rv) = reject.return_value {
                        if ExternReturnValue::open().write_all(&rv).is_err() {
                            trap()
                        }
                    }
                    code
                } else {
                    trap()
                }
            }
        }
    } else {
        trap()
    }
}
/// Set status to fix for next stage(claim).
/// Note: if not reached softcap, the sale will be cancelled.
///
/// Caller: contract instance owner only
/// Reject if:
/// - The sender is not the contract owner.
/// - Called before the end of the sale
fn contract_set_fixed<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    {
        if !ctx.sender().matches_account(&ctx.owner()) {
            {
                return Err(ContractError::Unauthorized);
            };
        }
    };
    let mut state = host.state_mut();
    {
        if !state.schedule.is_sale_closed(ctx.metadata().slot_time()) {
            {
                return Err(CustomContractError::InvalidSchedule.into());
            };
        }
    };
    if state.saleinfo.is_reached_sc() {
        state.status = SaleStatus::Fixed;
    } else {
        state.status = SaleStatus::Suspend;
    }
    Ok(())
}
/// Parameter type for the contract function `whitelisting`.
/// Currently user can be both account and contract.
/// [#TODO] But need to consider when user can be contract.
struct WhitelistingParams {
    /// the whitelist
    wl: Vec<AllowedUserParams>,
    /// If true, it means no further registration
    ready: bool,
}
#[automatically_derived]
impl ::core::fmt::Debug for WhitelistingParams {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "WhitelistingParams",
            "wl",
            &self.wl,
            "ready",
            &&self.ready,
        )
    }
}
#[automatically_derived]
impl concordium_std::Deserial for WhitelistingParams {
    fn deserial<__R: concordium_std::Read>(
        ________________source: &mut __R,
    ) -> concordium_std::ParseResult<Self> {
        let wl =
            <Vec<AllowedUserParams> as concordium_std::Deserial>::deserial(________________source)?;
        let ready = <bool as concordium_std::Deserial>::deserial(________________source)?;
        Ok(WhitelistingParams { wl, ready })
    }
}
#[automatically_derived]
impl concordium_std::Serial for WhitelistingParams {
    fn serial<W: concordium_std::Write>(&self, out: &mut W) -> Result<(), W::Err> {
        concordium_std::Serial::serial(&self.wl, out)?;
        concordium_std::Serial::serial(&self.ready, out)?;
        Ok(())
    }
}
struct AllowedUserParams {
    /// Users address to be whitelisted
    user: Address,
    /// Priority for participation in the sale
    prior: Prior,
}
#[automatically_derived]
impl ::core::fmt::Debug for AllowedUserParams {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field2_finish(
            f,
            "AllowedUserParams",
            "user",
            &self.user,
            "prior",
            &&self.prior,
        )
    }
}
#[automatically_derived]
impl concordium_std::Deserial for AllowedUserParams {
    fn deserial<__R: concordium_std::Read>(
        ________________source: &mut __R,
    ) -> concordium_std::ParseResult<Self> {
        let user = <Address as concordium_std::Deserial>::deserial(________________source)?;
        let prior = <Prior as concordium_std::Deserial>::deserial(________________source)?;
        Ok(AllowedUserParams { user, prior })
    }
}
#[automatically_derived]
impl concordium_std::Serial for AllowedUserParams {
    fn serial<W: concordium_std::Write>(&self, out: &mut W) -> Result<(), W::Err> {
        concordium_std::Serial::serial(&self.user, out)?;
        concordium_std::Serial::serial(&self.prior, out)?;
        Ok(())
    }
}
#[export_name = "pub_rido_usdc.whitelisting"]
pub extern "C" fn export_contract_whitelisting(amount: concordium_std::Amount) -> i32 {
    use concordium_std::{SeekFrom, StateBuilder, Logger, ExternHost, trap};
    if amount.micro_ccd != 0 {
        return concordium_std::Reject::from(concordium_std::NotPayableError)
            .error_code
            .get();
    }
    let ctx = ExternContext::<ExternReceiveContext>::open(());
    let state_api = ExternStateApi::open();
    if let Ok(state) = DeserialWithState::deserial_with_state(
        &state_api,
        &mut state_api.lookup_entry(&[]).unwrap_abort(),
    ) {
        let mut state_builder = StateBuilder::open(state_api);
        let mut host = ExternHost {
            state,
            state_builder,
        };
        match contract_whitelisting(&ctx, &mut host) {
            Ok(rv) => {
                if rv.serial(&mut ExternReturnValue::open()).is_err() {
                    trap()
                }
                let mut root_entry_end = host
                    .state_builder
                    .into_inner()
                    .lookup_entry(&[])
                    .unwrap_abort();
                host.state.serial(&mut root_entry_end).unwrap_abort();
                let new_state_size = root_entry_end.size().unwrap_abort();
                root_entry_end.truncate(new_state_size).unwrap_abort();
                0
            }
            Err(reject) => {
                let reject = Reject::from(reject);
                let code = reject.error_code.get();
                if code < 0 {
                    if let Some(rv) = reject.return_value {
                        if ExternReturnValue::open().write_all(&rv).is_err() {
                            trap()
                        }
                    }
                    code
                } else {
                    trap()
                }
            }
        }
    } else {
        trap()
    }
}
/// Whitelist users who can participate in the sale
/// Note: All user can be allocated just one unit.
///
/// Caller: contract instance owner only
/// Reject if:
/// - Fails to parse parameter
/// - The sender is not the contract owner.
/// - Status is not Prepare
fn contract_whitelisting<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    {
        if !ctx.sender().matches_account(&ctx.owner()) {
            {
                return Err(ContractError::Unauthorized);
            };
        }
    };
    let mut state = host.state_mut();
    {
        if !(state.status == SaleStatus::Prepare) {
            {
                return Err(CustomContractError::AlreadySaleStarted.into());
            };
        }
    };
    let params: WhitelistingParams = ctx.parameter_cursor().get()?;
    for AllowedUserParams { user, prior } in params.wl {
        if let Address::Account(_) = user {
            state.whitelisting(&user, prior);
        } else {
            {
                return Err(CustomContractError::AccountOnly.into());
            }
        };
    }
    if params.ready {
        state.status = SaleStatus::Ready;
    }
    Ok(())
}
#[export_name = "pub_rido_usdc.ovlClaim"]
pub extern "C" fn export_contract_ovl_claim(amount: concordium_std::Amount) -> i32 {
    use concordium_std::{SeekFrom, StateBuilder, Logger, ExternHost, trap};
    if amount.micro_ccd != 0 {
        return concordium_std::Reject::from(concordium_std::NotPayableError)
            .error_code
            .get();
    }
    let ctx = ExternContext::<ExternReceiveContext>::open(());
    let state_api = ExternStateApi::open();
    if let Ok(state) = DeserialWithState::deserial_with_state(
        &state_api,
        &mut state_api.lookup_entry(&[]).unwrap_abort(),
    ) {
        let mut state_builder = StateBuilder::open(state_api);
        let mut host = ExternHost {
            state,
            state_builder,
        };
        match contract_ovl_claim(&ctx, &mut host) {
            Ok(rv) => {
                if rv.serial(&mut ExternReturnValue::open()).is_err() {
                    trap()
                }
                let mut root_entry_end = host
                    .state_builder
                    .into_inner()
                    .lookup_entry(&[])
                    .unwrap_abort();
                host.state.serial(&mut root_entry_end).unwrap_abort();
                let new_state_size = root_entry_end.size().unwrap_abort();
                root_entry_end.truncate(new_state_size).unwrap_abort();
                0
            }
            Err(reject) => {
                let reject = Reject::from(reject);
                let code = reject.error_code.get();
                if code < 0 {
                    if let Some(rv) = reject.return_value {
                        if ExternReturnValue::open().write_all(&rv).is_err() {
                            trap()
                        }
                    }
                    code
                } else {
                    trap()
                }
            }
        }
    } else {
        trap()
    }
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
fn contract_ovl_claim<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    {
        if !ctx.sender().matches_account(&ctx.owner()) {
            {
                return Err(ContractError::Unauthorized);
            };
        }
    };
    let mut state = host.state_mut();
    {
        if !(state.status == SaleStatus::Fixed) {
            {
                return Err(CustomContractError::SaleNotFixed.into());
            };
        }
    };
    {
        if !state.project_token.is_some() {
            {
                return Err(CustomContractError::NotSetProjectToken.into());
            };
        }
    };
    {
        if !state.schedule.vesting_start.is_some() {
            {
                return Err(CustomContractError::NotSetTge.into());
            };
        }
    };
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
            &TransferParams::from(<[_]>::into_vec(
                #[rustc_box]
                ::alloc::boxed::Box::new([transfer]),
            )),
            EntrypointName::new_unchecked("transfer"),
            Amount::zero(),
        )?;
    }
    Ok(())
}
#[export_name = "pub_rido_usdc.bbbClaim"]
pub extern "C" fn export_contract_bbb_claim(amount: concordium_std::Amount) -> i32 {
    use concordium_std::{SeekFrom, StateBuilder, Logger, ExternHost, trap};
    if amount.micro_ccd != 0 {
        return concordium_std::Reject::from(concordium_std::NotPayableError)
            .error_code
            .get();
    }
    let ctx = ExternContext::<ExternReceiveContext>::open(());
    let state_api = ExternStateApi::open();
    if let Ok(state) = DeserialWithState::deserial_with_state(
        &state_api,
        &mut state_api.lookup_entry(&[]).unwrap_abort(),
    ) {
        let mut state_builder = StateBuilder::open(state_api);
        let mut host = ExternHost {
            state,
            state_builder,
        };
        match contract_bbb_claim(&ctx, &mut host) {
            Ok(rv) => {
                if rv.serial(&mut ExternReturnValue::open()).is_err() {
                    trap()
                }
                let mut root_entry_end = host
                    .state_builder
                    .into_inner()
                    .lookup_entry(&[])
                    .unwrap_abort();
                host.state.serial(&mut root_entry_end).unwrap_abort();
                let new_state_size = root_entry_end.size().unwrap_abort();
                root_entry_end.truncate(new_state_size).unwrap_abort();
                0
            }
            Err(reject) => {
                let reject = Reject::from(reject);
                let code = reject.error_code.get();
                if code < 0 {
                    if let Some(rv) = reject.return_value {
                        if ExternReturnValue::open().write_all(&rv).is_err() {
                            trap()
                        }
                    }
                    code
                } else {
                    trap()
                }
            }
        }
    } else {
        trap()
    }
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
fn contract_bbb_claim<S: HasStateApi>(
    ctx: &impl HasReceiveContext,
    host: &mut impl HasHost<State<S>, StateApiType = S>,
) -> ContractResult<()> {
    {
        if !ctx.sender().matches_account(&ctx.owner()) {
            {
                return Err(ContractError::Unauthorized);
            };
        }
    };
    let mut state = host.state_mut();
    {
        if !(state.status == SaleStatus::Fixed) {
            {
                return Err(CustomContractError::SaleNotFixed.into());
            };
        }
    };
    {
        if !state.project_token.is_some() {
            {
                return Err(CustomContractError::NotSetProjectToken.into());
            };
        }
    };
    {
        if !state.schedule.vesting_start.is_some() {
            {
                return Err(CustomContractError::NotSetTge.into());
            };
        }
    };
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
            &TransferParams::from(<[_]>::into_vec(
                #[rustc_box]
                ::alloc::boxed::Box::new([transfer]),
            )),
            EntrypointName::new_unchecked("transfer"),
            Amount::zero(),
        )?;
    }
    Ok(())
}
