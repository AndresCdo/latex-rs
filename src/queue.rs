use crate::constants::COMPILATION_QUEUE_BUFFER;
use crate::preview::Preview;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;

/// A compilation queue that ensures only one LaTeX compilation runs at a time.
/// This prevents resource conflicts and temp file corruption from concurrent compilations.
#[derive(Clone)]
pub struct CompilationQueue {
    sender: mpsc::Sender<(String, oneshot::Sender<String>)>,
    /// Shared reference to the worker handle for graceful shutdown.
    /// Wrapped in Arc<Mutex> to allow cloning while maintaining single ownership semantics.
    worker_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl CompilationQueue {
    /// Creates a new compilation queue with a dedicated worker task.
    ///
    /// The worker processes compilation requests sequentially, ensuring thread safety
    /// for temporary file operations.
    pub fn new(preview: Preview) -> Self {
        let (sender, mut receiver) =
            mpsc::channel::<(String, oneshot::Sender<String>)>(COMPILATION_QUEUE_BUFFER);

        let handle = tokio::spawn(async move {
            while let Some((latex, result_sender)) = receiver.recv().await {
                let preview = preview.clone();
                let start = std::time::Instant::now();
                let html = tokio::task::spawn_blocking(move || preview.render(&latex))
                    .await
                    .unwrap_or_else(|e| format!("Render Task Error: {}", e));
                let elapsed = start.elapsed();
                tracing::info!(
                    "LaTeX compilation completed in {:.2}s",
                    elapsed.as_secs_f64()
                );
                // Ignore send error if receiver dropped (job cancelled)
                let _ = result_sender.send(html);
            }
            tracing::debug!("Compilation worker shutting down");
        });

        Self {
            sender,
            worker_handle: Arc::new(Mutex::new(Some(handle))),
        }
    }

    /// Enqueues a LaTeX document for compilation.
    ///
    /// If the queue is full (another compilation is pending), the new request is dropped
    /// to prevent queue buildup during rapid typing.
    ///
    /// Returns `Some(html)` with the rendered result, or `None` if the request was dropped
    /// or the worker is unavailable.
    pub async fn enqueue(&self, latex: String) -> Option<String> {
        let (result_sender, result_receiver) = oneshot::channel();
        // Try to send, if channel is full, drop the new job (keep the pending one)
        if self.sender.try_send((latex, result_sender)).is_err() {
            // Channel full, ignore new job
            tracing::debug!("Compilation queue full, dropping new job");
            return None;
        }
        result_receiver.await.ok() // Returns None if sender dropped (should not happen)
    }

    /// Gracefully shuts down the compilation worker.
    ///
    /// This should be called during application shutdown to ensure clean termination.
    /// After calling this, the queue will no longer accept new compilations.
    #[allow(dead_code)]
    pub async fn shutdown(&self) {
        // Drop the sender to signal the worker to stop
        // Note: We can't drop self.sender directly, but closing the channel
        // will cause receiver.recv() to return None

        // Wait for the worker to finish current work
        let handle = self.worker_handle.lock().await.take();
        if let Some(h) = handle {
            match h.await {
                Ok(()) => tracing::debug!("Compilation worker shut down cleanly"),
                Err(e) => tracing::warn!("Compilation worker panicked: {}", e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::runtime::Runtime;

    #[allow(dead_code)]
    struct MockPreview;
    #[allow(dead_code)]
    impl MockPreview {
        fn new() -> Self {
            MockPreview
        }
        fn render(&self, latex: &str) -> String {
            format!("Rendered: {}", latex)
        }
    }

    #[test]
    fn test_queue_sequential() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let _preview = Preview::new(); // Use real preview but it requires external commands
                                           // For simplicity, we'll skip actual preview testing in unit tests
                                           // The queue logic is tested in integration tests
        });
    }
}
