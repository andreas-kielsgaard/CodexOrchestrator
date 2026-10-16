//! Owned executor for blocking application workers, including non-async command callers.
use std::sync::OnceLock;
#[derive(Default)]
pub struct WorkerExecutor(OnceLock<tokio::runtime::Runtime>);
impl WorkerExecutor {
    pub fn spawn_blocking<F, R>(&self, work: F) -> tokio::task::JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.0
            .get_or_init(|| {
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .expect("initialize application workers")
            })
            .spawn_blocking(work)
    }
}
impl Drop for WorkerExecutor {
    fn drop(&mut self) {
        if let Some(runtime) = self.0.take() {
            runtime.shutdown_background();
        }
    }
}
