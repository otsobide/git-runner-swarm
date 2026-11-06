use thiserror::Error;

/// A value refused by the value object it was given to build: which value
/// it was, and why it was refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid {field}: {reason}")]
pub struct InvalidValue {
    field: &'static str,
    reason: String,
}

impl InvalidValue {
    pub(crate) fn new(field: &'static str, reason: impl Into<String>) -> Self {
        Self {
            field,
            reason: reason.into(),
        }
    }

    /// The value that was refused, named as a person filling a form would
    /// know it: `runner name`, `label`, `URL`.
    pub fn field(&self) -> &'static str {
        self.field
    }

    /// Why it was refused.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn says_which_value_was_refused_and_why() {
        let error = InvalidValue::new("runner name", "it is empty");

        assert_eq!(error.field(), "runner name");
        assert_eq!(error.reason(), "it is empty");
        assert_eq!(error.to_string(), "invalid runner name: it is empty");
    }
}
