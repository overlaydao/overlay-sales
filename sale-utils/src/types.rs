use concordium_cis2::{TokenAmountU64, TokenIdUnit};
// use concordium_std::{SchemaType, Serialize};
use concordium_std::*;

pub type ContractTokenId = TokenIdUnit;
pub type ContractTokenAmount = TokenAmountU64;
pub type OvlCreditAmount = u64;
pub type MicroCcd = u64;
pub type UnitsAmount = u32;

pub type AllowedPercentage = u8;
pub type UsdcAmount = TokenAmountU64;
pub type MicroUsdc = u64;

#[derive(Debug, Serialize, SchemaType, Clone, PartialEq, Eq, PartialOrd)]
pub enum SaleStatus {
    Prepare,
    Ready,
    Fixed,
    Suspend,
}

#[derive(Debug, Serialize, SchemaType, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Tier {
    T0 = 0,
    T1,
    T2,
    T3,
    T4,
    T5,
}

impl From<u8> for Tier {
    fn from(n: u8) -> Self {
        match n {
            n if n == Tier::T1 as u8 => Tier::T1,
            n if n == Tier::T2 as u8 => Tier::T2,
            n if n == Tier::T3 as u8 => Tier::T3,
            n if n == Tier::T4 as u8 => Tier::T4,
            n if n == Tier::T5 as u8 => Tier::T5,
            _ => Tier::T0,
        }
    }
}

impl From<&str> for Tier {
    fn from(n: &str) -> Self {
        match n {
            "t1" => Tier::T1,
            "t2" => Tier::T2,
            "t3" => Tier::T3,
            "t4" => Tier::T4,
            "t5" => Tier::T5,
            _ => Tier::T0,
        }
    }
}

#[derive(Debug, Serialize, SchemaType, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Prior {
    TOP = 1,
    SECOND,
    ANY = 99,
}

// -----------------------------------------------

/// Part of the parameter type for PermitMessage.
#[derive(Debug, Serialize, SchemaType, Clone, PartialEq, Eq)]
pub enum PermitAction {
    AddKey,
    RemoveKey,
    Upgrade,
    Invoke(
        /// The invoking address.
        ContractAddress,
        /// The function to call on the invoking contract.
        OwnedEntrypointName,
    ),
}

/// Part of the parameter type for calling this contract.
/// Specifies the message that is signed.
#[derive(SchemaType, Serialize, Debug)]
pub struct PermitMessage {
    /// The contract_address that the signature is intended for.
    pub contract_address: ContractAddress,
    /// The entry_point that the signature is intended for.
    pub entry_point: OwnedEntrypointName,
    /// Enum to identify the action.
    pub action: PermitAction,
    /// A timestamp to make signatures expire.
    pub timestamp: Timestamp,
}

/// Part of the parameter type for calling this contract.
/// Specifies the message that is signed.
#[derive(SchemaType, Serialize, Debug)]
pub struct PermitMessageWithParameter {
    /// The contract_address that the signature is intended for.
    pub contract_address: ContractAddress,
    /// The entry_point that the signature is intended for.
    pub entry_point: OwnedEntrypointName,
    /// Enum to identify the action.
    pub action: PermitAction,
    /// A timestamp to make signatures expire.
    pub timestamp: Timestamp,
    /// The serialized parameter that should be forwarded to callee entrypoint.
    #[concordium(size_length = 2)]
    pub parameter: Vec<u8>,
}
