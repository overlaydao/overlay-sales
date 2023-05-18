use crate::{account_address_from_str, types::*, vec_to_arr};
use concordium_std::{from_bytes, to_bytes, PublicKeyEd25519};

pub fn create_init_operators_exp() -> anyhow::Result<Vec<u8>> {
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
