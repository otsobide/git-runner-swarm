use std::fmt;

use crate::InvalidValue;

/// The container image a runner runs, as an engine pulls it:
/// `ghcr.io/actions/actions-runner:latest`.
///
/// Checked only as far as any engine would agree: not empty, no longer
/// than [`ImageReference::MAX_LENGTH`], no whitespace or control
/// characters. Whether the image exists is for the engine to say when it
/// pulls it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageReference(String);

impl ImageReference {
    /// Longer than any reference an engine takes.
    pub const MAX_LENGTH: usize = 512;

    /// # Errors
    ///
    /// When the reference is empty, longer than
    /// [`ImageReference::MAX_LENGTH`], or holds whitespace or a control
    /// character.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidValue> {
        let value = value.into();
        if value.is_empty() {
            return Err(InvalidValue::new("image", "it is empty"));
        }
        if value.len() > Self::MAX_LENGTH {
            return Err(InvalidValue::new(
                "image",
                format!(
                    "it is longer than the {} characters allowed",
                    Self::MAX_LENGTH
                ),
            ));
        }
        if value.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(InvalidValue::new(
                "image",
                "it holds whitespace or a control character",
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ImageReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_reference_it_is_given() {
        let image = ImageReference::new("ghcr.io/actions/actions-runner:2.338.2").unwrap();

        assert_eq!(image.as_str(), "ghcr.io/actions/actions-runner:2.338.2");
    }

    #[test]
    fn refuses_what_no_engine_would_pull() {
        for value in [
            "",
            "actions runner",
            "image\n",
            &"a".repeat(ImageReference::MAX_LENGTH + 1),
        ] {
            assert_eq!(ImageReference::new(value).unwrap_err().field(), "image");
        }
    }
}
