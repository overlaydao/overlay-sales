use concordium_std::concordium_cfg_test;

#[concordium_cfg_test]
mod tests {

    use crate::*;
    use concordium_std::test_infrastructure::*;

    use concordium_std::{PublicKeyEd25519, SignatureEd25519};
    use ed25519_dalek::{Keypair, Signature, Signer};
    // use concordium_rust_sdk::eddsa_ed25519;

    const SELF_ADDRESS: ContractAddress = ContractAddress {
        index: 10,
        subindex: 0,
    };

    const ACCOUNT0: AccountAddress = AccountAddress([0u8; 32]);
    const ACCOUNT1: AccountAddress = AccountAddress([1u8; 32]);
    const ACCOUNT2: AccountAddress = AccountAddress([2u8; 32]);

    const KEY0: [u8; 64] = [
        41, 89, 149, 52, 220, 116, 206, 129, 30, 45, 49, 140, 88, 111, 167, 148, 20, 127, 177, 170,
        140, 41, 75, 169, 60, 61, 81, 169, 220, 71, 5, 231, 8, 211, 250, 223, 44, 39, 123, 51, 101,
        19, 5, 83, 106, 180, 107, 46, 184, 236, 224, 126, 222, 95, 136, 71, 4, 215, 10, 37, 43, 82,
        120, 12,
    ];

    const KEY1: [u8; 64] = [
        183, 53, 101, 173, 212, 166, 174, 137, 79, 55, 245, 87, 113, 225, 62, 115, 195, 27, 109,
        16, 133, 19, 160, 207, 7, 40, 124, 130, 45, 142, 242, 185, 254, 53, 190, 123, 97, 18, 124,
        38, 220, 190, 222, 208, 93, 187, 227, 177, 239, 68, 236, 56, 45, 114, 81, 251, 20, 56, 33,
        163, 97, 220, 128, 211,
    ];

    const KEY2: [u8; 64] = [
        161, 100, 154, 83, 186, 238, 126, 254, 76, 80, 246, 176, 241, 10, 87, 201, 236, 55, 61, 29,
        26, 72, 195, 196, 151, 177, 186, 190, 56, 82, 29, 12, 28, 240, 172, 156, 102, 146, 187, 42,
        192, 214, 5, 35, 86, 195, 78, 96, 170, 68, 127, 199, 76, 7, 130, 122, 182, 144, 50, 65, 66,
        231, 97, 233,
    ];

    const PUBKEY0: PublicKeyEd25519 = PublicKeyEd25519([
        8, 211, 250, 223, 44, 39, 123, 51, 101, 19, 5, 83, 106, 180, 107, 46, 184, 236, 224, 126,
        222, 95, 136, 71, 4, 215, 10, 37, 43, 82, 120, 12,
    ]);

    const PUBKEY1: PublicKeyEd25519 = PublicKeyEd25519([
        254, 53, 190, 123, 97, 18, 124, 38, 220, 190, 222, 208, 93, 187, 227, 177, 239, 68, 236,
        56, 45, 114, 81, 251, 20, 56, 33, 163, 97, 220, 128, 211,
    ]);

    const PUBKEY2: PublicKeyEd25519 = PublicKeyEd25519([
        28, 240, 172, 156, 102, 146, 187, 42, 192, 214, 5, 35, 86, 195, 78, 96, 170, 68, 127, 199,
        76, 7, 130, 122, 182, 144, 50, 65, 66, 231, 97, 233,
    ]);

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

    fn compare_operators<S: HasStateApi>(
        a: &StateMap<AccountAddress, PublicKeyEd25519, S>,
        b: &StateMap<AccountAddress, PublicKeyEd25519, S>,
    ) -> bool {
        if a.iter().count() != b.iter().count() {
            return false;
        }
        for (acc, pubkey) in a.iter() {
            let other_pubkey = b.get(&acc);
            if other_pubkey.is_none() {
                return false;
            }
            let other_pubkey = other_pubkey.unwrap();
            if pubkey.clone() != other_pubkey.clone() {
                return false;
            }
        }
        true
    }

    #[derive(Debug, Serialize)]
    struct WhitelistingParams {
        wl: Vec<AllowedUserParams>,
        ready: bool,
    }

    #[derive(Debug, Serialize)]
    struct AllowedUserParams {
        user: Address,
        prior: Prior,
    }

    #[concordium_test]
    fn test_init() {
        let mut builder = TestStateBuilder::new();

        let ops1 = OperatorWithKeyParam {
            account: ACCOUNT1,
            public_key: PUBKEY1,
        };
        let ops2 = OperatorWithKeyParam {
            account: ACCOUNT2,
            public_key: PUBKEY2,
        };

        let mut expected_operators = builder.new_map();
        for v in vec![&ops1, &ops2] {
            expected_operators.insert(v.account, v.public_key);
        }

        let params = InitParams {
            operators: vec![ops1, ops2],
        };
        let params_byte = to_bytes(&params);

        let ctx = init_context(ACCOUNT0, Timestamp::from_timestamp_millis(1), &params_byte);
        let result = contract_init(&ctx, &mut builder);
        claim!(result.is_ok());
        claim!(
            compare_operators(&result.unwrap().operators, &expected_operators),
            "both operators should be matched."
        );
    }

    #[concordium_test]
    #[cfg(feature = "crypto-primitives")]
    fn test_add_key() {
        let crypto_primitives = TestCryptoPrimitives::new();
        let mut state_builder = TestStateBuilder::new();
        let mut state = State {
            operators: state_builder.new_map(),
        };

        // initial state
        let ops0 = OperatorWithKeyParam {
            account: ACCOUNT0,
            public_key: PUBKEY0,
        };
        let ops1 = OperatorWithKeyParam {
            account: ACCOUNT1,
            public_key: PUBKEY1,
        };
        let ops2 = OperatorWithKeyParam {
            account: ACCOUNT2,
            public_key: PUBKEY2,
        };

        for v in vec![&ops0, &ops1] {
            state.operators.insert(v.account, v.public_key);
        }

        let mut expected_operators = state_builder.new_map();
        for v in vec![&ops0, &ops1, &ops2] {
            expected_operators.insert(v.account, v.public_key);
        }

        let operators: Vec<OperatorWithKeyParam> = vec![OperatorWithKeyParam {
            account: ACCOUNT2,
            public_key: PUBKEY2,
        }];

        let message = PermitMessageWithParameter {
            contract_address: ContractAddress {
                index: 10,
                subindex: 0,
            },
            entry_point: OwnedEntrypointName::new_unchecked("addOperatorKeys".into()),
            action: PermitAction::AddKey,
            timestamp: Timestamp::from_timestamp_millis(100),
            parameter: to_bytes(&operators),
        };

        let message_bytes = to_bytes(&message);
        let message_hash = crypto_primitives.hash_sha2_256(&message_bytes).0;

        let sig0: Signature = Keypair::from_bytes(&KEY0).unwrap().sign(&message_hash);
        let sig1: Signature = Keypair::from_bytes(&KEY1).unwrap().sign(&message_hash);

        let mut signatures = BTreeSet::new();
        signatures.insert((ACCOUNT0, SignatureEd25519(sig0.to_bytes())));
        signatures.insert((ACCOUNT1, SignatureEd25519(sig1.to_bytes())));

        //
        let params = UpdatePublicKeyParams {
            signatures,
            message,
        };

        let mut host = TestHost::new(state, state_builder);

        let params_bytes = to_bytes(&params);
        let mut ctx = receive_context(
            ACCOUNT1,
            ACCOUNT1,
            Address::from(ACCOUNT1),
            Timestamp::from_timestamp_millis(15),
            &params_bytes,
        );
        ctx.set_named_entrypoint(OwnedEntrypointName::new_unchecked("addOperatorKeys".into()));

        let result: ContractResult<_> = contract_add_operators(&ctx, &mut host, &crypto_primitives);
        claim!(result.is_ok(), "Results in rejection");
        claim_eq!(
            host.state().operators.iter().count(),
            3,
            "there should be three operators now."
        );
        claim!(
            compare_operators(&host.state().operators, &expected_operators),
            "both operators should be matched."
        );
    }

    #[concordium_test]
    #[cfg(feature = "crypto-primitives")]
    fn test_remove_key() {
        let crypto_primitives = TestCryptoPrimitives::new();
        let mut state_builder = TestStateBuilder::new();
        let mut state = State {
            operators: state_builder.new_map(),
        };

        // initial state
        let ops0 = OperatorWithKeyParam {
            account: ACCOUNT0,
            public_key: PUBKEY0,
        };
        let ops1 = OperatorWithKeyParam {
            account: ACCOUNT1,
            public_key: PUBKEY1,
        };
        let ops2 = OperatorWithKeyParam {
            account: ACCOUNT2,
            public_key: PUBKEY2,
        };

        for v in vec![&ops0, &ops1, &ops2] {
            state.operators.insert(v.account, v.public_key);
        }

        let mut expected_operators = state_builder.new_map();
        for v in vec![&ops1, &ops2] {
            expected_operators.insert(v.account, v.public_key);
        }

        let operators: Vec<OperatorWithKeyParam> = vec![OperatorWithKeyParam {
            account: ACCOUNT0,
            public_key: PUBKEY0,
        }];

        let message = PermitMessageWithParameter {
            contract_address: ContractAddress {
                index: 10,
                subindex: 0,
            },
            entry_point: OwnedEntrypointName::new_unchecked("removeOperatorKeys".into()),
            action: PermitAction::RemoveKey,
            timestamp: Timestamp::from_timestamp_millis(100),
            parameter: to_bytes(&operators),
        };

        let message_hash = crypto_primitives.hash_sha2_256(&to_bytes(&message)).0;

        let sig1: Signature = Keypair::from_bytes(&KEY1).unwrap().sign(&message_hash);
        let sig2: Signature = Keypair::from_bytes(&KEY2).unwrap().sign(&message_hash);

        let mut signatures = BTreeSet::new();
        signatures.insert((ACCOUNT1, SignatureEd25519(sig1.to_bytes())));
        signatures.insert((ACCOUNT2, SignatureEd25519(sig2.to_bytes())));

        //
        let params = UpdatePublicKeyParams {
            signatures,
            message,
        };

        let mut host = TestHost::new(state, state_builder);

        let params_bytes = to_bytes(&params);
        let mut ctx = receive_context(
            ACCOUNT1,
            ACCOUNT1,
            Address::from(ACCOUNT1),
            Timestamp::from_timestamp_millis(15),
            &params_bytes,
        );
        ctx.set_named_entrypoint(OwnedEntrypointName::new_unchecked(
            "removeOperatorKeys".into(),
        ));

        let result: ContractResult<_> =
            contract_remove_operators(&ctx, &mut host, &crypto_primitives);
        claim!(result.is_ok(), "Results in rejection");
        claim_eq!(
            host.state().operators.iter().count(),
            2,
            "there should be two operators now."
        );
        claim!(
            compare_operators(&host.state().operators, &expected_operators),
            "both operators should be matched."
        );
    }

    #[concordium_test]
    #[cfg(feature = "crypto-primitives")]
    fn test_invoke() {
        let crypto_primitives = TestCryptoPrimitives::new();
        let mut state_builder = TestStateBuilder::new();

        let ops0 = OperatorWithKeyParam {
            account: ACCOUNT0,
            public_key: PUBKEY0,
        };
        let ops1 = OperatorWithKeyParam {
            account: ACCOUNT1,
            public_key: PUBKEY1,
        };
        let ops2 = OperatorWithKeyParam {
            account: ACCOUNT2,
            public_key: PUBKEY2,
        };

        let mut state = State {
            operators: state_builder.new_map(),
        };

        for v in vec![&ops0, &ops1, &ops2] {
            state.operators.insert(v.account, v.public_key);
        }

        let mut host = TestHost::new(state, state_builder);
        host.setup_mock_entrypoint(
            ContractAddress {
                index: 9,
                subindex: 0,
            },
            OwnedEntrypointName::new_unchecked("whitelisting".into()),
            MockFn::new_v1(move |_parameter, _amount, _balance, _state| Ok((false, ()))),
        );

        let whitelist = vec![
            AllowedUserParams {
                user: Address::Account(AccountAddress([10u8; 32])),
                prior: Prior::TOP,
            },
            AllowedUserParams {
                user: Address::Account(AccountAddress([11u8; 32])),
                prior: Prior::TOP,
            },
            AllowedUserParams {
                user: Address::Account(AccountAddress([12u8; 32])),
                prior: Prior::SECOND,
            },
            AllowedUserParams {
                user: Address::Account(AccountAddress([13u8; 32])),
                prior: Prior::SECOND,
            },
            AllowedUserParams {
                user: Address::Account(AccountAddress([14u8; 32])),
                prior: Prior::ANY,
            },
        ];

        let invoke_params = WhitelistingParams {
            wl: whitelist,
            ready: true,
        };

        let message = PermitMessageWithParameter {
            contract_address: ContractAddress {
                index: 10,
                subindex: 0,
            },
            entry_point: OwnedEntrypointName::new_unchecked("whitelisting".into()),
            action: PermitAction::Invoke(
                ContractAddress {
                    index: 9,
                    subindex: 0,
                },
                OwnedEntrypointName::new_unchecked("whitelisting".into()),
            ),
            timestamp: Timestamp::from_timestamp_millis(100),
            parameter: to_bytes(&invoke_params),
        };

        let message_hash = crypto_primitives.hash_sha2_256(&to_bytes(&message)).0;

        let sig1: Signature = Keypair::from_bytes(&KEY1).unwrap().sign(&message_hash);
        let sig2: Signature = Keypair::from_bytes(&KEY2).unwrap().sign(&message_hash);

        let mut signatures = BTreeSet::new();
        signatures.insert((ACCOUNT1, SignatureEd25519(sig1.to_bytes())));
        signatures.insert((ACCOUNT2, SignatureEd25519(sig2.to_bytes())));

        // Deposit from user1
        let params = InvokeParams {
            signatures,
            message,
        };
        let params_bytes = to_bytes(&params);
        let mut ctx = receive_context(
            ACCOUNT0,
            ACCOUNT0,
            Address::from(ACCOUNT0),
            Timestamp::from_timestamp_millis(15),
            &params_bytes,
        );
        ctx.set_named_entrypoint(OwnedEntrypointName::new_unchecked("whitelisting".into()));

        let crypto_primitives = TestCryptoPrimitives::new();
        let result: ContractResult<_> = contract_invoke_sale(&ctx, &mut host, &crypto_primitives);
        claim!(result.is_ok(), "Results in rejection.");
    }

    #[concordium_test]
    #[cfg(feature = "crypto-primitives")]
    fn test_add_key_fail_with_only_one_sigs() {
        let crypto_primitives = TestCryptoPrimitives::new();
        let mut state_builder = TestStateBuilder::new();
        let mut state = State {
            operators: state_builder.new_map(),
        };

        // initial state
        let ops1 = OperatorWithKeyParam {
            account: ACCOUNT1,
            public_key: PUBKEY1,
        };
        let ops2 = OperatorWithKeyParam {
            account: ACCOUNT2,
            public_key: PUBKEY2,
        };

        let mut expected_operators = state_builder.new_map();
        for v in vec![&ops1, &ops2] {
            state.operators.insert(v.account, v.public_key);
            expected_operators.insert(v.account, v.public_key);
        }

        let operators: Vec<OperatorWithKeyParam> = vec![OperatorWithKeyParam {
            account: ACCOUNT0,
            public_key: PUBKEY0,
        }];

        let message = PermitMessageWithParameter {
            contract_address: ContractAddress {
                index: 10,
                subindex: 0,
            },
            entry_point: OwnedEntrypointName::new_unchecked("addOperatorKeys".into()),
            action: PermitAction::AddKey,
            timestamp: Timestamp::from_timestamp_millis(100),
            parameter: to_bytes(&operators),
        };

        let message_hash = crypto_primitives.hash_sha2_256(&to_bytes(&message)).0;

        let sig1: Signature = Keypair::from_bytes(&KEY1).unwrap().sign(&message_hash);

        let mut signatures = BTreeSet::new();
        signatures.insert((ACCOUNT1, SignatureEd25519(sig1.to_bytes())));

        //
        let params = UpdatePublicKeyParams {
            signatures,
            message,
        };

        let mut host = TestHost::new(state, state_builder);

        let params_bytes = to_bytes(&params);
        let mut ctx = receive_context(
            ACCOUNT1,
            ACCOUNT1,
            Address::from(ACCOUNT1),
            Timestamp::from_timestamp_millis(15),
            &params_bytes,
        );
        ctx.set_named_entrypoint(OwnedEntrypointName::new_unchecked("addOperatorKeys".into()));

        let result: ContractResult<_> = contract_add_operators(&ctx, &mut host, &crypto_primitives);
        claim!(result.is_err(), "Should cause error.");
        claim_eq!(
            result.expect_err_report("user deposit should reject"),
            ContractError::Unauthorized,
            "should reject with Unauthorized"
        );
        claim_eq!(
            host.state().operators.iter().count(),
            2,
            "there should be two operators now."
        );
        claim!(
            compare_operators(&host.state().operators, &expected_operators),
            "both operators should be matched."
        );
    }
}
