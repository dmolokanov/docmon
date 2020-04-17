use std::{collections::HashMap, time::Duration};

use bollard::{container::ListContainersOptions, Docker};
use futures_util::future;
use log::{error, info, warn};
use tokio::{
    sync::oneshot::{self, Receiver, Sender},
    time,
};

use crate::{stats::emit::Emitter, PublisherHandle, Stats};

pub struct Collector {
    docker: Docker,
    publisher_handle: PublisherHandle<Stats>,
    sender: Sender<()>,
    receiver: Receiver<()>,
}

impl Collector {
    pub fn new(docker: Docker, handle: PublisherHandle<Stats>) -> Self {
        let (sender, receiver) = oneshot::channel();
        Self {
            docker,
            publisher_handle: handle,
            sender,
            receiver,
        }
    }

    pub async fn run(mut self) {
        info!("starting stats collector");
        let version = self.docker.version().await.unwrap();
        info!("docker daemon version: {:?}", version.version);

        let mut containers = HashMap::new();

        loop {
            if self.receiver.try_recv().is_ok() {
                info!("stop requested");
                break;
            }

            let options = ListContainersOptions::<String>::default();
            match self.docker.list_containers(Some(options)).await {
                Ok(list) => {
                    // start stats emitter for each new container
                    for container in &list {
                        if !containers.contains_key(&container.id) {
                            info!("starting stats emitter for {}", container.id);
                            let emitter =
                                Emitter::new(self.docker.clone(), self.publisher_handle.clone());
                            let emitter_handle = emitter.shutdown_handle();
                            let join_handle = tokio::spawn(emitter.run());

                            containers.insert(container.id.clone(), (emitter_handle, join_handle));
                        }
                    }

                    // stop stats emitter for old containers
                    for container_id in containers
                        .keys()
                        .filter(|container_id| {
                            list.iter().any(|container| container.id == **container_id)
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                    {
                        info!("stopping stats emitter for {}", container_id);
                        if let Some((emitter_handle, join_handle)) =
                            containers.remove(&container_id)
                        {
                            emitter_handle.shutdown();

                            if let Err(e) = join_handle.await {
                                warn!(
                                    "Error occurred while stopping stats emitter for {}. {:?}",
                                    container_id, e
                                );
                            }
                        }
                    }
                }
                Err(e) => error!("Error occurred when containers list requested: {:?}", e),
            }

            time::delay_for(Duration::from_secs(1)).await;
        }

        info!("shutting down all stats emitters");
        let (shutdown_handles, join_handles): (Vec<_>, Vec<_>) =
            containers.into_iter().map(|(_, x)| x).unzip();

        for shutdown_handle in shutdown_handles {
            shutdown_handle.shutdown();
        }

        future::join_all(join_handles).await;

        info!("stats collector stopped");
    }
}
