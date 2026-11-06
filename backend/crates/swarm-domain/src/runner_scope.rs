use std::fmt;

use crate::{ForgeAddress, InvalidValue, OwnerName, RepositoryName};

/// What a runner takes jobs for: every repository of one owner (a user or
/// an organization), or a single repository.
///
/// Read from the URL a forge shows for either, which is also the URL a
/// runner registers with: `https://github.com/otsobide` for an owner,
/// `https://github.com/otsobide/mindweaver` for a repository.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunnerScope {
    forge: ForgeAddress,
    owner: OwnerName,
    repository: Option<RepositoryName>,
}

/// Whether a scope is an owner's or a single repository's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    Owner,
    Repository,
}

impl ScopeKind {
    /// `owner` or `repository`.
    pub fn as_str(self) -> &'static str {
        match self {
            ScopeKind::Owner => "owner",
            ScopeKind::Repository => "repository",
        }
    }
}

impl RunnerScope {
    pub fn new(forge: ForgeAddress, owner: OwnerName, repository: Option<RepositoryName>) -> Self {
        Self {
            forge,
            owner,
            repository,
        }
    }

    /// Reads a scope from an owner's or a repository's URL. Spaces around
    /// it, a trailing `/` and a repository's `.git` are ignored, so a URL
    /// copied from a forge's page or a clone command both work.
    ///
    /// # Errors
    ///
    /// When the part before the path is not a [`ForgeAddress`], when the
    /// URL has a query or a fragment, when its path is not one segment
    /// (the owner) or two (the owner and the repository), or when either
    /// segment is not an [`OwnerName`] or a [`RepositoryName`].
    pub fn parse_url(value: &str) -> Result<Self, InvalidValue> {
        let value = value.trim();
        let after_scheme = value.find("://").map_or(0, |at| at + 3);
        let (address, path) = match value[after_scheme..].find('/') {
            Some(at) => value.split_at(after_scheme + at),
            None => (value, ""),
        };
        let forge = ForgeAddress::parse(address).map_err(|error| invalid(error.reason()))?;

        if path.contains(['?', '#']) {
            return Err(invalid("a runner's URL has no query or fragment"));
        }
        let path = path.trim_start_matches('/');
        let path = path.strip_suffix('/').unwrap_or(path);
        let path = path.strip_suffix(".git").unwrap_or(path);
        let segments: Vec<&str> = if path.is_empty() {
            Vec::new()
        } else {
            path.split('/').collect()
        };

        match segments.as_slice() {
            [owner] => Ok(Self::new(forge, OwnerName::new(*owner)?, None)),
            [owner, repository] => Ok(Self::new(
                forge,
                OwnerName::new(*owner)?,
                Some(RepositoryName::new(*repository)?),
            )),
            [] => Err(invalid("it names no owner or repository")),
            _ => Err(invalid("it names more than an owner and a repository")),
        }
    }

    pub fn forge(&self) -> &ForgeAddress {
        &self.forge
    }

    pub fn owner(&self) -> &OwnerName {
        &self.owner
    }

    /// The repository, for a repository's scope; `None` for an owner's.
    pub fn repository(&self) -> Option<&RepositoryName> {
        self.repository.as_ref()
    }

    pub fn kind(&self) -> ScopeKind {
        if self.repository.is_some() {
            ScopeKind::Repository
        } else {
            ScopeKind::Owner
        }
    }

    /// Whether two scopes are the same owner's or the same repository's.
    /// Case is ignored in names, as forges ignore it.
    pub fn same_as(&self, other: &Self) -> bool {
        self.forge == other.forge
            && self.owner.same_as(&other.owner)
            && match (&self.repository, &other.repository) {
                (Some(mine), Some(theirs)) => mine.same_as(theirs),
                (None, None) => true,
                _ => false,
            }
    }
}

impl fmt::Display for RunnerScope {
    /// The scope's URL, the one a runner registers with.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.forge, self.owner)?;
        if let Some(repository) = &self.repository {
            write!(f, "/{repository}")?;
        }
        Ok(())
    }
}

fn invalid(reason: impl Into<String>) -> InvalidValue {
    InvalidValue::new("URL", reason)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(url: &str) -> RunnerScope {
        RunnerScope::parse_url(url).unwrap()
    }

    #[test]
    fn reads_a_repositorys_url() {
        let scope = scope("https://github.com/otsobide/mindweaver");

        assert_eq!(scope.forge().to_string(), "https://github.com");
        assert_eq!(scope.owner().as_str(), "otsobide");
        assert_eq!(
            scope.repository().map(RepositoryName::as_str),
            Some("mindweaver")
        );
        assert_eq!(scope.kind(), ScopeKind::Repository);
    }

    #[test]
    fn reads_an_owners_url() {
        let scope = scope("http://gitea.lan:3000/infra");

        assert_eq!(scope.owner().as_str(), "infra");
        assert_eq!(scope.repository(), None);
        assert_eq!(scope.kind(), ScopeKind::Owner);
    }

    #[test]
    fn writes_the_url_a_runner_registers_with() {
        assert_eq!(
            scope("https://github.com/otsobide/mindweaver").to_string(),
            "https://github.com/otsobide/mindweaver"
        );
        assert_eq!(
            scope("http://gitea.lan:3000/infra").to_string(),
            "http://gitea.lan:3000/infra"
        );
    }

    #[test]
    fn ignores_what_a_copied_url_may_carry() {
        for url in [
            "  https://github.com/otsobide/mindweaver  ",
            "https://github.com/otsobide/mindweaver/",
            "https://github.com/otsobide/mindweaver.git",
            "HTTPS://GitHub.com/otsobide/mindweaver",
        ] {
            assert_eq!(
                scope(url).to_string(),
                "https://github.com/otsobide/mindweaver",
                "{url:?}"
            );
        }
    }

    #[test]
    fn refuses_a_url_naming_no_owner_or_more_than_a_repository() {
        for url in [
            "https://github.com",
            "https://github.com/",
            "https://github.com/otsobide/mindweaver/tree/main",
            "https://github.com/otsobide//mindweaver",
        ] {
            assert!(RunnerScope::parse_url(url).is_err(), "{url:?} was accepted");
        }
    }

    #[test]
    fn refuses_a_url_with_a_query_or_a_fragment() {
        for url in [
            "https://github.com/otsobide?tab=repositories",
            "https://github.com/otsobide#top",
        ] {
            assert_eq!(RunnerScope::parse_url(url).unwrap_err().field(), "URL");
        }
    }

    #[test]
    fn refuses_a_url_whose_address_is_not_a_forges() {
        let error = RunnerScope::parse_url("github.com/otsobide").unwrap_err();

        assert_eq!(error.field(), "URL");
    }

    #[test]
    fn takes_urls_differing_in_case_alone_for_the_same_scope() {
        let scope = scope("https://github.com/Otsobide/MindWeaver");

        assert!(scope.same_as(&self::scope("https://github.com/otsobide/mindweaver")));
        assert!(!scope.same_as(&self::scope("https://github.com/otsobide")));
        assert!(!scope.same_as(&self::scope("https://ghe.example.com/otsobide/mindweaver")));
    }
}
