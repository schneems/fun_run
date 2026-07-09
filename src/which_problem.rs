//! Logic for `which_problem` integration
//!
#![cfg(feature = "which_problem")]

use crate::CmdError;
use std::ffi::OsString;
use std::process::Command;
use which_problem::Which;

/// Adds diagnostic information to a [`CmdError`] using `which_problem` if it is a [`CmdError::SystemError`]
///
/// A `CmdError::SystemError` means that the command could not be run (different than, it ran but
/// emitted an error). When that happens it usually means that either there's a typo in the command
/// program name, or there's an error with the system. For example if the PATH is empty, then the
/// OS will be be unable to find and run the executable.
///
/// To make this type of system debugging easier the `which_problem` crate simulates the logic of
/// `which <program name>` but emits detailed diagnostic information about the system including
/// things like missing or broken symlinks, invalid permissions, directories on the PATH that are
/// empty etc.
///
/// It's best used as a diagnostic for developers for why a CmdError::SystemError might have occurred.
/// For example, if the programmer executed the command with an empty PATH, this debugging tool
/// would help them find and fix the (otherwise) tedious to debug problem.
///
/// Using this feature may leak sensitive information about the system if the input is untrusted so
/// consider who has access to inputs, and who will view the outputs.
///
/// See the `which_problem` crate for more details.
///
/// This feature is experimental and may change in the future.
///
/// ```no_run
/// use fun_run::{self, CommandWithName};
/// use std::process::Command;
///
/// let mut cmd = Command::new("bundle");
/// cmd.arg("install");
/// cmd.named_output().map_err(|error| {
///     fun_run::map_which_problem(error, cmd.mut_cmd(), std::env::var_os("PATH"))
/// }).unwrap();
/// ```
#[cfg(feature = "which_problem")]
pub fn map_which_problem(
    error: CmdError,
    cmd: &mut Command,
    path_env: Option<OsString>,
) -> CmdError {
    match error {
        CmdError::SystemError(name, error) => {
            CmdError::SystemError(name, annotate_which_problem(error, cmd, path_env))
        }
        CmdError::NonZeroExitNotStreamed(_) | CmdError::NonZeroExitAlreadyStreamed(_) => error,
    }
}

/// Adds diagnostic information to an `std::io::Error` using `which_problem`
///
/// This feature is experimental
#[must_use]
#[cfg(feature = "which_problem")]
fn annotate_which_problem(
    error: std::io::Error,
    cmd: &mut Command,
    path_env: Option<OsString>,
) -> std::io::Error {
    let program = cmd.get_program().to_os_string();
    let current_working_dir = cmd.get_current_dir().map(std::path::Path::to_path_buf);
    let problem = Which {
        cwd: current_working_dir,
        program,
        path_env,
        ..Which::default()
    }
    .diagnose();

    let annotation = match problem {
        Ok(details) => format!("\nSystem diagnostic information:\n\n{details}"),
        Err(error) => {
            format!("\nInternal error while gathering diagnostic information:\n\n{error}")
        }
    };

    annotate_io_error(error, annotation)
}

/// Returns an IO error that displays the given annotation starting on
/// the next line.
///
/// Internal API used by `annotate_which_problem`
#[must_use]
#[cfg(feature = "which_problem")]
fn annotate_io_error(source: std::io::Error, annotation: String) -> std::io::Error {
    IoErrorAnnotation::new(source, annotation).into_io_error()
}

#[derive(Debug)]
#[cfg(feature = "which_problem")]
pub(crate) struct IoErrorAnnotation {
    source: std::io::Error,
    annotation: String,
}

#[cfg(feature = "which_problem")]
impl IoErrorAnnotation {
    pub(crate) fn new(source: std::io::Error, annotation: String) -> Self {
        Self { source, annotation }
    }

    pub(crate) fn into_io_error(self) -> std::io::Error {
        std::io::Error::new(self.source.kind(), self)
    }
}

#[cfg(feature = "which_problem")]
impl std::fmt::Display for IoErrorAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.source)?;
        f.write_str(&self.annotation)?;
        Ok(())
    }
}

#[cfg(feature = "which_problem")]
impl std::error::Error for IoErrorAnnotation {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }

    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}
