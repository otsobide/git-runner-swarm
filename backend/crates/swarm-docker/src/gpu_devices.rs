//! What a runner given the host's GPUs is given: the device files the
//! GPUs are used through, at the same paths in its container as on the
//! host, and NVIDIA's GPUs through the NVIDIA Container Toolkit.

use std::fmt;

use bollard::models::{DeviceMapping, DeviceRequest};

/// The GPUs runners given the host's are given, as the panel is told:
///
/// - device files of the host, each a path: `/dev/dri` for AMD's and
///   Intel's GPUs as Mesa drives them, to render, encode video and
///   compute, with `/dev/kfd` beside it for AMD's `ROCm`;
/// - `nvidia`: every NVIDIA GPU, as `docker run --gpus all` gives them,
///   which takes the NVIDIA Container Toolkit on the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuDevices {
    paths: Vec<String>,
    nvidia: bool,
}

impl GpuDevices {
    /// What runners are given unless the panel is told otherwise: the
    /// Direct Rendering Infrastructure's devices, which every AMD and
    /// Intel GPU, and many others, are used through.
    pub const DEFAULT: &'static str = "/dev/dri";

    /// The name that stands for NVIDIA's GPUs in a list.
    pub const NVIDIA: &'static str = "nvidia";

    /// The GPUs a comma-separated list names, as `/dev/dri,/dev/kfd` or
    /// `nvidia`: each an absolute path on the host, or `nvidia`. Spaces
    /// and empty items are ignored, and a path named twice is given once.
    /// `None` if the list names nothing, or something else.
    pub fn parse(list: &str) -> Option<Self> {
        let mut paths: Vec<String> = Vec::new();
        let mut nvidia = false;
        for item in list
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
        {
            if item.eq_ignore_ascii_case(Self::NVIDIA) {
                nvidia = true;
            } else if item.starts_with('/') {
                if !paths.iter().any(|path| path == item) {
                    paths.push(item.to_owned());
                }
            } else {
                return None;
            }
        }
        (nvidia || !paths.is_empty()).then_some(Self { paths, nvidia })
    }

    /// The device files given, as paths on the host.
    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    /// Whether NVIDIA's GPUs are given.
    pub fn nvidia(&self) -> bool {
        self.nvidia
    }

    /// The device files, mapped into a container at the paths they have
    /// on the host, for it to read, write and create nodes of. A
    /// directory gives every device in it.
    pub(crate) fn mappings(&self) -> Vec<DeviceMapping> {
        self.paths
            .iter()
            .map(|path| DeviceMapping {
                path_on_host: Some(path.clone()),
                path_in_container: Some(path.clone()),
                cgroup_permissions: Some("rwm".to_owned()),
            })
            .collect()
    }

    /// Every NVIDIA GPU, asked for as `docker run --gpus all` does, if
    /// they are given.
    pub(crate) fn requests(&self) -> Vec<DeviceRequest> {
        if !self.nvidia {
            return Vec::new();
        }
        vec![DeviceRequest {
            count: Some(-1),
            capabilities: Some(vec![vec!["gpu".to_owned()]]),
            ..DeviceRequest::default()
        }]
    }
}

impl Default for GpuDevices {
    fn default() -> Self {
        Self::parse(Self::DEFAULT).expect("the default GPU devices are a valid list")
    }
}

impl fmt::Display for GpuDevices {
    /// The list, as [`GpuDevices::parse`] reads it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nvidia = self.nvidia.then_some(Self::NVIDIA);
        let items: Vec<&str> = self
            .paths
            .iter()
            .map(String::as_str)
            .chain(nvidia)
            .collect();
        f.write_str(&items.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gives_the_direct_rendering_devices_unless_told_otherwise() {
        let gpus = GpuDevices::default();

        assert_eq!(gpus.paths(), ["/dev/dri"]);
        assert!(!gpus.nvidia());
        assert!(gpus.requests().is_empty());
    }

    #[test]
    fn reads_device_paths_and_nvidia_from_a_list() {
        let gpus = GpuDevices::parse(" /dev/dri, ,/dev/kfd,NVIDIA,/dev/dri ").unwrap();

        assert_eq!(gpus.paths(), ["/dev/dri", "/dev/kfd"]);
        assert!(gpus.nvidia());
        assert_eq!(gpus.to_string(), "/dev/dri,/dev/kfd,nvidia");
        assert_eq!(GpuDevices::parse(&gpus.to_string()), Some(gpus));
    }

    #[test]
    fn refuses_a_list_naming_nothing_or_something_else() {
        for list in ["", " , ", "dev/dri", "/dev/dri,amd", "all"] {
            assert_eq!(GpuDevices::parse(list), None, "{list:?}");
        }
    }

    #[test]
    fn maps_each_device_where_it_is_on_the_host() {
        let gpus = GpuDevices::parse("/dev/dri,/dev/kfd").unwrap();

        assert_eq!(
            gpus.mappings(),
            [
                DeviceMapping {
                    path_on_host: Some("/dev/dri".to_owned()),
                    path_in_container: Some("/dev/dri".to_owned()),
                    cgroup_permissions: Some("rwm".to_owned()),
                },
                DeviceMapping {
                    path_on_host: Some("/dev/kfd".to_owned()),
                    path_in_container: Some("/dev/kfd".to_owned()),
                    cgroup_permissions: Some("rwm".to_owned()),
                },
            ]
        );
    }

    #[test]
    fn asks_for_every_nvidia_gpu_as_the_docker_cli_does() {
        let gpus = GpuDevices::parse("nvidia").unwrap();

        assert!(gpus.mappings().is_empty());
        assert_eq!(
            gpus.requests(),
            [DeviceRequest {
                driver: None,
                count: Some(-1),
                device_ids: None,
                capabilities: Some(vec![vec!["gpu".to_owned()]]),
                options: None,
            }]
        );
    }
}
