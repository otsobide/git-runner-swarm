use crate::{CpuLimit, MemoryLimit};

/// What a runner may use of its host: CPU and memory, each limited or
/// not. Without limits, a runner and its jobs may use all the host has.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RunnerLimits {
    cpus: Option<CpuLimit>,
    memory: Option<MemoryLimit>,
}

impl RunnerLimits {
    /// No limits at all.
    pub fn none() -> Self {
        Self::default()
    }

    pub fn new(cpus: Option<CpuLimit>, memory: Option<MemoryLimit>) -> Self {
        Self { cpus, memory }
    }

    /// How many CPUs' worth of time it may use, if that is limited.
    pub fn cpus(self) -> Option<CpuLimit> {
        self.cpus
    }

    /// How much memory it may use, if that is limited.
    pub fn memory(self) -> Option<MemoryLimit> {
        self.memory
    }

    /// Whether nothing is limited.
    pub fn is_none(self) -> bool {
        self.cpus.is_none() && self.memory.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_nothing_unless_told_to() {
        assert!(RunnerLimits::none().is_none());
        assert_eq!(RunnerLimits::none(), RunnerLimits::new(None, None));
    }

    #[test]
    fn limits_what_it_is_given() {
        let cpus = CpuLimit::parse("2").unwrap();
        let memory = MemoryLimit::parse("4g").unwrap();

        let limits = RunnerLimits::new(Some(cpus), Some(memory));

        assert_eq!(limits.cpus(), Some(cpus));
        assert_eq!(limits.memory(), Some(memory));
        assert!(!limits.is_none());
        assert!(!RunnerLimits::new(None, Some(memory)).is_none());
    }
}
