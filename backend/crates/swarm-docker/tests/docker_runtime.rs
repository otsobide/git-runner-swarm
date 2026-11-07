//! The Docker runtime against a real engine: runners launched as
//! `busybox` containers that sleep, which is enough to see them created,
//! listed, stopped, started and removed the way runners would be.
//!
//! The tests that need an engine are ignored unless asked for:
//! `cargo test -p swarm-docker -- --include-ignored`.

use std::os::unix::net::UnixListener;
use std::process::Command;

use bollard::Docker;
use bollard::models::RestartPolicyNameEnum;
use bollard::query_parameters::InspectContainerOptions;
use swarm_application::ports::{
    EnvironmentVariable, LaunchSpec, RestartPolicy, RunnerRuntime, RuntimeError,
};
use swarm_docker::DockerRuntime;
use swarm_domain::{
    Forge, ImageReference, RunnerDefinition, RunnerId, RunnerLabels, RunnerLifecycle, RunnerName,
    RunnerScope, RunnerStatus,
};

const IMAGE: &str = "busybox:1.37";

fn runtime() -> DockerRuntime {
    DockerRuntime::connect().unwrap().with_stop_timeout(5)
}

fn definition(name: &str) -> RunnerDefinition {
    RunnerDefinition::new(
        RunnerId::generate(),
        Forge::GitHub,
        RunnerScope::parse_url("https://github.com/otsobide/mindweaver").unwrap(),
        RunnerName::new(name).unwrap(),
        RunnerLabels::parse_comma_separated("integration-test").unwrap(),
        RunnerLifecycle::Persistent,
    )
}

fn sleeping() -> LaunchSpec {
    LaunchSpec {
        image: ImageReference::new(IMAGE).unwrap(),
        command: vec!["sleep".to_owned(), "300".to_owned()],
        environment: vec![
            EnvironmentVariable::plain("RUNNER_NAME", "builder"),
            EnvironmentVariable::secret("RUNNER_TOKEN", "AESOHC2S"),
        ],
        restart: RestartPolicy::UnlessStopped,
    }
}

/// Removes a test's container when the test ends, passed or not, with the
/// Docker CLI, as a drop cannot wait on the runtime.
struct Removed(RunnerId);

impl Drop for Removed {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "--force", &format!("git-runner-swarm-{}", self.0)])
            .output();
    }
}

/// A runner launched for a test, removed when the test ends.
async fn launched(runtime: &DockerRuntime, name: &str) -> (RunnerDefinition, Removed) {
    let definition = definition(name);
    let removed = Removed(definition.id());
    runtime
        .ensure_image(&ImageReference::new(IMAGE).unwrap())
        .await
        .unwrap();
    runtime.launch(&definition, &sleeping()).await.unwrap();
    (definition, removed)
}

async fn status(runtime: &DockerRuntime, id: RunnerId) -> RunnerStatus {
    runtime.find(id).await.unwrap().unwrap().status()
}

#[tokio::test]
#[ignore = "needs a Docker engine; run with --include-ignored"]
async fn pulls_an_image_and_refuses_one_that_does_not_exist() {
    let runtime = runtime();

    runtime
        .ensure_image(&ImageReference::new(IMAGE).unwrap())
        .await
        .unwrap();
    let missing =
        ImageReference::new("ghcr.io/otsobide/git-runner-swarm-no-such-image:never").unwrap();
    let error = runtime.ensure_image(&missing).await.unwrap_err();

    assert!(matches!(error, RuntimeError::ImageUnavailable { image, .. } if image == missing));
}

#[tokio::test]
#[ignore = "needs a Docker engine; run with --include-ignored"]
async fn launches_a_runner_running_and_lists_it() {
    let runtime = runtime();

    let (definition, _removed) = launched(&runtime, "launched").await;

    let runner = runtime.find(definition.id()).await.unwrap().unwrap();
    assert_eq!(runner.definition(), &definition);
    assert_eq!(runner.image().as_str(), IMAGE);
    assert_eq!(runner.status(), RunnerStatus::Running);
    let listed = runtime.list().await.unwrap();
    assert!(listed.iter().any(|runner| runner.id() == definition.id()));
}

#[tokio::test]
#[ignore = "needs a Docker engine; run with --include-ignored"]
async fn starts_the_container_as_the_spec_says() {
    let runtime = runtime();
    let (definition, _removed) = launched(&runtime, "as-specified").await;

    let container = Docker::connect_with_defaults()
        .unwrap()
        .inspect_container(
            &format!("git-runner-swarm-{}", definition.id()),
            None::<InspectContainerOptions>,
        )
        .await
        .unwrap();

    let config = container.config.unwrap();
    let environment = config.env.unwrap();
    assert!(environment.contains(&"RUNNER_NAME=builder".to_owned()));
    assert!(environment.contains(&"RUNNER_TOKEN=AESOHC2S".to_owned()));
    assert_eq!(config.cmd.unwrap(), ["sleep", "300"]);
    assert_eq!(
        config
            .labels
            .unwrap()
            .get("git-runner-swarm.managed")
            .map(String::as_str),
        Some("true")
    );
    let host = container.host_config.unwrap();
    assert_eq!(host.init, Some(true));
    assert_eq!(
        host.restart_policy.unwrap().name,
        Some(RestartPolicyNameEnum::UNLESS_STOPPED)
    );
}

#[tokio::test]
#[ignore = "needs a Docker engine; run with --include-ignored"]
async fn stops_and_starts_a_runner_once_or_twice_alike() {
    let runtime = runtime();
    let (definition, _removed) = launched(&runtime, "stopped").await;
    let id = definition.id();

    runtime.stop(id).await.unwrap();
    runtime.stop(id).await.unwrap();
    let stopped = runtime.find(id).await.unwrap().unwrap();
    assert_eq!(stopped.status(), RunnerStatus::Exited);
    assert!(stopped.exit_code().is_some());

    runtime.start(id).await.unwrap();
    runtime.start(id).await.unwrap();
    assert_eq!(status(&runtime, id).await, RunnerStatus::Running);
}

#[tokio::test]
#[ignore = "needs a Docker engine; run with --include-ignored"]
async fn removes_a_runner_and_then_knows_it_no_more() {
    let runtime = runtime();
    let (definition, _removed) = launched(&runtime, "removed").await;
    let id = definition.id();

    runtime.remove(id).await.unwrap();

    assert_eq!(runtime.find(id).await.unwrap(), None);
    assert!(
        !runtime
            .list()
            .await
            .unwrap()
            .iter()
            .any(|runner| runner.id() == id)
    );
    assert!(matches!(
        runtime.remove(id).await,
        Err(RuntimeError::NotFound(_))
    ));
    assert!(matches!(
        runtime.start(id).await,
        Err(RuntimeError::NotFound(_))
    ));
    assert!(matches!(
        runtime.stop(id).await,
        Err(RuntimeError::NotFound(_))
    ));
}

#[tokio::test]
async fn says_when_the_engine_cannot_be_reached() {
    // A socket nothing listens on any more, as an engine that is down
    // leaves behind.
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("docker.sock");
    drop(UnixListener::bind(&socket).unwrap());
    let docker =
        Docker::connect_with_socket(socket.to_str().unwrap(), 5, bollard::API_DEFAULT_VERSION)
            .unwrap();
    let runtime = DockerRuntime::new(docker);

    let error = runtime.list().await.unwrap_err();

    assert!(matches!(error, RuntimeError::Unreachable(_)), "{error:?}");
}
