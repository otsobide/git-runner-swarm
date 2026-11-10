use std::fmt;

use crate::InvalidValue;

/// How much memory a runner may use, jobs included. One that would use
/// more is stopped short by the engine, its job failing.
///
/// Written as Docker writes sizes: a number, then `k`, `m`, `g` or `t` for
/// kibibytes, mebibytes, gibibytes or tebibytes (`512m`, `4g`), or `b`, or
/// nothing, for bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryLimit {
    bytes: u64,
}

const KIB: u64 = 1024;
const MIB: u64 = 1024 * KIB;
const GIB: u64 = 1024 * MIB;
const TIB: u64 = 1024 * GIB;

impl MemoryLimit {
    /// Six mebibytes, the least Docker allows; a runner needs far more.
    pub const MIN_BYTES: u64 = 6 * MIB;
    /// The most the engine can be told of, as it counts bytes in an `i64`.
    pub const MAX_BYTES: u64 = u64::MAX >> 1;

    /// Reads a size: a whole or decimal number, then optionally a space and
    /// a unit, `k`, `m`, `g` or `t`, in either case and followed or not by
    /// `ib` or `b` (`4g`, `4G`, `4GiB`, `1.5g`), or `b` alone for bytes.
    /// A number with no unit is a number of bytes, as Docker reads it.
    ///
    /// # Errors
    ///
    /// When it is empty, not such a size, less than
    /// [`MemoryLimit::MIN_BYTES`] or more than [`MemoryLimit::MAX_BYTES`].
    pub fn parse(value: &str) -> Result<Self, InvalidValue> {
        let value = value.trim();
        if value.is_empty() {
            return Err(invalid("it is empty"));
        }
        let not_a_size = || invalid(format!("`{value}` is not a size, such as 512m or 4g"));

        let number_end = value
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(value.len());
        let (number, unit) = value.split_at(number_end);
        let unit = unit.trim_start().to_ascii_lowercase();
        let multiplier = match unit.as_str() {
            "" | "b" => 1,
            "k" | "kb" | "kib" => KIB,
            "m" | "mb" | "mib" => MIB,
            "g" | "gb" | "gib" => GIB,
            "t" | "tb" | "tib" => TIB,
            _ => return Err(not_a_size()),
        };

        let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
        if whole.is_empty() || number.ends_with('.') || fraction.contains('.') {
            return Err(not_a_size());
        }
        let too_much = || invalid("it is more than the engine can be told of");
        let whole: u64 = whole.parse().map_err(|_| too_much())?;
        let mut bytes = whole.checked_mul(multiplier).ok_or_else(too_much)?;
        if !fraction.is_empty() {
            // The fraction of the unit, rounded down to a whole byte.
            let digits = fraction.len().min(18);
            let numerator: u128 = fraction[..digits].parse().map_err(|_| not_a_size())?;
            let denominator = 10_u128.pow(u32::try_from(digits).unwrap_or(18));
            let part = numerator * u128::from(multiplier) / denominator;
            bytes = bytes
                .checked_add(u64::try_from(part).map_err(|_| too_much())?)
                .ok_or_else(too_much)?;
        }

        if bytes < Self::MIN_BYTES {
            return Err(invalid(if unit.is_empty() {
                format!(
                    "{bytes} bytes is less than the 6 MiB allowed; \
                     give it a unit, as in {whole}m"
                )
            } else {
                "it is less than the 6 MiB allowed".to_owned()
            }));
        }
        if bytes > Self::MAX_BYTES {
            return Err(too_much());
        }
        Ok(Self { bytes })
    }

    /// The limit in bytes.
    pub fn bytes(self) -> u64 {
        self.bytes
    }
}

impl fmt::Display for MemoryLimit {
    /// The limit in the largest unit it is a whole number of: `4g`,
    /// `1536m`, `6291457b`. Read back, it is the same limit.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let units = [(TIB, 't'), (GIB, 'g'), (MIB, 'm'), (KIB, 'k')];
        match units.iter().find(|(size, _)| self.bytes % size == 0) {
            Some((size, unit)) => write!(f, "{}{unit}", self.bytes / size),
            None => write!(f, "{}b", self.bytes),
        }
    }
}

fn invalid(reason: impl Into<String>) -> InvalidValue {
    InvalidValue::new("memory limit", reason)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(value: &str) -> u64 {
        MemoryLimit::parse(value).unwrap().bytes()
    }

    fn reason(value: &str) -> String {
        MemoryLimit::parse(value).unwrap_err().reason().to_owned()
    }

    #[test]
    fn reads_sizes_in_binary_units() {
        assert_eq!(bytes("512m"), 512 * MIB);
        assert_eq!(bytes("4g"), 4 * GIB);
        assert_eq!(bytes("1t"), TIB);
        assert_eq!(bytes("8192k"), 8 * MIB);
    }

    #[test]
    fn reads_units_in_any_case_and_spelling() {
        for value in ["4g", "4G", "4gb", "4GB", "4GiB", "4gib", "4 g", " 4g "] {
            assert_eq!(bytes(value), 4 * GIB, "{value}");
        }
    }

    #[test]
    fn reads_a_number_without_a_unit_as_bytes() {
        assert_eq!(bytes("6291456"), 6 * MIB);
        assert_eq!(bytes("6291456b"), 6 * MIB);
    }

    #[test]
    fn reads_fractions_of_a_unit_rounded_down_to_a_byte() {
        assert_eq!(bytes("1.5g"), 1536 * MIB);
        assert_eq!(bytes("0.5g"), 512 * MIB);
        assert_eq!(bytes("6.0000001m"), 6 * MIB);
    }

    #[test]
    fn writes_itself_in_the_largest_whole_unit() {
        assert_eq!(MemoryLimit::parse("4096m").unwrap().to_string(), "4g");
        assert_eq!(MemoryLimit::parse("1.5g").unwrap().to_string(), "1536m");
        assert_eq!(MemoryLimit::parse("2t").unwrap().to_string(), "2t");
        assert_eq!(
            MemoryLimit::parse("6291457").unwrap().to_string(),
            "6291457b"
        );
    }

    #[test]
    fn reads_back_what_it_writes() {
        for value in ["512m", "1536m", "4g", "6145k", "6291457b"] {
            let limit = MemoryLimit::parse(value).unwrap();
            assert_eq!(MemoryLimit::parse(&limit.to_string()).unwrap(), limit);
        }
    }

    #[test]
    fn refuses_what_is_not_a_size() {
        for value in [
            "lots", "4x", "4 gigs", "-4g", ".5g", "4.g", "1.2.3g", "g", "4m4",
        ] {
            assert_eq!(
                reason(value),
                format!("`{value}` is not a size, such as 512m or 4g"),
                "{value}"
            );
        }
    }

    #[test]
    fn refuses_an_empty_limit() {
        assert_eq!(reason(" "), "it is empty");
    }

    #[test]
    fn refuses_less_than_docker_allows() {
        assert_eq!(reason("5m"), "it is less than the 6 MiB allowed");
        assert_eq!(
            reason("512"),
            "512 bytes is less than the 6 MiB allowed; give it a unit, as in 512m"
        );
    }

    #[test]
    fn refuses_more_than_the_engine_can_be_told_of() {
        assert_eq!(
            reason("99999999999t"),
            "it is more than the engine can be told of"
        );
        assert_eq!(
            reason("99999999999999999999"),
            "it is more than the engine can be told of"
        );
    }

    #[test]
    fn names_the_field_it_is() {
        assert_eq!(MemoryLimit::parse("x").unwrap_err().field(), "memory limit");
    }
}
