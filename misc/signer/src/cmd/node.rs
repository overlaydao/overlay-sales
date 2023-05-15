use anyhow::Context;
use concordium_rust_sdk::v2::{Client, Endpoint};

use crate::config::{NODE_ENDPOINT_V2_TEST_NOTLS, NODE_ENDPOINT_V2_TEST};

pub async fn nodeinfo(endpoint: Endpoint) -> anyhow::Result<()> {
    // let endpoint = Endpoint::from_static(NODE_ENDPOINT_V2_TEST_NOTLS);
    // let endpoint = Endpoint::from_static(NODE_ENDPOINT_V2_TEST);
    let mut client = Client::new(endpoint)
        .await
        .context("Cannot connect to the node.")?;

    let node_info = client.get_node_info().await;
    println!("{:#?}", node_info);

    Ok(())
}
