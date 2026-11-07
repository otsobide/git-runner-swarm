use std::fmt;

use swarm_domain::{Forge, ImageReference, RegistrationToken, RunnerDefinition};

/// A container image runners can run in, and how to start a container of
/// it so the runner inside registers with its forge and takes jobs.
///
/// One adapter per image. Each image has its own way of being told where
/// to register and with which token (arguments, environment variables, a
/// script of its own), and its adapter is the one place that knows it, so
/// supporting another image (Gitea's `act_runner`, Forgejo's runner, an
/// image of one's own) is a new adapter, and nothing else changes.
pub trait RunnerImage: Send + Sync {
    /// Names the image among the panel's: `github-official`. What the API
    /// and the panel choose an image by.
    fn key(&self) -> &str;

    /// A line saying what the image is, for a person choosing between
    /// images.
    fn description(&self) -> &str;

    /// The forge the runners it runs register with.
    fn forge(&self) -> Forge;

    /// The image containers are started from.
    fn image(&self) -> &ImageReference;

    /// How to start a container that registers `runner` with `token`.
    fn launch_spec(&self, runner: &RunnerDefinition, token: &RegistrationToken) -> LaunchSpec;
}

/// How to start the container a runner runs in, as a runner image says,
/// for whichever engine starts it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    pub image: ImageReference,
    /// What the container runs, in place of the image's own command. Empty
    /// to run the image's own.
    pub command: Vec<String>,
    pub environment: Vec<EnvironmentVariable>,
    pub restart: RestartPolicy,
}

/// A variable in the environment a runner's container starts with.
///
/// A secret one (a registration token) is never shown: its `Debug` hides
/// its value, so neither it nor the [`LaunchSpec`] holding it can print
/// the secret by accident.
#[derive(Clone, PartialEq, Eq)]
pub struct EnvironmentVariable {
    name: String,
    value: String,
    secret: bool,
}

impl EnvironmentVariable {
    pub fn plain(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            secret: false,
        }
    }

    pub fn secret(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            secret: true,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The value, for the engine that starts the container with it.
    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn is_secret(&self) -> bool {
        self.secret
    }
}

impl fmt::Debug for EnvironmentVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = if self.secret {
            "<redacted>"
        } else {
            &self.value
        };
        write!(f, "{}={value:?}", self.name)
    }
}

/// Whether the engine starts a runner's container again when it stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    /// Left stopped: what an ephemeral runner wants, done once its job is.
    Never,
    /// Started again, after a crash or when the engine itself starts,
    /// unless it was stopped by hand.
    UnlessStopped,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_shows_a_secret_variables_value() {
        let spec = LaunchSpec {
            image: ImageReference::new("runner:latest").unwrap(),
            command: Vec::new(),
            environment: vec![
                EnvironmentVariable::plain("RUNNER_NAME", "builder"),
                EnvironmentVariable::secret("RUNNER_TOKEN", "AESOHC2S"),
            ],
            restart: RestartPolicy::Never,
        };

        let debugged = format!("{spec:?}");

        assert!(debugged.contains(r#"RUNNER_NAME="builder""#));
        assert!(debugged.contains(r#"RUNNER_TOKEN="<redacted>""#));
        assert!(!debugged.contains("AESOHC2S"));
    }

    #[test]
    fn hands_the_engine_a_secret_variables_value() {
        let variable = EnvironmentVariable::secret("RUNNER_TOKEN", "AESOHC2S");

        assert_eq!(variable.value(), "AESOHC2S");
        assert!(variable.is_secret());
    }
}
