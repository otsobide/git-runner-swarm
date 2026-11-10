use std::fmt;

use crate::InvalidValue;

/// How much CPU time a runner may use, counted in CPUs: `2` for two whole
/// CPUs' worth, `0.5` for half of one. Its jobs are slowed down rather
/// than stopped when they would use more.
///
/// Kept in thousandths of a CPU, finer than anyone asks for, so that the
/// limit read back is exactly the one written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CpuLimit {
    millis: u32,
}

impl CpuLimit {
    /// A hundredth of a CPU, the least an engine such as Docker allows.
    pub const MIN_MILLIS: u32 = 10;
    /// More CPUs than any one host has.
    pub const MAX_MILLIS: u32 = 1024 * 1000;

    /// Reads a number of CPUs, written as a whole number or with up to
    /// three decimals: `2`, `1.5`, `0.25`. Spaces around it are ignored.
    ///
    /// # Errors
    ///
    /// When it is empty, not such a number, finer than a thousandth of a
    /// CPU, less than a hundredth of one, or more than 1024 CPUs.
    pub fn parse(value: &str) -> Result<Self, InvalidValue> {
        let value = value.trim();
        if value.is_empty() {
            return Err(invalid("it is empty"));
        }
        let not_a_number = || {
            invalid(format!(
                "`{value}` is not a number of CPUs, such as 2 or 0.5"
            ))
        };

        let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
        let is_digits = |part: &str| part.chars().all(|c| c.is_ascii_digit());
        if whole.is_empty() || !is_digits(whole) || !is_digits(fraction) {
            return Err(not_a_number());
        }
        if value.ends_with('.') {
            return Err(not_a_number());
        }
        if fraction.len() > 3 {
            return Err(invalid(format!(
                "`{value}` is finer than a thousandth of a CPU"
            )));
        }

        let whole: u64 = whole.parse().map_err(|_| too_many())?;
        let fraction: u64 = format!("{fraction:0<3}").parse().unwrap_or(0);
        let millis = whole
            .checked_mul(1000)
            .and_then(|millis| millis.checked_add(fraction))
            .ok_or_else(too_many)?;
        Self::from_millis(u32::try_from(millis).map_err(|_| too_many())?)
    }

    /// A limit of `millis` thousandths of a CPU.
    ///
    /// # Errors
    ///
    /// When it is less than [`CpuLimit::MIN_MILLIS`] or more than
    /// [`CpuLimit::MAX_MILLIS`].
    pub fn from_millis(millis: u32) -> Result<Self, InvalidValue> {
        if millis < Self::MIN_MILLIS {
            return Err(invalid("it is less than 0.01 CPUs, the least allowed"));
        }
        if millis > Self::MAX_MILLIS {
            return Err(too_many());
        }
        Ok(Self { millis })
    }

    /// The limit in thousandths of a CPU: `1500` for `1.5`.
    pub fn millis(self) -> u32 {
        self.millis
    }
}

impl fmt::Display for CpuLimit {
    /// The limit as it is read: `2`, `1.5`, `0.25`, with no trailing zeros.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (whole, fraction) = (self.millis / 1000, self.millis % 1000);
        if fraction == 0 {
            write!(f, "{whole}")
        } else {
            let fraction = format!("{fraction:03}");
            write!(f, "{whole}.{}", fraction.trim_end_matches('0'))
        }
    }
}

fn invalid(reason: impl Into<String>) -> InvalidValue {
    InvalidValue::new("CPU limit", reason)
}

fn too_many() -> InvalidValue {
    invalid("it is more than 1024 CPUs")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn millis(value: &str) -> u32 {
        CpuLimit::parse(value).unwrap().millis()
    }

    fn reason(value: &str) -> String {
        CpuLimit::parse(value).unwrap_err().reason().to_owned()
    }

    #[test]
    fn reads_whole_cpus() {
        assert_eq!(millis("2"), 2000);
        assert_eq!(millis("16"), 16_000);
    }

    #[test]
    fn reads_fractions_of_a_cpu() {
        assert_eq!(millis("1.5"), 1500);
        assert_eq!(millis("0.25"), 250);
        assert_eq!(millis("0.125"), 125);
        assert_eq!(millis("0.01"), 10);
    }

    #[test]
    fn ignores_spaces_around_it() {
        assert_eq!(millis(" 2 "), 2000);
    }

    #[test]
    fn writes_itself_as_it_is_read() {
        for value in ["2", "1.5", "0.25", "0.125", "0.01", "1024"] {
            assert_eq!(CpuLimit::parse(value).unwrap().to_string(), value);
        }
        assert_eq!(CpuLimit::parse("2.50").unwrap().to_string(), "2.5");
        assert_eq!(CpuLimit::parse("1.000").unwrap().to_string(), "1");
    }

    #[test]
    fn refuses_what_is_not_a_number_of_cpus() {
        for value in ["two", "-1", "+1", "1e3", ".5", "1.", "1,5", "0x10", "1.5.2"] {
            assert_eq!(
                reason(value),
                format!("`{value}` is not a number of CPUs, such as 2 or 0.5"),
                "{value}"
            );
        }
    }

    #[test]
    fn refuses_an_empty_limit() {
        assert_eq!(reason("  "), "it is empty");
    }

    #[test]
    fn refuses_a_limit_finer_than_a_thousandth() {
        assert_eq!(
            reason("0.0005"),
            "`0.0005` is finer than a thousandth of a CPU"
        );
    }

    #[test]
    fn refuses_less_than_a_hundredth_of_a_cpu() {
        assert_eq!(reason("0"), "it is less than 0.01 CPUs, the least allowed");
        assert_eq!(
            reason("0.005"),
            "it is less than 0.01 CPUs, the least allowed"
        );
    }

    #[test]
    fn refuses_more_cpus_than_any_host_has() {
        assert_eq!(reason("1024.001"), "it is more than 1024 CPUs");
        assert_eq!(reason("99999999999999999999"), "it is more than 1024 CPUs");
    }

    #[test]
    fn names_the_field_it_is() {
        assert_eq!(CpuLimit::parse("x").unwrap_err().field(), "CPU limit");
    }
}
