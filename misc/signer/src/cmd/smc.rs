pub mod ovl_operator;
pub mod pub_rido_usdc;

use crate::NODE_ENDPOINT_V2;

use anyhow::{bail, Context};
use concordium_rust_sdk::{
    common::types::TransactionTime,
    smart_contracts::common::Amount,
    types::{
        hashes::{HashBytes, ModuleReferenceMarker},
        smart_contracts::{OwnedContractName, OwnedParameter, OwnedReceiveName},
        transactions::{
            send, BlockItem, EncodedPayload, InitContractPayload, UpdateContractPayload,
        },
        AccountInfo, AccountTransactionEffects, BlockItemSummary, BlockItemSummaryDetails, Nonce,
        WalletAccount,
    },
    v2::{BlockIdentifier, Client, Endpoint},
};

use std::str::FromStr;

pub async fn initialize(
    mod_ref: &str,
    contract_name: &str,
    init_params: Vec<u8>,
) -> anyhow::Result<BlockItemSummary> {
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

    println!("{:?}", mod_ref);
    println!("{:?}", contract_name);
    println!("{:?}", init_params);

    let amount = Amount::zero();
    let item = create_init_tx_item(
        &keys1,
        acc_info.account_nonce,
        mod_ref,
        contract_name,
        init_params,
        amount,
    );

    let summary = crate::broadcast(&mut client, item).await?;

    // if let Some(reason) = summary.is_rejected_account_transaction() {
    //     println!("Error occured! The reason is {:#?}", reason);
    // } else {
    //     if let BlockItemSummaryDetails::AccountTransaction(detail) = summary.details {
    //         println!("Cost: {:#?}", detail.cost);
    //         match &detail.effects {
    //             AccountTransactionEffects::ModuleDeployed { .. } => {},
    //             AccountTransactionEffects::ContractInitialized { data } => {
    //                 println!("Detail address: {:#?}", data.address);
    //             },
    //             AccountTransactionEffects::ContractUpdateIssued { .. } => {},
    //             _ => {},
    //         }
    //     };
    // }

    Ok(summary)
}

pub async fn update(
    index: u64,
    contract_name: &str,
    method: &str,
    update_params: Vec<u8>,
) -> anyhow::Result<BlockItemSummary> {
    // sender
    let keys1: WalletAccount = WalletAccount::from_json_file("./keys/keys.json")
        .context("Could not read the keys file.")?;

    let mut client = Client::new(Endpoint::from_static(NODE_ENDPOINT_V2))
        .await
        .context("Cannot connect.")?;

    let acc_info: AccountInfo = client
        .get_account_info(&keys1.address.into(), &BlockIdentifier::Best)
        .await
        .context("Cannot connect.")?
        .response;

    // Parameter for Tx
    let nonce = acc_info.account_nonce;
    let amount = Amount::zero();

    let item = create_update_tx_item(
        &keys1,
        nonce,
        index,
        contract_name,
        method,
        update_params,
        amount,
    );

    let summary = crate::broadcast(&mut client, item).await?;
    Ok(summary)
}

// =========================================================

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

fn create_update_tx_item(
    sender: &WalletAccount,
    nonce: Nonce,
    index: u64,
    contract: &str,
    method: &str,
    params: Vec<u8>,
    amount: Amount,
) -> BlockItem<EncodedPayload> {
    use concordium_rust_sdk::smart_contracts::types::concordium_contracts_common::ContractAddress;

    let expiry: TransactionTime =
        TransactionTime::from_seconds((chrono::Utc::now().timestamp() + 300) as u64);

    let address = ContractAddress::new(index, 0);
    let receive_name =
        OwnedReceiveName::new_unchecked(format!("{}.{}", contract, method).to_string());
    let message = OwnedParameter::try_from(params).unwrap();

    let payload = UpdateContractPayload {
        amount,
        address,
        receive_name,
        message,
    };

    let tx = send::update_contract(
        sender,
        sender.address,
        nonce,
        expiry,
        payload,
        30000u64.into(),
    );

    BlockItem::AccountTransaction(tx)
}
