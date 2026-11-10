//! What the API takes and answers, as JSON, and how it is read into the
//! application's commands and the domain's value objects. Values are
//! checked by the value objects themselves, so the API refuses exactly
//! what the domain does, with the domain's reasons.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use swarm_application::CreateRunner;
use swarm_application::ports::RunnerImage;
use swarm_domain::{
    CpuLimit, InvalidValue, MemoryLimit, OwnerName, RegistrationToken, RepositoryName, Runner,
    RunnerFilter, RunnerLabel, RunnerLabels, RunnerLifecycle, RunnerLimits, RunnerName,
    RunnerScope,
};

/// A runner, as the API answers it.
#[derive(Debug, Serialize)]
pub(crate) struct RunnerResponse {
    id: String,
    forge: &'static str,
    /// The URL the runner registers with.
    url: String,
    owner: String,
    repository: Option<String>,
    /// `owner` or `repository`.
    scope: &'static str,
    name: String,
    labels: Vec<String>,
    lifecycle: &'static str,
    /// The most CPUs it may use, `1.5`; `null` for no limit.
    cpus: Option<String>,
    /// The most memory it may use, `4g`; `null` for no limit.
    memory: Option<String>,
    image: String,
    status: &'static str,
    exit_code: Option<i64>,
    /// Seconds since the Unix epoch.
    created_at: u64,
}

impl From<&Runner> for RunnerResponse {
    fn from(runner: &Runner) -> Self {
        let definition = runner.definition();
        let scope = definition.scope();
        Self {
            id: runner.id().to_string(),
            forge: definition.forge().as_str(),
            url: scope.to_string(),
            owner: scope.owner().to_string(),
            repository: scope.repository().map(ToString::to_string),
            scope: scope.kind().as_str(),
            name: definition.name().to_string(),
            labels: definition
                .labels()
                .iter()
                .map(ToString::to_string)
                .collect(),
            lifecycle: definition.lifecycle().as_str(),
            cpus: definition.limits().cpus().map(|cpus| cpus.to_string()),
            memory: definition
                .limits()
                .memory()
                .map(|memory| memory.to_string()),
            image: runner.image().to_string(),
            status: runner.status().as_str(),
            exit_code: runner.exit_code(),
            created_at: seconds_since_epoch(runner.created_at()),
        }
    }
}

fn seconds_since_epoch(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// A runner image runners can be created from, as the API answers it.
#[derive(Debug, Serialize)]
pub(crate) struct ImageResponse {
    key: String,
    forge: &'static str,
    image: String,
    description: String,
}

impl From<&dyn RunnerImage> for ImageResponse {
    fn from(image: &dyn RunnerImage) -> Self {
        Self {
            key: image.key().to_owned(),
            forge: image.forge().as_str(),
            image: image.image().to_string(),
            description: image.description().to_owned(),
        }
    }
}

/// What a runner is to be created as, as the API takes it. Its token is
/// never logged: the type has no `Debug`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateRunnerRequest {
    #[serde(default = "github")]
    forge: String,
    url: String,
    name: String,
    token: String,
    #[serde(default)]
    labels: Vec<String>,
    /// `persistent`, unless `ephemeral` is asked for.
    #[serde(default)]
    lifecycle: Option<String>,
    /// The most CPUs it may use, as a number or a string: `1.5`.
    #[serde(default)]
    cpus: Option<Cpus>,
    /// The most memory it may use: `4g`, `512m`.
    #[serde(default)]
    memory: Option<String>,
    /// The key of the image to create the runner from; the forge's default
    /// unless given.
    #[serde(default)]
    image: Option<String>,
}

fn github() -> String {
    "github".to_owned()
}

/// A number of CPUs, as JSON gives it: a number (`1.5`), or a string
/// (`"1.5"`), as a form sends it.
#[derive(Deserialize)]
#[serde(untagged)]
enum Cpus {
    Number(serde_json::Number),
    Text(String),
}

impl Cpus {
    fn into_text(self) -> String {
        match self {
            Cpus::Number(number) => number.to_string(),
            Cpus::Text(text) => text,
        }
    }
}

impl CreateRunnerRequest {
    /// The command the request asks for. Spaces around the name and the
    /// labels are ignored, as are empty labels and an empty image, as a
    /// form sends them.
    pub(crate) fn into_command(self) -> Result<CreateRunner, InvalidValue> {
        let labels = self
            .labels
            .iter()
            .map(|label| label.trim())
            .filter(|label| !label.is_empty())
            .map(RunnerLabel::new)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CreateRunner {
            forge: self.forge.parse()?,
            scope: RunnerScope::parse_url(&self.url)?,
            name: RunnerName::new(self.name.trim())?,
            labels: RunnerLabels::new(labels),
            lifecycle: present(self.lifecycle)
                .map_or(Ok(RunnerLifecycle::default()), |lifecycle| {
                    lifecycle.parse()
                })?,
            limits: RunnerLimits::new(
                present(self.cpus.map(Cpus::into_text))
                    .map(|cpus| CpuLimit::parse(&cpus))
                    .transpose()?,
                present(self.memory)
                    .map(|memory| MemoryLimit::parse(&memory))
                    .transpose()?,
            ),
            token: RegistrationToken::new(self.token)?,
            image: present(self.image),
        })
    }
}

/// Which runners to list, as query parameters, each optional. An empty
/// one is no criterion, as a form with a blank field sends it.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListQuery {
    forge: Option<String>,
    owner: Option<String>,
    repository: Option<String>,
    /// `owner` or `repository`.
    scope: Option<String>,
    status: Option<String>,
    label: Option<String>,
    /// Text to look for in runners' names and URLs.
    q: Option<String>,
}

impl ListQuery {
    pub(crate) fn into_filter(self) -> Result<RunnerFilter, InvalidValue> {
        let mut filter = RunnerFilter::any();
        if let Some(forge) = present(self.forge) {
            filter = filter.with_forge(forge.parse()?);
        }
        if let Some(owner) = present(self.owner) {
            filter = filter.with_owner(OwnerName::new(owner)?);
        }
        if let Some(repository) = present(self.repository) {
            filter = filter.with_repository(RepositoryName::new(repository)?);
        }
        if let Some(scope) = present(self.scope) {
            filter = filter.with_kind(scope.parse()?);
        }
        if let Some(status) = present(self.status) {
            filter = filter.with_status(status.parse()?);
        }
        if let Some(label) = present(self.label) {
            filter = filter.with_label(RunnerLabel::new(label)?);
        }
        if let Some(text) = present(self.q) {
            filter = filter.with_search(&text);
        }
        Ok(filter)
    }
}

/// The value, trimmed, unless there is none or it is blank.
fn present(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use swarm_domain::{
        Forge, ImageReference, RunnerDefinition, RunnerId, RunnerStatus, ScopeKind,
    };

    use super::*;

    fn request(body: Value) -> CreateRunnerRequest {
        serde_json::from_value(body).unwrap()
    }

    fn query(body: Value) -> ListQuery {
        serde_json::from_value(body).unwrap()
    }

    fn minimal() -> Value {
        json!({
            "url": "https://github.com/otsobide/mindweaver",
            "name": "builder",
            "token": "AESOHC2S",
        })
    }

    fn field_refused(body: Value) -> &'static str {
        request(body).into_command().map(drop).unwrap_err().field()
    }

    #[test]
    fn reads_a_minimal_request_with_the_defaults() {
        let command = request(minimal()).into_command().unwrap();

        assert_eq!(command.forge, Forge::GitHub);
        assert_eq!(
            command.scope.to_string(),
            "https://github.com/otsobide/mindweaver"
        );
        assert_eq!(command.name.as_str(), "builder");
        assert_eq!(command.token.expose(), "AESOHC2S");
        assert!(command.labels.is_empty());
        assert_eq!(command.lifecycle, RunnerLifecycle::Persistent);
        assert!(command.limits.is_none());
        assert_eq!(command.image, None);
    }

    #[test]
    fn reads_every_field_a_request_can_give() {
        let command = request(json!({
            "forge": "github",
            "url": "https://github.com/otsobide",
            "name": "  shared  ",
            "token": "AESOHC2S",
            "labels": ["gpu", " large ", ""],
            "lifecycle": "ephemeral",
            "cpus": "1.5",
            "memory": "4g",
            "image": "github-official",
        }))
        .into_command()
        .unwrap();

        assert_eq!(command.name.as_str(), "shared");
        assert_eq!(command.labels.to_comma_separated(), "gpu,large");
        assert_eq!(command.lifecycle, RunnerLifecycle::Ephemeral);
        assert_eq!(command.limits.cpus().unwrap().millis(), 1500);
        assert_eq!(command.limits.memory().unwrap().to_string(), "4g");
        assert_eq!(command.image.as_deref(), Some("github-official"));
    }

    #[test]
    fn reads_a_number_of_cpus_given_as_a_number() {
        let mut body = minimal();
        body["cpus"] = json!(0.5);

        let limits = request(body).into_command().unwrap().limits;

        assert_eq!(limits.cpus().unwrap().millis(), 500);
    }

    #[test]
    fn takes_blank_limits_for_none() {
        let mut body = minimal();
        body["cpus"] = json!(" ");
        body["memory"] = json!("");

        assert!(request(body).into_command().unwrap().limits.is_none());
    }

    #[test]
    fn takes_a_blank_image_for_the_forges_default() {
        let mut body = minimal();
        body["image"] = json!("  ");

        assert_eq!(request(body).into_command().unwrap().image, None);
    }

    #[test]
    fn refuses_a_request_with_the_domains_reasons() {
        for (field, value, refused) in [
            ("url", json!("github.com/otsobide"), "URL"),
            ("name", json!("two words"), "runner name"),
            ("token", json!(""), "registration token"),
            ("labels", json!(["gpu,large"]), "label"),
            ("lifecycle", json!("forever"), "lifecycle"),
            ("forge", json!("gitlab"), "forge"),
            ("cpus", json!("lots"), "CPU limit"),
            ("cpus", json!(-1), "CPU limit"),
            ("memory", json!("4x"), "memory limit"),
        ] {
            let mut body = minimal();
            body[field] = value;
            assert_eq!(field_refused(body), refused, "{field}");
        }
    }

    #[test]
    fn refuses_a_field_it_does_not_know() {
        let mut body = minimal();
        body["ephemeral"] = json!(true);

        assert!(serde_json::from_value::<CreateRunnerRequest>(body).is_err());
    }

    #[test]
    fn reads_no_criteria_from_an_empty_query() {
        assert_eq!(
            ListQuery::default().into_filter().unwrap(),
            RunnerFilter::any()
        );
        assert_eq!(
            query(json!({ "owner": "", "status": "  " }))
                .into_filter()
                .unwrap(),
            RunnerFilter::any()
        );
    }

    #[test]
    fn reads_every_criterion_a_query_can_give() {
        let filter = query(json!({
            "forge": "github",
            "owner": "otsobide",
            "repository": "mindweaver",
            "scope": "repository",
            "status": "running",
            "label": "gpu",
            "q": "build",
        }))
        .into_filter()
        .unwrap();

        let expected = RunnerFilter::any()
            .with_forge(Forge::GitHub)
            .with_owner(OwnerName::new("otsobide").unwrap())
            .with_repository(RepositoryName::new("mindweaver").unwrap())
            .with_kind(ScopeKind::Repository)
            .with_status(RunnerStatus::Running)
            .with_label(RunnerLabel::new("gpu").unwrap())
            .with_search("build");
        assert_eq!(filter, expected);
    }

    #[test]
    fn refuses_a_criterion_no_runner_could_meet() {
        let error = query(json!({ "status": "online" }))
            .into_filter()
            .unwrap_err();

        assert_eq!(error.field(), "status");
    }

    #[test]
    fn answers_a_runner_with_everything_a_panel_shows_of_it() {
        let definition = RunnerDefinition::new(
            "67e55044-10b1-426f-9247-bb680e5fe0c8"
                .parse::<RunnerId>()
                .unwrap(),
            Forge::GitHub,
            RunnerScope::parse_url("https://github.com/otsobide/mindweaver").unwrap(),
            RunnerName::new("builder").unwrap(),
            RunnerLabels::parse_comma_separated("gpu,large").unwrap(),
            RunnerLifecycle::Ephemeral,
        );
        let runner = Runner::new(
            definition,
            ImageReference::new("ghcr.io/actions/actions-runner:latest").unwrap(),
            RunnerStatus::Exited,
            UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000),
        )
        .with_exit_code(1);

        let answered = serde_json::to_value(RunnerResponse::from(&runner)).unwrap();

        assert_eq!(
            answered,
            json!({
                "id": "67e55044-10b1-426f-9247-bb680e5fe0c8",
                "forge": "github",
                "url": "https://github.com/otsobide/mindweaver",
                "owner": "otsobide",
                "repository": "mindweaver",
                "scope": "repository",
                "name": "builder",
                "labels": ["gpu", "large"],
                "lifecycle": "ephemeral",
                "cpus": null,
                "memory": null,
                "image": "ghcr.io/actions/actions-runner:latest",
                "status": "exited",
                "exit_code": 1,
                "created_at": 1_700_000_000,
            })
        );
    }

    #[test]
    fn answers_a_runners_limits_as_they_are_written() {
        let definition = RunnerDefinition::new(
            RunnerId::generate(),
            Forge::GitHub,
            RunnerScope::parse_url("https://github.com/otsobide").unwrap(),
            RunnerName::new("builder").unwrap(),
            RunnerLabels::default(),
            RunnerLifecycle::Persistent,
        )
        .with_limits(RunnerLimits::new(
            Some(CpuLimit::parse("0.50").unwrap()),
            Some(MemoryLimit::parse("4096m").unwrap()),
        ));
        let runner = Runner::new(
            definition,
            ImageReference::new("ghcr.io/actions/actions-runner:latest").unwrap(),
            RunnerStatus::Running,
            UNIX_EPOCH,
        );

        let answered = serde_json::to_value(RunnerResponse::from(&runner)).unwrap();

        assert_eq!(answered["cpus"], json!("0.5"));
        assert_eq!(answered["memory"], json!("4g"));
    }
}
