//! Writing what pace says to the standard output, as an error instead of a panic.
//!
//! Every command writes through [`print_lines`] or [`print_bytes`]. A reader
//! that closed the pipe, as `head` does, had what it wanted, so pace ends
//! quietly then. Any other failed write is a [`PaceError`].

use std::fmt::Display;
use std::io::{self, Write};

use pace_error::{ErrorKind, PaceError, PaceResult};

/// Write `lines` to the standard output, each with a newline.
///
/// # Errors
///
/// If the standard output takes no more, other than a closed pipe.
pub fn print_lines<I>(lines: I) -> PaceResult<()>
where
    I: IntoIterator,
    I::Item: Display,
{
    write_lines(&mut io::stdout().lock(), lines)
}

/// Write `bytes` to the standard output as they are.
///
/// # Errors
///
/// If the standard output takes no more, other than a closed pipe.
pub fn print_bytes(bytes: &[u8]) -> PaceResult<()> {
    let mut out = io::stdout().lock();
    written(out.write_all(bytes).and_then(|()| out.flush()))
}

/// Write `lines` to `out`, each with a newline, and flush it.
///
/// # Errors
///
/// If `out` takes no more, other than a closed pipe.
pub fn write_lines<I>(out: &mut impl Write, lines: I) -> PaceResult<()>
where
    I: IntoIterator,
    I::Item: Display,
{
    let write = || -> io::Result<()> {
        for line in lines {
            writeln!(out, "{line}")?;
        }
        out.flush()
    };
    written(write())
}

/// A closed pipe is done; any other failure an error.
fn written(result: io::Result<()>) -> PaceResult<()> {
    match result {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result.map_err(|error| {
            PaceError::with_source(
                ErrorKind::InputOutput,
                "cannot write to the standard output",
                error,
            )
            .attach_hint(
                "send the output where it fits, such as a disk with space; `pace now` shows what \
                 pace recorded",
            )
            .attach_error_code("pace::output::write")
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A writer that fails every write with `kind`.
    struct Failing(io::ErrorKind);

    impl Write for Failing {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from(self.0))
        }
    }

    #[test]
    fn each_line_ends_with_newline() -> PaceResult<()> {
        let mut out = Vec::new();

        write_lines(&mut out, ["Began ad5e at 09:00.", "Project: acme."])?;

        assert_eq!(out, b"Began ad5e at 09:00.\nProject: acme.\n");
        Ok(())
    }

    #[test]
    fn closed_pipe_is_no_error() {
        let written = write_lines(&mut Failing(io::ErrorKind::BrokenPipe), ["line"]);

        assert!(written.is_ok(), "{written:?}");
    }

    #[test]
    fn full_disk_is_error() {
        let written = write_lines(&mut Failing(io::ErrorKind::StorageFull), ["line"]);

        assert!(
            written.is_err_and(|error| error.to_string() == "cannot write to the standard output")
        );
    }
}
