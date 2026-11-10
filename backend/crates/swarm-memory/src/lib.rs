//! A runtime that keeps runners in memory, as if an engine ran them.
//!
//! For trying the panel without a container engine, and for testing what
//! drives the application (the HTTP API, for one) without one.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::SystemTime;

use async_trait::async_trait;
use swarm_application::ports::{LaunchSpec, RunnerRuntime, RuntimeError};
use swarm_domain::{
    ImageReference, LogLine, LogTail, Runner, RunnerDefinition, RunnerId, RunnerStatus,
};

/// Runners kept in memory, and lost with it. A launched runner is running,
/// a stopped one has exited with 0, and every image is there to pull.
#[derive(Debug, Default)]
pub struct InMemoryRuntime {
    runners: Mutex<HashMap<RunnerId, Runner>>,
    images: Mutex<HashSet<ImageReference>>,
}

impl InMemoryRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether `image` was pulled.
    ///
    /// # Panics
    ///
    /// When another thread panicked holding the runtime's lock.
    pub fn has_image(&self, image: &ImageReference) -> bool {
        self.images.lock().unwrap().contains(image)
    }

    fn update(&self, id: RunnerId, status: RunnerStatus) -> Result<(), RuntimeError> {
        let mut runners = self.runners.lock().unwrap();
        let runner = runners.get_mut(&id).ok_or(RuntimeError::NotFound(id))?;
        let updated = Runner::new(
            runner.definition().clone(),
            runner.image().clone(),
            status,
            runner.created_at(),
        );
        *runner = match status {
            RunnerStatus::Exited => updated.with_exit_code(0),
            _ => updated,
        };
        Ok(())
    }
}

#[async_trait]
impl RunnerRuntime for InMemoryRuntime {
    async fn ensure_image(&self, image: &ImageReference) -> Result<(), RuntimeError> {
        self.images.lock().unwrap().insert(image.clone());
        Ok(())
    }

    async fn launch(
        &self,
        runner: &RunnerDefinition,
        spec: &LaunchSpec,
    ) -> Result<Runner, RuntimeError> {
        let launched = Runner::new(
            runner.clone(),
            spec.image.clone(),
            RunnerStatus::Running,
            SystemTime::now(),
        );
        self.runners
            .lock()
            .unwrap()
            .insert(runner.id(), launched.clone());
        Ok(launched)
    }

    async fn list(&self) -> Result<Vec<Runner>, RuntimeError> {
        Ok(self.runners.lock().unwrap().values().cloned().collect())
    }

    async fn find(&self, id: RunnerId) -> Result<Option<Runner>, RuntimeError> {
        Ok(self.runners.lock().unwrap().get(&id).cloned())
    }

    async fn start(&self, id: RunnerId) -> Result<(), RuntimeError> {
        self.update(id, RunnerStatus::Running)
    }

    async fn stop(&self, id: RunnerId) -> Result<(), RuntimeError> {
        self.update(id, RunnerStatus::Exited)
    }

    async fn remove(&self, id: RunnerId) -> Result<(), RuntimeError> {
        self.runners
            .lock()
            .unwrap()
            .remove(&id)
            .map(drop)
            .ok_or(RuntimeError::NotFound(id))
    }

    /// Nothing runs, so a runner has logged nothing.
    async fn logs(&self, id: RunnerId, _tail: LogTail) -> Result<Vec<LogLine>, RuntimeError> {
        if self.runners.lock().unwrap().contains_key(&id) {
            Ok(Vec::new())
        } else {
            Err(RuntimeError::NotFound(id))
        }
    }
}

#[cfg(test)]
mod tests {
    use swarm_application::ports::RestartPolicy;
    use swarm_domain::{Forge, RunnerLabels, RunnerLifecycle, RunnerName, RunnerScope};

    use super::*;

    fn definition() -> RunnerDefinition {
        RunnerDefinition::new(
            RunnerId::generate(),
            Forge::GitHub,
            RunnerScope::parse_url("https://github.com/otsobide/mindweaver").unwrap(),
            RunnerName::new("builder").unwrap(),
            RunnerLabels::default(),
            RunnerLifecycle::Persistent,
        )
    }

    fn spec() -> LaunchSpec {
        LaunchSpec {
            image: ImageReference::new("ghcr.io/actions/actions-runner:latest").unwrap(),
            command: Vec::new(),
            environment: Vec::new(),
            restart: RestartPolicy::UnlessStopped,
        }
    }

    #[tokio::test]
    async fn pulls_any_image() {
        let runtime = InMemoryRuntime::new();
        let image = ImageReference::new("anything:at-all").unwrap();

        runtime.ensure_image(&image).await.unwrap();

        assert!(runtime.has_image(&image));
    }

    #[tokio::test]
    async fn logs_nothing_as_nothing_runs() {
        let runtime = InMemoryRuntime::new();
        let definition = definition();
        runtime.launch(&definition, &spec()).await.unwrap();

        let logged = runtime.logs(definition.id(), LogTail::default()).await;
        let unknown = runtime.logs(RunnerId::generate(), LogTail::default()).await;

        assert_eq!(logged.unwrap(), []);
        assert!(matches!(unknown, Err(RuntimeError::NotFound(_))));
    }

    #[tokio::test]
    async fn holds_a_launched_runner_running() {
        let runtime = InMemoryRuntime::new();
        let definition = definition();

        let launched = runtime.launch(&definition, &spec()).await.unwrap();

        assert_eq!(launched.status(), RunnerStatus::Running);
        assert_eq!(
            runtime.find(definition.id()).await.unwrap(),
            Some(launched.clone())
        );
        assert_eq!(runtime.list().await.unwrap(), [launched]);
    }

    #[tokio::test]
    async fn stops_a_runner_as_exited_cleanly_and_starts_it_again() {
        let runtime = InMemoryRuntime::new();
        let id = runtime.launch(&definition(), &spec()).await.unwrap().id();

        runtime.stop(id).await.unwrap();
        let stopped = runtime.find(id).await.unwrap().unwrap();
        runtime.start(id).await.unwrap();
        let started = runtime.find(id).await.unwrap().unwrap();

        assert_eq!(
            (stopped.status(), stopped.exit_code()),
            (RunnerStatus::Exited, Some(0))
        );
        assert_eq!(
            (started.status(), started.exit_code()),
            (RunnerStatus::Running, None)
        );
    }

    #[tokio::test]
    async fn forgets_a_removed_runner() {
        let runtime = InMemoryRuntime::new();
        let id = runtime.launch(&definition(), &spec()).await.unwrap().id();

        runtime.remove(id).await.unwrap();

        assert_eq!(runtime.find(id).await.unwrap(), None);
        assert!(matches!(
            runtime.remove(id).await,
            Err(RuntimeError::NotFound(_))
        ));
        assert!(matches!(
            runtime.start(id).await,
            Err(RuntimeError::NotFound(_))
        ));
    }
}
