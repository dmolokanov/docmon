use anyhow::{Context, Result};
use bollard::Docker;
use clap::{app_from_crate, crate_authors, crate_description, crate_name, crate_version, Arg};
use docmon::{Client, Collector, Config, Publisher};
use futures_util::{
    future::{self, Either},
    pin_mut, StreamExt,
};
use log::info;
use tokio::signal::unix::{signal, SignalKind};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let config = app_from_crate!()
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a config file")
                .takes_value(true)
                .default_value("config.toml"),
        )
        .get_matches()
        .value_of("config")
        .map(Config::from_file)
        .transpose()?
        .expect("config");

    let (client_config, publisher_config) = config.into_parts();

    let client = Client::new(client_config).unwrap();
    let (publisher, publisher_handle) = Publisher::new(client, publisher_config);
    let join_handle = tokio::spawn(publisher.run());

    let docker = Docker::connect_with_unix_defaults()
        .with_context(|| "unable to connect to docker daemon")?;

    let shutdown_signal = shutdown();
    pin_mut!(shutdown_signal);

    let collector = Collector::new(docker, publisher_handle);
    collector.run(shutdown_signal).await;

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
