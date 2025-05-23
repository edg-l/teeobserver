use std::collections::HashMap;

use reqwest::Client;
use tracing::debug;

use crate::structures::{ServerList, ServerListMap};

pub async fn fetch_master(client: &Client) -> Result<ServerListMap, reqwest::Error> {
    debug!("making request to master");
    let res: ServerList = client
        .get("https://master1.ddnet.org/ddnet/15/servers.json")
        .send()
        .await?
        .json()
        .await?;
    debug!("got {} servers", res.servers.len());
    let servers: HashMap<_, _> = res
        .servers
        .into_iter()
        .filter_map(|x| x.addresses.first().cloned().map(|address| (address, x)))
        .collect();
    Ok(ServerListMap { servers })
}
