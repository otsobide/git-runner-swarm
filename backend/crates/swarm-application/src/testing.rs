//! Fakes of the ports, for the application's own tests.

use swarm_domain::{Forge, ImageReference, RegistrationToken, RunnerDefinition};

use crate::ports::{EnvironmentVariable, LaunchSpec, RestartPolicy, RunnerImage};

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
