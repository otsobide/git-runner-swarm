use async_trait::async_trait;
use swarm_domain::{ImageReference, LogLine, LogTail, Runner, RunnerDefinition, RunnerId};
use thiserror::Error;

use crate::ports::LaunchSpec;

/// Where runners run: a container engine, seen as the runners it holds.
///
/// The runtime is also where runners are kept. A runner's definition is
/// stored with its container, so the runners there are, the panel manages,
/// and no runner can be listed whose container is gone, nor a container
/// left unlisted.
#[async_trait]
pub trait RunnerRuntime: Send + Sync {
    /// Makes sure containers can be started from `image`, pulling it if
    /// the engine does not have it.
    async fn ensure_image(&self, image: &ImageReference) -> Result<(), RuntimeError>;

    /// Creates the container `runner` runs in, as `spec` says, starts it,
    /// and answers the runner as it is once started.
    async fn launch(
        &self,
        runner: &RunnerDefinition,
        spec: &LaunchSpec,
    ) -> Result<Runner, RuntimeError>;

    /// Every runner the panel manages, in no particular order.
    async fn list(&self) -> Result<Vec<Runner>, RuntimeError>;

    /// The runner with this id, if the panel manages one.
    async fn find(&self, id: RunnerId) -> Result<Option<Runner>, RuntimeError>;

    /// Starts a runner's container. Starting one already running does
    /// nothing.
    async fn start(&self, id: RunnerId) -> Result<(), RuntimeError>;

    /// Stops a runner's container, giving the runner time to stop cleanly.
    /// Stopping one already stopped does nothing.
    async fn stop(&self, id: RunnerId) -> Result<(), RuntimeError>;

    /// Removes a runner's container, stopping it first if it runs.
    async fn remove(&self, id: RunnerId) -> Result<(), RuntimeError>;
    /// The latest lines a runner's container logged, as many as `tail`
    /// says at most, oldest first.
    async fn logs(&self, id: RunnerId, tail: LogTail) -> Result<Vec<LogLine>, RuntimeError>;
}

/// Why a runtime could not do what it was asked.
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("there is no runner {0}")]
    NotFound(RunnerId),

    #[error("the image {image} could not be pulled: {reason}")]
    ImageUnavailable {
        image: ImageReference,
        reason: String,
    },

    #[error("the container engine cannot be reached: {0}")]
    Unreachable(String),

    #[error("the container engine failed: {0}")]
    Failed(String),
}
