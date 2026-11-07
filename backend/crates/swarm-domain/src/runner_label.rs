use std::fmt;

use crate::InvalidValue;
use crate::validate::check_name;

/// A label a runner registers with, which a workflow's `runs-on` selects
/// it by.
///
/// 1 to [`RunnerLabel::MAX_LENGTH`] characters: ASCII letters, digits, `-`,
/// `_` and `.`. Never a comma, which is what separates labels when a runner
/// registers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunnerLabel(String);

impl RunnerLabel {
    /// The longest label a runner can have.
    pub const MAX_LENGTH: usize = 100;

    /// # Errors
    ///
    /// When the label is empty, longer than [`RunnerLabel::MAX_LENGTH`], or
    /// holds a character other than an ASCII letter, a digit, `-`, `_` or
    /// `.`.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidValue> {
        let value = value.into();
        check_name("label", &value, Self::MAX_LENGTH)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether two labels select the same runners: forges match labels
    /// without case.
    pub fn same_as(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl fmt::Display for RunnerLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The labels a runner registers with, beyond those its forge gives every
/// runner (`self-hosted`, its OS, its architecture).
///
/// Each label is held once, the first spelling given kept: two that differ
/// only in case are the same label to a forge.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunnerLabels(Vec<RunnerLabel>);

impl RunnerLabels {
    /// The labels given, each kept once, in the order given.
    pub fn new(labels: impl IntoIterator<Item = RunnerLabel>) -> Self {
        let mut kept: Vec<RunnerLabel> = Vec::new();
        for label in labels {
            if !kept.iter().any(|held| held.same_as(&label)) {
                kept.push(label);
            }
        }
        Self(kept)
    }

    /// Labels written as forges take them when a runner registers:
    /// separated by commas, spaces around each ignored, empty ones skipped.
    ///
    /// # Errors
    ///
    /// When a label between the commas is not one [`RunnerLabel::new`]
    /// accepts.
    pub fn parse_comma_separated(value: &str) -> Result<Self, InvalidValue> {
        value
            .split(',')
            .map(str::trim)
            .filter(|label| !label.is_empty())
            .map(RunnerLabel::new)
            .collect::<Result<Vec<_>, _>>()
            .map(Self::new)
    }

    /// The labels separated by commas, as forges take them when a runner
    /// registers. Empty when there are none.
    pub fn to_comma_separated(&self) -> String {
        self.0
            .iter()
            .map(RunnerLabel::as_str)
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Whether a runner registered with these labels has `label`.
    pub fn contains(&self, label: &RunnerLabel) -> bool {
        self.0.iter().any(|held| held.same_as(label))
    }

    pub fn iter(&self) -> impl Iterator<Item = &RunnerLabel> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(value: &str) -> RunnerLabel {
        RunnerLabel::new(value).unwrap()
    }

    #[test]
    fn refuses_a_label_with_a_comma_or_a_space() {
        for value in ["gpu,large", "two words", ""] {
            let error = RunnerLabel::new(value).unwrap_err();
            assert_eq!(error.field(), "label");
        }
    }

    #[test]
    fn keeps_each_label_once_whatever_its_case() {
        let labels = RunnerLabels::new([label("gpu"), label("GPU"), label("linux-arm")]);

        assert_eq!(labels.len(), 2);
        assert_eq!(labels.to_comma_separated(), "gpu,linux-arm");
    }

    #[test]
    fn reads_labels_separated_by_commas() {
        let labels = RunnerLabels::parse_comma_separated(" gpu , large,,docker ").unwrap();

        assert_eq!(labels.to_comma_separated(), "gpu,large,docker");
    }

    #[test]
    fn reads_no_labels_from_an_empty_list() {
        let labels = RunnerLabels::parse_comma_separated("").unwrap();

        assert!(labels.is_empty());
        assert_eq!(labels.to_comma_separated(), "");
    }

    #[test]
    fn refuses_a_list_holding_a_label_it_would_refuse_alone() {
        assert!(RunnerLabels::parse_comma_separated("gpu,two words").is_err());
    }

    #[test]
    fn finds_a_label_whatever_its_case() {
        let labels = RunnerLabels::new([label("Docker")]);

        assert!(labels.contains(&label("docker")));
        assert!(!labels.contains(&label("gpu")));
    }
}
