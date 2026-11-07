use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

use crate::InvalidValue;

/// Identifies a runner the panel manages.
///
/// The panel assigns it when it creates the runner, so it names the runner
/// for as long as the panel manages it, apart from the name the forge lists
/// it by and from whatever the container engine calls its container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RunnerId(Uuid);

impl RunnerId {
    /// A new id, random, so no two runners are given the same one.
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for RunnerId {
    /// The id as a hyphenated, lowercase UUID.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.hyphenated().fmt(f)
    }
}

impl FromStr for RunnerId {
    type Err = InvalidValue;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| InvalidValue::new("runner id", format!("`{value}` is not a UUID")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_a_different_id_each_time() {
        assert_ne!(RunnerId::generate(), RunnerId::generate());
    }

    #[test]
    fn reads_back_the_id_it_writes() {
        let id = RunnerId::generate();

        assert_eq!(id.to_string().parse(), Ok(id));
    }

    #[test]
    fn writes_a_hyphenated_lowercase_uuid() {
        let id: RunnerId = "67E55044-10B1-426F-9247-BB680E5FE0C8".parse().unwrap();

        assert_eq!(id.to_string(), "67e55044-10b1-426f-9247-bb680e5fe0c8");
    }

    #[test]
    fn refuses_what_is_not_a_uuid() {
        let error = "runner-1".parse::<RunnerId>().unwrap_err();

        assert_eq!(error.field(), "runner id");
    }
}
