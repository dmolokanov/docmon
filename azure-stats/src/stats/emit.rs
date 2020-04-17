use std::future::Future;

use bollard::Docker;
use tokio::{
    sync::oneshot::{self, Receiver, Sender},
    time,
};

use crate::{PublisherHandle, Stats};
use log::info;

pub struct Emitter<F> {
    // sender: Sender<()>,
    // receiver: Receiver<()>,
    shutdown_signal: F,
}

impl<F> Emitter<F>
where
    F: Future<Output = ()>,
{
    pub fn new(
        docker: Docker,
        publisher_handler: PublisherHandle<Stats>,
        shutdown_signal: F,
    ) -> Self {
        // let (sender, receiver) = oneshot::channel();
        // Self { sender, receiver }
        Self { shutdown_signal }
    }

    // pub fn shutdown_handle(&self) -> EmitterShutdownHandler {
    //     EmitterShutdownHandler(self.sender.clone())
    // }

    pub async fn run(self) {
        info!("starting stats emitter");
        loop {
            info!("emit data");
            time::delay_for(std::time::Duration::from_secs(1)).await;
        }
        info!("stopped stats emitter");
    }
}

// #[derive(Debug, Clone)]
// pub struct EmitterShutdownHandler;

// impl EmitterShutdownHandler {
//     pub fn shutdown(self) {}
// }
