use bollard::Docker;

use crate::{PublisherHandle, Stats};

pub struct Emitter;

impl Emitter {
    pub fn new(docker: Docker, publisher_handler: PublisherHandle<Stats>) -> Self {
        Self
    }

    pub fn shutdown_handle(&self) -> EmitterShutdownHandler {
        EmitterShutdownHandler
    }

    pub async fn run(self) {}
}

#[derive(Debug, Clone)]
pub struct EmitterShutdownHandler;

impl EmitterShutdownHandler {
    pub fn shutdown(self) {}
}
