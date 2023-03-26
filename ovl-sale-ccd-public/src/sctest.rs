#![allow(unused)]
use crate::{test_infrastructure::*, *};
use core::fmt::Debug;
use std::sync::atomic::{AtomicU8, Ordering};

static ADDRESS_COUNTER: AtomicU8 = AtomicU8::new(10);
const OVL_TEAM_ACC: AccountAddress = AccountAddress([0u8; 32]);
const OVL_TEAM_ADDR: Address = Address::Account(OVL_TEAM_ACC);
const PJ_ADMIN_ACC: AccountAddress = AccountAddress([1u8; 32]);
const PJ_ADMIN_ADDR: Address = Address::Account(PJ_ADMIN_ACC);
const ADDR_OVL: Address = Address::Account(AccountAddress([2u8; 32]));
const ADDR_BBB: Address = Address::Contract(ContractAddress {
    index: 100,
    subindex: 0,
});

fn new_account() -> AccountAddress {
    let account = AccountAddress([ADDRESS_COUNTER.load(Ordering::SeqCst); 32]);
    ADDRESS_COUNTER.fetch_add(1, Ordering::SeqCst);
    account
}

fn get_account(n: u8) -> AccountAddress {
    let account = AccountAddress([n; 32]);
    account
}

pub(crate) fn init_parameter(vesting_period: BTreeMap<Duration, AllowedPercentage>) -> InitParams {
    InitParams {
        proj_admin: PJ_ADMIN_ACC,
        addr_ovl: ADDR_OVL,
        addr_bbb: ADDR_BBB,
        open_at: BTreeMap::from([
            (Timestamp::from_timestamp_millis(10), Prior::TOP),
            (Timestamp::from_timestamp_millis(20), Prior::SECOND),
        ]),
        close_at: Timestamp::from_timestamp_millis(30),
        max_units: 100,
        min_units: 50,
        price_per_token: 5_000_000,
        token_per_unit: 200.into(),
        vesting_period: if vesting_period.is_empty() {
            BTreeMap::from([
                (Duration::from_days(1), 25),
                (Duration::from_days(2), 40),
                (Duration::from_days(3), 35),
            ])
        } else {
            vesting_period
        },
    }
}

fn init_ctx(
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

fn receive_ctx(
    owner: AccountAddress,
    sender: AccountAddress,
    slot_time: SlotTime,
    parameter_bytes: &[u8],
) -> TestReceiveContext {
    let mut ctx = TestReceiveContext::empty();
    ctx.set_self_address(ContractAddress::new(10, 0));
    ctx.set_sender(Address::Account(sender));
    ctx.set_invoker(sender);
    ctx.set_owner(owner);
    ctx.set_metadata_slot_time(slot_time);
    ctx.set_parameter(parameter_bytes);
    ctx
}

fn initial_state<S: HasStateApi>(
    state_builder: &mut StateBuilder<S>,
    schedule: Option<SaleSchedule>,
    saleinfo: Option<SaleInfo>,
) -> State<S> {
    let params = init_parameter(BTreeMap::new());

    let schedule = if schedule.is_none() {
        SaleSchedule::new(
            Timestamp::from_timestamp_millis(1),
            params.open_at,
            params.close_at,
            params.vesting_period,
        )
        .unwrap_abort()
    } else {
        schedule.unwrap()
    };

    let saleinfo = if saleinfo.is_none() {
        SaleInfo::new(
            params.price_per_token,
            params.token_per_unit,
            params.max_units,
            params.min_units,
        )
        .unwrap_abort()
    } else {
        saleinfo.unwrap()
    };

    let state = State::new(
        state_builder,
        params.proj_admin,
        params.addr_ovl,
        params.addr_bbb,
        schedule,
        saleinfo,
    );
    state
}

fn expect_error<E, T>(expr: Result<T, E>, err: E, msg: &str)
where
    E: Eq + Debug,
    T: Debug,
{
    let actual = expr.expect_err_report(msg);
    claim_eq!(actual, err);
}

#[concordium_cfg_test]
mod test_ovlteam {
    use super::*;

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

    #[concordium_test]
    fn test_init() {
        let parameter_bytes: Vec<u8> = to_bytes(&init_parameter(BTreeMap::new()));
        let ctx = init_ctx(
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(1),
            &parameter_bytes,
        );
        let mut state_builder = TestStateBuilder::new();

        let state_result = contract_init(&ctx, &mut state_builder);
        state_result.expect_report("Contract initialization results in error");
    }

    #[concordium_test]
    fn test_init_cant_start_back_in_time() {
        let parameter_bytes: Vec<u8> = to_bytes(&init_parameter(BTreeMap::new()));
        let ctx = init_ctx(
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(50),
            &parameter_bytes,
        );
        let mut state_builder = TestStateBuilder::new();

        let error = contract_init(&ctx, &mut state_builder);
        expect_error(
            error,
            CustomContractError::InvalidSchedule.into(),
            "this call should fail when timeline is wrong!",
        );
    }

    #[concordium_test]
    fn test_init_invalid_vesting_total() {
        let vesting = BTreeMap::from([
            (Duration::from_days(30), 30),
            (Duration::from_days(60), 40),
            (Duration::from_days(90), 50),
        ]);
        let parameter_bytes: Vec<u8> = to_bytes(&init_parameter(vesting));
        let ctx = init_ctx(
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(1),
            &parameter_bytes,
        );
        let mut state_builder = TestStateBuilder::new();

        let error = contract_init(&ctx, &mut state_builder);
        expect_error(
            error,
            CustomContractError::Inappropriate.into(),
            "this call should fail when vesting is wrong!",
        );
    }

    #[concordium_test]
    fn test_whitelisted() {
        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder, None, None);
        let mut host = TestHost::new(state, state_builder);

        let params = vec![
            AllowedUserParams {
                user: Address::Account(new_account()),
                prior: Prior::TOP,
            },
            AllowedUserParams {
                user: Address::Account(new_account()),
                prior: Prior::TOP,
            },
            AllowedUserParams {
                user: Address::Account(new_account()),
                prior: Prior::SECOND,
            },
            AllowedUserParams {
                user: Address::Account(new_account()),
                prior: Prior::SECOND,
            },
            AllowedUserParams {
                user: Address::Account(new_account()),
                prior: Prior::ANY,
            },
        ];

        let params_bytes: Vec<u8> = to_bytes(&params);
        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(5),
            &params_bytes,
        );
        let result: ContractResult<()> = contract_whitelisting(&ctx, &mut host);
        result.expect_report("Contract results in error");
        claim_eq!(
            host.state().status,
            SaleStatus::Ready,
            "Sale status should change after whitelisted"
        );
    }

    #[concordium_test]
    fn test_whitelisted_contract_account() {
        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder, None, None);
        let mut host = TestHost::new(state, state_builder);

        let params = vec![
            AllowedUserParams {
                user: Address::Account(new_account()),
                prior: Prior::TOP,
            },
            AllowedUserParams {
                user: Address::Account(new_account()),
                prior: Prior::TOP,
            },
            AllowedUserParams {
                user: Address::Account(new_account()),
                prior: Prior::SECOND,
            },
            AllowedUserParams {
                user: Address::Contract(ContractAddress::new(123, 0)),
                prior: Prior::SECOND,
            },
            AllowedUserParams {
                user: Address::Account(new_account()),
                prior: Prior::ANY,
            },
        ];

        let params_bytes: Vec<u8> = to_bytes(&params);
        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(5),
            &params_bytes,
        );
        let error: ContractResult<()> = contract_whitelisting(&ctx, &mut host);
        expect_error(
            error,
            CustomContractError::AccountOnly.into(),
            "this call should fail when contract address is registered",
        );
    }

    #[concordium_quickcheck(num_tests = 10)]
    fn test_whitelisted_pbt(participants: Vec<AccountAddress>, prior: Vec<u8>) -> bool {
        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder, None, None);
        let mut host = TestHost::new(state, state_builder);
        host.set_self_balance(Amount::from_ccd(100));

        let mut params = Vec::new();
        for (n, addr) in participants.into_iter().enumerate() {
            params.push(AllowedUserParams {
                user: Address::from(addr),
                prior: match prior.get(n) {
                    Some(x) => match x {
                        x if x > &50 => Prior::TOP,
                        _ => Prior::SECOND,
                    },
                    None => Prior::ANY,
                },
            });
        }

        let params_bytes: Vec<u8> = to_bytes(&params);
        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(5),
            &params_bytes,
        );
        let ret = contract_whitelisting(&ctx, &mut host);
        ret.is_ok()
    }

    #[concordium_test]
    fn test_ovl_claim() {
        let mut logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder, None, None);
        state.status = SaleStatus::Fixed;
        state.project_token = Some(ContractAddress::new(200, 0));
        state.schedule.vesting_start = Some(Timestamp::from_timestamp_millis(50));
        state.saleinfo.applied_units = 80;

        let mut host = TestHost::new(state, state_builder);
        host.setup_mock_entrypoint(
            ContractAddress::new(200, 0),
            OwnedEntrypointName::new_unchecked("transfer".into()),
            MockFn::returning_ok(()),
        );

        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(60)
                .checked_add(Duration::from_days(1))
                .expect_report("Failed to calculate the date"),
            &[],
        );

        let ret = contract_ovl_claim(&ctx, &mut host, &mut logger);
        claim!(ret.is_ok(), "Results in rejection");

        claim_eq!(
            logger.logs[0],
            to_bytes(&OvlSaleEvent::Claim(ClaimEvent {
                to: OVL_TEAM_ADDR,
                amount: 200,
                inc: 1,
            })),
            "Something wrong with event emitted"
        );

        let state = host.state();
        claim_eq!(
            state.ovl_claimed_inc,
            1_u8,
            "Something wrong with user claim."
        );
    }

    #[concordium_test]
    fn test_ovl_claim2() {
        let mut state_builder = TestStateBuilder::new();

        let schedule = SaleSchedule::new(
            Timestamp::from_timestamp_millis(1),
            BTreeMap::from([
                (Timestamp::from_timestamp_millis(10), Prior::TOP),
                (Timestamp::from_timestamp_millis(20), Prior::SECOND),
            ]),
            Timestamp::from_timestamp_millis(30),
            BTreeMap::from([
                (Duration::from_days(1), 25),
                (Duration::from_days(2), 40),
                (Duration::from_days(3), 35),
            ]),
        )
        .ok();
        let saleinfo = SaleInfo::new(5_000_000, 200.into(), 1000, 500).ok();

        let mut state = initial_state(&mut state_builder, schedule, saleinfo);
        state.status = SaleStatus::Fixed;
        state.project_token = Some(ContractAddress::new(200, 0));
        state.schedule.vesting_start = Some(Timestamp::from_timestamp_millis(50));
        state.saleinfo.applied_units = 800;

        let mut host = TestHost::new(state, state_builder);
        host.setup_mock_entrypoint(
            ContractAddress::new(200, 0),
            OwnedEntrypointName::new_unchecked("transfer".into()),
            MockFn::returning_ok(()),
        );

        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(60)
                .checked_add(Duration::from_days(2))
                .expect_report("Failed to calculate the date"),
            &[],
        );
        let mut logger = TestLogger::init();
        let ret = contract_ovl_claim(&ctx, &mut host, &mut logger);
        // println!("{ret:?}");
        claim!(ret.is_ok(), "Results in rejection");
        // claim_eq!(
        //     host.state().ovl_claimed_inc,
        //     1_u8,
        //     "Something wrong with user claim."
        // );
    }

    #[concordium_test]
    fn test_contract_upgrade() {
        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder, None, None);
        let mut host = TestHost::new(state, state_builder);

        host.setup_mock_upgrade(ModuleReference::from([9u8; 32]), Ok(()));

        let key1 = RegisterPublicKeyParam {
            account: OVL_TEAM_ACC,
            public_key: KEY1,
        };
        let key2 = RegisterPublicKeyParam {
            account: PJ_ADMIN_ACC,
            public_key: KEY2,
        };
        let parameter = RegisterPublicKeyParams(vec![key1, key2]);
        let params_bytes = to_bytes(&parameter);

        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(5),
            &params_bytes,
        );
        let _: ContractResult<()> = contract_regist_updkeys(&ctx, &mut host);

        let mut signature_map = BTreeSet::new();
        signature_map.insert((OVL_TEAM_ACC, SIGNATURE1));
        signature_map.insert((PJ_ADMIN_ACC, SIGNATURE2));

        let permit_param = UpgradeParams {
            module: ModuleReference::from([9u8; 32]),
            migrate: None,
            signatures: signature_map,
            message: PermitMessage {
                contract_address: ContractAddress {
                    index: 10,
                    subindex: 0,
                },
                entry_point: OwnedEntrypointName::new_unchecked("contract_upgrade".into()),
                payload: PermitPayload::Upgrade,
                timestamp: Timestamp::from_timestamp_millis(100),
            },
        };
        let params_bytes = to_bytes(&permit_param);
        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(5),
            &params_bytes,
        );

        let crypto_primitives = TestCryptoPrimitives::new();
        let message_hash = crypto_primitives
            .hash_sha2_256(&to_bytes(&permit_param.message))
            .0;
        // println!("{message_hash:?}");

        let ret: ContractResult<()> = contract_upgrade(&ctx, &mut host, &crypto_primitives);
        claim!(ret.is_ok(), "Results in rejection");
    }
}

#[concordium_cfg_test]
mod test_proj {
    use super::*;

    #[concordium_test]
    fn test_proj_claim() {
        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder, None, None);
        state.status = SaleStatus::Fixed;
        let mut host = TestHost::new(state, state_builder);
        let balance = Amount::from_ccd(100);
        host.set_self_balance(balance);

        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            PJ_ADMIN_ACC,
            Timestamp::from_timestamp_millis(5),
            &[],
        );
        let ret = contract_project_claim(&ctx, &mut host);
        claim_eq!(
            host.get_transfers(),
            [(PJ_ADMIN_ACC, balance)],
            "Something wrong with project claim."
        );
    }
}

#[concordium_cfg_test]
mod test_user {
    use super::*;

    #[concordium_test]
    fn test_deposit() {
        let acc1 = new_account();
        let acc2 = new_account();

        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder, None, None);
        let mut host = TestHost::new(state, state_builder);
        host.set_self_balance(Amount::from_ccd(100));

        let params = vec![
            AllowedUserParams {
                user: Address::Account(acc1),
                prior: Prior::TOP,
            },
            AllowedUserParams {
                user: Address::Account(acc2),
                prior: Prior::SECOND,
            },
        ];
        let params_bytes: Vec<u8> = to_bytes(&params);
        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(5),
            &params_bytes,
        );
        let _ = contract_whitelisting(&ctx, &mut host);

        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            acc1,
            Timestamp::from_timestamp_millis(15),
            &[],
        );
        let amount = Amount::from_micro_ccd(5_000_000 * 200 * 1);
        let result: ContractResult<()> = contract_user_deposit(&ctx, &mut host, amount);
        // println!("{:?}", result);
        claim!(result.is_ok(), "Results in rejection");

        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            acc2,
            Timestamp::from_timestamp_millis(25),
            &[],
        );
        let amount = Amount::from_micro_ccd(5_000_000 * 200 * 1);
        let result: ContractResult<()> = contract_user_deposit(&ctx, &mut host, amount);
        // println!("{:?}", result);
        claim!(result.is_ok(), "Results in rejection");
    }

    #[concordium_test]
    fn test_deposit_before_ready() {
        let parameter_bytes: Vec<u8> = to_bytes(&init_parameter(BTreeMap::new()));
        let ctx = init_ctx(
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(1),
            &parameter_bytes,
        );
        let mut state_builder = TestStateBuilder::new();
        let mut state = contract_init(&ctx, &mut state_builder).unwrap();
        // state.status = SaleStatus::Prepare;
        let mut host = TestHost::new(state, state_builder);

        // let ctx = receive_ctx(
        //     OVL_TEAM_ACC,
        //     OVL_TEAM_ACC,
        //     Timestamp::from_timestamp_millis(12),
        //     &[],
        // );
        // let _ = contract_set_prepare(&ctx, &mut host);

        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            new_account(),
            Timestamp::from_timestamp_millis(15),
            &[],
        );
        let error: ContractResult<()> =
            contract_user_deposit(&ctx, &mut host, Amount::from_micro_ccd(100));
        expect_error(
            error,
            CustomContractError::SaleNotReady.into(),
            "this call should fail when sale is not ready",
        );
        // claim_eq!(
        //     error,
        //     Err(CustomContractError::SaleNotReady.into()),
        //     "Function should throw an error."
        // );
    }

    #[concordium_test]
    fn test_deposit_before_open() {
        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder, None, None);
        state.status = SaleStatus::Ready;

        let mut host = TestHost::new(state, state_builder);

        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            new_account(),
            Timestamp::from_timestamp_millis(5),
            &[],
        );
        let amount = Amount::from_ccd(100);
        let result: ContractResult<()> = contract_user_deposit(&ctx, &mut host, amount);
        expect_error(
            result,
            CustomContractError::InvalidSchedule.into(),
            "this call should fail when sale is not open",
        );
    }

    #[concordium_test]
    fn test_quit() {
        let acc1 = new_account();
        let acc2 = new_account();

        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder, None, None);
        let mut host = TestHost::new(state, state_builder);
        let balance = Amount::from_micro_ccd(5_000_000 * 200 * 10);
        host.set_self_balance(balance);

        let params = vec![
            AllowedUserParams {
                user: Address::Account(acc1),
                prior: Prior::TOP,
            },
            AllowedUserParams {
                user: Address::Account(acc2),
                prior: Prior::SECOND,
            },
        ];
        let params_bytes: Vec<u8> = to_bytes(&params);
        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            OVL_TEAM_ACC,
            Timestamp::from_timestamp_millis(5),
            &params_bytes,
        );
        let _ = contract_whitelisting(&ctx, &mut host);

        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            acc1,
            Timestamp::from_timestamp_millis(15),
            &[],
        );
        let amount = Amount::from_micro_ccd(5_000_000 * 200 * 1);
        let _ = contract_user_deposit(&ctx, &mut host, amount);

        let ctx = receive_ctx(
            OVL_TEAM_ACC,
            acc1,
            Timestamp::from_timestamp_millis(25),
            &[],
        );
        let ret = contract_user_quit(&ctx, &mut host);
        expect_error(
            ret,
            CustomContractError::DisabledForNow.into(),
            "this call should fail because disabled",
        );
        // claim_eq!(
        //     host.get_transfers(),
        //     [(acc1, amount)],
        //     "Something wrong with user claim."
        // );
        // for (addr, user_state) in host.state_mut().participants.iter() {
        //     println!("{:?}", *addr);
        //     println!("{:?}", *user_state);
        // }
    }

    #[concordium_test]
    fn test_user_claim() {
        let accounts = vec![new_account(), new_account(), new_account()];

        let schedule = SaleSchedule::new(
            Timestamp::from_timestamp_millis(1),
            BTreeMap::from([
                (Timestamp::from_timestamp_millis(10), Prior::TOP),
                (Timestamp::from_timestamp_millis(20), Prior::SECOND),
            ]),
            Timestamp::from_timestamp_millis(30),
            BTreeMap::from([
                (Duration::from_millis(10), 25),
                (Duration::from_millis(20), 40),
                (Duration::from_millis(30), 35),
            ]),
        )
        .ok();
        let saleinfo = SaleInfo::new(5_000_000, 200.into(), 100, 50).ok();

        let mut logger = TestLogger::init();
        let mut state_builder = TestStateBuilder::new();
        let mut state = initial_state(&mut state_builder, schedule, saleinfo);
        state.status = SaleStatus::Fixed;
        state.project_token = Some(ContractAddress::new(1, 0));
        state.schedule.vesting_start = Some(Timestamp::from_timestamp_millis(50));
        state.saleinfo.applied_units = 80;

        for acc in accounts.iter() {
            state.participants.insert(
                Address::from(*acc),
                UserState {
                    prior: Prior::TOP,
                    deposit_ccd: Amount::from_micro_ccd(5_000_000 * 200 * 1),
                    tgt_units: TARGET_UNITS,
                    win_units: 1,
                    claimed_inc: 0,
                },
            );
        }

        let mut host = TestHost::new(state, state_builder);
        host.setup_mock_entrypoint(
            ContractAddress::new(1, 0),
            OwnedEntrypointName::new_unchecked("transfer".into()),
            MockFn::returning_ok(()),
        );

        let mut ctx = TestReceiveContext::empty();
        ctx.set_self_address(ContractAddress::new(0, 0));
        ctx.set_owner(OVL_TEAM_ACC);
        ctx.set_sender(Address::from(accounts[0]));

        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(60));
        let ret: ContractResult<()> = contract_user_claim(&ctx, &mut host, &mut logger);
        claim!(ret.is_ok(), "Results in rejection");
        claim_eq!(logger.logs.len(), 1, "Only one event should be logged");
        claim_eq!(
            logger.logs[0],
            to_bytes(&OvlSaleEvent::Claim(ClaimEvent {
                to: Address::from(accounts[0]),
                amount: 45,
                inc: 1,
            })),
            "Something wrong with event emitted"
        );

        let ret: ContractResult<()> = contract_user_claim(&ctx, &mut host, &mut logger);
        claim!(ret.is_ok(), "Results in rejection");
        claim_eq!(logger.logs.len(), 2, "Two events should be logged");
        claim_eq!(
            logger.logs[1],
            to_bytes(&OvlSaleEvent::Claim(ClaimEvent {
                to: Address::from(accounts[0]),
                amount: 0,
                inc: 1,
            })),
            "Something wrong with event emitted"
        );

        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(70));
        let ret: ContractResult<()> = contract_user_claim(&ctx, &mut host, &mut logger);
        claim!(ret.is_ok(), "Results in rejection");
        claim_eq!(logger.logs.len(), 3, "Three events should be logged");
        claim_eq!(
            logger.logs[2],
            to_bytes(&OvlSaleEvent::Claim(ClaimEvent {
                to: Address::from(accounts[0]),
                amount: 72,
                inc: 2,
            })),
            "Something wrong with event emitted"
        );

        let ret: ContractResult<()> = contract_user_claim(&ctx, &mut host, &mut logger);
        claim!(ret.is_ok(), "Results in rejection");
        claim_eq!(logger.logs.len(), 4, "Four events should be logged");
        claim_eq!(
            logger.logs[3],
            to_bytes(&OvlSaleEvent::Claim(ClaimEvent {
                to: Address::from(accounts[0]),
                amount: 0,
                inc: 2,
            })),
            "Something wrong with event emitted"
        );

        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(80));
        let ret: ContractResult<()> = contract_user_claim(&ctx, &mut host, &mut logger);
        claim!(ret.is_ok(), "Results in rejection");
        claim_eq!(
            logger.logs[4],
            to_bytes(&OvlSaleEvent::Claim(ClaimEvent {
                to: Address::from(accounts[0]),
                amount: 63,
                inc: 3,
            })),
            "Something wrong with event emitted"
        );

        let ret: ContractResult<()> = contract_user_claim(&ctx, &mut host, &mut logger);
        claim!(ret.is_ok(), "Results in rejection");
        claim_eq!(
            logger.logs[5],
            to_bytes(&OvlSaleEvent::Claim(ClaimEvent {
                to: Address::from(accounts[0]),
                amount: 0,
                inc: 3,
            })),
            "Something wrong with event emitted"
        );

        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(90));
        let ret: ContractResult<()> = contract_user_claim(&ctx, &mut host, &mut logger);
        claim!(ret.is_ok(), "Results in rejection");
        claim_eq!(
            logger.logs[6],
            to_bytes(&OvlSaleEvent::Claim(ClaimEvent {
                to: Address::from(accounts[0]),
                amount: 0,
                inc: 3,
            })),
            "Something wrong with event emitted"
        );

        ctx.set_sender(Address::from(accounts[1]));
        ctx.set_metadata_slot_time(Timestamp::from_timestamp_millis(100));
        let ret: ContractResult<()> = contract_user_claim(&ctx, &mut host, &mut logger);
        claim!(ret.is_ok(), "Results in rejection");
        claim_eq!(
            logger.logs[7],
            to_bytes(&OvlSaleEvent::Claim(ClaimEvent {
                to: Address::from(accounts[1]),
                amount: 180,
                inc: 3,
            })),
            "Something wrong with event emitted"
        );
    }
}
