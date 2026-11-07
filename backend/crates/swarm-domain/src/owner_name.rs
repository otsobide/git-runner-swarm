use std::fmt;

use crate::InvalidValue;
use crate::validate::check_name;

/// The user or organization a runner serves, or whose repository it
/// serves: `otsobide` in `https://github.com/otsobide/mindweaver`.
///
/// 1 to [`OwnerName::MAX_LENGTH`] characters: ASCII letters, digits, `-`,
/// `_` and `.`, the characters GitHub, Gitea and Forgejo names are made of
/// between them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnerName(String);

impl OwnerName {
    /// Longer than any forge allows a user or organization name to be.
    pub const MAX_LENGTH: usize = 64;

    /// # Errors
    ///
    /// When the name is empty, longer than [`OwnerName::MAX_LENGTH`], or
    /// holds a character other than an ASCII letter, a digit, `-`, `_` or
    /// `.`.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidValue> {
        let value = value.into();
        check_name("owner", &value, Self::MAX_LENGTH)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether two names name the same owner: forges ignore case in them.
    pub fn same_as(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl fmt::Display for OwnerName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_name_it_is_given() {
        assert_eq!(OwnerName::new("otsobide").unwrap().as_str(), "otsobide");
    }

    #[test]
    fn refuses_what_could_not_be_a_path_segment() {
        for value in ["", "otso/bide", "otso bide"] {
            assert_eq!(OwnerName::new(value).unwrap_err().field(), "owner");
        }
    }

    #[test]
    fn takes_names_differing_in_case_alone_for_the_same_owner() {
        let owner = OwnerName::new("Otsobide").unwrap();

        assert!(owner.same_as(&OwnerName::new("otsobide").unwrap()));
        assert!(!owner.same_as(&OwnerName::new("someone").unwrap()));
    }
}
