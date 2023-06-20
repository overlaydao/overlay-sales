use concordium_contracts_common::{AccountAddress, Amount};

pub const NODE_ENDPOINT_V2: &str = "http://153.126.181.131:20001";
pub const ACCOUNT_ADDRESS_SIZE: usize = 32;

pub const ACC_ADDR_OWNER: AccountAddress = AccountAddress([0u8; 32]);
pub const ACC_ADDR_OTHER: AccountAddress = AccountAddress([1u8; 32]);
pub const AMOUNT_INIT: Amount = Amount::from_ccd(1000);
pub const AMOUNT_ZERO: Amount = Amount::zero();

pub const TARGET_DIR: &str = "../../target/concordium/wasm32-unknown-unknown/release/";

pub const CONTRACT_OPERATOR: &str = "ovl_operator";
pub const INDEX_OPERATOR: u64 = 1;

pub const CONTRACT_USDC: &str = "cis2-bridgeable";
pub const INDEX_USDC: u64 = 3496;

pub const CONTRACT_PROJECT_TOKEN: &str = "cis2_OVL";
pub const INDEX_PROJECT_TOKEN: u64 = 1001;

pub const CONTRACT_PUB_RIDO_USDC: &str = "pub_rido_usdc";
pub const INDEX_PUB_RIDO_USDC: u64 = 10;

pub const CONTRACT_PUB_RIDO_CCD: &str = "pub_rido_ccd";
pub const INDEX_PUB_RIDO_CCD: u64 = 11;
