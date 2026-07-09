//! Module for CmdError

use crate::exit_status;
use crate::NamedOutput;
use std::fmt::Display;
use std::process::{ExitStatus, Output};

/// Who says ([`Command`]) errors can't be fun?
///
/// Fun run errors include all the info a user needs to debug, like
/// the name of the command that failed and any outputs (like error messages
/// in stderr).
///
/// Fun run errors don't overwhelm end users, so by default if stderr is already
/// streamed the output won't be duplicated.
///
/// Enjoy if you want, skip if you don't. Fun run errors are not mandatory.
///
/// Error output formatting is unstable
#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum CmdError {
    /// Command encountered an [`std::io::Error`] while trying to run
    ///
    /// The reasons why this can happen are platform specific, mostly it means that the process
    /// could not be launched for some reason. The most common reason is that program name
    /// doesn't exist or cannot be found:
    ///
    /// ```
    /// use fun_run::CommandWithName;
    ///
    /// let result = std::process::Command::new("commandDoesNotExist").named_output();
    /// match result{
    ///     Err(fun_run::CmdError::SystemError(_, _)) => println!("could not boot"),
    ///     _ => unimplemented!()
    /// }
    /// ```
    SystemError(String, std::io::Error),

    /// Command booted, was NOT streamed, but [`ExitStatus::success`] reported that it failed.
    ///
    /// Will display the stdout/stderr to the user when displayed (since it wasn't previously)
    /// streamed.
    NonZeroExitNotStreamed(NamedOutput),

    /// Command booted, WAS streamed, but [`ExitStatus::success`] reported that it failed.
    ///
    /// The command WAS streamed to the end user, so stdout/stderr does not
    /// need to be printed again when rendering the error.
    NonZeroExitAlreadyStreamed(NamedOutput),
}

impl Display for CmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmdError::SystemError(name, error) => {
                write!(f, "Could not run command `{name}`. {error}")
            }
            CmdError::NonZeroExitNotStreamed(named_output) => {
                let stdout = display_out_or_empty(&named_output.output.stdout);
                let stderr = display_out_or_empty(&named_output.output.stderr);

                writeln!(f, "Command failed `{name}`", name = named_output.name())?;
                writeln!(
                    f,
                    "exit status: {status}",
                    status = exit_status::bashify(&named_output.output.status)
                )?;
                if let Some(signal_line) = exit_status::signal_line(&named_output.output.status) {
                    writeln!(f, "{signal_line}")?;
                }
                writeln!(f, "stdout: {stdout}",)?;
                write!(f, "stderr: {stderr}",)
            }
            CmdError::NonZeroExitAlreadyStreamed(named_output) => {
                writeln!(f, "Command failed `{name}`", name = named_output.name())?;
                writeln!(
                    f,
                    "exit status: {status}",
                    status = exit_status::bashify(&named_output.output.status)
                )?;
                if let Some(signal_line) = exit_status::signal_line(&named_output.output.status) {
                    writeln!(f, "{signal_line}")?;
                }
                writeln!(f, "stdout: <see above>")?;
                write!(f, "stderr: <see above>")
            }
        }
    }
}

impl std::error::Error for CmdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CmdError::SystemError(_, io_err) => Some(io_err),
            CmdError::NonZeroExitNotStreamed(_) | CmdError::NonZeroExitAlreadyStreamed(_) => None,
        }
    }
}

impl CmdError {
    /// Returns a display representation of the command that failed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fun_run::CommandWithName;
    /// use std::process::Command;
    ///
    /// let result = Command::new("cat")
    ///     .arg("mouse.txt")
    ///     .named_output();
    ///
    /// match result {
    ///     Ok(_) => unimplemented!(),
    ///     Err(error) => assert_eq!(error.name().to_string(), "cat mouse.txt")
    /// }
    /// ```
    #[must_use]
    pub fn name(&self) -> std::borrow::Cow<'_, str> {
        match self {
            CmdError::SystemError(name, _) => name.into(),
            CmdError::NonZeroExitNotStreamed(out) | CmdError::NonZeroExitAlreadyStreamed(out) => {
                out.name.as_str().into()
            }
        }
    }

    /// Returns named output if the command ran
    pub fn output(&self) -> Option<&NamedOutput> {
        match self {
            CmdError::SystemError(_, _) => None,
            CmdError::NonZeroExitNotStreamed(named_output) => Some(named_output),
            CmdError::NonZeroExitAlreadyStreamed(named_output) => Some(named_output),
        }
    }

    /// Returns the OS [`ExitStatus`] of the command.
    ///
    /// For [`CmdError::SystemError`] the command never ran, so there is no real
    /// OS exit status. In that case a value derived from the underlying
    /// [`std::io::Error`] is returned. It is only guaranteed to be non-zero, so
    /// prefer inspecting the [`std::io::Error`] and its [`std::io::ErrorKind`]
    /// directly rather than relying on the exact code.
    ///
    /// For more control you can construct your own status:
    ///
    /// ```
    /// #[cfg(not(unix))] { return; }
    /// use std::process::{Command, ExitStatus};
    /// use fun_run::{CommandWithName, ExitStatusFromCode};
    ///
    /// let mut command = Command::new("becho");
    /// command
    ///     .arg("hello world");
    ///
    /// let result = command
    ///     .named_output();
    ///
    /// let status = result
    ///     .unwrap_err()
    ///     .output()
    ///     .map(|output| output.status().to_owned())
    ///     .unwrap_or_else(|| ExitStatus::from_code(1));
    ///
    /// assert!(!status.success());
    /// ```
    pub fn status(&self) -> ExitStatus {
        match self {
            CmdError::SystemError(_, error) => exit_status::status_from_error(error),
            CmdError::NonZeroExitNotStreamed(named_output) => named_output.status().to_owned(),
            CmdError::NonZeroExitAlreadyStreamed(named_output) => named_output.status().to_owned(),
        }
    }
}

impl From<CmdError> for NamedOutput {
    /// When the error is a [`CmdError::SystemError`], the resulting
    /// [`ExitStatus`] is synthetic and imprecise. The only stability guarantee
    /// is that it will be non-zero.
    fn from(value: CmdError) -> Self {
        match value {
            CmdError::SystemError(name, error) => NamedOutput {
                name,
                output: Output {
                    status: exit_status::status_from_error(&error),
                    stdout: Vec::new(),
                    stderr: error.to_string().into_bytes(),
                },
            },
            CmdError::NonZeroExitNotStreamed(named)
            | CmdError::NonZeroExitAlreadyStreamed(named) => named,
        }
    }
}

fn display_out_or_empty(contents: &[u8]) -> String {
    let contents = String::from_utf8_lossy(contents);
    if contents.trim().is_empty() {
        "<empty>".to_string()
    } else {
        contents.to_string()
    }
}
