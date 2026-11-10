//! The use cases with real adapters behind their ports: GitHub's official
//! image as the runner image, and the in-memory runtime as the engine.
//! What the application's own fakes cannot show is that the adapters it
//! ships with fit its ports and carry a runner through its whole life.

use std::sync::Arc;

use swarm_application::ports::RunnerImage;
use swarm_application::{CreateRunner, RunnerError, RunnerImages, RunnerService};
use swarm_domain::{
    Forge, ImageReference, OwnerName, RegistrationToken, Runner, RunnerFilter, RunnerLabel,
    RunnerLabels, RunnerLifecycle, RunnerLimits, RunnerName, RunnerScope, RunnerStatus, ScopeKind,
};
use swarm_github::GitHubOfficialImage;
use swarm_memory::InMemoryRuntime;

fn service() -> (RunnerService, Arc<InMemoryRuntime>) {
    let runtime = Arc::new(InMemoryRuntime::new());
    let images =
        RunnerImages::new([Arc::new(GitHubOfficialImage::default()) as Arc<dyn RunnerImage>]);
    (RunnerService::new(runtime.clone(), images), runtime)
}

fn command(url: &str, name: &str, labels: &str) -> CreateRunner {
    CreateRunner {
        forge: Forge::GitHub,
        scope: RunnerScope::parse_url(url).unwrap(),
        name: RunnerName::new(name).unwrap(),
        labels: RunnerLabels::parse_comma_separated(labels).unwrap(),
        lifecycle: RunnerLifecycle::Persistent,
        limits: RunnerLimits::none(),
        token: RegistrationToken::new("AESOHC2SEXAMPLE").unwrap(),
        image: None,
    }
}

fn names(runners: &[Runner]) -> Vec<&str> {
    let mut names: Vec<&str> = runners
        .iter()
        .map(|runner| runner.definition().name().as_str())
        .collect();
    names.sort_unstable();
    names
}

#[tokio::test]
async fn creates_a_github_runner_from_the_official_image_pulled_first() {
    let (service, runtime) = service();

    let runner = service
        .create(command(
            "https://github.com/otsobide/mindweaver",
            "builder",
            "gpu",
        ))
        .await
        .unwrap();

    let official = ImageReference::new("ghcr.io/actions/actions-runner:latest").unwrap();
    assert_eq!(runner.image(), &official);
    assert!(runtime.has_image(&official));
    assert_eq!(runner.status(), RunnerStatus::Running);
    assert_eq!(
        runner.definition().scope().to_string(),
        "https://github.com/otsobide/mindweaver"
    );
}

#[tokio::test]
async fn creates_a_runner_from_the_image_asked_for_by_key() {
    let (service, _) = service();
    let mut command = command("https://github.com/otsobide", "shared", "");
    command.image = Some(GitHubOfficialImage::KEY.to_owned());
    command.lifecycle = RunnerLifecycle::Ephemeral;

    let runner = service.create(command).await.unwrap();

    assert_eq!(runner.definition().lifecycle(), RunnerLifecycle::Ephemeral);
    assert_eq!(runner.definition().scope().kind(), ScopeKind::Owner);
}

#[tokio::test]
async fn carries_a_runner_through_its_whole_life() {
    let (service, _) = service();
    let id = service
        .create(command(
            "https://github.com/otsobide/mindweaver",
            "builder",
            "",
        ))
        .await
        .unwrap()
        .id();

    let stopped = service.stop(id).await.unwrap();
    let started = service.start(id).await.unwrap();
    service.delete(id).await.unwrap();

    assert_eq!(
        (stopped.status(), stopped.exit_code()),
        (RunnerStatus::Exited, Some(0))
    );
    assert_eq!(started.status(), RunnerStatus::Running);
    assert!(matches!(
        service.get(id).await,
        Err(RunnerError::NotFound(_))
    ));
    assert!(service.list(&RunnerFilter::any()).await.unwrap().is_empty());
}

#[tokio::test]
async fn finds_runners_among_many_owners_by_what_tells_them_apart() {
    let (service, _) = service();
    for (url, name, labels) in [
        (
            "https://github.com/otsobide/mindweaver",
            "builder",
            "gpu,large",
        ),
        ("https://github.com/otsobide/mindweaver", "tester", ""),
        ("https://github.com/otsobide", "shared", "gpu"),
        ("https://github.com/acme/site", "deployer", "deploy"),
    ] {
        service.create(command(url, name, labels)).await.unwrap();
    }
    let tester = service
        .list(&RunnerFilter::any().with_search("tester"))
        .await
        .unwrap()[0]
        .id();
    service.stop(tester).await.unwrap();
    let list = |filter: RunnerFilter| {
        let service = &service;
        async move { service.list(&filter).await.unwrap() }
    };

    let owner = || RunnerFilter::any().with_owner(OwnerName::new("OTSOBIDE").unwrap());
    assert_eq!(names(&list(owner()).await), ["builder", "shared", "tester"]);
    assert_eq!(
        names(&list(owner().with_kind(ScopeKind::Owner)).await),
        ["shared"]
    );
    assert_eq!(
        names(&list(owner().with_label(RunnerLabel::new("gpu").unwrap())).await),
        ["builder", "shared"]
    );
    assert_eq!(
        names(&list(owner().with_status(RunnerStatus::Exited)).await),
        ["tester"]
    );
    assert_eq!(
        names(&list(RunnerFilter::any().with_search("acme")).await),
        ["deployer"]
    );
}

#[tokio::test]
async fn keeps_one_runner_per_name_in_a_scope_whatever_the_case() {
    let (service, _) = service();
    service
        .create(command(
            "https://github.com/otsobide/mindweaver",
            "builder",
            "",
        ))
        .await
        .unwrap();

    let again = service
        .create(command(
            "https://github.com/Otsobide/MindWeaver.git",
            "BUILDER",
            "",
        ))
        .await;
    let elsewhere = service
        .create(command("https://github.com/otsobide/other", "builder", ""))
        .await;

    assert!(matches!(again, Err(RunnerError::NameTaken { .. })));
    assert!(elsewhere.is_ok());
}

#[tokio::test]
async fn refuses_an_image_the_panel_was_not_given() {
    let (service, _) = service();
    let mut command = command("https://github.com/otsobide/mindweaver", "builder", "");
    command.image = Some("gitea-act-runner".to_owned());

    assert!(matches!(
        service.create(command).await,
        Err(RunnerError::UnknownImage(key)) if key == "gitea-act-runner"
    ));
}
