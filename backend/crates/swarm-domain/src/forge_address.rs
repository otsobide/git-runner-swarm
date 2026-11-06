use std::fmt;

use crate::InvalidValue;

/// Where a forge is served: its scheme, host and port, and no path.
/// `https://github.com`, `http://gitea.lan:3000`.
///
/// The host is kept lowercase, and a scheme's own port is dropped, so one
/// forge always has one address however it was written.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForgeAddress {
    scheme: Scheme,
    host: String,
    port: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Scheme {
    Http,
    Https,
}

impl Scheme {
    fn as_str(self) -> &'static str {
        match self {
            Scheme::Http => "http",
            Scheme::Https => "https",
        }
    }

    fn default_port(self) -> u16 {
        match self {
            Scheme::Http => 80,
            Scheme::Https => 443,
        }
    }
}

impl ForgeAddress {
    /// Reads an address written as a URL with no path: a scheme, `http` or
    /// `https`, then a host, then a port if it is not the scheme's own.
    ///
    /// # Errors
    ///
    /// When the scheme is missing or is neither `http` nor `https`, when
    /// the host is not a hostname or an IPv4 address, when the port is not
    /// a number from 1 to 65535, or when anything follows the port but a
    /// single `/`.
    pub fn parse(value: &str) -> Result<Self, InvalidValue> {
        let (scheme, rest) = value
            .split_once("://")
            .ok_or_else(|| invalid("it has no scheme, such as `https://`"))?;
        let scheme = match scheme.to_ascii_lowercase().as_str() {
            "https" => Scheme::Https,
            "http" => Scheme::Http,
            other => return Err(invalid(format!("`{other}` is not http or https"))),
        };
        let authority = rest.strip_suffix('/').unwrap_or(rest);
        if authority.contains(['/', '?', '#']) {
            return Err(invalid("a forge's address has no path"));
        }
        if authority.contains('@') {
            return Err(invalid("a forge's address holds no credentials"));
        }

        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => (host, Some(parse_port(port)?)),
            None => (authority, None),
        };
        let host = host.to_ascii_lowercase();
        check_host(&host)?;

        Ok(Self {
            scheme,
            host,
            port: port.filter(|&port| port != scheme.default_port()),
        })
    }

    /// The host, lowercase: `github.com`.
    pub fn host(&self) -> &str {
        &self.host
    }
}

impl fmt::Display for ForgeAddress {
    /// The address as a URL with no path, and no trailing `/`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", self.scheme.as_str(), self.host)?;
        if let Some(port) = self.port {
            write!(f, ":{port}")?;
        }
        Ok(())
    }
}

fn invalid(reason: impl Into<String>) -> InvalidValue {
    InvalidValue::new("forge address", reason)
}

fn parse_port(value: &str) -> Result<u16, InvalidValue> {
    value
        .parse::<u16>()
        .ok()
        .filter(|&port| port != 0)
        .ok_or_else(|| invalid(format!("`{value}` is not a port from 1 to 65535")))
}

/// A hostname (or an IPv4 address, which is one as far as its characters
/// go): dot-separated labels of 1 to 63 letters, digits and `-`, none of
/// them starting or ending with `-`.
fn check_host(host: &str) -> Result<(), InvalidValue> {
    if host.is_empty() {
        return Err(invalid("it has no host"));
    }
    if host.len() > 253 {
        return Err(invalid("its host is longer than 253 characters"));
    }
    let valid_label = |label: &str| {
        (1..=63).contains(&label.len())
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    };
    if !host.split('.').all(valid_label) {
        return Err(invalid(format!("`{host}` is not a hostname")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn address(value: &str) -> String {
        ForgeAddress::parse(value).unwrap().to_string()
    }

    #[test]
    fn reads_a_scheme_and_a_host() {
        assert_eq!(address("https://github.com"), "https://github.com");
        assert_eq!(
            ForgeAddress::parse("https://github.com").unwrap().host(),
            "github.com"
        );
    }

    #[test]
    fn keeps_a_port_that_is_not_the_schemes_own() {
        assert_eq!(address("http://gitea.lan:3000"), "http://gitea.lan:3000");
        assert_eq!(address("http://10.0.0.5:8080/"), "http://10.0.0.5:8080");
    }

    #[test]
    fn writes_one_forge_one_way_however_it_was_written() {
        assert_eq!(address("HTTPS://GitHub.com/"), "https://github.com");
        assert_eq!(address("https://github.com:443"), "https://github.com");
        assert_eq!(address("http://gitea.lan:80"), "http://gitea.lan");
    }

    #[test]
    fn refuses_an_address_without_http_or_https() {
        for value in ["github.com", "ssh://github.com", "ftp://github.com"] {
            assert_eq!(
                ForgeAddress::parse(value).unwrap_err().field(),
                "forge address"
            );
        }
    }

    #[test]
    fn refuses_a_path_a_query_or_credentials() {
        for value in [
            "https://github.com/otsobide",
            "https://github.com?tab=1",
            "https://token@github.com",
        ] {
            assert!(
                ForgeAddress::parse(value).is_err(),
                "{value:?} was accepted"
            );
        }
    }

    #[test]
    fn refuses_a_host_or_a_port_that_is_not_one() {
        for value in [
            "https://",
            "https://git hub.com",
            "https://-github.com",
            "https://github..com",
            "https://github.com:0",
            "https://github.com:70000",
            "https://github.com:http",
        ] {
            assert!(
                ForgeAddress::parse(value).is_err(),
                "{value:?} was accepted"
            );
        }
    }
}
