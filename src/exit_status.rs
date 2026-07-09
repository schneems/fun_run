//! Exit status logic
use std::process::ExitStatus;

/// Extension trait for [`ExitStatus`] to build an instance based on exit code
///
/// Confusingly the [`std::os::unix::process::ExitStatusExt::from_raw`] function does NOT return its own input on Unix:
///
/// ```
/// # #[cfg(unix)] {
/// use std::process::ExitStatus;
/// use std::os::unix::process::ExitStatusExt;
///
/// let status = ExitStatus::from_raw(41);
/// assert_eq!(None, status.code());
///
/// // The input has to be bit-shifted by 8 to get it to work:
/// let status = ExitStatus::from_raw(41<<8);
/// assert_eq!(Some(41), status.code());
/// # }
/// ```
///
/// That's because the input isn't an exit code but rather data structure from `wait` <https://pubs.opengroup.org/onlinepubs/9799919799/functions/wait.html>.
/// To input a known status code, you can bitshift it by 8 OR you can use this nice helper extension
/// trait:
///
/// ```
/// use std::process::ExitStatus;
/// use fun_run::ExitStatusFromCode;
///
/// let status = ExitStatus::from_code(41);
/// assert_eq!(Some(41), status.code());
/// ```
///
/// The main use case for generating a manual [`ExitStatus`] is for unit testing.
pub trait ExitStatusFromCode {
    /// Build an [`ExitStatus`] from an exit code.
    ///
    /// Takes a [`u8`] on Unix because exit codes there are limited to
    /// `0..=255`. Non-uniform type signature with windows provides infallible api
    /// for a correctness tradeoff.
    #[cfg(unix)]
    #[must_use]
    fn from_code(code: u8) -> ExitStatus;

    /// Build an [`ExitStatus`] from an exit code.
    ///
    /// Takes a [`u32`] on Windows because exit codes there are full 32-bit
    /// `DWORD` values.  Non-uniform type signature with unix provides infallible api
    /// for a correctness tradeoff.
    #[cfg(windows)]
    #[must_use]
    fn from_code(code: u32) -> ExitStatus;
}

impl ExitStatusFromCode for ExitStatus {
    #[cfg(unix)]
    fn from_code(code: u8) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;

        // `from_raw` expects a `wait`-style status (the exit code lives in the
        // upper 8 bits) as an i32. Because `code` is a `u8` (0..=255), the
        // `as i32` cast is always lossless, then we shift it into place.
        //
        // Binary layout for `code = 0x2A` (42):
        //
        //   code as i32     0000_0000 0000_0000 0000_0000 0010_1010  = 42
        //   (..) << 8       0000_0000 0000_0000 0010_1010 0000_0000  = 0x2A00
        //
        // The `u8` input is what keeps this correct: callers cannot supply a
        // value above 255, so there are no high bits to mask off.
        ExitStatus::from_raw((code as i32) << 8)
    }

    #[cfg(windows)]
    fn from_code(code: u32) -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;

        // On Windows `from_raw` takes the exit code directly. Unlike Unix there
        // is no `wait`-style encoding, so no `& 0xff` masking/shifting is needed
        // (Windows exit codes are full u32 values, not limited to 0..=255).
        ExitStatus::from_raw(code)
    }
}

/// Fakes an [`ExitStatus`] based on a [`std::io::Error`]
pub(crate) fn status_from_error(error: &std::io::Error) -> ExitStatus {
    use std::io::ErrorKind;

    let code = match error.kind() {
        ErrorKind::NotFound => 127, // ENOENT
        // Found but not executable -> "cannot execute"
        ErrorKind::PermissionDenied      // EACCES
        | ErrorKind::IsADirectory        // EISDIR
        | ErrorKind::NotADirectory       // ENOTDIR
        | ErrorKind::ArgumentListTooLong // E2BIG
        | ErrorKind::OutOfMemory         // ENOMEM
        | ErrorKind::ExecutableFileBusy  // ETXTBSY
        => 126,
        _ => 1,
    };
    ExitStatus::from_code(code)
}

/// Returns a printable description of the signal that killed a process
/// (if it was killed by a signal)
pub(crate) fn signal_line(status: &ExitStatus) -> Option<String> {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;

        status.signal().map(|signal| {
            // https://pubs.opengroup.org/onlinepubs/9799919799/utilities/kill.html
            let name = match signal {
                1 => "SIGHUP",
                2 => "SIGINT",
                3 => "SIGQUIT",
                6 => "SIGABRT",
                9 => "SIGKILL",
                14 => "SIGALRM",
                15 => "SIGTERM",
                _ => "",
            };

            if name.is_empty() {
                format!("signal: {signal}")
            } else {
                format!("signal: {signal} ({name})")
            }
        })
    }
    #[cfg(windows)]
    {
        let _ = status;
        None
    }
}

/// Unix does not return a code for a process that was terminated
/// via a signal (like SIGKILL). People are used to bash convention
/// where a SIGKILL-ing something would give `$?` of `143`. That
/// is because bash makes a simplifying assumption to collapse signal and exit code
/// into a single shared number:
///
/// [From GNU bash](https://web.archive.org/web/20260625050034/https://www.gnu.org/software/bash/manual/bash.html#Exit-Status-1)
///
/// > [...] a fatal signal whose number is N, Bash uses the value 128+N as the exit status.
///
/// People expect this number implicitly, so we replicate that behavior.
pub(crate) fn bashify(status: &ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;

        status
            .code()
            .or_else(|| status.signal().map(|s| 128 + s))
            .unwrap_or(1)
    }
    #[cfg(windows)]
    {
        status.code().unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(unix)]
    fn maps_signal_to_exit() {
        use std::os::unix::process::ExitStatusExt;

        let sigterm = 15;
        let status = ExitStatus::from_raw(sigterm);
        assert_eq!(None, status.code());
        assert_eq!(Some(15), status.signal());

        let code = bashify(&status);
        assert_eq!(143, code);
    }

    #[test]
    #[cfg(unix)]
    fn maps_signal_to_human_readable_output() {
        use std::os::unix::process::ExitStatusExt;

        let sigterm = 15;
        let status = ExitStatus::from_raw(sigterm);
        assert_eq!(None, status.code());
        assert_eq!(Some(15), status.signal());

        assert_eq!(
            String::from("signal: 15 (SIGTERM)"),
            signal_line(&status).unwrap()
        );

        let signal = 126;
        let status = ExitStatus::from_raw(signal);
        assert_eq!(None, status.code());
        assert_eq!(Some(126), status.signal());

        assert_eq!(String::from("signal: 126"), signal_line(&status).unwrap());
    }

    #[test]
    fn maps_error_kinds_to_shell_codes() {
        use std::io::{Error, ErrorKind};

        assert_eq!(
            Some(127),
            status_from_error(&Error::from(ErrorKind::NotFound)).code()
        );
        assert_eq!(
            Some(126),
            status_from_error(&Error::from(ErrorKind::PermissionDenied)).code()
        );
        assert_eq!(
            Some(1),
            status_from_error(&Error::from(ErrorKind::Other)).code()
        );
    }
}
