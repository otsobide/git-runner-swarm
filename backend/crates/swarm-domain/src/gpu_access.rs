use std::fmt;
use std::str::FromStr;

use crate::InvalidValue;

/// Whether a runner's jobs may use the GPUs of the host the runner runs
/// on: to render, to encode and decode video, or to compute on them.
///
/// A GPU is a device of the host's, shared by everything that uses it and
/// driven by the host's kernel. So it is never given unless asked for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GpuAccess {
    /// The runner's jobs have no GPU to use.
    #[default]
    None,
    /// The runner's jobs use the GPUs of the host the runner runs on.
    Host,
}

impl GpuAccess {
    pub const ALL: [GpuAccess; 2] = [GpuAccess::None, GpuAccess::Host];

    /// `none` or `host`.
    pub fn as_str(self) -> &'static str {
        match self {
            GpuAccess::None => "none",
            GpuAccess::Host => "host",
        }
    }

    pub fn is_host(self) -> bool {
        self == GpuAccess::Host
    }
}

impl fmt::Display for GpuAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for GpuAccess {
    type Err = InvalidValue;

    /// Reads an access from its name, in any case.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        GpuAccess::ALL
            .into_iter()
            .find(|access| access.as_str().eq_ignore_ascii_case(value))
            .ok_or_else(|| {
                InvalidValue::new("GPU access", format!("`{value}` is neither none nor host"))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_none_unless_told_otherwise() {
        assert_eq!(GpuAccess::default(), GpuAccess::None);
        assert!(!GpuAccess::default().is_host());
    }

    #[test]
    fn reads_back_what_it_writes() {
        for access in GpuAccess::ALL {
            assert_eq!(access.to_string().parse(), Ok(access));
        }
        assert_eq!("HOST".parse(), Ok(GpuAccess::Host));
    }

    #[test]
    fn refuses_an_access_it_does_not_know() {
        let error = "all".parse::<GpuAccess>().unwrap_err();

        assert_eq!(error.field(), "GPU access");
        assert_eq!(error.reason(), "`all` is neither none nor host");
    }
}
