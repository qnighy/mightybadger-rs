use futures::channel::oneshot;
use hyper::server::Server;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tokio::task::JoinHandle;

pub use crate::data::ErrorData;

mod data;
mod service;
pub mod sync;

#[derive(Debug)]
pub struct TestServer {
    data: Arc<RwLock<ErrorData>>,
    addr: SocketAddr,
    start_shutdown: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl TestServer {
    pub fn new() -> Self {
        let data = Arc::new(RwLock::new(ErrorData::default()));

        let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
        let service = crate::service::Service::new(&data);
        let server = Server::bind(&addr).serve(service);
        let addr = server.local_addr();

        let (tx, rx) = oneshot::channel();
        let server = server.with_graceful_shutdown(async {
            rx.await.ok();
        });
        let task = tokio::spawn(async {
            server.await.unwrap();
        });

        Self {
            data,
            addr,
            start_shutdown: Some(tx),
            task: Some(task),
        }
    }

    pub fn data(&self) -> &Arc<RwLock<ErrorData>> {
        &self.data
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn start_shutdown(&mut self) {
        if let Some(start_shutdown) = self.start_shutdown.take() {
            start_shutdown.send(()).ok();
        }
    }

    pub async fn shutdown(&mut self) {
        self.start_shutdown();
        if let Some(task) = self.task.take() {
            task.await.ok();
        }
    }

    pub fn take_shutdown(&mut self) -> Option<JoinHandle<()>> {
        self.task.take()
    }
}
