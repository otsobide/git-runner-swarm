use std::fmt;
use std::str::FromStr;

use crate::InvalidValue;

/// Whether a runner takes jobs for as long as it runs, or a single one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RunnerLifecycle {
    /// Takes job after job, keeping whatever each leaves behind, until it
    /// is stopped.
    #[default]
    Persistent,
    /// Takes one job, then its forge removes it and its container exits,
    /// so no job ever starts where another has run.
    Ephemeral,
}

impl RunnerLifecycle {
    pub const ALL: [RunnerLifecycle; 2] = [RunnerLifecycle::Persistent, RunnerLifecycle::Ephemeral];

    /// `persistent` or `ephemeral`.
    pub fn as_str(self) -> &'static str {
        match self {
            RunnerLifecycle::Persistent => "persistent",
            RunnerLifecycle::Ephemeral => "ephemeral",
        }
    }

    pub fn is_ephemeral(self) -> bool {
        self == RunnerLifecycle::Ephemeral
    }
}

impl fmt::Display for RunnerLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RunnerLifecycle {
    type Err = InvalidValue;

    /// Reads a lifecycle from its name, in any case.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        RunnerLifecycle::ALL
            .into_iter()
            .find(|lifecycle| lifecycle.as_str().eq_ignore_ascii_case(value))
            .ok_or_else(|| {
                InvalidValue::new(
                    "lifecycle",
                    format!("`{value}` is neither persistent nor ephemeral"),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_persistent_unless_asked_otherwise() {
        assert_eq!(RunnerLifecycle::default(), RunnerLifecycle::Persistent);
        assert!(!RunnerLifecycle::default().is_ephemeral());
    }

    #[test]
    fn reads_back_every_lifecycle_it_writes() {
        for lifecycle in RunnerLifecycle::ALL {
            assert_eq!(lifecycle.to_string().parse(), Ok(lifecycle));
        }
    }

    #[test]
    fn refuses_a_lifecycle_it_has_no_variant_for() {
        assert_eq!(
            "forever".parse::<RunnerLifecycle>().unwrap_err().field(),
            "lifecycle"
        );
    }
}
