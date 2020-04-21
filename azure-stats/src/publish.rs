use log::{debug, info, warn};
use serde::Serialize;
use tokio::sync::mpsc::{self, error::TryRecvError, UnboundedReceiver, UnboundedSender};

use crate::Client;

pub struct Publisher<D> {
    sender: UnboundedSender<Message<D>>,
    receiver: UnboundedReceiver<Message<D>>,
    client: Client,
}

impl<D> Publisher<D>
where
    D: Serialize + std::fmt::Debug,
{
    pub fn new(client: Client) -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver,
            client,
        }
    }

    pub fn handle(&self) -> PublisherHandle<D> {
        PublisherHandle(self.sender.clone())
    }

    pub async fn run(mut self) {
        info!("starting publisher");

        let mut items = Vec::new();
        let mut close = false;
        let mut wait = false;
        loop {
            debug!("staring to collect messages from channel");
            loop {
                match self.receiver.try_recv() {
                    Ok(Message::Data(data)) => {
                        debug!("new item available in the channel");
                        items.push(data);

                        if items.len() >= 100 {
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
                    if let Err(e) = self.client.send(data.into_bytes()).await {
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
                let timeout = std::time::Duration::from_secs(10);
                debug!("waiting default timeout {} sec", timeout.as_secs());
                tokio::time::delay_for(timeout).await;

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

        let config = ClientConfig::new("", "", "StatEntries");
        let client = Client::from_config(config).unwrap();

        let publisher = Publisher::new(client);
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
