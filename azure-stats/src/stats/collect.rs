use std::{collections::HashMap, future::Future, time::Duration};

use bollard::{container::ListContainersOptions, Docker};
use futures_util::{
    future::{self, FutureExt},
    pin_mut,
};
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
                    debug!("received a list of {} containers", list.len());
                    // start stats emitter for each new container
                    for container in &list {
                        if !containers.contains_key(&container.id) {
                            // let emitter =
                            //     Emitter::new(self.docker.clone(), self.publisher_handle.clone());
                            // let emitter_handle = emitter.shutdown_handle();
                            // let join_handle = tokio::spawn(emitter.run());

                            let (tx, rx) = oneshot::channel();
                            let a = emit_container_stats(container.id.clone(), rx.map(drop));
                            let join_handle = tokio::spawn(a);

                            containers.insert(container.id.clone(), (tx, join_handle));
                        }
                    }

                    // stop stats emitter for old containers
                    let var_name = containers
                        .keys()
                        .filter(|container_id| {
                            !list.iter().any(|container| container.id == **container_id)
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    // dbg!(&list.iter().any(|container| container.id != **container_id));
                    // dbg!(&var_name);
                    for container_id in var_name {
                        info!("stopping stats emitter for {}", container_id);
                        if let Some((shutdown_handle, join_handle)) =
                            containers.remove(&container_id)
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

        info!("shutting down all stats emitters");
        let (shutdown_handles, join_handles): (Vec<_>, Vec<_>) =
            containers.into_iter().map(|(_, x)| x).unzip();

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
}

pub async fn emit_container_stats<F>(container_id: String, shutdown_signal: F)
where
    F: Future<Output = ()> + Unpin,
{
    info!("starting stats emitter for {}", container_id);
    let id = container_id.clone();

    let emitter = async move {
        loop {
            info!("emit data for {}", id);
            time::delay_for(std::time::Duration::from_secs(1)).await;
        }
    };

    pin_mut!(emitter);

    future::select(emitter, shutdown_signal).await;

    info!("stopped stats emitter for {}", container_id);
}
