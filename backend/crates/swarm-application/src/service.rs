use std::fmt;
use std::sync::Arc;

use swarm_domain::{
    Forge, RegistrationToken, Runner, RunnerDefinition, RunnerFilter, RunnerId, RunnerLabels,
    RunnerLifecycle, RunnerName, RunnerScope,
};
use thiserror::Error;

use crate::RunnerImages;
use crate::ports::{RunnerImage, RunnerRuntime, RuntimeError};

/// What the panel does with runners: each method is one use case.
pub struct RunnerService {
    runtime: Arc<dyn RunnerRuntime>,
    images: RunnerImages,
}

/// What a runner is to be created as, and the token it registers with.
#[derive(Debug, Clone)]
pub struct CreateRunner {
    pub forge: Forge,
    pub scope: RunnerScope,
    pub name: RunnerName,
    pub labels: RunnerLabels,
    pub lifecycle: RunnerLifecycle,
    pub token: RegistrationToken,
    /// The key of the image to create it from; its forge's default if
    /// `None`.
    pub image: Option<String>,
}

/// Why a use case could not be carried out.
#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("there is no runner {0}")]
    NotFound(RunnerId),

    #[error("a runner named {name} already serves {scope}")]
    NameTaken {
        name: RunnerName,
        scope: RunnerScope,
    },

    #[error("there is no runner image `{0}`")]
    UnknownImage(String),

    #[error("there is no runner image for {0}")]
    NoImageForForge(Forge),

    #[error("the runner image `{image}` runs runners for {serves}, not {asked}")]
    ImageForgeMismatch {
        image: String,
        serves: Forge,
        asked: Forge,
    },

    #[error(transparent)]
    Runtime(RuntimeError),
}

impl From<RuntimeError> for RunnerError {
    fn from(error: RuntimeError) -> Self {
        match error {
            RuntimeError::NotFound(id) => RunnerError::NotFound(id),
            error => RunnerError::Runtime(error),
        }
    }
}

impl fmt::Debug for RunnerService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunnerService")
            .field("images", &self.images)
            .finish_non_exhaustive()
    }
}

impl RunnerService {
    pub fn new(runtime: Arc<dyn RunnerRuntime>, images: RunnerImages) -> Self {
        Self { runtime, images }
    }

    /// The images runners can be created from.
    pub fn images(&self) -> &RunnerImages {
        &self.images
    }

    /// Creates a runner and starts it, pulling its image first if the
    /// runtime does not have it.
    ///
    /// # Errors
    ///
    /// When the image asked for does not exist or serves another forge,
    /// when there is no image for the forge, when another runner already
    /// goes by the name in the scope (registering would replace it on the
    /// forge), or when the runtime fails to pull the image or launch the
    /// runner.
    pub async fn create(&self, command: CreateRunner) -> Result<Runner, RunnerError> {
        let image = self.image_for(command.forge, command.image.as_deref())?;
        let definition = RunnerDefinition::new(
            RunnerId::generate(),
            command.forge,
            command.scope,
            command.name,
            command.labels,
            command.lifecycle,
        );

        let runners = self.runtime.list().await?;
        if runners
            .iter()
            .any(|runner| runner.definition().conflicts_with(&definition))
        {
            return Err(RunnerError::NameTaken {
                name: definition.name().clone(),
                scope: definition.scope().clone(),
            });
        }

        let spec = image.launch_spec(&definition, &command.token);
        self.runtime.ensure_image(&spec.image).await?;
        Ok(self.runtime.launch(&definition, &spec).await?)
    }

    /// The runners `filter` matches, newest first, and by name among
    /// runners created at once.
    ///
    /// # Errors
    ///
    /// When the runtime cannot list its runners.
    pub async fn list(&self, filter: &RunnerFilter) -> Result<Vec<Runner>, RunnerError> {
        let mut runners: Vec<Runner> = self
            .runtime
            .list()
            .await?
            .into_iter()
            .filter(|runner| filter.matches(runner))
            .collect();
        runners.sort_by(|a, b| {
            b.created_at().cmp(&a.created_at()).then_with(|| {
                a.definition()
                    .name()
                    .as_str()
                    .cmp(b.definition().name().as_str())
            })
        });
        Ok(runners)
    }

    /// The runner with this id.
    ///
    /// # Errors
    ///
    /// When there is no such runner, or the runtime cannot be asked.
    pub async fn get(&self, id: RunnerId) -> Result<Runner, RunnerError> {
        self.runtime
            .find(id)
            .await?
            .ok_or(RunnerError::NotFound(id))
    }

    fn image_for(&self, forge: Forge, key: Option<&str>) -> Result<&dyn RunnerImage, RunnerError> {
        let Some(key) = key else {
            return self
                .images
                .default_for(forge)
                .ok_or(RunnerError::NoImageForForge(forge));
        };
        let image = self
            .images
            .find(key)
            .ok_or_else(|| RunnerError::UnknownImage(key.to_owned()))?;
        if image.forge() != forge {
            return Err(RunnerError::ImageForgeMismatch {
                image: key.to_owned(),
                serves: image.forge(),
                asked: forge,
            });
        }
        Ok(image)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use swarm_domain::{ImageReference, OwnerName, RunnerStatus};

    use super::*;
    use crate::testing::{FakeImage, FakeRuntime};

    fn service(runtime: &Arc<FakeRuntime>) -> RunnerService {
        RunnerService::new(
            runtime.clone(),
            RunnerImages::new([
                Arc::new(FakeImage::new("github-official", Forge::GitHub)) as Arc<dyn RunnerImage>,
                Arc::new(FakeImage::new("github-custom", Forge::GitHub)),
            ]),
        )
    }

    fn command(url: &str, name: &str) -> CreateRunner {
        CreateRunner {
            forge: Forge::GitHub,
            scope: RunnerScope::parse_url(url).unwrap(),
            name: RunnerName::new(name).unwrap(),
            labels: RunnerLabels::parse_comma_separated("gpu").unwrap(),
            lifecycle: RunnerLifecycle::Persistent,
            token: RegistrationToken::new("AESOHC2S").unwrap(),
            image: None,
        }
    }

    fn existing(url: &str, name: &str) -> Runner {
        existing_at(url, name, 0)
    }

    fn existing_at(url: &str, name: &str, created_secs: u64) -> Runner {
        Runner::new(
            RunnerDefinition::new(
                RunnerId::generate(),
                Forge::GitHub,
                RunnerScope::parse_url(url).unwrap(),
                RunnerName::new(name).unwrap(),
                RunnerLabels::default(),
                RunnerLifecycle::Persistent,
            ),
            ImageReference::new("github-official:latest").unwrap(),
            RunnerStatus::Running,
            SystemTime::UNIX_EPOCH + Duration::from_secs(created_secs),
        )
    }

    fn names(runners: &[Runner]) -> Vec<&str> {
        runners
            .iter()
            .map(|runner| runner.definition().name().as_str())
            .collect()
    }

    #[tokio::test]
    async fn creates_a_runner_from_its_forges_default_image() {
        let runtime = Arc::new(FakeRuntime::default());

        let runner = service(&runtime)
            .create(command("https://github.com/otsobide/mindweaver", "builder"))
            .await
            .unwrap();

        assert_eq!(runner.definition().name().as_str(), "builder");
        assert_eq!(runner.image().as_str(), "github-official:latest");
        assert_eq!(runtime.runner(runner.id()), Some(runner));
    }

    #[tokio::test]
    async fn pulls_the_image_before_launching_the_runner() {
        let runtime = Arc::new(FakeRuntime::default());

        service(&runtime)
            .create(command("https://github.com/otsobide/mindweaver", "builder"))
            .await
            .unwrap();

        let image = ImageReference::new("github-official:latest").unwrap();
        assert_eq!(runtime.launched()[0].image, image);
        assert_eq!(runtime.pulled(), [image]);
    }

    #[tokio::test]
    async fn launches_the_runner_as_its_image_says_with_its_token() {
        let runtime = Arc::new(FakeRuntime::default());

        service(&runtime)
            .create(command("https://github.com/otsobide/mindweaver", "builder"))
            .await
            .unwrap();

        let spec = &runtime.launched()[0];
        let token = spec
            .environment
            .iter()
            .find(|variable| variable.name() == "RUNNER_TOKEN")
            .unwrap();
        assert_eq!(token.value(), "AESOHC2S");
        assert!(token.is_secret());
    }

    #[tokio::test]
    async fn creates_a_runner_from_the_image_asked_for() {
        let runtime = Arc::new(FakeRuntime::default());
        let mut command = command("https://github.com/otsobide/mindweaver", "builder");
        command.image = Some("github-custom".to_owned());

        let runner = service(&runtime).create(command).await.unwrap();

        assert_eq!(runner.image().as_str(), "github-custom:latest");
    }

    #[tokio::test]
    async fn refuses_an_image_it_does_not_have() {
        let runtime = Arc::new(FakeRuntime::default());
        let mut command = command("https://github.com/otsobide/mindweaver", "builder");
        command.image = Some("gitea".to_owned());

        let error = service(&runtime).create(command).await.unwrap_err();

        assert!(matches!(error, RunnerError::UnknownImage(key) if key == "gitea"));
        assert!(runtime.launched().is_empty());
    }

    #[tokio::test]
    async fn refuses_a_forge_it_has_no_image_for() {
        let runtime = Arc::new(FakeRuntime::default());
        let service = RunnerService::new(runtime.clone(), RunnerImages::default());

        let error = service
            .create(command("https://github.com/otsobide/mindweaver", "builder"))
            .await
            .unwrap_err();

        assert!(matches!(error, RunnerError::NoImageForForge(Forge::GitHub)));
    }

    #[tokio::test]
    async fn refuses_a_name_another_runner_goes_by_in_the_scope() {
        let runtime = Arc::new(FakeRuntime::holding([existing(
            "https://github.com/otsobide/mindweaver",
            "builder",
        )]));

        let error = service(&runtime)
            .create(command("https://github.com/Otsobide/mindweaver", "Builder"))
            .await
            .unwrap_err();

        assert!(matches!(error, RunnerError::NameTaken { .. }));
        assert!(runtime.pulled().is_empty());
        assert!(runtime.launched().is_empty());
    }

    #[tokio::test]
    async fn takes_a_name_another_runner_goes_by_elsewhere() {
        let runtime = Arc::new(FakeRuntime::holding([existing(
            "https://github.com/otsobide/other",
            "builder",
        )]));

        let created = service(&runtime)
            .create(command("https://github.com/otsobide/mindweaver", "builder"))
            .await;

        assert!(created.is_ok());
    }

    #[tokio::test]
    async fn launches_nothing_when_the_image_cannot_be_pulled() {
        let runtime = Arc::new(FakeRuntime::failing_pulls());

        let error = service(&runtime)
            .create(command("https://github.com/otsobide/mindweaver", "builder"))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            RunnerError::Runtime(RuntimeError::ImageUnavailable { .. })
        ));
        assert!(runtime.launched().is_empty());
    }

    #[tokio::test]
    async fn lists_the_runners_a_filter_matches() {
        let runtime = Arc::new(FakeRuntime::holding([
            existing("https://github.com/otsobide/mindweaver", "builder"),
            existing("https://github.com/someone/else", "builder"),
        ]));
        let filter = RunnerFilter::any().with_owner(OwnerName::new("otsobide").unwrap());

        let runners = service(&runtime).list(&filter).await.unwrap();

        assert_eq!(runners.len(), 1);
        assert_eq!(runners[0].definition().scope().owner().as_str(), "otsobide");
    }

    #[tokio::test]
    async fn lists_the_newest_runners_first_and_by_name_among_equals() {
        let runtime = Arc::new(FakeRuntime::holding([
            existing_at("https://github.com/otsobide/mindweaver", "oldest", 1),
            existing_at("https://github.com/otsobide/mindweaver", "newest-b", 3),
            existing_at("https://github.com/otsobide/mindweaver", "newest-a", 3),
        ]));

        let runners = service(&runtime).list(&RunnerFilter::any()).await.unwrap();

        assert_eq!(names(&runners), ["newest-a", "newest-b", "oldest"]);
    }

    #[tokio::test]
    async fn gets_a_runner_by_its_id() {
        let runner = existing("https://github.com/otsobide/mindweaver", "builder");
        let runtime = Arc::new(FakeRuntime::holding([runner.clone()]));

        assert_eq!(service(&runtime).get(runner.id()).await.unwrap(), runner);
    }

    #[tokio::test]
    async fn says_when_there_is_no_runner_with_an_id() {
        let runtime = Arc::new(FakeRuntime::default());
        let id = RunnerId::generate();

        let error = service(&runtime).get(id).await.unwrap_err();

        assert!(matches!(error, RunnerError::NotFound(missing) if missing == id));
    }
}
