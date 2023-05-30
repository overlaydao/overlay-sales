use concordium_cis2::{TokenAmountU256, TokenIdU64};
use concordium_std::{
    to_bytes, AccountAddress, Address, ContractAddress, Deserial, OwnedEntrypointName,
    PublicKeyEd25519, SchemaType, Serial, Serialize, SignatureEd25519, Timestamp,
};
use serde::Serialize as serde_serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

// ------------------------------------------------------
// params
// ------------------------------------------------------
pub mod usdc {
    use concordium_contracts_common::{schema::VersionedModuleSchema, Cursor};

    use super::*;

    const ACCOUNT_1: AccountAddress = AccountAddress([1u8; 32]);
    const ADDRESS_1: Address = Address::Account(ACCOUNT_1);

    #[derive(Serialize, SchemaType)]
    pub struct DepositParams {
        pub address: Address,
        pub amount: TokenAmountU256,
        pub token_id: TokenIdU64,
    }

    fn token_amount(amount: u64) -> TokenAmountU256 {
        let amount = primitive_types::U256::from(amount);
        TokenAmountU256::from(amount)
    }

    pub fn test(schema: &VersionedModuleSchema, contract: &str, func: &str) -> anyhow::Result<()> {
        let types: concordium_contracts_common::schema::Type =
            schema.get_receive_param_schema(contract, func)?;

        let deposit_param = DepositParams {
            address: ADDRESS_1,
            amount: token_amount(10),
            token_id: TokenIdU64(0),
        };
        let parameter_bytes = to_bytes(&deposit_param);
        println!("{:?}", parameter_bytes);

        let mut state_cursor = Cursor::new(parameter_bytes);
        match types.to_json(&mut state_cursor) {
            Ok(schema) => {
                println!("{:?}", schema);
                let json = serde_json::to_string_pretty(&schema).unwrap();
                println!("{}", json);
                Ok(())
            },
            Err(e) => anyhow::bail!("x"),
        }
    }
}
