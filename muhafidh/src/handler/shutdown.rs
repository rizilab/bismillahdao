use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Debug, Clone)]
pub struct ShutdownSignal {
    pub signal: Arc<Notify>,
}

impl ShutdownSignal {
    pub fn new() -> Self {
        Self {
            signal: Arc::new(Notify::new()),
        }
    }
    
    pub fn shutdown(&self) {
        self.signal.notify_waiters();
    }
    
    pub async fn wait_for_shutdown(&self) {
        self.signal.notified().await;
    }
}