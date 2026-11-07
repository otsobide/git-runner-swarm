use swarm_application::ports::{EnvironmentVariable, LaunchSpec, RestartPolicy, RunnerImage};
use swarm_domain::{Forge, ImageReference, RegistrationToken, RunnerDefinition, RunnerLifecycle};

/// GitHub's own runner image, `ghcr.io/actions/actions-runner`: the
/// runner on Ubuntu, made for Actions Runner Controller.
///
/// The image has no entrypoint that registers the runner: its command is
/// a shell, left for a controller to run the runner's own scripts in. So
/// its containers run [`SCRIPT`] instead, which registers the runner the
/// first time a container starts and runs it every time, from what the
/// environment says.
#[derive(Debug, Clone)]
pub struct GitHubOfficialImage {
    image: ImageReference,
}

/// What a container of the official image runs.
///
/// The first start registers the runner (`config.sh`) from the
/// environment; a container started again finds the registration in
/// `.runner` and does not register twice, since the token it was given
/// has likely expired by then. `--replace`, so a runner recreated under a
/// name GitHub still lists, offline, takes that runner's place.
///
/// The token is unset before the runner runs, so the jobs it runs, which
/// inherit its environment, do not see it. `exec`, so the runner is the
/// process `docker stop` signals, and stops cleanly: the image sets
/// `RUNNER_MANUALLY_TRAP_SIG` for `run.sh` to pass the signal on.
pub const SCRIPT: &str = r#"set -eu
if [ ! -f .runner ]; then
  set -- --unattended --replace --work _work \
    --url "$RUNNER_URL" --token "$RUNNER_TOKEN" --name "$RUNNER_NAME"
  if [ -n "${RUNNER_LABELS:-}" ]; then
    set -- "$@" --labels "$RUNNER_LABELS"
  fi
  if [ "${RUNNER_EPHEMERAL:-false}" = true ]; then
    set -- "$@" --ephemeral
  fi
  ./config.sh "$@"
fi
unset RUNNER_TOKEN
exec ./run.sh
"#;

impl GitHubOfficialImage {
    /// The key the panel knows this image by.
    pub const KEY: &'static str = "github-official";

    /// The image pulled unless another tag (or a mirror of it) is given.
    pub const DEFAULT_IMAGE: &'static str = "ghcr.io/actions/actions-runner:latest";

    /// The adapter for `image`, a tag of the official image or a mirror of
    /// it.
    pub fn new(image: ImageReference) -> Self {
        Self { image }
    }
}

impl Default for GitHubOfficialImage {
    fn default() -> Self {
        Self::new(
            ImageReference::new(Self::DEFAULT_IMAGE)
                .expect("the default image is a valid reference"),
        )
    }
}

impl RunnerImage for GitHubOfficialImage {
    fn key(&self) -> &'static str {
        Self::KEY
    }

    fn description(&self) -> &'static str {
        "GitHub's official runner image: the Actions runner on Ubuntu, and nothing else"
    }

    fn forge(&self) -> Forge {
        Forge::GitHub
    }

    fn image(&self) -> &ImageReference {
        &self.image
    }

    fn launch_spec(&self, runner: &RunnerDefinition, token: &RegistrationToken) -> LaunchSpec {
        let ephemeral = runner.lifecycle().is_ephemeral();
        LaunchSpec {
            image: self.image.clone(),
            command: vec!["bash".to_owned(), "-c".to_owned(), SCRIPT.to_owned()],
            environment: vec![
                EnvironmentVariable::plain("RUNNER_URL", runner.scope().to_string()),
                EnvironmentVariable::plain("RUNNER_NAME", runner.name().as_str()),
                EnvironmentVariable::plain("RUNNER_LABELS", runner.labels().to_comma_separated()),
                EnvironmentVariable::plain("RUNNER_EPHEMERAL", ephemeral.to_string()),
                EnvironmentVariable::secret("RUNNER_TOKEN", token.expose()),
            ],
            restart: match runner.lifecycle() {
                RunnerLifecycle::Persistent => RestartPolicy::UnlessStopped,
                RunnerLifecycle::Ephemeral => RestartPolicy::Never,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use swarm_domain::{RunnerId, RunnerLabels, RunnerName, RunnerScope};

    use super::*;

    fn definition(labels: &str, lifecycle: RunnerLifecycle) -> RunnerDefinition {
        RunnerDefinition::new(
            RunnerId::generate(),
            Forge::GitHub,
            RunnerScope::parse_url("https://github.com/otsobide/mindweaver").unwrap(),
            RunnerName::new("builder").unwrap(),
            RunnerLabels::parse_comma_separated(labels).unwrap(),
            lifecycle,
        )
    }

    fn spec(labels: &str, lifecycle: RunnerLifecycle) -> LaunchSpec {
        GitHubOfficialImage::default().launch_spec(
            &definition(labels, lifecycle),
            &RegistrationToken::new("AESOHC2S").unwrap(),
        )
    }

    fn variable<'a>(spec: &'a LaunchSpec, name: &str) -> &'a EnvironmentVariable {
        spec.environment
            .iter()
            .find(|variable| variable.name() == name)
            .unwrap_or_else(|| panic!("no {name} in the environment"))
    }

    #[test]
    fn runs_github_runners_from_the_official_image() {
        let image = GitHubOfficialImage::default();

        assert_eq!(image.key(), "github-official");
        assert_eq!(image.forge(), Forge::GitHub);
        assert_eq!(
            image.image().as_str(),
            "ghcr.io/actions/actions-runner:latest"
        );
    }

    #[test]
    fn runs_the_tag_or_mirror_it_is_given() {
        let image = GitHubOfficialImage::new(
            ImageReference::new("mirror.lan/actions-runner:2.338.2").unwrap(),
        );

        let spec = image.launch_spec(
            &definition("", RunnerLifecycle::Persistent),
            &RegistrationToken::new("AESOHC2S").unwrap(),
        );

        assert_eq!(spec.image.as_str(), "mirror.lan/actions-runner:2.338.2");
    }

    #[test]
    fn runs_its_registration_script_in_bash() {
        let spec = spec("", RunnerLifecycle::Persistent);

        assert_eq!(spec.command, ["bash", "-c", SCRIPT]);
    }

    #[test]
    fn tells_the_script_where_to_register_and_as_what() {
        let spec = spec("gpu,large", RunnerLifecycle::Persistent);

        assert_eq!(
            variable(&spec, "RUNNER_URL").value(),
            "https://github.com/otsobide/mindweaver"
        );
        assert_eq!(variable(&spec, "RUNNER_NAME").value(), "builder");
        assert_eq!(variable(&spec, "RUNNER_LABELS").value(), "gpu,large");
        assert_eq!(variable(&spec, "RUNNER_EPHEMERAL").value(), "false");
    }

    #[test]
    fn passes_the_token_as_a_secret() {
        let spec = spec("", RunnerLifecycle::Persistent);
        let token = variable(&spec, "RUNNER_TOKEN");

        assert_eq!(token.value(), "AESOHC2S");
        assert!(token.is_secret());
    }

    #[test]
    fn restarts_a_persistent_runner_unless_it_was_stopped() {
        let spec = spec("", RunnerLifecycle::Persistent);

        assert_eq!(spec.restart, RestartPolicy::UnlessStopped);
    }

    #[test]
    fn leaves_an_ephemeral_runner_stopped_once_its_job_is_done() {
        let spec = spec("", RunnerLifecycle::Ephemeral);

        assert_eq!(variable(&spec, "RUNNER_EPHEMERAL").value(), "true");
        assert_eq!(spec.restart, RestartPolicy::Never);
    }
}
