use std::future::Future;

use bollard::Docker;
use futures_util::{future, pin_mut};
use log::info;
use tokio::time;

use crate::{PublisherHandle, Stats};

pub struct Emitter {
    container_id: String,
    docker: Docker,
    publisher_handle: PublisherHandle<Stats>,
}

impl Emitter {
    pub fn new(
        container_id: String,
        docker: Docker,
        publisher_handle: PublisherHandle<Stats>,
    ) -> Self {
        Self {
            container_id,
            docker,
            publisher_handle,
        }
    }

    pub async fn run<F>(self, shutdown_signal: F)
    where
        F: Future<Output = ()> + Unpin,
    {
        info!("starting stats emitter for {}", self.container_id);
        let id = self.container_id.clone();

        let emitter = async move {
            loop {
                info!("emit data for {}", id);
                time::delay_for(std::time::Duration::from_secs(1)).await;
            }
        };

        pin_mut!(emitter);

        future::select(emitter, shutdown_signal).await;

        info!("stopped stats emitter for {}", self.container_id);
    }
}
