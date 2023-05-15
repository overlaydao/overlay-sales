use crate::{
    account_address_from_str,
    config::{CONTRACT_PUB_RIDO_USDC, MODREF_PUB_RIDO_USDC},
    get_keypair_from_wallet_keys,
    types::*,
    vec_to_arr, CONTRACT_OPERATOR, INDEX_OPERATOR, MODREF_OPERATOR, NODE_ENDPOINT_V2,
};
use anyhow::{bail, Context};
use concordium_rust_sdk::{
    common::types::{KeyPair, TransactionTime},
    smart_contracts::common::Amount,
    types::{
        hashes::{HashBytes, ModuleReferenceMarker},
        smart_contracts::{
            concordium_contracts_common::ContractAddress as CA, OwnedContractName, OwnedParameter,
            OwnedReceiveName,
        },
        transactions::{
            send, BlockItem, EncodedPayload, InitContractPayload, UpdateContractPayload,
        },
        AccountInfo, Nonce, WalletAccount,
    },
    v2::{BlockIdentifier, Client, Endpoint},
};
use concordium_std::{
    from_bytes, to_bytes, AccountAddress, Address, ContractAddress, Duration, PublicKeyEd25519,
    SignatureEd25519, Timestamp,
};
use sale_utils::types::{
    ContractTokenAmount, PermitAction, PermitMessageWithParameter, Prior, UnitsAmount,
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    str::FromStr,
};

fn create_init_operators_exp() -> anyhow::Result<Vec<u8>> {
    let ops = vec![
        (
            "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1",
            "61f2c1500d2694aff6d67cd1ec139f735de8ff6de1188ca3d9e2147ce8b49147",
        ),
        (
            "4bXHyEX6pJT29X8Mmn8UmhLRbW4ApdciqSq8AX1JdMXqNFmvUc",
            "69e5f3eba67291e2d5f10203f3d3d4c9542d4b02ccd156a229f0fafff3e81ba7",
        ),
    ];

    let mut operators: Vec<operators::OperatorWithKeyParam> = Vec::new();
    for (addr, pubkey) in ops {
        let addr = account_address_from_str(addr).unwrap();
        let pubkey: [u8; 32] = vec_to_arr(hex::decode(pubkey).unwrap());
        operators.push(operators::OperatorWithKeyParam {
            account: addr,
            public_key: PublicKeyEd25519(pubkey),
        });
    }

    let params = operators::InitParams { operators };
    let param_encoded = hex::encode(to_bytes(&params));

    let param_byte = hex::decode(&param_encoded)?;
    let init_param = from_bytes::<operators::InitParams>(&param_byte)?;

    Ok(to_bytes(&init_param))
}

fn create_init_pub_rido_usdc_exp() -> anyhow::Result<Vec<u8>> {
    let ops = vec![
        (
            "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1",
            "61f2c1500d2694aff6d67cd1ec139f735de8ff6de1188ca3d9e2147ce8b49147",
        ),
        (
            "4bXHyEX6pJT29X8Mmn8UmhLRbW4ApdciqSq8AX1JdMXqNFmvUc",
            "69e5f3eba67291e2d5f10203f3d3d4c9542d4b02ccd156a229f0fafff3e81ba7",
        ),
    ];

    let mut operators: Vec<operators::OperatorWithKeyParam> = Vec::new();
    for (addr, pubkey) in ops {
        let addr = account_address_from_str(addr).unwrap();
        let pubkey: [u8; 32] = vec_to_arr(hex::decode(pubkey).unwrap());
        operators.push(operators::OperatorWithKeyParam {
            account: addr,
            public_key: PublicKeyEd25519(pubkey),
        });
    }

    let params = pub_usdc::InitParams {
        operator: Address::from(ContractAddress::new(4513, 0)),
        usdc_contract: ContractAddress::new(3496, 0),
        proj_admin: account_address_from_str("3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1")?,
        addr_ovl: Address::from(account_address_from_str(
            "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1",
        )?),
        addr_bbb: Address::from(ContractAddress::new(4513, 0)),
        open_at: BTreeMap::from([
            (Timestamp::from_timestamp_millis(10), Prior::TOP),
            (Timestamp::from_timestamp_millis(20), Prior::SECOND),
        ]),
        close_at: Timestamp::from_timestamp_millis(30),
        vesting_period: BTreeMap::from([
            (Duration::from_days(1), 25),
            (Duration::from_days(2), 40),
            (Duration::from_days(3), 35),
        ]),
        price_per_token: 500,
        token_per_unit: ContractTokenAmount::from(200),
        max_units: 100,
        min_units: 1,
    };
    let param_encoded = hex::encode(to_bytes(&params));

    let param_byte = hex::decode(&param_encoded)?;
    let init_param = from_bytes::<operators::InitParams>(&param_byte)?;

    Ok(to_bytes(&init_param))
}

fn create_init_tx_item(
    sender: &WalletAccount,
    nonce: Nonce,
    module: &str,
    contract: &str,
    init_param: Vec<u8>,
    amount: Amount,
) -> BlockItem<EncodedPayload> {
    let expiry: TransactionTime =
        TransactionTime::from_seconds((chrono::Utc::now().timestamp() + 300) as u64);

    let mod_ref = HashBytes::<ModuleReferenceMarker>::from_str(module).unwrap();
    let init_name = OwnedContractName::new_unchecked(format!("init_{}", contract).to_string());
    let param = OwnedParameter::try_from(init_param).unwrap();

    let payload = InitContractPayload {
        amount,
        mod_ref,
        init_name,
        param,
    };

    let tx = send::init_contract(
        sender,
        sender.address,
        nonce,
        expiry,
        payload,
        10000u64.into(),
    );

    BlockItem::AccountTransaction(tx)
}

pub async fn initialize() -> anyhow::Result<()> {
    let mut client = Client::new(Endpoint::from_static(NODE_ENDPOINT_V2))
        .await
        .context("Cannot connect.")?;

    let keys1: WalletAccount = WalletAccount::from_json_file("./keys/keys.json")
        .context("Could not read the keys file.")?;

    let acc_info: AccountInfo = client
        .get_account_info(&keys1.address.into(), &BlockIdentifier::Best)
        .await
        .context("Cannot connect.")?
        .response;

    // #[Todo]
    let mode = "ops";
    let (modref, contract, init_params) = match mode {
        "ops" => {
            let params_bytes = create_init_operators_exp()?;
            (MODREF_OPERATOR, CONTRACT_OPERATOR, params_bytes)
        },
        "usdc" => {
            let params_bytes = create_init_pub_rido_usdc_exp()?;
            (MODREF_PUB_RIDO_USDC, CONTRACT_PUB_RIDO_USDC, params_bytes)
        },
        _ => {
            let params_bytes = create_init_operators_exp()?;
            (MODREF_OPERATOR, CONTRACT_OPERATOR, params_bytes)
        },
    };

    let amount = Amount::zero();
    let item = create_init_tx_item(
        &keys1,
        acc_info.account_nonce,
        modref,
        contract,
        init_params,
        amount,
    );

    crate::broadcast(&mut client, item);

    Ok(())
}
