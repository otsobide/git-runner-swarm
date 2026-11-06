use crate::{Forge, OwnerName, RepositoryName, Runner, RunnerLabel, RunnerStatus, ScopeKind};

/// Which runners to list: those that meet every criterion given.
///
/// A filter with no criteria matches every runner. Names are compared
/// without case, as forges compare them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunnerFilter {
    forge: Option<Forge>,
    owner: Option<OwnerName>,
    repository: Option<RepositoryName>,
    kind: Option<ScopeKind>,
    status: Option<RunnerStatus>,
    label: Option<RunnerLabel>,
    search: Option<String>,
}

impl RunnerFilter {
    /// A filter that matches every runner, for the criteria below to
    /// narrow.
    pub fn any() -> Self {
        Self::default()
    }

    /// Runners of this forge.
    #[must_use]
    pub fn with_forge(mut self, forge: Forge) -> Self {
        self.forge = Some(forge);
        self
    }

    /// Runners serving this user or organization, or a repository of it.
    #[must_use]
    pub fn with_owner(mut self, owner: OwnerName) -> Self {
        self.owner = Some(owner);
        self
    }

    /// Runners serving a repository of this name, whoever owns it.
    #[must_use]
    pub fn with_repository(mut self, repository: RepositoryName) -> Self {
        self.repository = Some(repository);
        self
    }

    /// Runners serving a whole owner, or a single repository.
    #[must_use]
    pub fn with_kind(mut self, kind: ScopeKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Runners whose container is in this state.
    #[must_use]
    pub fn with_status(mut self, status: RunnerStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Runners registered with this label.
    #[must_use]
    pub fn with_label(mut self, label: RunnerLabel) -> Self {
        self.label = Some(label);
        self
    }

    /// Runners whose name or URL holds this text. Spaces around it are
    /// ignored, and nothing but spaces narrows nothing.
    #[must_use]
    pub fn with_search(mut self, text: &str) -> Self {
        let text = text.trim();
        self.search = (!text.is_empty()).then(|| text.to_lowercase());
        self
    }

    pub fn matches(&self, runner: &Runner) -> bool {
        let definition = runner.definition();
        let scope = definition.scope();

        self.forge.is_none_or(|forge| definition.forge() == forge)
            && self
                .owner
                .as_ref()
                .is_none_or(|owner| scope.owner().same_as(owner))
            && self.repository.as_ref().is_none_or(|repository| {
                scope
                    .repository()
                    .is_some_and(|held| held.same_as(repository))
            })
            && self.kind.is_none_or(|kind| scope.kind() == kind)
            && self.status.is_none_or(|status| runner.status() == status)
            && self
                .label
                .as_ref()
                .is_none_or(|label| definition.labels().contains(label))
            && self.search.as_ref().is_none_or(|text| {
                definition.name().as_str().to_lowercase().contains(text)
                    || scope.to_string().to_lowercase().contains(text)
            })
    }
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use super::*;
    use crate::{
        ImageReference, RunnerDefinition, RunnerId, RunnerLabels, RunnerLifecycle, RunnerName,
        RunnerScope,
    };

    fn runner(url: &str, name: &str, labels: &str, status: RunnerStatus) -> Runner {
        Runner::new(
            RunnerDefinition::new(
                RunnerId::generate(),
                Forge::GitHub,
                RunnerScope::parse_url(url).unwrap(),
                RunnerName::new(name).unwrap(),
                RunnerLabels::parse_comma_separated(labels).unwrap(),
                RunnerLifecycle::Persistent,
            ),
            ImageReference::new("ghcr.io/actions/actions-runner:latest").unwrap(),
            status,
            SystemTime::UNIX_EPOCH,
        )
    }

    fn mindweaver_builder() -> Runner {
        runner(
            "https://github.com/otsobide/mindweaver",
            "builder",
            "gpu,large",
            RunnerStatus::Running,
        )
    }

    fn owner(value: &str) -> OwnerName {
        OwnerName::new(value).unwrap()
    }

    #[test]
    fn matches_every_runner_without_criteria() {
        assert!(RunnerFilter::any().matches(&mindweaver_builder()));
    }

    #[test]
    fn matches_runners_of_an_owner_whatever_its_case() {
        let runner = mindweaver_builder();

        assert!(
            RunnerFilter::any()
                .with_owner(owner("Otsobide"))
                .matches(&runner)
        );
        assert!(
            !RunnerFilter::any()
                .with_owner(owner("someone"))
                .matches(&runner)
        );
    }

    #[test]
    fn matches_an_owners_own_runners_and_its_repositories_alike() {
        let filter = RunnerFilter::any().with_owner(owner("otsobide"));
        let owner_wide = runner(
            "https://github.com/otsobide",
            "shared",
            "",
            RunnerStatus::Running,
        );

        assert!(filter.matches(&owner_wide));
        assert!(filter.matches(&mindweaver_builder()));
    }

    #[test]
    fn matches_runners_of_a_repository_alone() {
        let filter =
            RunnerFilter::any().with_repository(RepositoryName::new("MindWeaver").unwrap());
        let owner_wide = runner(
            "https://github.com/otsobide",
            "shared",
            "",
            RunnerStatus::Running,
        );

        assert!(filter.matches(&mindweaver_builder()));
        assert!(!filter.matches(&owner_wide));
    }

    #[test]
    fn matches_runners_of_one_kind_of_scope() {
        let runner = mindweaver_builder();

        assert!(
            RunnerFilter::any()
                .with_kind(ScopeKind::Repository)
                .matches(&runner)
        );
        assert!(
            !RunnerFilter::any()
                .with_kind(ScopeKind::Owner)
                .matches(&runner)
        );
    }

    #[test]
    fn matches_runners_in_a_state() {
        let runner = mindweaver_builder();

        assert!(
            RunnerFilter::any()
                .with_status(RunnerStatus::Running)
                .matches(&runner)
        );
        assert!(
            !RunnerFilter::any()
                .with_status(RunnerStatus::Exited)
                .matches(&runner)
        );
    }

    #[test]
    fn matches_runners_with_a_label_whatever_its_case() {
        let runner = mindweaver_builder();

        assert!(
            RunnerFilter::any()
                .with_label(RunnerLabel::new("GPU").unwrap())
                .matches(&runner)
        );
        assert!(
            !RunnerFilter::any()
                .with_label(RunnerLabel::new("arm").unwrap())
                .matches(&runner)
        );
    }

    #[test]
    fn matches_runners_whose_name_or_url_holds_the_text_searched() {
        let runner = mindweaver_builder();

        assert!(RunnerFilter::any().with_search("BUILD").matches(&runner));
        assert!(RunnerFilter::any().with_search("mindweav").matches(&runner));
        assert!(!RunnerFilter::any().with_search("deploy").matches(&runner));
    }

    #[test]
    fn narrows_nothing_with_an_empty_search() {
        assert_eq!(RunnerFilter::any().with_search("   "), RunnerFilter::any());
    }

    #[test]
    fn matches_only_runners_meeting_every_criterion() {
        let filter = RunnerFilter::any()
            .with_owner(owner("otsobide"))
            .with_status(RunnerStatus::Exited);

        assert!(!filter.matches(&mindweaver_builder()));
    }
}
