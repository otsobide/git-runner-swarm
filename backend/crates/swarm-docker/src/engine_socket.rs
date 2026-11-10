//! How a runner's jobs are given the Docker engine the runner runs on:
//! the engine's socket, mounted into the runner's container where the
//! Docker CLI looks for it, and the group that may use the socket, which
//! the container's user joins.

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Where the Docker CLI, in a runner's container, looks for the engine.
pub(crate) const IN_CONTAINER: &str = "/var/run/docker.sock";

/// The engine's socket as runners are given it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineSocket {
    host_path: String,
    group: Option<u32>,
}

impl EngineSocket {
    /// Where Docker puts its socket, unless it is told otherwise.
    pub const DEFAULT_PATH: &'static str = "/var/run/docker.sock";

    /// The socket at `host_path` on the host, a path the engine itself
    /// resolves when it mounts it, so a path on the host even when the
    /// panel runs in a container; and the group that may use it, which
    /// a runner's user joins, if it is known.
    pub fn new(host_path: impl Into<String>, group: Option<u32>) -> Self {
        Self {
            host_path: host_path.into(),
            group,
        }
    }

    pub fn host_path(&self) -> &str {
        &self.host_path
    }

    pub fn group(&self) -> Option<u32> {
        self.group
    }

    /// The bind that mounts the socket into a runner's container.
    pub(crate) fn bind(&self) -> String {
        format!("{}:{IN_CONTAINER}", self.host_path)
    }
}

impl Default for EngineSocket {
    fn default() -> Self {
        Self::new(Self::DEFAULT_PATH, None)
    }
}

/// The local socket this process reaches the engine at, as the Docker CLI
/// would find it from `docker_host` (`DOCKER_HOST`'s value): its path if
/// it is a `unix://` address, Docker's default socket if it is unset, and
/// `None` for an engine reached over the network.
pub fn local_socket(docker_host: Option<&str>) -> Option<PathBuf> {
    match docker_host.map(str::trim).filter(|host| !host.is_empty()) {
        None => Some(PathBuf::from(EngineSocket::DEFAULT_PATH)),
        Some(host) => host.strip_prefix("unix://").map(PathBuf::from),
    }
}

/// The group owning the socket at `path`: the one whose members may use
/// the engine. `None` if there is nothing there to ask.
pub fn socket_group(path: &Path) -> Option<u32> {
    std::fs::metadata(path).ok().map(|metadata| metadata.gid())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mounts_the_hosts_socket_where_the_docker_cli_looks() {
        let socket = EngineSocket::new("/run/user/1000/docker.sock", Some(967));

        assert_eq!(
            socket.bind(),
            "/run/user/1000/docker.sock:/var/run/docker.sock"
        );
        assert_eq!(socket.group(), Some(967));
    }

    #[test]
    fn is_dockers_own_socket_unless_told_otherwise() {
        assert_eq!(
            EngineSocket::default(),
            EngineSocket::new("/var/run/docker.sock", None)
        );
    }

    #[test]
    fn finds_the_local_socket_as_the_docker_cli_does() {
        assert_eq!(
            local_socket(None),
            Some(PathBuf::from("/var/run/docker.sock"))
        );
        assert_eq!(
            local_socket(Some("")),
            Some(PathBuf::from("/var/run/docker.sock"))
        );
        assert_eq!(
            local_socket(Some("unix:///run/user/1000/docker.sock")),
            Some(PathBuf::from("/run/user/1000/docker.sock"))
        );
        assert_eq!(local_socket(Some("tcp://10.0.0.2:2375")), None);
    }

    #[test]
    fn says_which_group_owns_a_socket() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let expected = std::fs::metadata(file.path()).unwrap().gid();

        assert_eq!(socket_group(file.path()), Some(expected));
        assert_eq!(socket_group(Path::new("/nonexistent/docker.sock")), None);
    }
}
