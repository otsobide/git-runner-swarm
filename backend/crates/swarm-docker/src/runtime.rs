use std::collections::HashMap;
use std::time::SystemTime;

use async_trait::async_trait;
use bollard::Docker;
use bollard::errors::Error as DockerError;
use bollard::models::{
    ContainerCreateBody, ContainerInspectResponse, ContainerStateStatusEnum, HostConfig,
    RestartPolicy as DockerRestartPolicy, RestartPolicyNameEnum,
};
use bollard::query_parameters::{
    CreateContainerOptions, CreateImageOptions, InspectContainerOptions, ListContainersOptions,
    RemoveContainerOptions, StartContainerOptions, StopContainerOptions,
};
use futures_util::StreamExt;
use futures_util::future::join_all;
use swarm_application::ports::{LaunchSpec, RestartPolicy, RunnerRuntime, RuntimeError};
use swarm_domain::{ImageReference, Runner, RunnerDefinition, RunnerId, RunnerStatus};

use crate::labels;

/// What every runner's container is named, before its runner's id.
const CONTAINER_PREFIX: &str = "git-runner-swarm-";

/// Runs runners as containers of a Docker engine.
#[derive(Debug, Clone)]
pub struct DockerRuntime {
    docker: Docker,
    stop_timeout: i32,
}

impl DockerRuntime {
    /// How long a runner is given to stop cleanly before it is killed, in
    /// seconds: time for the runner to cancel a job it runs and tell its
    /// forge it is going offline.
    pub const DEFAULT_STOP_TIMEOUT: i32 = 30;

    /// Connects to the engine the way the Docker CLI does: at `DOCKER_HOST`
    /// if it is set, at the local socket otherwise. Nothing is sent until
    /// the runtime is first used; [`DockerRuntime::ping`] checks the engine
    /// answers.
    ///
    /// # Errors
    ///
    /// When `DOCKER_HOST` is not an address Docker can be reached at.
    pub fn connect() -> Result<Self, RuntimeError> {
        Docker::connect_with_defaults()
            .map(Self::new)
            .map_err(|error| RuntimeError::Unreachable(error.to_string()))
    }

    pub fn new(docker: Docker) -> Self {
        Self {
            docker,
            stop_timeout: Self::DEFAULT_STOP_TIMEOUT,
        }
    }

    /// The runtime, giving runners `seconds` to stop cleanly.
    #[must_use]
    pub fn with_stop_timeout(mut self, seconds: i32) -> Self {
        self.stop_timeout = seconds;
        self
    }

    /// Checks the engine answers.
    ///
    /// # Errors
    ///
    /// When it does not.
    pub async fn ping(&self) -> Result<(), RuntimeError> {
        self.docker.ping().await.map(drop).map_err(engine_error)
    }

    /// The runner a container runs, if the container exists and is a
    /// runner's. One whose labels cannot be read back is left out, with a
    /// warning, rather than failing every list it would be in.
    async fn inspect(&self, container: &str) -> Result<Option<Runner>, RuntimeError> {
        match self
            .docker
            .inspect_container(container, None::<InspectContainerOptions>)
            .await
        {
            Ok(inspected) => Ok(runner_from(&inspected)
                .inspect_err(|reason| {
                    tracing::warn!(
                        container,
                        "leaving out a container labelled as a runner's: {reason}"
                    );
                })
                .ok()),
            Err(error) if status_of(&error) == Some(404) => Ok(None),
            Err(error) => Err(engine_error(error)),
        }
    }
}

#[async_trait]
impl RunnerRuntime for DockerRuntime {
    async fn ensure_image(&self, image: &ImageReference) -> Result<(), RuntimeError> {
        match self.docker.inspect_image(image.as_str()).await {
            Ok(_) => return Ok(()),
            Err(error) if status_of(&error) == Some(404) => {}
            Err(error) => return Err(engine_error(error)),
        }

        tracing::info!(%image, "pulling the image, which the engine does not have");
        let mut progress = self
            .docker
            .create_image(Some(pull_options(image)), None, None);
        while let Some(step) = progress.next().await {
            step.map_err(|error| RuntimeError::ImageUnavailable {
                image: image.clone(),
                reason: match error {
                    DockerError::DockerResponseServerError { message, .. } => message,
                    DockerError::DockerStreamError { error } => error,
                    other => other.to_string(),
                },
            })?;
        }
        Ok(())
    }

    async fn launch(
        &self,
        runner: &RunnerDefinition,
        spec: &LaunchSpec,
    ) -> Result<Runner, RuntimeError> {
        let id = runner.id();
        let name = container_name(id);
        let options = CreateContainerOptions {
            name: Some(name.clone()),
            ..CreateContainerOptions::default()
        };
        self.docker
            .create_container(Some(options), container_body(runner, spec))
            .await
            .map_err(engine_error)?;

        if let Err(error) = self
            .docker
            .start_container(&name, None::<StartContainerOptions>)
            .await
        {
            // A runner that never started is not left behind half-made.
            let remove = RemoveContainerOptions {
                force: true,
                ..RemoveContainerOptions::default()
            };
            if let Err(cleanup) = self.docker.remove_container(&name, Some(remove)).await {
                tracing::warn!(
                    container = name,
                    "could not remove a runner that failed to start: {cleanup}"
                );
            }
            return Err(engine_error(error));
        }

        self.inspect(&name).await?.ok_or(RuntimeError::NotFound(id))
    }

    async fn list(&self) -> Result<Vec<Runner>, RuntimeError> {
        let options = ListContainersOptions {
            all: true,
            filters: Some(HashMap::from([(
                "label".to_owned(),
                vec![format!("{}=true", labels::MANAGED)],
            )])),
            ..ListContainersOptions::default()
        };
        let containers = self
            .docker
            .list_containers(Some(options))
            .await
            .map_err(engine_error)?;

        let inspected = join_all(
            containers
                .iter()
                .filter_map(|container| container.id.as_deref())
                .map(|id| self.inspect(id)),
        )
        .await;
        inspected
            .into_iter()
            .filter_map(Result::transpose)
            .collect()
    }

    async fn find(&self, id: RunnerId) -> Result<Option<Runner>, RuntimeError> {
        self.inspect(&container_name(id)).await
    }

    async fn start(&self, id: RunnerId) -> Result<(), RuntimeError> {
        let started = self
            .docker
            .start_container(&container_name(id), None::<StartContainerOptions>)
            .await;
        unless_already_done(id, started)
    }

    async fn stop(&self, id: RunnerId) -> Result<(), RuntimeError> {
        let options = StopContainerOptions {
            t: Some(self.stop_timeout),
            signal: None,
        };
        let stopped = self
            .docker
            .stop_container(&container_name(id), Some(options))
            .await;
        unless_already_done(id, stopped)
    }

    async fn remove(&self, id: RunnerId) -> Result<(), RuntimeError> {
        let options = RemoveContainerOptions {
            force: true,
            v: true,
            ..RemoveContainerOptions::default()
        };
        self.docker
            .remove_container(&container_name(id), Some(options))
            .await
            .map_err(|error| runner_error(id, error))
    }
}

/// The name of the container a runner runs in.
pub(crate) fn container_name(id: RunnerId) -> String {
    format!("{CONTAINER_PREFIX}{id}")
}

/// The container a runner runs in, as its launch spec says, with its
/// definition in its labels and limited as the definition asks. Docker's
/// own init runs as PID 1, to reap what jobs leave behind and pass signals
/// on to the runner.
fn container_body(runner: &RunnerDefinition, spec: &LaunchSpec) -> ContainerCreateBody {
    let limits = runner.limits();
    ContainerCreateBody {
        image: Some(spec.image.to_string()),
        cmd: (!spec.command.is_empty()).then(|| spec.command.clone()),
        env: Some(
            spec.environment
                .iter()
                .map(|variable| format!("{}={}", variable.name(), variable.value()))
                .collect(),
        ),
        labels: Some(labels::encode(runner)),
        host_config: Some(HostConfig {
            init: Some(true),
            restart_policy: Some(DockerRestartPolicy {
                name: Some(match spec.restart {
                    RestartPolicy::Never => RestartPolicyNameEnum::NO,
                    RestartPolicy::UnlessStopped => RestartPolicyNameEnum::UNLESS_STOPPED,
                }),
                maximum_retry_count: None,
            }),
            nano_cpus: limits
                .cpus()
                .map(|cpus| i64::from(cpus.millis()) * 1_000_000),
            memory: limits
                .memory()
                .map(|memory| i64::try_from(memory.bytes()).unwrap_or(i64::MAX)),
            ..HostConfig::default()
        }),
        ..ContainerCreateBody::default()
    }
}

/// What to ask the engine to pull for `image`: its name and tag, `latest`
/// when it has none, since asking for a name without a tag pulls every tag
/// there is. A reference by digest is asked for as it is.
fn pull_options(image: &ImageReference) -> CreateImageOptions {
    let reference = image.as_str();
    let name_start = reference.rfind('/').map_or(0, |at| at + 1);
    let (from_image, tag) = if reference.contains('@') {
        (reference, None)
    } else if let Some(colon) = reference[name_start..].rfind(':') {
        let at = name_start + colon;
        (&reference[..at], Some(&reference[at + 1..]))
    } else {
        (reference, Some("latest"))
    };
    CreateImageOptions {
        from_image: Some(from_image.to_owned()),
        tag: tag.map(str::to_owned),
        ..CreateImageOptions::default()
    }
}

/// The runner a container runs, read from what the engine says of it.
fn runner_from(container: &ContainerInspectResponse) -> Result<Runner, String> {
    let config = container
        .config
        .as_ref()
        .ok_or("the engine says nothing of its configuration")?;
    let no_labels = HashMap::new();
    let definition = labels::decode(config.labels.as_ref().unwrap_or(&no_labels))
        .map_err(|error| error.to_string())?;
    let image = ImageReference::new(config.image.clone().unwrap_or_default())
        .map_err(|error| error.to_string())?;
    let state = container.state.as_ref();
    let status = status_from(state.and_then(|state| state.status));
    let created_at = container
        .created
        .map_or(SystemTime::UNIX_EPOCH, SystemTime::from);

    // A container being restarted has exited, and why it did is what tells
    // a runner restarted after a crash from one that cannot register.
    let runner = Runner::new(definition, image, status, created_at);
    Ok(match (status, state.and_then(|state| state.exit_code)) {
        (RunnerStatus::Exited | RunnerStatus::Restarting | RunnerStatus::Dead, Some(code)) => {
            runner.with_exit_code(code)
        }
        _ => runner,
    })
}

/// A container's state, as a runner's. One still stopping is running
/// until it has stopped; one being removed is as good as dead.
fn status_from(status: Option<ContainerStateStatusEnum>) -> RunnerStatus {
    match status {
        Some(ContainerStateStatusEnum::CREATED) => RunnerStatus::Created,
        Some(ContainerStateStatusEnum::RUNNING | ContainerStateStatusEnum::STOPPING) => {
            RunnerStatus::Running
        }
        Some(ContainerStateStatusEnum::RESTARTING) => RunnerStatus::Restarting,
        Some(ContainerStateStatusEnum::PAUSED) => RunnerStatus::Paused,
        Some(ContainerStateStatusEnum::DEAD | ContainerStateStatusEnum::REMOVING) => {
            RunnerStatus::Dead
        }
        Some(ContainerStateStatusEnum::EXITED | ContainerStateStatusEnum::EMPTY) | None => {
            RunnerStatus::Exited
        }
    }
}

fn status_of(error: &DockerError) -> Option<u16> {
    match error {
        DockerError::DockerResponseServerError { status_code, .. } => Some(*status_code),
        _ => None,
    }
}

/// A failure of the engine, as the application tells failures apart: an
/// engine that could not be reached, or one that answered with a refusal.
fn engine_error(error: DockerError) -> RuntimeError {
    match error {
        DockerError::DockerResponseServerError { message, .. } => RuntimeError::Failed(message),
        DockerError::SocketNotFoundError(_)
        | DockerError::IOError { .. }
        | DockerError::HyperLegacyError { .. }
        | DockerError::HyperResponseError { .. }
        | DockerError::RequestTimeoutError => RuntimeError::Unreachable(error.to_string()),
        other => RuntimeError::Failed(other.to_string()),
    }
}

/// A failure acting on a runner's container, where a container that is not
/// there means a runner that is not.
fn runner_error(id: RunnerId, error: DockerError) -> RuntimeError {
    if status_of(&error) == Some(404) {
        RuntimeError::NotFound(id)
    } else {
        engine_error(error)
    }
}

/// Starting a running container, or stopping a stopped one, is answered
/// 304 Not Modified: done already, which is all that was asked.
fn unless_already_done(id: RunnerId, result: Result<(), DockerError>) -> Result<(), RuntimeError> {
    match result {
        Err(error) if status_of(&error) == Some(304) => Ok(()),
        other => other.map_err(|error| runner_error(id, error)),
    }
}

#[cfg(test)]
mod tests {
    use bollard::models::{ContainerConfig, ContainerState};
    use swarm_application::ports::EnvironmentVariable;
    use swarm_domain::{
        CpuLimit, Forge, MemoryLimit, RunnerLabels, RunnerLifecycle, RunnerLimits, RunnerName,
        RunnerScope,
    };

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

    fn spec(restart: RestartPolicy) -> LaunchSpec {
        LaunchSpec {
            image: ImageReference::new("ghcr.io/actions/actions-runner:latest").unwrap(),
            command: vec![
                "bash".to_owned(),
                "-c".to_owned(),
                "exec ./run.sh".to_owned(),
            ],
            environment: vec![
                EnvironmentVariable::plain("RUNNER_NAME", "builder"),
                EnvironmentVariable::secret("RUNNER_TOKEN", "AESOHC2S"),
            ],
            restart,
        }
    }

    fn pulled(reference: &str) -> (Option<String>, Option<String>) {
        let options = pull_options(&ImageReference::new(reference).unwrap());
        (options.from_image, options.tag)
    }

    fn inspected(definition: &RunnerDefinition, state: ContainerState) -> ContainerInspectResponse {
        ContainerInspectResponse {
            config: Some(ContainerConfig {
                image: Some("ghcr.io/actions/actions-runner:latest".to_owned()),
                labels: Some(labels::encode(definition)),
                ..ContainerConfig::default()
            }),
            state: Some(state),
            ..ContainerInspectResponse::default()
        }
    }

    #[test]
    fn names_a_runners_container_after_its_id() {
        let id: RunnerId = "67e55044-10b1-426f-9247-bb680e5fe0c8".parse().unwrap();

        assert_eq!(
            container_name(id),
            "git-runner-swarm-67e55044-10b1-426f-9247-bb680e5fe0c8"
        );
    }

    #[test]
    fn creates_the_container_as_the_spec_says() {
        let definition = definition();

        let body = container_body(&definition, &spec(RestartPolicy::UnlessStopped));

        assert_eq!(
            body.image.as_deref(),
            Some("ghcr.io/actions/actions-runner:latest")
        );
        assert_eq!(body.cmd.unwrap(), ["bash", "-c", "exec ./run.sh"]);
        assert_eq!(
            body.env.unwrap(),
            ["RUNNER_NAME=builder", "RUNNER_TOKEN=AESOHC2S"]
        );
        assert_eq!(labels::decode(&body.labels.unwrap()).unwrap(), definition);
    }

    #[test]
    fn limits_the_container_as_the_runner_asks() {
        let definition = definition().with_limits(RunnerLimits::new(
            Some(CpuLimit::parse("1.5").unwrap()),
            Some(MemoryLimit::parse("512m").unwrap()),
        ));

        let host = container_body(&definition, &spec(RestartPolicy::Never))
            .host_config
            .unwrap();

        assert_eq!(host.nano_cpus, Some(1_500_000_000));
        assert_eq!(host.memory, Some(512 * 1024 * 1024));
    }

    #[test]
    fn leaves_the_container_unlimited_unless_asked() {
        let host = container_body(&definition(), &spec(RestartPolicy::Never))
            .host_config
            .unwrap();

        assert_eq!(host.nano_cpus, None);
        assert_eq!(host.memory, None);
    }

    #[test]
    fn runs_the_images_own_command_when_the_spec_gives_none() {
        let mut spec = spec(RestartPolicy::Never);
        spec.command.clear();

        assert_eq!(container_body(&definition(), &spec).cmd, None);
    }

    #[test]
    fn runs_an_init_and_the_restart_policy_the_spec_asks_for() {
        for (restart, expected) in [
            (
                RestartPolicy::UnlessStopped,
                RestartPolicyNameEnum::UNLESS_STOPPED,
            ),
            (RestartPolicy::Never, RestartPolicyNameEnum::NO),
        ] {
            let host = container_body(&definition(), &spec(restart))
                .host_config
                .unwrap();

            assert_eq!(host.init, Some(true));
            assert_eq!(host.restart_policy.unwrap().name, Some(expected));
        }
    }

    #[test]
    fn pulls_the_tag_an_image_names() {
        assert_eq!(
            pulled("ghcr.io/actions/actions-runner:2.338.2"),
            (
                Some("ghcr.io/actions/actions-runner".to_owned()),
                Some("2.338.2".to_owned())
            )
        );
    }

    #[test]
    fn pulls_latest_of_an_image_naming_no_tag() {
        assert_eq!(
            pulled("busybox"),
            (Some("busybox".to_owned()), Some("latest".to_owned()))
        );
        assert_eq!(
            pulled("registry.lan:5000/runner"),
            (
                Some("registry.lan:5000/runner".to_owned()),
                Some("latest".to_owned())
            )
        );
    }

    #[test]
    fn pulls_an_image_named_by_digest_as_it_is() {
        let reference =
            "busybox@sha256:0000000000000000000000000000000000000000000000000000000000000000";

        assert_eq!(pulled(reference), (Some(reference.to_owned()), None));
    }

    #[test]
    fn reads_the_runner_a_container_runs() {
        let definition = definition();
        let state = ContainerState {
            status: Some(ContainerStateStatusEnum::RUNNING),
            exit_code: Some(0),
            ..ContainerState::default()
        };

        let runner = runner_from(&inspected(&definition, state)).unwrap();

        assert_eq!(runner.definition(), &definition);
        assert_eq!(
            runner.image().as_str(),
            "ghcr.io/actions/actions-runner:latest"
        );
        assert_eq!(runner.status(), RunnerStatus::Running);
        assert_eq!(runner.exit_code(), None);
    }

    #[test]
    fn keeps_the_code_a_stopped_container_exited_with() {
        let state = ContainerState {
            status: Some(ContainerStateStatusEnum::EXITED),
            exit_code: Some(1),
            ..ContainerState::default()
        };

        let runner = runner_from(&inspected(&definition(), state)).unwrap();

        assert_eq!(runner.status(), RunnerStatus::Exited);
        assert_eq!(runner.exit_code(), Some(1));
    }

    #[test]
    fn keeps_the_code_a_restarting_container_last_exited_with() {
        let state = ContainerState {
            status: Some(ContainerStateStatusEnum::RESTARTING),
            exit_code: Some(1),
            ..ContainerState::default()
        };

        let runner = runner_from(&inspected(&definition(), state)).unwrap();

        assert_eq!(runner.status(), RunnerStatus::Restarting);
        assert_eq!(runner.exit_code(), Some(1));
    }

    #[test]
    fn refuses_a_container_whose_labels_are_not_a_runners() {
        let container = ContainerInspectResponse {
            config: Some(ContainerConfig {
                image: Some("busybox".to_owned()),
                ..ContainerConfig::default()
            }),
            ..ContainerInspectResponse::default()
        };

        assert!(runner_from(&container).is_err());
    }

    #[test]
    fn reads_each_container_state_as_a_runners() {
        for (state, expected) in [
            (ContainerStateStatusEnum::CREATED, RunnerStatus::Created),
            (ContainerStateStatusEnum::RUNNING, RunnerStatus::Running),
            (ContainerStateStatusEnum::STOPPING, RunnerStatus::Running),
            (
                ContainerStateStatusEnum::RESTARTING,
                RunnerStatus::Restarting,
            ),
            (ContainerStateStatusEnum::PAUSED, RunnerStatus::Paused),
            (ContainerStateStatusEnum::EXITED, RunnerStatus::Exited),
            (ContainerStateStatusEnum::REMOVING, RunnerStatus::Dead),
            (ContainerStateStatusEnum::DEAD, RunnerStatus::Dead),
        ] {
            assert_eq!(status_from(Some(state)), expected, "{state:?}");
        }
    }

    #[test]
    fn takes_a_missing_container_for_a_missing_runner() {
        let id = RunnerId::generate();
        let error = DockerError::DockerResponseServerError {
            status_code: 404,
            message: "No such container".to_owned(),
        };

        assert!(
            matches!(runner_error(id, error), RuntimeError::NotFound(missing) if missing == id)
        );
    }

    #[test]
    fn takes_not_modified_for_done_already() {
        let error = DockerError::DockerResponseServerError {
            status_code: 304,
            message: String::new(),
        };

        assert!(unless_already_done(RunnerId::generate(), Err(error)).is_ok());
    }

    #[test]
    fn tells_an_engine_that_refused_from_one_out_of_reach() {
        let refused = DockerError::DockerResponseServerError {
            status_code: 409,
            message: "Conflict".to_owned(),
        };
        let out_of_reach = DockerError::SocketNotFoundError("/var/run/docker.sock".to_owned());

        assert!(
            matches!(engine_error(refused), RuntimeError::Failed(message) if message == "Conflict")
        );
        assert!(matches!(
            engine_error(out_of_reach),
            RuntimeError::Unreachable(_)
        ));
    }
}
