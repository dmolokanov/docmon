use std::{convert::TryInto, future::Future};

use bollard::Docker;
use futures_util::{future, pin_mut, StreamExt};
use log::{info, trace, warn};

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
        let container_id = self.container_id.clone();

        let emitter = async move {
            loop {
                let options = bollard::container::StatsOptions { stream: true };
                let mut stats = self.docker.stats(&self.container_id, Some(options));

                while let Some(stats) = stats.next().await {
                    match stats {
                        Ok(stats) => {
                            trace!("emit docker stats for {}", self.container_id);
                            if let Ok(stats) = stats.try_into() {
                                self.publisher_handle.send(stats);
                            }
                        }
                        Err(e) => warn!(
                            "unable to read docker stats for {}. {:?}",
                            self.container_id, e
                        ),
                    }
                }
            }
        };

        pin_mut!(emitter);

        future::select(emitter, shutdown_signal).await;

        info!("stopped stats emitter for {}", container_id);
    }
}
