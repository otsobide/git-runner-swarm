//! Checks shared by the value objects that hold a name.

use crate::InvalidValue;

/// The characters a name is made of, wherever the domain takes one: ASCII
/// letters and digits, `-`, `_` and `.`. The same in a URL, a container
/// name, a label and a forge's list, so a name never needs escaping.
pub(crate) fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
}

/// Checks a name is 1 to `max` characters long, each one a
/// [`is_name_char`].
pub(crate) fn check_name(field: &'static str, value: &str, max: usize) -> Result<(), InvalidValue> {
    if value.is_empty() {
        return Err(InvalidValue::new(field, "it is empty"));
    }
    let length = value.chars().count();
    if length > max {
        return Err(InvalidValue::new(
            field,
            format!("it is {length} characters long, more than the {max} allowed"),
        ));
    }
    if let Some(c) = value.chars().find(|&c| !is_name_char(c)) {
        return Err(InvalidValue::new(
            field,
            format!("`{c}` is not allowed; only letters, digits, `-`, `_` and `.` are"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_letters_digits_dashes_underscores_and_dots() {
        assert_eq!(check_name("name", "Runner_1.a-b", 64), Ok(()));
    }

    #[test]
    fn refuses_an_empty_name() {
        let error = check_name("name", "", 64).unwrap_err();

        assert_eq!(error.reason(), "it is empty");
    }

    #[test]
    fn refuses_a_name_longer_than_the_maximum() {
        assert_eq!(check_name("name", &"a".repeat(4), 4), Ok(()));
        assert!(check_name("name", &"a".repeat(5), 4).is_err());
    }

    #[test]
    fn refuses_a_name_with_a_character_outside_the_set() {
        for name in ["with space", "with/slash", "with,comma", "ñ", "tab\t"] {
            assert!(
                check_name("name", name, 64).is_err(),
                "{name:?} was accepted"
            );
        }
    }
}
