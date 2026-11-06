use std::time::SystemTime;

use crate::{ImageReference, RunnerDefinition, RunnerId, RunnerStatus};

/// A runner as it is: what it was asked to be, the image it runs, and the
/// state its container is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runner {
    definition: RunnerDefinition,
    image: ImageReference,
    status: RunnerStatus,
    exit_code: Option<i64>,
    created_at: SystemTime,
}

impl Runner {
    pub fn new(
        definition: RunnerDefinition,
        image: ImageReference,
        status: RunnerStatus,
        created_at: SystemTime,
    ) -> Self {
        Self {
            definition,
            image,
            status,
            exit_code: None,
            created_at,
        }
    }

    /// The runner, with the code its container last exited with.
    #[must_use]
    pub fn with_exit_code(mut self, exit_code: i64) -> Self {
        self.exit_code = Some(exit_code);
        self
    }

    pub fn id(&self) -> RunnerId {
        self.definition.id()
    }

    pub fn definition(&self) -> &RunnerDefinition {
        &self.definition
    }

    pub fn image(&self) -> &ImageReference {
        &self.image
    }

    pub fn status(&self) -> RunnerStatus {
        self.status
    }

    /// The code the runner's container last exited with, while it is
    /// stopped or being restarted: 0 when it stopped cleanly, anything else
    /// when the runner failed, for one because it could not register.
    pub fn exit_code(&self) -> Option<i64> {
        self.exit_code
    }

    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Forge, RunnerLabels, RunnerLifecycle, RunnerName, RunnerScope};

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

    #[test]
    fn is_identified_by_its_definitions_id() {
        let definition = definition();
        let id = definition.id();

        let runner = Runner::new(
            definition,
            ImageReference::new("ghcr.io/actions/actions-runner:latest").unwrap(),
            RunnerStatus::Running,
            SystemTime::UNIX_EPOCH,
        );

        assert_eq!(runner.id(), id);
        assert_eq!(runner.status(), RunnerStatus::Running);
        assert_eq!(runner.exit_code(), None);
    }

    #[test]
    fn keeps_the_code_its_container_exited_with() {
        let runner = Runner::new(
            definition(),
            ImageReference::new("ghcr.io/actions/actions-runner:latest").unwrap(),
            RunnerStatus::Exited,
            SystemTime::UNIX_EPOCH,
        )
        .with_exit_code(1);

        assert_eq!(runner.exit_code(), Some(1));
    }
}
