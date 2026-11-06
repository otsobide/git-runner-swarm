//! What the server is configured with: environment variables, each with a
//! default, so the panel runs as it is and is told only what differs.

use std::net::SocketAddr;
use std::path::PathBuf;

use swarm_domain::{ImageReference, InvalidValue};
use thiserror::Error;

/// The address the panel listens on unless `GRS_LISTEN` says otherwise.
pub const DEFAULT_LISTEN: &str = "0.0.0.0:8080";

/// The image GitHub runners run unless `GRS_GITHUB_IMAGE` says otherwise.
pub const DEFAULT_GITHUB_IMAGE: &str = "ghcr.io/actions/actions-runner:latest";

/// Seconds a runner is given to stop unless `GRS_STOP_TIMEOUT` says
/// otherwise.
pub const DEFAULT_STOP_TIMEOUT: i32 = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// `GRS_LISTEN`: the address and port to listen on.
    pub listen: SocketAddr,
    /// `GRS_WEB_DIR`: the directory the web panel was built into. Without
    /// one, the server is the API alone.
    pub web_dir: Option<PathBuf>,
    /// `GRS_RUNTIME`: `docker`, or `memory` to try the panel without an
    /// engine.
    pub runtime: RuntimeKind,
    /// `GRS_GITHUB_IMAGE`: the tag (or mirror) of GitHub's official image
    /// that GitHub runners run.
    pub github_image: ImageReference,
    /// `GRS_STOP_TIMEOUT`: how many seconds a runner is given to stop
    /// cleanly before it is killed.
    pub stop_timeout: i32,
}

/// Where runners run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKind {
    /// A Docker engine, at `DOCKER_HOST` or the local socket.
    Docker,
    /// Memory: nothing runs, and everything is lost when the panel stops.
    Memory,
}

/// A variable set to something the server cannot run with.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("{variable} is `{value}`, which is not {expected}")]
    Invalid {
        variable: &'static str,
        value: String,
        expected: &'static str,
    },

    #[error("{variable} is not a container image: {error}")]
    InvalidImage {
        variable: &'static str,
        error: InvalidValue,
    },
}

impl Config {
    /// The configuration the variables `lookup` finds say, defaults for
    /// those it does not. A variable set but empty is taken as not set.
    ///
    /// # Errors
    ///
    /// When a variable is set to something the server cannot run with.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let variable = |name: &str| lookup(name).filter(|value| !value.trim().is_empty());

        let listen = variable("GRS_LISTEN").unwrap_or_else(|| DEFAULT_LISTEN.to_owned());
        let listen = listen.parse().map_err(|_| ConfigError::Invalid {
            variable: "GRS_LISTEN",
            value: listen.clone(),
            expected: "an address and a port, such as 0.0.0.0:8080",
        })?;

        let runtime = match variable("GRS_RUNTIME").as_deref() {
            None | Some("docker") => RuntimeKind::Docker,
            Some("memory") => RuntimeKind::Memory,
            Some(other) => {
                return Err(ConfigError::Invalid {
                    variable: "GRS_RUNTIME",
                    value: other.to_owned(),
                    expected: "docker or memory",
                });
            }
        };

        let github_image = ImageReference::new(
            variable("GRS_GITHUB_IMAGE").unwrap_or_else(|| DEFAULT_GITHUB_IMAGE.to_owned()),
        )
        .map_err(|error| ConfigError::InvalidImage {
            variable: "GRS_GITHUB_IMAGE",
            error,
        })?;

        let stop_timeout =
            match variable("GRS_STOP_TIMEOUT") {
                None => DEFAULT_STOP_TIMEOUT,
                Some(value) => value.parse().ok().filter(|seconds| *seconds >= 0).ok_or(
                    ConfigError::Invalid {
                        variable: "GRS_STOP_TIMEOUT",
                        value,
                        expected: "a number of seconds",
                    },
                )?,
            };

        Ok(Self {
            listen,
            web_dir: variable("GRS_WEB_DIR").map(PathBuf::from),
            runtime,
            github_image,
            stop_timeout,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config(variables: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let variables: HashMap<String, String> = variables
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
        Config::from_lookup(|name| variables.get(name).cloned())
    }

    #[test]
    fn runs_on_docker_on_port_8080_with_the_official_image_unless_told_otherwise() {
        let config = config(&[]).unwrap();

        assert_eq!(config.listen, "0.0.0.0:8080".parse().unwrap());
        assert_eq!(config.web_dir, None);
        assert_eq!(config.runtime, RuntimeKind::Docker);
        assert_eq!(
            config.github_image.as_str(),
            "ghcr.io/actions/actions-runner:latest"
        );
        assert_eq!(config.stop_timeout, 30);
    }

    #[test]
    fn reads_every_variable_it_is_given() {
        let config = config(&[
            ("GRS_LISTEN", "127.0.0.1:9000"),
            ("GRS_WEB_DIR", "/app/web"),
            ("GRS_RUNTIME", "memory"),
            ("GRS_GITHUB_IMAGE", "ghcr.io/actions/actions-runner:2.338.2"),
            ("GRS_STOP_TIMEOUT", "60"),
        ])
        .unwrap();

        assert_eq!(config.listen, "127.0.0.1:9000".parse().unwrap());
        assert_eq!(config.web_dir, Some(PathBuf::from("/app/web")));
        assert_eq!(config.runtime, RuntimeKind::Memory);
        assert_eq!(
            config.github_image.as_str(),
            "ghcr.io/actions/actions-runner:2.338.2"
        );
        assert_eq!(config.stop_timeout, 60);
    }

    #[test]
    fn takes_an_empty_variable_for_one_not_set() {
        let config = config(&[("GRS_LISTEN", ""), ("GRS_WEB_DIR", " ")]).unwrap();

        assert_eq!(config.listen, "0.0.0.0:8080".parse().unwrap());
        assert_eq!(config.web_dir, None);
    }

    #[test]
    fn says_which_variable_it_cannot_run_with() {
        for (variable, value) in [
            ("GRS_LISTEN", "8080"),
            ("GRS_RUNTIME", "podman"),
            ("GRS_STOP_TIMEOUT", "-1"),
            ("GRS_STOP_TIMEOUT", "soon"),
            ("GRS_GITHUB_IMAGE", "two words"),
        ] {
            let error = config(&[(variable, value)]).unwrap_err();
            assert!(error.to_string().starts_with(variable), "{error}");
        }
    }
}
