use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use tokio::sync::Notify;

#[derive(Debug, Clone)]
pub struct ShutdownSignal {
    pub signal: Arc<Notify>,
    shutdown_triggered: Arc<AtomicBool>,
}

impl ShutdownSignal {
    pub fn new() -> Self {
        Self {
            signal: Arc::new(Notify::new()),
            shutdown_triggered: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn shutdown(&self) {
        self.shutdown_triggered.store(true, Ordering::SeqCst);
        self.signal.notify_waiters();
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown_triggered.load(Ordering::SeqCst)
    }

    pub async fn wait_for_shutdown(&self) {
        self.signal.notified().await;
    }
}
