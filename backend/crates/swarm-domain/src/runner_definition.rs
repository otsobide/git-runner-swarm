use crate::{
    Forge, RunnerId, RunnerLabels, RunnerLifecycle, RunnerLimits, RunnerName, RunnerScope,
};

/// What a runner is asked to be when it is created: everything about it
/// that stays the same for as long as it exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerDefinition {
    id: RunnerId,
    forge: Forge,
    scope: RunnerScope,
    name: RunnerName,
    labels: RunnerLabels,
    lifecycle: RunnerLifecycle,
    limits: RunnerLimits,
}

impl RunnerDefinition {
    pub fn new(
        id: RunnerId,
        forge: Forge,
        scope: RunnerScope,
        name: RunnerName,
        labels: RunnerLabels,
        lifecycle: RunnerLifecycle,
    ) -> Self {
        Self {
            id,
            forge,
            scope,
            name,
            labels,
            lifecycle,
            limits: RunnerLimits::none(),
        }
    }

    /// The same runner, limited to `limits` of its host.
    #[must_use]
    pub fn with_limits(mut self, limits: RunnerLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn id(&self) -> RunnerId {
        self.id
    }

    pub fn forge(&self) -> Forge {
        self.forge
    }

    pub fn scope(&self) -> &RunnerScope {
        &self.scope
    }

    pub fn name(&self) -> &RunnerName {
        &self.name
    }

    pub fn labels(&self) -> &RunnerLabels {
        &self.labels
    }

    pub fn lifecycle(&self) -> RunnerLifecycle {
        self.lifecycle
    }

    /// What it may use of its host; nothing is limited unless asked.
    pub fn limits(&self) -> RunnerLimits {
        self.limits
    }

    /// Whether `other` is another runner that would register under this
    /// one's name in this one's scope, which a forge allows only one runner
    /// to do: the second would take the first one's place.
    pub fn conflicts_with(&self, other: &Self) -> bool {
        self.id != other.id
            && self.forge == other.forge
            && self.scope.same_as(&other.scope)
            && self.name.same_as(&other.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition(url: &str, name: &str) -> RunnerDefinition {
        RunnerDefinition::new(
            RunnerId::generate(),
            Forge::GitHub,
            RunnerScope::parse_url(url).unwrap(),
            RunnerName::new(name).unwrap(),
            RunnerLabels::default(),
            RunnerLifecycle::Persistent,
        )
    }

    #[test]
    fn conflicts_with_another_runner_of_the_same_name_in_the_same_scope() {
        let first = definition("https://github.com/otsobide/mindweaver", "builder");
        let second = definition("https://github.com/Otsobide/MindWeaver", "Builder");

        assert!(first.conflicts_with(&second));
        assert!(second.conflicts_with(&first));
    }

    #[test]
    fn limits_nothing_unless_told_to() {
        let runner = definition("https://github.com/otsobide/mindweaver", "builder");

        assert!(runner.limits().is_none());
    }

    #[test]
    fn keeps_the_limits_it_is_given() {
        let limits = RunnerLimits::new(
            Some(crate::CpuLimit::parse("1.5").unwrap()),
            Some(crate::MemoryLimit::parse("4g").unwrap()),
        );

        let runner =
            definition("https://github.com/otsobide/mindweaver", "builder").with_limits(limits);

        assert_eq!(runner.limits(), limits);
    }

    #[test]
    fn does_not_conflict_with_itself() {
        let runner = definition("https://github.com/otsobide/mindweaver", "builder");

        assert!(!runner.conflicts_with(&runner.clone()));
    }

    #[test]
    fn does_not_conflict_with_a_runner_of_the_same_name_elsewhere() {
        let runner = definition("https://github.com/otsobide/mindweaver", "builder");

        for elsewhere in [
            definition("https://github.com/otsobide/other", "builder"),
            definition("https://github.com/otsobide", "builder"),
            definition("https://ghe.example.com/otsobide/mindweaver", "builder"),
        ] {
            assert!(!runner.conflicts_with(&elsewhere), "{}", elsewhere.scope());
        }
    }

    #[test]
    fn does_not_conflict_with_a_runner_of_another_name_in_the_same_scope() {
        let runner = definition("https://github.com/otsobide/mindweaver", "builder");

        assert!(!runner.conflicts_with(&definition(
            "https://github.com/otsobide/mindweaver",
            "builder-2"
        )));
    }
}
