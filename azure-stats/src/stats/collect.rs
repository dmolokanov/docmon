use std::{collections::HashMap, future::Future, time::Duration};

use bollard::{container::ListContainersOptions, Docker};
use futures_util::{
    future::{self, FutureExt},
    select,
};
use log::{debug, error, info, warn};
use tokio::{
    sync::oneshot::{self, Sender},
    task::JoinHandle,
    time,
};

use super::emit::Emitter;
use crate::{PublisherHandle, Stats};

pub struct Collector {
    docker: Docker,
    publisher_handle: PublisherHandle<Stats>,
    containers: HashMap<String, (Sender<()>, JoinHandle<()>)>,
}

impl Collector {
    pub fn new(docker: Docker, handle: PublisherHandle<Stats>) -> Self {
        Self {
            docker,
            publisher_handle: handle,
            containers: HashMap::new(),
        }
    }

    pub async fn run<F>(mut self, shutdown_signal: F)
    where
        F: Future<Output = ()> + Unpin,
    {
        info!("starting stats collector");

        select! {
            _ = self.collect().fuse() => {
                error!("stats collection finished unexpectedly");
            }
            _ = shutdown_signal.fuse() => {
                info!("shutdown signal received. stopping collector");
            }
        }

        info!("shutting down all stats emitters");
        let (shutdown_handles, join_handles): (Vec<_>, Vec<_>) =
            self.containers.into_iter().map(|(_, x)| x).unzip();

        for shutdown_handle in shutdown_handles {
            if let Err(e) = shutdown_handle.send(()) {
                warn!(
                    "error occurred when sending shutdown signal to stats emitter: {:?}",
                    e
                );
            }
        }

        future::join_all(join_handles).await;

        info!("stats collector stopped");
    }

    async fn collect(&mut self) {
        loop {
            let options = ListContainersOptions::<String>::default();
            match self.docker.list_containers(Some(options)).await {
                Ok(list) => {
                    debug!("received a list of {} containers", list.len());
                    // start stats emitter for each new container
                    for container in &list {
                        if !self.containers.contains_key(&container.id) {
                            let (tx, rx) = oneshot::channel();
                            let emitter = Emitter::new(
                                container.id.clone(),
                                self.docker.clone(),
                                self.publisher_handle.clone(),
                            );
                            let join_handle = tokio::spawn(emitter.run(rx.map(drop)));

                            self.containers
                                .insert(container.id.clone(), (tx, join_handle));
                        }
                    }

                    // stop stats emitter for old containers
                    for container_id in self
                        .containers
                        .keys()
                        .filter(|container_id| {
                            !list.iter().any(|container| container.id == **container_id)
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                    {
                        info!("stopping stats emitter for {}", container_id);
                        if let Some((shutdown_handle, join_handle)) =
                            self.containers.remove(&container_id)
                        {
                            if let Err(e) = shutdown_handle.send(()) {
                                warn!("error occurred when sending shutdown signal to stats emitter: {:?}", e);
                            }

                            if let Err(e) = join_handle.await {
                                warn!("error occurred while stopping stats emitter: {:?}", e);
                            }
                        }
                    }
                }
                Err(e) => error!("error occurred when containers list requested: {:?}", e),
            }

            time::delay_for(Duration::from_secs(1)).await;
        }
    }
}
