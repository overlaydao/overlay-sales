use concordium_std::concordium_cfg_test;

#[concordium_cfg_test]
mod tests {

    use crate::{sctest::*, *};

    #[concordium_test]
    fn test_init() {
        let mut state_builder = TestStateBuilder::new();

        let expected_state = State {
            usdc_contract: USDC,
            proj_admin: PJ_ADMIN_ACC,
            status: SaleStatus::Prepare,
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

        let params = InitParams {
            usdc_contract: USDC,
            proj_admin: PJ_ADMIN_ACC,
            addr_ovl: ADDR_OVL,
            addr_bbb: ADDR_BBB,
            open_at: BTreeMap::from([
                (Timestamp::from_timestamp_millis(10), Prior::TOP),
                (Timestamp::from_timestamp_millis(20), Prior::SECOND),
            ]),
            close_at: Timestamp::from_timestamp_millis(30),
            max_units: 100,
            min_units: 1,
            price_per_token: 5_000_000,
            token_per_unit: 200.into(),
            vesting_period: BTreeMap::from([
                (Duration::from_days(1), 25),
                (Duration::from_days(2), 40),
                (Duration::from_days(3), 35),
            ]),
        };

        let params_byte = to_bytes(&params);
        let ctx = init_context(
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(1),
            &params_byte,
        );

        let result = contract_init(&ctx, &mut state_builder);
        claim!(result.is_ok());
        claim_eq!(
            result.unwrap(),
            expected_state,
            "state has been changed unexpectedly..."
        );
    }
}
