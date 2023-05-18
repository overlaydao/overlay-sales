use crate::{account_address_from_str, timestamp_from_str, types::*, INDEX_OPERATOR, INDEX_USDC};
use concordium_std::{from_bytes, to_bytes, Address, ContractAddress, Duration};
use sale_utils::types::{ContractTokenAmount, Prior};
use std::collections::BTreeMap;

pub fn create_init_pub_rido_usdc_exp() -> anyhow::Result<Vec<u8>> {
    let params = pub_usdc::InitParams {
        operator: Address::from(ContractAddress::new(INDEX_OPERATOR, 0)),
        usdc_contract: ContractAddress::new(INDEX_USDC, 0),
        proj_admin: account_address_from_str("3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1")?,
        addr_ovl: Address::from(account_address_from_str(
            "3jfAuU1c4kPE6GkpfYw4KcgvJngkgpFrD9SkDBgFW3aHmVB5r1",
        )?),
        addr_bbb: Address::from(ContractAddress::new(INDEX_OPERATOR, 0)),
        open_at: BTreeMap::from([
            (timestamp_from_str("2023-05-18T00:00:00+09:00")?, Prior::TOP),
            (
                timestamp_from_str("2023-05-18T10:00:00+09:00")?,
                Prior::SECOND,
            ),
        ]),
        close_at: timestamp_from_str("2023-05-20T00:00:00+09:00")?,
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
    let init_param = from_bytes::<pub_usdc::InitParams>(&param_byte)?;

    Ok(to_bytes(&init_param))
}
