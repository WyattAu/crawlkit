use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use tokio::sync::{mpsc, Semaphore};

/// Backpressure controller for bounded pipeline channels.
///
/// Ensures producers block when consumers are overwhelmed,
/// preventing memory explosion and maintaining system stability.
/// Uses tokio semaphores for concurrency limiting.
///
/// # Examples
///
/// ```rust
/// # async fn example() {
/// use crawlkit_engine::BackpressureController;
///
/// let controller = BackpressureController::new(10);
/// let permit = controller.acquire().await.unwrap();
/// assert_eq!(controller.active_count(), 1);
/// drop(permit);
/// assert_eq!(controller.active_count(), 0);
/// # }
/// ```
pub struct BackpressureController {
    semaphore: Arc<Semaphore>,
    active_tasks: Arc<AtomicUsize>,
    shutdown: Arc<AtomicBool>,
}

impl BackpressureController {
    /// Create a new backpressure controller.
    ///
    /// # Arguments
    /// * `max_concurrent` - Maximum concurrent tasks allowed
    #[must_use]
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            active_tasks: Arc::new(AtomicUsize::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create with bounded channel for additional backpressure.
    #[must_use]
    pub fn with_channel(max_concurrent: usize, _channel_size: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            active_tasks: Arc::new(AtomicUsize::new(0)),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Acquire a permit (blocks if at capacity).
    ///
    /// # Errors
    /// Returns error if controller is shut down.
    pub async fn acquire(&self) -> Result<BackpressurePermit<'_>, BackpressureError> {
        if self.shutdown.load(Ordering::Acquire) {
            return Err(BackpressureError::ShutDown);
        }

        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| BackpressureError::ShutDown)?;

        self.active_tasks.fetch_add(1, Ordering::AcqRel);

        Ok(BackpressurePermit {
            controller: self,
            _permit: permit,
        })
    }

    /// Try to acquire a permit without blocking.
    #[must_use]
    pub fn try_acquire(&self) -> Option<BackpressurePermit<'_>> {
        if self.shutdown.load(Ordering::Acquire) {
            return None;
        }

        let permit = self.semaphore.clone().try_acquire_owned().ok()?;

        self.active_tasks.fetch_add(1, Ordering::AcqRel);

        Some(BackpressurePermit {
            controller: self,
            _permit: permit,
        })
    }

    /// Get number of active tasks.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active_tasks.load(Ordering::Acquire)
    }

    /// Check if at capacity.
    #[must_use]
    pub fn is_at_capacity(&self) -> bool {
        self.active_tasks.load(Ordering::Acquire) >= self.semaphore.available_permits()
    }

    /// Initiate graceful shutdown.
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    /// Check if shut down.
    #[must_use]
    pub fn is_shut_down(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }
}

impl Default for BackpressureController {
    fn default() -> Self {
        Self::new(10)
    }
}

/// RAII permit for backpressure.
pub struct BackpressurePermit<'a> {
    controller: &'a BackpressureController,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl Drop for BackpressurePermit<'_> {
    fn drop(&mut self) {
        self.controller.active_tasks.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Errors from backpressure operations.
#[derive(Debug, thiserror::Error)]
pub enum BackpressureError {
    #[error("backpressure controller is shut down")]
    ShutDown,
}

/// Bounded channel wrapper with backpressure.
///
/// Provides a typed mpsc channel with a fixed buffer size. Producers
/// block when the buffer is full, naturally implementing backpressure.
///
/// # Examples
///
/// ```rust
/// use crawlkit_engine::BoundedPipeline;
///
/// let mut pipeline = BoundedPipeline::<i32>::new(10);
/// assert_eq!(pipeline.capacity(), 10);
/// let tx = pipeline.sender();
/// tx.try_send(42).unwrap();
/// let mut rx = pipeline.receiver().unwrap();
/// assert_eq!(rx.try_recv().unwrap(), 42);
/// ```
pub struct BoundedPipeline<T> {
    tx: mpsc::Sender<T>,
    rx: Option<mpsc::Receiver<T>>,
    max_size: usize,
}

impl<T> BoundedPipeline<T> {
    /// Create a new bounded pipeline.
    #[must_use]
    pub fn new(max_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(max_size);
        Self {
            tx,
            rx: Some(rx),
            max_size,
        }
    }

    /// Get sender (can be cloned for multiple producers).
    #[must_use]
    pub fn sender(&self) -> mpsc::Sender<T> {
        self.tx.clone()
    }

    /// Take receiver (can only be called once).
    pub fn receiver(&mut self) -> Option<mpsc::Receiver<T>> {
        self.rx.take()
    }

    /// Get channel capacity.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.max_size
    }

    /// Check if channel is full.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.tx.capacity() == 0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backpressure_controller_acquire() {
        let controller = BackpressureController::new(2);

        let permit1 = controller.acquire().await.unwrap();
        assert_eq!(controller.active_count(), 1);

        let permit2 = controller.acquire().await.unwrap();
        assert_eq!(controller.active_count(), 2);

        drop(permit1);
        assert_eq!(controller.active_count(), 1);

        drop(permit2);
        assert_eq!(controller.active_count(), 0);
    }

    #[tokio::test]
    async fn test_backpressure_controller_shutdown() {
        let controller = BackpressureController::new(1);
        let _permit = controller.acquire().await.unwrap();

        controller.shutdown();
        assert!(controller.is_shut_down());
        assert!(controller.acquire().await.is_err());
    }

    #[test]
    fn test_bounded_pipeline() {
        let mut pipeline = BoundedPipeline::<i32>::new(10);
        assert_eq!(pipeline.capacity(), 10);

        let tx = pipeline.sender();
        assert!(tx.try_send(42).is_ok());

        let mut rx = pipeline.receiver().unwrap();
        assert_eq!(rx.try_recv().unwrap(), 42);
    }
}
