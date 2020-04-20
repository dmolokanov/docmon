use anyhow::{Context, Result};
use azure_stats::{Client, ClientConfig, Collector, Publisher};
use bollard::Docker;
use futures_util::{
    future::{self, Either},
    StreamExt,
};
use log::info;
use tokio::signal::unix::{signal, SignalKind};

#[tokio::main]
async fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;

    let customer_id = "";
    let shared_key = "";

    let config = ClientConfig::new(customer_id, shared_key, "StatEntries");
    let client = Client::from_config(config).unwrap();

    let publisher = Publisher::new(client);
    let publisher_handle = publisher.handle();
    let join_handle = tokio::spawn(publisher.run());

    let docker = Docker::connect_with_unix_defaults()
        .with_context(|| "unable to connect to docker daemon")?;

    let shutdown_signal = shutdown();
    let shutdown_signal = Box::pin(shutdown_signal);
    // todo pin on stack instead
    // futures_util::pin_mut!(shutdown_signal);

    let collector = Collector::new(docker, publisher_handle.clone());
    tokio::spawn(collector.run(shutdown_signal)).await?;

    publisher_handle.shutdown();
    join_handle.await?;
    Ok(())
}

async fn shutdown() {
    let mut terminate = signal(SignalKind::terminate()).expect("SIGTERM signal handling failure");
    let mut interrupt = signal(SignalKind::interrupt()).expect("SIGINT signal handling failure");

    match future::select(terminate.next(), interrupt.next()).await {
        Either::Left(_) => info!("SIGTERM received"),
        Either::Right(_) => info!("SIGINT received"),
    };
}
