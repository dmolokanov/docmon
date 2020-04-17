use std::time::Duration;

use anyhow::{Context, Result};
use azure_stats::{Client, ClientConfig, Collector, Publisher};
use bollard::Docker;
use tokio::time;

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Debug)?;

    let config = ClientConfig::new(customer_id, shared_key, "StatEntries");
    let client = Client::from_config(config).unwrap();

    let publisher = Publisher::new(client);
    let handle = publisher.handle();
    let join_handle = tokio::spawn(publisher.run());

    let docker = Docker::connect_with_unix_defaults()
        .with_context(|| "unable to connect to docker daemon")?;

    let collector = Collector::new(docker, handle.clone());
    tokio::spawn(collector.run());

    time::delay_for(Duration::from_secs(10)).await;
    handle.shutdown();
    join_handle.await?;
    Ok(())
}
