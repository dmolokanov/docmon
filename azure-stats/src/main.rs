use std::time::Duration;

use anyhow::{Context, Result};
use azure_stats::{Client, ClientConfig, Collector, Publisher};
use bollard::Docker;
use tokio::time;

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;

    let customer_id = "";
    let shared_key = "";

    let config = ClientConfig::new(customer_id, shared_key, "StatEntries");
    let client = Client::from_config(config).unwrap();

    let publisher = Publisher::new(client);
    let publisher_handle = publisher.handle();
    // let join_handle = tokio::spawn(publisher.run());

    let docker = Docker::connect_with_unix_defaults()
        .with_context(|| "unable to connect to docker daemon")?;

    let collector = Collector::new(docker, publisher_handle.clone());
    tokio::spawn(collector.run()).await?;

    time::delay_for(Duration::from_secs(10)).await;
    publisher_handle.shutdown();
    // join_handle.await?;
    Ok(())
}
