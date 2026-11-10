//! What a runner logs: the lines the runner, and the jobs it runs, write
//! to their output.

use crate::InvalidValue;

/// How many of a runner's latest log lines to read: a bounded number, so
/// that reading a log never means reading all of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LogTail(u32);

impl LogTail {
    /// How many lines are read unless more or fewer are asked for.
    pub const DEFAULT_LINES: u32 = 200;
    /// The most lines that may be read at once.
    pub const MAX_LINES: u32 = 5000;

    /// The last `lines` lines.
    ///
    /// # Errors
    ///
    /// When `lines` is 0, or more than [`LogTail::MAX_LINES`].
    pub fn new(lines: u32) -> Result<Self, InvalidValue> {
        if lines == 0 {
            return Err(invalid("it is 0; at least one line is read"));
        }
        if lines > Self::MAX_LINES {
            return Err(invalid(format!(
                "it is more than the {} lines that may be read at once",
                Self::MAX_LINES
            )));
        }
        Ok(Self(lines))
    }

    /// Reads a number of lines, written in digits. Spaces around it are
    /// ignored.
    ///
    /// # Errors
    ///
    /// When it is not a whole number, or [`LogTail::new`] refuses it.
    pub fn parse(value: &str) -> Result<Self, InvalidValue> {
        let value = value.trim();
        if value.is_empty() || !value.chars().all(|c| c.is_ascii_digit()) {
            return Err(invalid(format!("`{value}` is not a number of lines")));
        }
        Self::new(value.parse().unwrap_or(u32::MAX))
    }

    pub fn lines(self) -> u32 {
        self.0
    }
}

impl Default for LogTail {
    fn default() -> Self {
        Self(Self::DEFAULT_LINES)
    }
}

/// Which output a line was written to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogStream {
    /// The standard output.
    Out,
    /// The standard error.
    Err,
}

impl LogStream {
    /// `stdout` or `stderr`.
    pub fn as_str(self) -> &'static str {
        match self {
            LogStream::Out => "stdout",
            LogStream::Err => "stderr",
        }
    }
}

/// One line of a runner's log, without its line break.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    stream: LogStream,
    text: String,
}

impl LogLine {
    pub fn new(stream: LogStream, text: impl Into<String>) -> Self {
        Self {
            stream,
            text: text.into(),
        }
    }

    pub fn stream(&self) -> LogStream {
        self.stream
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

fn invalid(reason: impl Into<String>) -> InvalidValue {
    InvalidValue::new("lines", reason)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_last_200_lines_unless_told_otherwise() {
        assert_eq!(LogTail::default().lines(), 200);
    }

    #[test]
    fn reads_as_many_lines_as_asked_up_to_the_most_allowed() {
        assert_eq!(LogTail::parse("1").unwrap().lines(), 1);
        assert_eq!(LogTail::parse(" 50 ").unwrap().lines(), 50);
        assert_eq!(LogTail::parse("5000").unwrap().lines(), 5000);
    }

    #[test]
    fn refuses_to_read_no_lines_or_too_many() {
        assert_eq!(
            LogTail::parse("0").unwrap_err().reason(),
            "it is 0; at least one line is read"
        );
        for value in ["5001", "99999999999"] {
            assert_eq!(
                LogTail::parse(value).unwrap_err().reason(),
                "it is more than the 5000 lines that may be read at once",
                "{value}"
            );
        }
    }

    #[test]
    fn refuses_what_is_not_a_number_of_lines() {
        for value in ["", "all", "-1", "1.5", "+3"] {
            let error = LogTail::parse(value).unwrap_err();
            assert_eq!(error.field(), "lines");
            assert_eq!(
                error.reason(),
                format!("`{}` is not a number of lines", value.trim())
            );
        }
    }

    #[test]
    fn names_each_output_as_a_terminal_does() {
        assert_eq!(LogStream::Out.as_str(), "stdout");
        assert_eq!(LogStream::Err.as_str(), "stderr");
    }

    #[test]
    fn holds_a_line_and_the_output_it_was_written_to() {
        let line = LogLine::new(LogStream::Err, "Http response code: NotFound");

        assert_eq!(line.stream(), LogStream::Err);
        assert_eq!(line.text(), "Http response code: NotFound");
    }
}
