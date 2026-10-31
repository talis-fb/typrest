use std::any::Any;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use typrest_core::{
    Admission, CompilationError, CompileJob, Document, ExecutionError, RenderExecutor,
};

pub const MAX_CONCURRENT_RENDERS_LIMIT: usize = 256;
pub const MAX_QUEUED_RENDERS_LIMIT: usize = 1024;
pub const DEFAULT_QUEUE_FACTOR: usize = 2;

const UNKNOWN_PANIC_MESSAGE: &str = "compilation panicked";

/// Limits applied to request-time compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderLimits {
    pub max_concurrent_renders: usize,
    pub max_queued_renders: usize,
}

impl RenderLimits {
    pub fn new(max_concurrent_renders: usize, max_queued_renders: usize) -> Self {
        Self {
            max_concurrent_renders,
            max_queued_renders,
        }
    }
}

impl Default for RenderLimits {
    fn default() -> Self {
        let max_concurrent_renders = default_max_concurrent_renders();
        Self::new(
            max_concurrent_renders,
            default_max_queued_renders(max_concurrent_renders),
        )
    }
}

pub fn default_max_concurrent_renders() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .clamp(1, MAX_CONCURRENT_RENDERS_LIMIT)
}

pub fn default_max_queued_renders(max_concurrent_renders: usize) -> usize {
    (max_concurrent_renders * DEFAULT_QUEUE_FACTOR).min(MAX_QUEUED_RENDERS_LIMIT)
}

/// A point-in-time view of the executor, for gauges and shutdown draining.
#[allow(dead_code)] // consumed by the render-metrics and operational-limits plans
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutorStats {
    pub max_concurrent: usize,
    pub max_queued: usize,
    pub in_flight: usize,
    pub queued: usize,
}

/// Runs compilation jobs on Tokio blocking threads behind a bounded FIFO admission queue.
///
/// At most `max_concurrent_renders` jobs run at once. At most `max_queued_renders` more
/// `Admission::Request` jobs wait for a slot; further requests fail with
/// `ExecutionError::Saturated`. `Admission::Background` jobs wait for a slot and are never
/// rejected, but they count toward the admitted total while they wait or run.
pub struct BlockingRenderExecutor {
    limits: RenderLimits,
    semaphore: Arc<Semaphore>,
    admitted: Arc<AtomicUsize>,
}

impl BlockingRenderExecutor {
    pub fn new(limits: RenderLimits) -> Self {
        Self {
            limits,
            semaphore: Arc::new(Semaphore::new(limits.max_concurrent_renders)),
            admitted: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[allow(dead_code)] // consumed by the render-metrics and operational-limits plans
    pub fn stats(&self) -> ExecutorStats {
        let in_flight = self.limits.max_concurrent_renders - self.semaphore.available_permits();
        let admitted = self.admitted.load(Ordering::SeqCst);
        ExecutorStats {
            max_concurrent: self.limits.max_concurrent_renders,
            max_queued: self.limits.max_queued_renders,
            in_flight,
            queued: admitted.saturating_sub(in_flight),
        }
    }

    fn admit(&self, admission: Admission) -> Result<AdmissionGuard, ExecutionError> {
        match admission {
            Admission::Request => {
                let capacity = self.limits.max_concurrent_renders + self.limits.max_queued_renders;
                AdmissionGuard::try_admit(&self.admitted, capacity).ok_or(ExecutionError::Saturated)
            }
            Admission::Background => Ok(AdmissionGuard::admit(&self.admitted)),
        }
    }
}

#[async_trait]
impl RenderExecutor for BlockingRenderExecutor {
    async fn execute(
        &self,
        admission: Admission,
        job: CompileJob,
    ) -> Result<Result<Document, CompilationError>, ExecutionError> {
        // Dropping this future while it waits releases the guard and so the queue position.
        let guard = self.admit(admission)?;
        let permit = Arc::clone(&self.semaphore)
            .acquire_owned()
            .await
            .map_err(|_| ExecutionError::Aborted {
                message: "render executor is closed".to_owned(),
            })?;

        // The guard and permit move into the blocking task, so an admitted compilation keeps its
        // slot until it ends even when this future is dropped.
        let handle = tokio::task::spawn_blocking(move || run_job(job, guard, permit));
        match handle.await {
            Ok(result) => Ok(result),
            Err(error) => Err(ExecutionError::Aborted {
                message: match error.try_into_panic() {
                    Ok(payload) => panic_message(payload.as_ref()),
                    Err(error) => error.to_string(),
                },
            }),
        }
    }
}

fn run_job(
    job: CompileJob,
    _guard: AdmissionGuard,
    _permit: OwnedSemaphorePermit,
) -> Result<Document, CompilationError> {
    job()
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| UNKNOWN_PANIC_MESSAGE.to_owned())
}

/// Holds one admitted position, queued or running, until it is dropped.
struct AdmissionGuard {
    admitted: Arc<AtomicUsize>,
}

impl AdmissionGuard {
    fn try_admit(admitted: &Arc<AtomicUsize>, capacity: usize) -> Option<Self> {
        admitted
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                (count < capacity).then_some(count + 1)
            })
            .ok()
            .map(|_| Self {
                admitted: Arc::clone(admitted),
            })
    }

    fn admit(admitted: &Arc<AtomicUsize>) -> Self {
        admitted.fetch_add(1, Ordering::SeqCst);
        Self {
            admitted: Arc::clone(admitted),
        }
    }
}

impl Drop for AdmissionGuard {
    fn drop(&mut self) {
        self.admitted.fetch_sub(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use bytes::Bytes;
    use typrest_core::{
        Admission, CompilationError, CompileJob, Document, ExecutionError, RenderExecutor,
    };

    use super::{BlockingRenderExecutor, RenderLimits};

    fn executor(max_concurrent: usize, max_queued: usize) -> Arc<BlockingRenderExecutor> {
        Arc::new(BlockingRenderExecutor::new(RenderLimits::new(
            max_concurrent,
            max_queued,
        )))
    }

    fn document() -> Document {
        Document::new(Bytes::from_static(b"%PDF-1.4 mock"))
    }

    fn quick_job() -> CompileJob {
        Box::new(|| Ok(document()))
    }

    /// A job that signals when it starts and then blocks until the returned sender is used.
    fn gated_job() -> (CompileJob, mpsc::Receiver<()>, mpsc::Sender<()>) {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel::<()>();
        let job: CompileJob = Box::new(move || {
            started_tx.send(()).unwrap();
            let _ = release_rx.recv();
            Ok(document())
        });
        (job, started_rx, release_tx)
    }

    async fn wait_until(mut condition: impl FnMut() -> bool) {
        for _ in 0..2000 {
            if condition() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        panic!("condition was not reached");
    }

    async fn received(receiver: mpsc::Receiver<()>) -> mpsc::Receiver<()> {
        tokio::task::spawn_blocking(move || {
            receiver.recv().unwrap();
            receiver
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn runs_a_job_and_returns_its_result() {
        let executor = executor(1, 0);

        let result = executor.execute(Admission::Request, quick_job()).await;

        assert!(result.unwrap().unwrap().bytes().starts_with(b"%PDF"));
        assert_eq!(executor.stats().in_flight, 0);
        assert_eq!(executor.stats().queued, 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn in_flight_never_exceeds_max_concurrent() {
        let executor = executor(2, 16);
        let running = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();

        for _ in 0..10 {
            let executor = executor.clone();
            let running = running.clone();
            let peak = peak.clone();
            tasks.push(tokio::spawn(async move {
                let job: CompileJob = Box::new(move || {
                    let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(5));
                    running.fetch_sub(1, Ordering::SeqCst);
                    Ok(document())
                });
                executor.execute(Admission::Request, job).await
            }));
        }
        for task in tasks {
            task.await.unwrap().unwrap().unwrap();
        }

        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn rejects_requests_beyond_slots_and_queue() {
        let executor = executor(1, 1);
        let (running_job, started, release) = gated_job();
        let running = tokio::spawn({
            let executor = executor.clone();
            async move { executor.execute(Admission::Request, running_job).await }
        });
        let started = received(started).await;
        let queued = tokio::spawn({
            let executor = executor.clone();
            async move { executor.execute(Admission::Request, quick_job()).await }
        });
        wait_until(|| executor.stats().queued == 1).await;

        let rejected = executor.execute(Admission::Request, quick_job()).await;

        assert!(matches!(rejected, Err(ExecutionError::Saturated)));
        release.send(()).unwrap();
        running.await.unwrap().unwrap().unwrap();
        queued.await.unwrap().unwrap().unwrap();
        drop(started);
        assert_eq!(executor.stats().in_flight, 0);
        assert_eq!(executor.stats().queued, 0);
    }

    #[tokio::test]
    async fn zero_queue_rejects_when_all_slots_are_busy() {
        let executor = executor(1, 0);
        let (job, started, release) = gated_job();
        let running = tokio::spawn({
            let executor = executor.clone();
            async move { executor.execute(Admission::Request, job).await }
        });
        let _started = received(started).await;

        let rejected = executor.execute(Admission::Request, quick_job()).await;

        assert!(matches!(rejected, Err(ExecutionError::Saturated)));
        release.send(()).unwrap();
        running.await.unwrap().unwrap().unwrap();
    }

    #[tokio::test]
    async fn dropping_a_queued_request_frees_its_queue_position() {
        let executor = executor(1, 1);
        let (job, started, release) = gated_job();
        let running = tokio::spawn({
            let executor = executor.clone();
            async move { executor.execute(Admission::Request, job).await }
        });
        let _started = received(started).await;
        let queued = tokio::spawn({
            let executor = executor.clone();
            async move { executor.execute(Admission::Request, quick_job()).await }
        });
        wait_until(|| executor.stats().queued == 1).await;

        queued.abort();
        let _ = queued.await;
        wait_until(|| executor.stats().queued == 0).await;
        let replacement = tokio::spawn({
            let executor = executor.clone();
            async move { executor.execute(Admission::Request, quick_job()).await }
        });
        wait_until(|| executor.stats().queued == 1).await;

        release.send(()).unwrap();
        running.await.unwrap().unwrap().unwrap();
        replacement.await.unwrap().unwrap().unwrap();
    }

    #[tokio::test]
    async fn dropping_an_admitted_request_keeps_the_slot_until_the_job_ends() {
        let executor = executor(1, 0);
        let (job, started, release) = gated_job();
        let running = tokio::spawn({
            let executor = executor.clone();
            async move { executor.execute(Admission::Request, job).await }
        });
        let _started = received(started).await;

        running.abort();
        let _ = running.await;

        assert_eq!(executor.stats().in_flight, 1);
        let rejected = executor.execute(Admission::Request, quick_job()).await;
        assert!(matches!(rejected, Err(ExecutionError::Saturated)));
        release.send(()).unwrap();
        wait_until(|| executor.stats().in_flight == 0).await;
        executor
            .execute(Admission::Request, quick_job())
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn waiters_are_admitted_in_first_come_order() {
        let executor = executor(1, 8);
        let order = Arc::new(Mutex::new(Vec::new()));
        let (job, started, release) = gated_job();
        let running = tokio::spawn({
            let executor = executor.clone();
            async move { executor.execute(Admission::Request, job).await }
        });
        let _started = received(started).await;

        let mut waiters = Vec::new();
        for index in 0..4 {
            let task_executor = executor.clone();
            let order = order.clone();
            waiters.push(tokio::spawn(async move {
                let job: CompileJob = Box::new(move || {
                    order.lock().unwrap().push(index);
                    Ok(document())
                });
                task_executor.execute(Admission::Request, job).await
            }));
            wait_until(|| executor.stats().queued == index + 1).await;
        }
        release.send(()).unwrap();
        running.await.unwrap().unwrap().unwrap();
        for waiter in waiters {
            waiter.await.unwrap().unwrap().unwrap();
        }

        assert_eq!(*order.lock().unwrap(), [0, 1, 2, 3]);
    }

    #[tokio::test]
    async fn background_jobs_wait_and_are_never_rejected() {
        let executor = executor(1, 0);
        let (job, started, release) = gated_job();
        let running = tokio::spawn({
            let executor = executor.clone();
            async move { executor.execute(Admission::Request, job).await }
        });
        let _started = received(started).await;
        let background = tokio::spawn({
            let executor = executor.clone();
            async move { executor.execute(Admission::Background, quick_job()).await }
        });
        wait_until(|| executor.stats().queued == 1).await;

        release.send(()).unwrap();
        running.await.unwrap().unwrap().unwrap();

        background.await.unwrap().unwrap().unwrap();
    }

    #[tokio::test]
    async fn a_panicking_job_aborts_and_releases_its_slot() {
        let executor = executor(1, 0);
        let job: CompileJob = Box::new(|| panic!("typst exploded"));

        let result = executor.execute(Admission::Request, job).await;

        match result {
            Err(ExecutionError::Aborted { message }) => assert_eq!(message, "typst exploded"),
            other => panic!("expected an aborted execution, got {other:?}"),
        }
        assert_eq!(executor.stats().in_flight, 0);
        assert_eq!(executor.stats().queued, 0);
        executor
            .execute(Admission::Request, quick_job())
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn compilation_errors_pass_through_unchanged() {
        let executor = executor(1, 0);
        let job: CompileJob = Box::new(|| Err(CompilationError::failed("bad template", Some(3))));

        let result = executor.execute(Admission::Request, job).await.unwrap();

        assert_eq!(result.unwrap_err().primary_message(), "bad template");
    }

    #[test]
    fn defaults_follow_available_parallelism() {
        let limits = RenderLimits::default();

        assert!(limits.max_concurrent_renders >= 1);
        assert_eq!(limits.max_queued_renders, limits.max_concurrent_renders * 2);
    }
}
