use concordium_std::{
    to_bytes, AccountAddress, Address, ContractAddress, Deserial, OwnedEntrypointName,
    PublicKeyEd25519, Serial, SignatureEd25519, Timestamp,
};
use sale_utils::types::{PermitMessageWithParameter, Prior};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

// ------------------------------------------------------
// params
// ------------------------------------------------------
pub mod pub_usdc {
    use concordium_std::Duration;
    use sale_utils::types::{AllowedPercentage, ContractTokenAmount, MicroUsdc, UnitsAmount};

    use super::*;

    #[derive(Debug, Serial, Deserial)]
    pub struct InitParams {
        /// Contract owner
        pub operator: Address,
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
}

pub mod operators {
    use super::*;

    #[derive(Clone, Copy, Debug)]
    pub struct OperatorWithKeyParam {
        /// Account that a public key will be registered to.
        pub(crate) account: AccountAddress,
        /// The public key that should be linked to the above account.
        pub(crate) public_key: PublicKeyEd25519,
    }

    impl concordium_std::Serial for OperatorWithKeyParam {
        fn serial<W: concordium_std::Write>(&self, out: &mut W) -> Result<(), W::Err> {
            concordium_std::Serial::serial(&self.account, out)?;
            concordium_std::Serial::serial(&self.public_key, out)?;
            std::result::Result::Ok(())
        }
    }

    impl concordium_std::Deserial for OperatorWithKeyParam {
        fn deserial<R: concordium_std::Read>(src: &mut R) -> concordium_std::ParseResult<Self> {
            let account = <AccountAddress as concordium_std::Deserial>::deserial(src)?;
            let public_key = <PublicKeyEd25519 as concordium_std::Deserial>::deserial(src)?;
            Ok(OperatorWithKeyParam {
                account,
                public_key,
            })
        }
    }

    pub struct InitParams {
        pub(crate) operators: Vec<OperatorWithKeyParam>,
    }

    impl concordium_std::Serial for InitParams {
        fn serial<W: concordium_std::Write>(&self, out: &mut W) -> Result<(), W::Err> {
            {
                concordium_std::SerialCtx::serial_ctx(
                    &self.operators,
                    concordium_std::schema::SizeLength::U8,
                    out,
                )?;
            }
            Ok(())
        }
    }

    impl concordium_std::Deserial for InitParams {
        fn deserial<R: concordium_std::Read>(src: &mut R) -> concordium_std::ParseResult<Self> {
            let operators =
                <Vec<OperatorWithKeyParam> as concordium_std::DeserialCtx>::deserial_ctx(
                    concordium_std::schema::SizeLength::U8,
                    false,
                    src,
                )?;
            Ok(InitParams { operators })
        }
    }

    pub struct ParamsWithSignatures {
        /// Signatures of those who approve calling the contract.
        pub(crate) signatures: BTreeSet<(AccountAddress, SignatureEd25519)>,
        /// Message that was signed.
        pub(crate) message: PermitMessageWithParameter,
    }

    impl concordium_std::Serial for ParamsWithSignatures {
        fn serial<W: concordium_std::Write>(&self, out: &mut W) -> Result<(), W::Err> {
            concordium_std::Serial::serial(&self.signatures, out)?;
            concordium_std::Serial::serial(&self.message, out)?;
            Ok(())
        }
    }
}
// ------------------------------------------------------
// types
// ------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeypairString {
    pub sign_key: String,
    pub verify_key: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct KeyContent {
    pub keys: HashMap<u8, KeypairString>,
    pub threshold: u8,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Keys {
    pub keys: HashMap<u8, KeyContent>,
    pub threshold: u8,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct AccountKeys {
    #[serde(rename = "accountKeys")]
    pub account_keys: Keys,
    pub address: String,
}
