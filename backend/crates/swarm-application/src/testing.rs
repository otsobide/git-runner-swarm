//! Fakes of the ports, for the application's own tests.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::SystemTime;

use async_trait::async_trait;
use swarm_domain::{
    Forge, ImageReference, LogLine, LogStream, LogTail, RegistrationToken, Runner,
    RunnerDefinition, RunnerId, RunnerStatus,
};

use crate::ports::{
    EnvironmentVariable, LaunchSpec, RestartPolicy, RunnerImage, RunnerRuntime, RuntimeError,
};

/// A runner image that starts `<key>:latest` and passes the runner's name
/// and token in its environment.
pub(crate) struct FakeImage {
    key: String,
    forge: Forge,
    image: ImageReference,
}

impl FakeImage {
    pub(crate) fn new(key: &str, forge: Forge) -> Self {
        Self {
            key: key.to_owned(),
            forge,
            image: ImageReference::new(format!("{key}:latest")).unwrap(),
        }
    }
}

impl RunnerImage for FakeImage {
    fn key(&self) -> &str {
        &self.key
    }

    fn description(&self) -> &'static str {
        "A fake runner image"
    }

    fn forge(&self) -> Forge {
        self.forge
    }

    fn image(&self) -> &ImageReference {
        &self.image
    }

    fn launch_spec(&self, runner: &RunnerDefinition, token: &RegistrationToken) -> LaunchSpec {
        LaunchSpec {
            image: self.image.clone(),
            command: vec!["run".to_owned()],
            environment: vec![
                EnvironmentVariable::plain("RUNNER_NAME", runner.name().as_str()),
                EnvironmentVariable::secret("RUNNER_TOKEN", token.expose()),
            ],
            restart: RestartPolicy::UnlessStopped,
        }
    }
}

/// A runtime holding runners in memory, which records the images it was
/// asked to pull and the specs it was asked to launch.
#[derive(Default)]
pub(crate) struct FakeRuntime {
    runners: Mutex<Vec<Runner>>,
    pulled: Mutex<Vec<ImageReference>>,
    launched: Mutex<Vec<LaunchSpec>>,
    logs: Mutex<HashMap<RunnerId, Vec<LogLine>>>,
    pulls_fail: bool,
}

impl FakeRuntime {
    pub(crate) fn holding(runners: impl IntoIterator<Item = Runner>) -> Self {
        Self {
            runners: Mutex::new(runners.into_iter().collect()),
            ..Self::default()
        }
    }

    /// A runtime that cannot pull any image.
    pub(crate) fn failing_pulls() -> Self {
        Self {
            pulls_fail: true,
            ..Self::default()
        }
    }

    pub(crate) fn pulled(&self) -> Vec<ImageReference> {
        self.pulled.lock().unwrap().clone()
    }

    pub(crate) fn launched(&self) -> Vec<LaunchSpec> {
        self.launched.lock().unwrap().clone()
    }

    /// Has a runner log `lines`, to its standard output.
    pub(crate) fn logged<'a>(&self, id: RunnerId, lines: impl IntoIterator<Item = &'a str>) {
        self.logs.lock().unwrap().entry(id).or_default().extend(
            lines
                .into_iter()
                .map(|line| LogLine::new(LogStream::Out, line)),
        );
    }

    pub(crate) fn runner(&self, id: RunnerId) -> Option<Runner> {
        self.runners
            .lock()
            .unwrap()
            .iter()
            .find(|runner| runner.id() == id)
            .cloned()
    }

    fn set_status(&self, id: RunnerId, status: RunnerStatus) -> Result<(), RuntimeError> {
        let mut runners = self.runners.lock().unwrap();
        let runner = runners
            .iter_mut()
            .find(|runner| runner.id() == id)
            .ok_or(RuntimeError::NotFound(id))?;
        *runner = Runner::new(
            runner.definition().clone(),
            runner.image().clone(),
            status,
            runner.created_at(),
        );
        Ok(())
    }
}

#[async_trait]
impl RunnerRuntime for FakeRuntime {
    async fn ensure_image(&self, image: &ImageReference) -> Result<(), RuntimeError> {
        if self.pulls_fail {
            return Err(RuntimeError::ImageUnavailable {
                image: image.clone(),
                reason: "the registry is down".to_owned(),
            });
        }
        self.pulled.lock().unwrap().push(image.clone());
        Ok(())
    }

    async fn launch(
        &self,
        runner: &RunnerDefinition,
        spec: &LaunchSpec,
    ) -> Result<Runner, RuntimeError> {
        self.launched.lock().unwrap().push(spec.clone());
        let runner = Runner::new(
            runner.clone(),
            spec.image.clone(),
            RunnerStatus::Running,
            SystemTime::now(),
        );
        self.runners.lock().unwrap().push(runner.clone());
        Ok(runner)
    }

    async fn list(&self) -> Result<Vec<Runner>, RuntimeError> {
        Ok(self.runners.lock().unwrap().clone())
    }

    async fn find(&self, id: RunnerId) -> Result<Option<Runner>, RuntimeError> {
        Ok(self.runner(id))
    }

    async fn start(&self, id: RunnerId) -> Result<(), RuntimeError> {
        self.set_status(id, RunnerStatus::Running)
    }

    async fn stop(&self, id: RunnerId) -> Result<(), RuntimeError> {
        self.set_status(id, RunnerStatus::Exited)
    }

    async fn remove(&self, id: RunnerId) -> Result<(), RuntimeError> {
        let mut runners = self.runners.lock().unwrap();
        let before = runners.len();
        runners.retain(|runner| runner.id() != id);
        if runners.len() == before {
            return Err(RuntimeError::NotFound(id));
        }
        Ok(())
    }

    async fn logs(&self, id: RunnerId, tail: LogTail) -> Result<Vec<LogLine>, RuntimeError> {
        if self.runner(id).is_none() {
            return Err(RuntimeError::NotFound(id));
        }
        let logs = self.logs.lock().unwrap();
        let lines = logs.get(&id).map_or(&[][..], Vec::as_slice);
        let from = lines.len().saturating_sub(tail.lines() as usize);
        Ok(lines[from..].to_vec())
    }
}
