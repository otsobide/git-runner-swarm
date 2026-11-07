//! How a runner's definition is kept with its container: as labels, which
//! Docker stores with the container and lists containers by.

use std::collections::HashMap;

use swarm_domain::{
    InvalidValue, RunnerDefinition, RunnerId, RunnerLabels, RunnerLifecycle, RunnerName,
    RunnerScope,
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
    HashMap::from([
        (MANAGED.to_owned(), "true".to_owned()),
        (ID.to_owned(), runner.id().to_string()),
        (FORGE.to_owned(), runner.forge().to_string()),
        (URL.to_owned(), runner.scope().to_string()),
        (NAME.to_owned(), runner.name().to_string()),
        (LABELS.to_owned(), runner.labels().to_comma_separated()),
        (LIFECYCLE.to_owned(), runner.lifecycle().to_string()),
    ])
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
