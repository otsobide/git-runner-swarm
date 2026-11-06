use std::fmt;
use std::str::FromStr;

use crate::InvalidValue;

/// The kind of forge a runner takes jobs from.
///
/// GitHub alone for now. Gitea and Forgejo join as variants of their own,
/// each with the runner image that serves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Forge {
    GitHub,
}

impl Forge {
    /// Every forge there is a variant for.
    pub const ALL: [Forge; 1] = [Forge::GitHub];

    /// The forge's name, lowercase, as it is written in URLs, labels and
    /// the API: `github`.
    pub fn as_str(self) -> &'static str {
        match self {
            Forge::GitHub => "github",
        }
    }
}

impl fmt::Display for Forge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Forge {
    type Err = InvalidValue;

    /// Reads a forge from its name, in any case.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Forge::ALL
            .into_iter()
            .find(|forge| forge.as_str().eq_ignore_ascii_case(value))
            .ok_or_else(|| {
                InvalidValue::new(
                    "forge",
                    format!("`{value}` is not a forge runners can be made for"),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_forge_from_its_name_in_any_case() {
        assert_eq!("github".parse(), Ok(Forge::GitHub));
        assert_eq!("GitHub".parse(), Ok(Forge::GitHub));
    }

    #[test]
    fn refuses_a_forge_it_has_no_variant_for() {
        let error = "gitlab".parse::<Forge>().unwrap_err();

        assert_eq!(error.field(), "forge");
    }

    #[test]
    fn writes_its_name_as_it_reads_it() {
        for forge in Forge::ALL {
            assert_eq!(forge.to_string().parse(), Ok(forge));
        }
    }
}
