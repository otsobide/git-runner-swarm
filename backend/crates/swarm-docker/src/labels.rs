//! How a runner's definition is kept with its container: as labels, which
//! Docker stores with the container and lists containers by.
//!
//! Labels added since the first release are optional, read as the default
//! their value has when missing, so a container made by an earlier panel is
//! read as the runner it was then.

use std::collections::HashMap;

use swarm_domain::{
    CpuLimit, EngineAccess, InvalidValue, MemoryLimit, RunnerDefinition, RunnerId, RunnerLabels,
    RunnerLifecycle, RunnerLimits, RunnerName, RunnerScope,
};
use thiserror::Error;

/// Every container of a runner carries this label, set to `true`: what the
/// runtime lists its runners by, leaving every other container alone.
pub(crate) const MANAGED: &str = "git-runner-swarm.managed";

const ID: &str = "git-runner-swarm.id";
const FORGE: &str = "git-runner-swarm.forge";
const URL: &str = "git-runner-swarm.url";
const NAME: &str = "git-runner-swarm.name";
const LABELS: &str = "git-runner-swarm.labels";
const LIFECYCLE: &str = "git-runner-swarm.lifecycle";
/// Since 0.2: the CPU limit, as `CpuLimit` writes it; none when missing.
const CPUS: &str = "git-runner-swarm.cpus";
/// Since 0.2: the memory limit, as `MemoryLimit` writes it; none when
/// missing.
const MEMORY: &str = "git-runner-swarm.memory";
/// Since 0.2: `host` when the runner's jobs are given the engine; isolated
/// when missing.
const ENGINE: &str = "git-runner-swarm.engine";

/// Why a container's labels could not be read back into a runner's
/// definition.
#[derive(Debug, Error)]
pub(crate) enum UnreadableLabels {
    #[error("it is not a runner's container: it has no `{MANAGED}=true` label")]
    NotManaged,

    #[error("it has no `{0}` label")]
    Missing(&'static str),

    #[error("its `{label}` label holds an {value}")]
    Invalid {
        label: &'static str,
        value: InvalidValue,
    },
}

/// The labels a runner's container carries its definition in.
pub(crate) fn encode(runner: &RunnerDefinition) -> HashMap<String, String> {
    let mut labels = HashMap::from([
        (MANAGED.to_owned(), "true".to_owned()),
        (ID.to_owned(), runner.id().to_string()),
        (FORGE.to_owned(), runner.forge().to_string()),
        (URL.to_owned(), runner.scope().to_string()),
        (NAME.to_owned(), runner.name().to_string()),
        (LABELS.to_owned(), runner.labels().to_comma_separated()),
        (LIFECYCLE.to_owned(), runner.lifecycle().to_string()),
    ]);
    let limits = runner.limits();
    if let Some(cpus) = limits.cpus() {
        labels.insert(CPUS.to_owned(), cpus.to_string());
    }
    if let Some(memory) = limits.memory() {
        labels.insert(MEMORY.to_owned(), memory.to_string());
    }
    if runner.engine_access().is_host() {
        labels.insert(ENGINE.to_owned(), runner.engine_access().to_string());
    }
    labels
}

/// The definition a runner's container carries in its labels.
pub(crate) fn decode(
    labels: &HashMap<String, String>,
) -> Result<RunnerDefinition, UnreadableLabels> {
    if labels.get(MANAGED).map(String::as_str) != Some("true") {
        return Err(UnreadableLabels::NotManaged);
    }
    Ok(RunnerDefinition::new(
        read(labels, ID, str::parse::<RunnerId>)?,
        read(labels, FORGE, str::parse)?,
        read(labels, URL, RunnerScope::parse_url)?,
        read(labels, NAME, RunnerName::new)?,
        read(labels, LABELS, RunnerLabels::parse_comma_separated)?,
        read(labels, LIFECYCLE, str::parse::<RunnerLifecycle>)?,
    )
    .with_limits(RunnerLimits::new(
        read_optional(labels, CPUS, CpuLimit::parse)?,
        read_optional(labels, MEMORY, MemoryLimit::parse)?,
    ))
    .with_engine_access(
        read_optional(labels, ENGINE, str::parse::<EngineAccess>)?.unwrap_or_default(),
    ))
}

fn read<'a, T>(
    labels: &'a HashMap<String, String>,
    label: &'static str,
    parse: impl FnOnce(&'a str) -> Result<T, InvalidValue>,
) -> Result<T, UnreadableLabels> {
    let value = labels.get(label).ok_or(UnreadableLabels::Missing(label))?;
    parse(value).map_err(|value| UnreadableLabels::Invalid { label, value })
}

/// A label that may be missing, read as `None` if it is.
fn read_optional<'a, T>(
    labels: &'a HashMap<String, String>,
    label: &'static str,
    parse: impl FnOnce(&'a str) -> Result<T, InvalidValue>,
) -> Result<Option<T>, UnreadableLabels> {
    labels
        .get(label)
        .map(|value| parse(value).map_err(|value| UnreadableLabels::Invalid { label, value }))
        .transpose()
}

#[cfg(test)]
mod tests {
    use swarm_domain::Forge;

    use super::*;

    fn definition() -> RunnerDefinition {
        RunnerDefinition::new(
            RunnerId::generate(),
            Forge::GitHub,
            RunnerScope::parse_url("https://github.com/otsobide/mindweaver").unwrap(),
            RunnerName::new("builder").unwrap(),
            RunnerLabels::parse_comma_separated("gpu,large").unwrap(),
            RunnerLifecycle::Ephemeral,
        )
    }

    #[test]
    fn reads_back_the_definition_it_writes() {
        let definition = definition();

        assert_eq!(decode(&encode(&definition)).unwrap(), definition);
    }

    #[test]
    fn marks_the_container_as_a_runners() {
        assert_eq!(
            encode(&definition()).get(MANAGED).map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn keeps_a_runner_without_labels_without_any() {
        let definition = RunnerDefinition::new(
            RunnerId::generate(),
            Forge::GitHub,
            RunnerScope::parse_url("https://github.com/otsobide").unwrap(),
            RunnerName::new("shared").unwrap(),
            RunnerLabels::default(),
            RunnerLifecycle::Persistent,
        );

        assert_eq!(decode(&encode(&definition)).unwrap(), definition);
    }

    #[test]
    fn reads_back_the_limits_it_writes() {
        let limits = RunnerLimits::new(
            Some(CpuLimit::parse("1.5").unwrap()),
            Some(MemoryLimit::parse("4g").unwrap()),
        );
        let definition = definition().with_limits(limits);

        let labels = encode(&definition);

        assert_eq!(labels.get(CPUS).map(String::as_str), Some("1.5"));
        assert_eq!(labels.get(MEMORY).map(String::as_str), Some("4g"));
        assert_eq!(decode(&labels).unwrap(), definition);
    }

    #[test]
    fn writes_no_limit_labels_for_a_runner_without_limits() {
        let labels = encode(&definition());

        assert!(!labels.contains_key(CPUS));
        assert!(!labels.contains_key(MEMORY));
    }

    #[test]
    fn reads_a_container_of_the_first_release_as_a_runner_without_limits() {
        let labels: HashMap<String, String> = [
            (MANAGED, "true"),
            (ID, "67e55044-10b1-426f-9247-bb680e5fe0c8"),
            (FORGE, "github"),
            (URL, "https://github.com/otsobide/mindweaver"),
            (NAME, "builder"),
            (LABELS, "gpu"),
            (LIFECYCLE, "persistent"),
        ]
        .into_iter()
        .map(|(label, value)| (label.to_owned(), value.to_owned()))
        .collect();

        let definition = decode(&labels).unwrap();

        assert_eq!(definition.name().as_str(), "builder");
        assert!(definition.limits().is_none());
    }

    #[test]
    fn reads_back_a_runner_given_the_engine() {
        let definition = definition().with_engine_access(EngineAccess::Host);

        let labels = encode(&definition);

        assert_eq!(labels.get(ENGINE).map(String::as_str), Some("host"));
        assert_eq!(decode(&labels).unwrap(), definition);
    }

    #[test]
    fn reads_a_runner_without_an_engine_label_as_isolated() {
        let labels = encode(&definition());

        assert!(!labels.contains_key(ENGINE));
        assert_eq!(
            decode(&labels).unwrap().engine_access(),
            EngineAccess::Isolated
        );
    }

    #[test]
    fn says_which_limit_label_holds_what_it_should_not() {
        let mut labels = encode(&definition());
        labels.insert(MEMORY.to_owned(), "lots".to_owned());

        assert!(matches!(
            decode(&labels),
            Err(UnreadableLabels::Invalid { label: MEMORY, .. })
        ));
    }

    #[test]
    fn refuses_a_container_that_is_not_a_runners() {
        let mut labels = encode(&definition());
        labels.remove(MANAGED);

        assert!(matches!(decode(&labels), Err(UnreadableLabels::NotManaged)));
    }

    #[test]
    fn says_which_label_is_missing() {
        let mut labels = encode(&definition());
        labels.remove(URL);

        assert!(matches!(
            decode(&labels),
            Err(UnreadableLabels::Missing(URL))
        ));
    }

    #[test]
    fn says_which_label_holds_what_it_should_not() {
        let mut labels = encode(&definition());
        labels.insert(NAME.to_owned(), "two words".to_owned());

        let error = decode(&labels).unwrap_err();

        assert!(matches!(
            error,
            UnreadableLabels::Invalid { label: NAME, .. }
        ));
        assert!(
            error
                .to_string()
                .starts_with("its `git-runner-swarm.name` label holds an invalid runner name")
        );
    }
}
