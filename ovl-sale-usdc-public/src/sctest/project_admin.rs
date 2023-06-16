use concordium_std::concordium_cfg_test;

#[concordium_cfg_test]
mod tests {

    use crate::{sctest::*, *};

    #[concordium_test]
    fn test_proj_claim() {
        let acc1 = Address::Account(AccountAddress([11u8; 32]));
        let acc2 = Address::Account(AccountAddress([12u8; 32]));
        let acc3 = Address::Account(AccountAddress([13u8; 32]));

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

        // deposit
        for v in participants.iter() {
            state
                .deposit(
                    &Address::from(v.0),
                    ContractTokenAmount::from(5_000_000 * 200 * TARGET_UNITS as u64),
                    1,
                )
                .unwrap_abort();
        }

        // Track changes in state
        let mut expected_participants = state_builder.new_map();
        for v in &participants {
            expected_participants.insert(
                v.0,
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
            status: SaleStatus::Fixed,
            paused: false,
            addr_ovl: ADDR_OVL,
            addr_bbb: ADDR_BBB,
            ovl_claimed_inc: 0,
            bbb_claimed_inc: 0,
            project_token: None,
            schedule: def_sale_schedule(),
            saleinfo: def_sale_info(3),
            participants: expected_participants,
        };

        let mut host = TestHost::new(state, state_builder);

        host.setup_mock_entrypoint(
            USDC,
            OwnedEntrypointName::new_unchecked("transfer".into()),
            MockFn::new_v1(move |parameter, _amount, _balance, _state| {
                let transfer = Transfer {
                    from: Address::from(SELF_ADDRESS),
                    to: Receiver::Account(PJ_ADMIN_ACC),
                    token_id: TokenIdUnit(),
                    amount: ContractTokenAmount::from(5_000_000 * 200 * 3),
                    data: AdditionalData::empty(),
                };
                let transfer_params = TransferParams::from(vec![transfer]);
                let expected_bytes = to_bytes(&transfer_params);
                let param_bytes = parameter.as_ref();
                claim_eq!(param_bytes, expected_bytes);
                Ok((false, ()))
            }),
        );

        // Status to be set Fixed
        let ctx = receive_context(
            OVL_TEAM_ACC,
            OVL_TEAM_ACC,
            Address::from(OVL_TEAM_ACC),
            Timestamp::from_timestamp_millis(35),
            &[],
        );
        let ret = contract_set_fixed(&ctx, &mut host);
        claim!(ret.is_ok(), "Results in rejection");

        // Claim
        let ctx = receive_context(
            OVL_TEAM_ACC,
            PJ_ADMIN_ACC,
            Address::from(PJ_ADMIN_ACC),
            Timestamp::from_timestamp_millis(40),
            &[],
        );
        let ret = contract_project_claim(&ctx, &mut host);
        claim!(ret.is_ok(), "Results in rejection");

        claim_eq!(
            *host.state(),
            expected_state,
            "state has been changed unexpectedly..."
        );
    }

    #[concordium_test]
    fn test_proj_claim_invoke_error() {
        let acc1 = Address::Account(AccountAddress([11u8; 32]));
        let acc2 = Address::Account(AccountAddress([12u8; 32]));
        let acc3 = Address::Account(AccountAddress([13u8; 32]));

        let mut state_builder = TestStateBuilder::new();

        let mut state = State {
            usdc_contract: USDC,
            proj_admin: PJ_ADMIN_ACC,
            status: SaleStatus::Fixed,
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

        // deposit
        for v in participants.iter() {
            state
                .deposit(
                    &Address::from(v.0),
                    ContractTokenAmount::from(5_000_000 * 200 * TARGET_UNITS as u64),
                    1,
                )
                .unwrap_abort();
        }

        let mut host = TestHost::new(state, state_builder);
        host.setup_mock_entrypoint(
            ContractAddress::new(1, 0),
            OwnedEntrypointName::new_unchecked("transfer".into()),
            MockFn::returning_err::<()>(CallContractError::AmountTooLarge),
        );

        // Claim
        let ctx = receive_context(
            OVL_TEAM_ACC,
            PJ_ADMIN_ACC,
            Address::from(PJ_ADMIN_ACC),
            Timestamp::from_timestamp_millis(40),
            &[],
        );
        let ret = contract_project_claim(&ctx, &mut host);
        claim!(ret.is_err());
        claim_eq!(
            ret.expect_err_report("claim should reject"),
            CustomContractError::AmountTooLarge.into(),
            "claim should reject with AmountTooLarge"
        );
    }
}
