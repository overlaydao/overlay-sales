use concordium_std::concordium_cfg_test;

#[concordium_cfg_test]
mod tests {
    #![allow(unused)]

    use crate::*;
    use concordium_std::test_infrastructure::*;

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
            max_units: 2,
            min_units: 1,
            applied_units,
        }
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

    #[concordium_test]
    fn test_user_deposit() {
        let acc1 = AccountAddress([11u8; 32]);
        let acc2 = AccountAddress([12u8; 32]);

        let mut state_builder = TestStateBuilder::new();

        let mut state = State {
            usdc_contract: USDC,
            proj_admin: PJ_ADMIN_ACC,
            status: SaleStatus::Ready,
            paused: false,
            addr_ovl: ADDR_OVL,
            addr_bbb: ADDR_BBB,
            ovl_claimed_inc: 0,
            bbb_claimed_inc: 0,
            project_token: None,
            schedule: def_sale_schedule(),
            saleinfo: def_sale_info(0),
            participants: state_builder.new_map(),
        };

        // whitelisting
        let participants = vec![(acc1, Prior::TOP), (acc2, Prior::SECOND)];
        for v in &participants {
            state.whitelisting(&Address::from(v.0), v.1.clone());
        }

        // Track changes in state
        let mut expected_participants = state_builder.new_map();
        for v in &participants {
            expected_participants.insert(
                Address::from(v.0),
                UserState {
                    prior: v.1.clone(),
                    deposit_usdc: ContractTokenAmount::from(5_000_000 * 200 * TARGET_UNITS as u64),
                    tgt_units: TARGET_UNITS,
                    win_units: 1,
                    claimed_inc: 0,
                },
            );
        }

        let expected_state = State {
            usdc_contract: USDC,
            proj_admin: PJ_ADMIN_ACC,
            status: SaleStatus::Ready,
            paused: false,
            addr_ovl: ADDR_OVL,
            addr_bbb: ADDR_BBB,
            ovl_claimed_inc: 0,
            bbb_claimed_inc: 0,
            project_token: None,
            schedule: def_sale_schedule(),
            saleinfo: def_sale_info(2),
            participants: expected_participants,
        };

        let mut host = TestHost::new(state, state_builder);

        // Deposit from user1
        let params = OnReceivingCis2Params {
            token_id: TokenIdUnit(),
            amount: ContractTokenAmount::from(5_000_000 * 200 * TARGET_UNITS as u64),
            from: Address::from(acc1),
            data: AdditionalData::empty(),
        };
        let params_bytes = to_bytes(&params);
        let ctx = receive_context(
            OVL_TEAM_ACC,
            acc1,
            Address::from(USDC),
            Timestamp::from_timestamp_millis(15),
            &params_bytes,
        );
        let result: ContractResult<()> = contract_user_deposit(&ctx, &mut host);
        claim!(result.is_ok(), "Results in rejection with user1 deposit");

        // Deposit from user2
        let params = OnReceivingCis2Params {
            token_id: TokenIdUnit(),
            amount: ContractTokenAmount::from(5_000_000 * 200 * TARGET_UNITS as u64),
            from: Address::from(acc2),
            data: AdditionalData::empty(),
        };
        let params_bytes = to_bytes(&params);
        let ctx = receive_context(
            OVL_TEAM_ACC,
            acc2,
            Address::from(USDC),
            Timestamp::from_timestamp_millis(25),
            &params_bytes,
        );
        let result: ContractResult<()> = contract_user_deposit(&ctx, &mut host);
        claim!(result.is_ok(), "Results in rejection with user2 deposit");

        claim_eq!(
            *host.state(),
            expected_state,
            "state has been changed unexpectedly..."
        );
    }

    #[concordium_test]
    fn test_user_deposit_failed() {
        let acc1 = AccountAddress([11u8; 32]);
        let acc2 = AccountAddress([12u8; 32]);

        let mut state_builder = TestStateBuilder::new();

        let mut state = State {
            usdc_contract: USDC,
            proj_admin: PJ_ADMIN_ACC,
            status: SaleStatus::Ready,
            paused: false,
            addr_ovl: ADDR_OVL,
            addr_bbb: ADDR_BBB,
            ovl_claimed_inc: 0,
            bbb_claimed_inc: 0,
            project_token: None,
            schedule: def_sale_schedule(),
            saleinfo: def_sale_info(0),
            participants: state_builder.new_map(),
        };

        // whitelisting
        let participants = vec![(acc1, Prior::TOP), (acc2, Prior::SECOND)];
        for v in &participants {
            state.whitelisting(&Address::from(v.0), v.1.clone());
        }

        // Track changes in state
        let mut expected_participants = state_builder.new_map();
        for v in &participants {
            expected_participants.insert(
                Address::from(v.0),
                UserState {
                    prior: v.1.clone(),
                    deposit_usdc: ContractTokenAmount::from(0u64),
                    tgt_units: TARGET_UNITS,
                    win_units: 0,
                    claimed_inc: 0,
                },
            );
        }

        let expected_state = State {
            usdc_contract: USDC,
            proj_admin: PJ_ADMIN_ACC,
            status: SaleStatus::Ready,
            paused: false,
            addr_ovl: ADDR_OVL,
            addr_bbb: ADDR_BBB,
            ovl_claimed_inc: 0,
            bbb_claimed_inc: 0,
            project_token: None,
            schedule: def_sale_schedule(),
            saleinfo: def_sale_info(0),
            participants: expected_participants,
        };

        let mut host = TestHost::new(state, state_builder);

        // Deposit from user1 - too early
        let params = OnReceivingCis2Params {
            token_id: TokenIdUnit(),
            amount: ContractTokenAmount::from(5_000_000 * 200 * TARGET_UNITS as u64),
            from: Address::from(acc1),
            data: AdditionalData::empty(),
        };
        let params_bytes = to_bytes(&params);
        let ctx = receive_context(
            OVL_TEAM_ACC,
            acc1,
            Address::from(USDC),
            Timestamp::from_timestamp_millis(5),
            &params_bytes,
        );
        let result: ContractResult<()> = contract_user_deposit(&ctx, &mut host);
        claim_eq!(
            result.expect_err_report("user deposit should reject"),
            CustomContractError::InvalidSchedule.into(),
            "should reject with InvalidSchedule"
        );

        // Deposit from user1 - called by self account
        let params = OnReceivingCis2Params {
            token_id: TokenIdUnit(),
            amount: ContractTokenAmount::from(5_000_000 * 200 * TARGET_UNITS as u64),
            from: Address::from(acc1),
            data: AdditionalData::empty(),
        };
        let params_bytes = to_bytes(&params);
        let ctx = receive_context(
            OVL_TEAM_ACC,
            acc1,
            Address::from(acc1),
            Timestamp::from_timestamp_millis(15),
            &params_bytes,
        );
        let result: ContractResult<()> = contract_user_deposit(&ctx, &mut host);
        claim_eq!(
            result.expect_err_report("user deposit should reject"),
            CustomContractError::ContractOnly.into(),
            "should reject with ContractOnly"
        );

        // Deposit from another contract
        let params = OnReceivingCis2Params {
            token_id: TokenIdUnit(),
            amount: ContractTokenAmount::from(5_000_000 * 200 * TARGET_UNITS as u64),
            from: Address::from(acc1),
            data: AdditionalData::empty(),
        };
        let params_bytes = to_bytes(&params);
        let ctx = receive_context(
            OVL_TEAM_ACC,
            acc1,
            Address::from(ContractAddress {
                index: 999,
                subindex: 0,
            }),
            Timestamp::from_timestamp_millis(15),
            &params_bytes,
        );
        let result: ContractResult<()> = contract_user_deposit(&ctx, &mut host);
        claim_eq!(
            result.expect_err_report("user deposit should reject"),
            ContractError::Unauthorized,
            "should reject with Unauthorized"
        );

        // Deposit from user2 - not permitted yet
        let params = OnReceivingCis2Params {
            token_id: TokenIdUnit(),
            amount: ContractTokenAmount::from(5_000_000 * 200 * TARGET_UNITS as u64),
            from: Address::from(acc2),
            data: AdditionalData::empty(),
        };
        let params_bytes = to_bytes(&params);
        let ctx = receive_context(
            OVL_TEAM_ACC,
            acc2,
            Address::from(USDC),
            Timestamp::from_timestamp_millis(15),
            &params_bytes,
        );
        let result: ContractResult<()> = contract_user_deposit(&ctx, &mut host);
        claim_eq!(
            result.expect_err_report("user deposit should reject"),
            ContractError::Unauthorized,
            "should reject with Unauthorized"
        );

        claim_eq!(
            *host.state(),
            expected_state,
            "state has been changed unexpectedly..."
        );
    }

    #[concordium_test]
    fn test_user_deposit_when_no_room() {
        let acc1 = AccountAddress([11u8; 32]);
        let acc2 = AccountAddress([12u8; 32]);
        let acc3 = AccountAddress([13u8; 32]);

        let mut state_builder = TestStateBuilder::new();

        let mut state = State {
            usdc_contract: USDC,
            proj_admin: PJ_ADMIN_ACC,
            status: SaleStatus::Ready,
            paused: false,
            addr_ovl: ADDR_OVL,
            addr_bbb: ADDR_BBB,
            ovl_claimed_inc: 0,
            bbb_claimed_inc: 0,
            project_token: None,
            schedule: def_sale_schedule(),
            saleinfo: def_sale_info(0),
            participants: state_builder.new_map(),
        };

        // whitelisting
        let participants = vec![
            (acc1, Prior::TOP),
            (acc2, Prior::SECOND),
            (acc3, Prior::SECOND),
        ];
        for v in &participants {
            state.whitelisting(&Address::from(v.0), v.1.clone());
        }

        // Track changes in state
        let mut expected_participants = state_builder.new_map();

        let room = 2;
        for (k, v) in participants.iter().enumerate() {
            let deposit_usdc = if k < room {
                ContractTokenAmount::from(5_000_000 * 200 * TARGET_UNITS as u64)
            } else {
                ContractTokenAmount::from(0u64)
            };
            let win_units = if k < room { 1 } else { 0 };

            expected_participants.insert(
                Address::from(v.0),
                UserState {
                    prior: v.1.clone(),
                    deposit_usdc,
                    tgt_units: TARGET_UNITS,
                    win_units,
                    claimed_inc: 0,
                },
            );
        }

        let expected_state = State {
            usdc_contract: USDC,
            proj_admin: PJ_ADMIN_ACC,
            status: SaleStatus::Ready,
            paused: false,
            addr_ovl: ADDR_OVL,
            addr_bbb: ADDR_BBB,
            ovl_claimed_inc: 0,
            bbb_claimed_inc: 0,
            project_token: None,
            schedule: def_sale_schedule(),
            saleinfo: def_sale_info(2),
            participants: expected_participants,
        };

        let mut host = TestHost::new(state, state_builder);

        // Deposit from user1
        let params = OnReceivingCis2Params {
            token_id: TokenIdUnit(),
            amount: ContractTokenAmount::from(5_000_000 * 200 * TARGET_UNITS as u64),
            from: Address::from(acc1),
            data: AdditionalData::empty(),
        };
        let params_bytes = to_bytes(&params);
        let ctx = receive_context(
            OVL_TEAM_ACC,
            acc1,
            Address::from(USDC),
            Timestamp::from_timestamp_millis(15),
            &params_bytes,
        );
        let result: ContractResult<()> = contract_user_deposit(&ctx, &mut host);
        claim!(result.is_ok(), "Results in rejection");

        // Deposit from user2
        let params = OnReceivingCis2Params {
            token_id: TokenIdUnit(),
            amount: ContractTokenAmount::from(5_000_000 * 200 * TARGET_UNITS as u64),
            from: Address::from(acc2),
            data: AdditionalData::empty(),
        };
        let params_bytes = to_bytes(&params);
        let ctx = receive_context(
            OVL_TEAM_ACC,
            acc2,
            Address::from(USDC),
            Timestamp::from_timestamp_millis(25),
            &params_bytes,
        );
        let result: ContractResult<()> = contract_user_deposit(&ctx, &mut host);
        claim!(result.is_ok(), "Results in rejection");

        // Deposit from user3 - not allowed
        let params = OnReceivingCis2Params {
            token_id: TokenIdUnit(),
            amount: ContractTokenAmount::from(5_000_000 * 200 * TARGET_UNITS as u64),
            from: Address::from(acc3),
            data: AdditionalData::empty(),
        };
        let params_bytes = to_bytes(&params);
        let ctx = receive_context(
            OVL_TEAM_ACC,
            acc3,
            Address::from(USDC),
            Timestamp::from_timestamp_millis(25),
            &params_bytes,
        );
        let result: ContractResult<()> = contract_user_deposit(&ctx, &mut host);
        claim_eq!(
            result.expect_err_report("user deposit should reject"),
            CustomContractError::AlreadySaleClosed.into(),
            "should reject with AlreadySaleClosed"
        );

        claim_eq!(
            *host.state(),
            expected_state,
            "state has been changed unexpectedly..."
        );
    }
}
