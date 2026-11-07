use std::fmt;

use crate::InvalidValue;
use crate::validate::check_name;

/// A repository's name within its owner: `mindweaver` in
/// `https://github.com/otsobide/mindweaver`.
///
/// 1 to [`RepositoryName::MAX_LENGTH`] characters: ASCII letters, digits,
/// `-`, `_` and `.`, though never `.` or `..` alone, which a path would read
/// as a directory rather than a name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepositoryName(String);

impl RepositoryName {
    /// The longest repository name GitHub allows.
    pub const MAX_LENGTH: usize = 100;

    /// # Errors
    ///
    /// When the name is empty, longer than [`RepositoryName::MAX_LENGTH`],
    /// `.` or `..`, or holds a character other than an ASCII letter, a
    /// digit, `-`, `_` or `.`.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidValue> {
        let value = value.into();
        check_name("repository", &value, Self::MAX_LENGTH)?;
        if value == "." || value == ".." {
            return Err(InvalidValue::new(
                "repository",
                format!("`{value}` is a directory, not a name"),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether two names name the same repository of an owner: forges
    /// ignore case in them.
    pub fn same_as(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl fmt::Display for RepositoryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_name_it_is_given() {
        let name = RepositoryName::new("mind.weaver_2").unwrap();

        assert_eq!(name.as_str(), "mind.weaver_2");
    }

    #[test]
    fn refuses_a_directory_for_a_name() {
        for value in [".", ".."] {
            assert_eq!(
                RepositoryName::new(value).unwrap_err().field(),
                "repository"
            );
        }
    }

    #[test]
    fn refuses_what_could_not_be_a_path_segment() {
        for value in ["", "a/b", "a b"] {
            assert!(
                RepositoryName::new(value).is_err(),
                "{value:?} was accepted"
            );
        }
    }

    #[test]
    fn takes_names_differing_in_case_alone_for_the_same_repository() {
        let name = RepositoryName::new("MindWeaver").unwrap();

        assert!(name.same_as(&RepositoryName::new("mindweaver").unwrap()));
    }
}
