use std::net::SocketAddr;
use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use tokio::runtime;

pub use crate::ErrorData;
use crate::TestServer as AsyncTestServer;

#[derive(Debug)]
pub struct TestServer {
    inner: AsyncTestServer,
    thread: Option<JoinHandle<()>>,
}

impl TestServer {
    pub fn new() -> Self {
        let rt = runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();
        let (tx, rx) = mpsc::sync_channel(0);
        let thread = thread::spawn(move || {
            rt.block_on(async move {
                let mut inner = AsyncTestServer::new();
                let waiter = inner.take_shutdown().unwrap();
                tx.send(inner).ok();
                waiter.await.ok();
            });
        });
        let inner = rx.recv().expect("cannot start sync::TestServer");
        Self {
            inner,
            thread: Some(thread),
        }
    }

    pub fn data(&self) -> &Arc<RwLock<ErrorData>> {
        self.inner.data()
    }

    pub fn addr(&self) -> SocketAddr {
        self.inner.addr()
    }

    pub fn start_shutdown(&mut self) {
        self.inner.start_shutdown();
    }

    pub fn shutdown(&mut self) {
        self.start_shutdown();
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }

    pub fn take_shutdown(&mut self) -> Option<JoinHandle<()>> {
        self.thread.take()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}
