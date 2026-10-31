//! Port for running CPU-bound compilation jobs off the async runtime.

use async_trait::async_trait;
use thiserror::Error;

use crate::{CompilationError, Document};

/// An owned, CPU-bound compilation. It runs on a thread chosen by the executor.
pub type CompileJob = Box<dyn FnOnce() -> Result<Document, CompilationError> + Send + 'static>;

/// How a job is admitted by a [`RenderExecutor`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Admission {
    /// Interactive request: bounded queue, may be rejected as saturated.
    Request,
    /// Warmup and reloads: waits for a slot and is never rejected.
    Background,
}

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("render capacity exceeded")]
    Saturated,

    #[error("render execution failed: {message}")]
    Aborted { message: String },
}

/// Runs compilation jobs on blocking threads behind an admission limit.
///
/// A returned future is cancel-safe while the job is still waiting for admission. Once a job has
/// started it cannot be cancelled and keeps its capacity until it finishes.
#[async_trait]
pub trait RenderExecutor: Send + Sync {
    async fn execute(
        &self,
        admission: Admission,
        job: CompileJob,
    ) -> Result<Result<Document, CompilationError>, ExecutionError>;
}

/// Runs each job on the caller's thread with no limit. Only for tests.
#[cfg(any(test, feature = "test-util"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct InlineExecutor;

#[cfg(any(test, feature = "test-util"))]
#[async_trait]
impl RenderExecutor for InlineExecutor {
    async fn execute(
        &self,
        _admission: Admission,
        job: CompileJob,
    ) -> Result<Result<Document, CompilationError>, ExecutionError> {
        Ok(job())
    }
}
