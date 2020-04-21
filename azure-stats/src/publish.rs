use std::time::Duration;

use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::mpsc::{self, error::TryRecvError, UnboundedReceiver, UnboundedSender},
    time,
};

use crate::Client;

pub struct Publisher<D> {
    sender: UnboundedSender<Message<D>>,
    receiver: UnboundedReceiver<Message<D>>,
    client: Client,
    log_name: String,
    interval: Duration,
    batch_size: usize,
}

impl<D> Publisher<D>
where
    D: Serialize + std::fmt::Debug,
{
    pub fn new(client: Client, config: PublisherConfig) -> Self {
        let (log_name, batch_size, interval) = config.into_parts();
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver,
            client,
            log_name,
            batch_size,
            interval,
        }
    }

    pub fn handle(&self) -> PublisherHandle<D> {
        PublisherHandle(self.sender.clone())
    }

    pub async fn run(mut self) {
        info!("starting publisher");

        let mut items = Vec::with_capacity(self.batch_size);
        let mut close = false;
        let mut wait = false;
        loop {
            debug!("staring to collect messages from channel");
            loop {
                match self.receiver.try_recv() {
                    Ok(Message::Data(data)) => {
                        debug!("new item available in the channel");
                        items.push(data);

                        if items.len() >= items.capacity() {
                            info!("accumulated too many items {}. try to send", items.len());
                            break;
                        }
                    }
                    Ok(Message::Stop) => {
                        info!("stop requested");
                        close = true;
                        break;
                    }
                    Err(TryRecvError::Empty) => {
                        debug!("no messages in the channel. abort");
                        wait = true;
                        break;
                    }
                    Err(TryRecvError::Closed) => {
                        warn!("channel closed unexpectedly. abort");
                        close = true;
                        break;
                    }
                }
            }

            // send all messages to log analytics if any
            if !items.is_empty() {
                info!("sending data: {} item(s)", items.len());
                if let Ok(data) = serde_json::to_string(&items) {
                    if let Err(e) = self.client.send(&self.log_name, data.into_bytes()).await {
                        warn!("cannot send data: {}", e);
                    } else {
                        info!("successfully sent data");
                        items.clear();
                    }
                }
            } else {
                info!("no items to send")
            }

            if close {
                info!("stopping publisher");
                break;
            }

            if wait {
                debug!("waiting {} sec", self.interval.as_secs());
                time::delay_for(self.interval).await;

                wait = false;
            }
        }

        info!("publisher stopped");
    }
}

#[derive(Debug, Clone)]
pub struct PublisherHandle<D>(UnboundedSender<Message<D>>);

impl<D> PublisherHandle<D>
where
    D: std::fmt::Debug,
{
    pub fn send(&self, data: D) {
        self.send_message(Message::Data(data));
    }

    pub fn shutdown(self) {
        self.send_message(Message::Stop);
    }

    fn send_message(&self, message: Message<D>) {
        let message_type = match message {
            Message::Stop => "stop",
            Message::Data(_) => "data",
        };

        if let Err(e) = self.0.send(message) {
            warn!(
                "Unable to send a {} message to channel: {:?}",
                message_type, e
            );
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

#[derive(Debug)]
pub enum Message<D> {
    Stop,
    Data(D),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ClientConfig;

    #[tokio::test]
    async fn it_publishes_data_from_channel() {
        let _ = env_logger::builder().is_test(true).try_init();

        let config = ClientConfig::new("", "");
        let client = Client::new(config).unwrap();

        let config = PublisherConfig::new("StatEntries", 100, 10);
        let publisher = Publisher::new(client, config);
        let publisher_handle = publisher.handle();
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

        publisher_handle.shutdown();
        task.await.unwrap();
    }
}
