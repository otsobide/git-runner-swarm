use std::fmt;

use crate::InvalidValue;

/// The secret a forge issues to register a runner with.
///
/// Never shown: its `Debug` says only that a token is there, and it has no
/// `Display`, so it cannot end up in a log line or an error message by
/// accident. [`RegistrationToken::expose`] hands it to whatever registers
/// the runner, which is the one place it is meant to go.
#[derive(Clone, PartialEq, Eq)]
pub struct RegistrationToken(String);

impl RegistrationToken {
    /// Longer than any token a forge issues.
    pub const MAX_LENGTH: usize = 512;

    /// Spaces around the token are ignored, as a token pasted into a form
    /// often carries them.
    ///
    /// # Errors
    ///
    /// When the token is empty, longer than
    /// [`RegistrationToken::MAX_LENGTH`], or holds whitespace or a control
    /// character within it. The error never repeats the token.
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidValue> {
        let value = value.into();
        let value = value.trim();
        if value.is_empty() {
            return Err(InvalidValue::new("registration token", "it is empty"));
        }
        if value.len() > Self::MAX_LENGTH {
            return Err(InvalidValue::new(
                "registration token",
                format!(
                    "it is longer than the {} characters allowed",
                    Self::MAX_LENGTH
                ),
            ));
        }
        if value.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err(InvalidValue::new(
                "registration token",
                "it holds whitespace or a control character",
            ));
        }
        Ok(Self(value.to_owned()))
    }

    /// The token itself, for whatever registers the runner with it.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RegistrationToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RegistrationToken(<redacted>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "AESOHC2SULAFI7EXAMPLE5Y2Y2";

    #[test]
    fn hands_over_the_token_without_the_spaces_around_it() {
        let token = RegistrationToken::new(format!("  {TOKEN}\n")).unwrap();

        assert_eq!(token.expose(), TOKEN);
    }

    #[test]
    fn never_shows_the_token_when_debugged() {
        let token = RegistrationToken::new(TOKEN).unwrap();

        let debugged = format!("{token:?}");

        assert!(!debugged.contains(TOKEN));
        assert_eq!(debugged, "RegistrationToken(<redacted>)");
    }

    #[test]
    fn refuses_an_empty_token() {
        assert!(RegistrationToken::new("   ").is_err());
    }

    #[test]
    fn refuses_a_token_with_whitespace_within_it_without_repeating_it() {
        let error = RegistrationToken::new("AESO HC2S").unwrap_err();

        assert_eq!(error.field(), "registration token");
        assert!(!error.to_string().contains("AESO"));
    }

    #[test]
    fn refuses_a_token_longer_than_any_forge_issues() {
        let long = "a".repeat(RegistrationToken::MAX_LENGTH + 1);

        assert!(RegistrationToken::new(long).is_err());
    }
}
