use std::fmt;

use crate::InvalidValue;
use crate::validate::check_name;

/// A runner's name on its forge: what the forge lists it as, unique among
/// the runners of the scope it registers in.
///
/// 1 to [`RunnerName::MAX_LENGTH`] characters: ASCII letters, digits, `-`,
/// `_` and `.`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunnerName(String);

impl RunnerName {
    /// The longest name a runner can have.
    pub const MAX_LENGTH: usize = 64;

    /// # Errors
    ///
    /// When the name is empty, longer than [`RunnerName::MAX_LENGTH`], or
    /// holds a character other than an ASCII letter, a digit, `-`, `_` or
    /// `.`.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidValue> {
        let value = value.into();
        check_name("runner name", &value, Self::MAX_LENGTH)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether two names name the same runner. Case is ignored, so no two
    /// runners in one list are told apart by case alone.
    pub fn same_as(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl fmt::Display for RunnerName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_name_it_is_given() {
        let name = RunnerName::new("build-box_01.eu").unwrap();

        assert_eq!(name.as_str(), "build-box_01.eu");
        assert_eq!(name.to_string(), "build-box_01.eu");
    }

    #[test]
    fn refuses_a_name_no_forge_would_list() {
        for name in [
            "",
            "two words",
            "a/b",
            &"a".repeat(RunnerName::MAX_LENGTH + 1),
        ] {
            let error = RunnerName::new(name).unwrap_err();
            assert_eq!(error.field(), "runner name");
        }
    }

    #[test]
    fn takes_names_differing_in_case_alone_for_the_same_runner() {
        let lower = RunnerName::new("builder").unwrap();
        let upper = RunnerName::new("Builder").unwrap();
        let other = RunnerName::new("builder-2").unwrap();

        assert!(lower.same_as(&upper));
        assert!(!lower.same_as(&other));
    }
}
