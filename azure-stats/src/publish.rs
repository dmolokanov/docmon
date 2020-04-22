use std::time::Duration;

use futures_util::{select, FutureExt, StreamExt};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time,
};

use crate::Client;

pub struct Publisher<D> {
    receiver: UnboundedReceiver<D>,
    client: Client,
    log_name: String,
    interval: Duration,
    batch_size: usize,
}

impl<D> Publisher<D>
where
    D: Serialize + std::fmt::Debug,
{
    pub fn new(client: Client, config: PublisherConfig) -> (Publisher<D>, PublisherHandle<D>) {
        let (log_name, batch_size, interval) = config.into_parts();
        let (sender, receiver) = mpsc::unbounded_channel();

        let publisher = Publisher {
            receiver,
            client,
            log_name,
            batch_size,
            interval,
        };
        let handle = PublisherHandle(sender);

        (publisher, handle)
    }

    pub async fn run(mut self) {
        info!("starting publisher");

        let mut items = Vec::with_capacity(self.batch_size);
        let mut stop_requested = false;

        loop {
            select! {
                closed = collect(&mut self.receiver, &mut items).fuse() => {
                    debug!("collected {} item(s)", items.len());
                    stop_requested = closed;
                },
                _ = time::delay_for(self.interval).fuse() => {
                    debug!("default interval expired");
                }
            }

            if !items.is_empty() {
                info!("sending data: {} item(s)", items.len());
                if let Err(e) = self.client.send(&self.log_name, &items).await {
                    warn!("cannot send data: {}", e);
                } else {
                    info!("successfully sent data");
                    items.clear();
                }
            } else {
                info!("no items to send")
            }

            if stop_requested {
                info!("stopping publisher");
                break;
            }
        }

        info!("publisher stopped");
    }
}

async fn collect<D>(receiver: &mut UnboundedReceiver<D>, items: &mut Vec<D>) -> bool {
    loop {
        if let Some(item) = receiver.next().await {
            debug!("new item available in the channel");
            items.push(item);

            if items.len() >= items.capacity() {
                info!("items batch is full");
                return false;
            }
        } else {
            info!("channel is closed");
            return true;
        }
    }
}

#[derive(Debug, Clone)]
pub struct PublisherHandle<D>(UnboundedSender<D>);

impl<D> PublisherHandle<D>
where
    D: std::fmt::Debug,
{
    pub fn send(&self, data: D) {
        if let Err(e) = self.0.send(data) {
            warn!("Unable to send a message to channel: {:?}", e);
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PublisherConfig {
    log_name: String,
    batch_size: usize,
    interval: usize,
}

impl PublisherConfig {
    pub fn new(log_name: impl Into<String>, batch_size: usize, interval: usize) -> Self {
        Self {
            log_name: log_name.into(),
            batch_size,
            interval,
        }
    }

    pub fn into_parts(self) -> (String, usize, Duration) {
        (
            self.log_name,
            self.batch_size,
            Duration::from_secs(self.interval as u64),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientConfig;

    #[tokio::test]
    async fn it_publishes_data_from_channel() {
        let _ = env_logger::builder().is_test(true).try_init();

        let customer_id = "";
        let shared_key = "";

        let config = ClientConfig::new(customer_id, shared_key);
        let client = Client::new(config).unwrap();

        let config = PublisherConfig::new("StatEntries", 10, 2);
        let (publisher, publisher_handle) = Publisher::new(client, config);
        let task = tokio::spawn(publisher.run());

        let data = serde_json::json!(
            {"DemoField1":"DemoValue1","DemoField2":"DemoValue2"}
        );

        publisher_handle.send(data.clone());
        publisher_handle.send(data.clone());
        publisher_handle.send(data.clone());
        publisher_handle.send(data.clone());
        publisher_handle.send(data.clone());
        publisher_handle.send(data.clone());
        publisher_handle.send(data);

        drop(publisher_handle);

        task.await.unwrap();
    }
}
