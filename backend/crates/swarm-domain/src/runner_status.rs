use std::fmt;
use std::str::FromStr;

use crate::InvalidValue;

/// The state a runner's container is in, as its engine reports it.
///
/// Whether the runner is online on its forge, idle or busy with a job, is
/// the forge's to say, and not part of this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunnerStatus {
    /// Created and never started.
    Created,
    Running,
    /// Being started again by its restart policy, after it stopped.
    Restarting,
    Paused,
    /// Stopped: by hand, or on its own (an ephemeral runner whose job has
    /// run, a runner that could not register).
    Exited,
    /// Stopped and beyond starting again, as the engine could not stop or
    /// remove it cleanly.
    Dead,
}

impl RunnerStatus {
    pub const ALL: [RunnerStatus; 6] = [
        RunnerStatus::Created,
        RunnerStatus::Running,
        RunnerStatus::Restarting,
        RunnerStatus::Paused,
        RunnerStatus::Exited,
        RunnerStatus::Dead,
    ];

    /// The status, lowercase, as the API writes it: `running`.
    pub fn as_str(self) -> &'static str {
        match self {
            RunnerStatus::Created => "created",
            RunnerStatus::Running => "running",
            RunnerStatus::Restarting => "restarting",
            RunnerStatus::Paused => "paused",
            RunnerStatus::Exited => "exited",
            RunnerStatus::Dead => "dead",
        }
    }
}

impl fmt::Display for RunnerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RunnerStatus {
    type Err = InvalidValue;

    /// Reads a status from its name, in any case.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        RunnerStatus::ALL
            .into_iter()
            .find(|status| status.as_str().eq_ignore_ascii_case(value))
            .ok_or_else(|| InvalidValue::new("status", format!("`{value}` is not a runner status")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_back_every_status_it_writes() {
        for status in RunnerStatus::ALL {
            assert_eq!(status.to_string().parse(), Ok(status));
        }
    }

    #[test]
    fn reads_a_status_in_any_case() {
        assert_eq!("Running".parse(), Ok(RunnerStatus::Running));
    }

    #[test]
    fn refuses_a_status_it_has_no_variant_for() {
        assert_eq!(
            "online".parse::<RunnerStatus>().unwrap_err().field(),
            "status"
        );
    }
}
