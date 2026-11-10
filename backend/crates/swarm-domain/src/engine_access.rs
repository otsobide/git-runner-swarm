use std::fmt;
use std::str::FromStr;

use crate::InvalidValue;

/// Whether a runner's jobs may use the container engine the runner itself
/// runs on: to build images and run containers, as many workflows do.
///
/// An engine's access is its host's: jobs given it can do anything on the
/// host that root can. So it is never given unless asked for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum EngineAccess {
    /// The runner's jobs have no engine to use.
    #[default]
    Isolated,
    /// The runner's jobs use the engine of the host the runner runs on.
    Host,
}

impl EngineAccess {
    pub const ALL: [EngineAccess; 2] = [EngineAccess::Isolated, EngineAccess::Host];

    /// `isolated` or `host`.
    pub fn as_str(self) -> &'static str {
        match self {
            EngineAccess::Isolated => "isolated",
            EngineAccess::Host => "host",
        }
    }

    pub fn is_host(self) -> bool {
        self == EngineAccess::Host
    }
}

impl fmt::Display for EngineAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EngineAccess {
    type Err = InvalidValue;

    /// Reads an access from its name, in any case.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        EngineAccess::ALL
            .into_iter()
            .find(|access| access.as_str().eq_ignore_ascii_case(value))
            .ok_or_else(|| {
                InvalidValue::new(
                    "engine access",
                    format!("`{value}` is neither isolated nor host"),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_isolated_unless_told_otherwise() {
        assert_eq!(EngineAccess::default(), EngineAccess::Isolated);
        assert!(!EngineAccess::default().is_host());
    }

    #[test]
    fn reads_back_what_it_writes() {
        for access in EngineAccess::ALL {
            assert_eq!(access.to_string().parse(), Ok(access));
        }
        assert_eq!("HOST".parse(), Ok(EngineAccess::Host));
    }

    #[test]
    fn refuses_an_access_it_does_not_know() {
        let error = "root".parse::<EngineAccess>().unwrap_err();

        assert_eq!(error.field(), "engine access");
        assert_eq!(error.reason(), "`root` is neither isolated nor host");
    }
}
