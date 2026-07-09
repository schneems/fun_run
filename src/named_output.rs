//! Logic for NamedOutput
//!
//!
use crate::CmdError;
use std::process::{ExitStatus, Output};

#[cfg(doc)]
use crate::ExitStatusFromCode;

/// Holds an [`Output`] of a command's execution along with its "name"
///
/// When paired with [`CmdError`] a `Result<NamedOutput, CmdError>` will retain the
/// "name" of the command regardless of success or failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedOutput {
    name: String,
    output: Output,
}

impl NamedOutput {
    /// Check status and convert into an error if nonzero (include output in error)
    ///
    /// Because the [NamedOutput] does not contain information about whether it was originally
    /// streamed or not, use this associated function when the output has not been made
    /// available to the user. This has the effect of showing it in the event of [CmdError].
    ///
    /// If the output was streamed to the user use [NamedOutput::nonzero_streamed]
    ///
    /// # Errors
    ///
    /// Returns an error if the status is not zero
    pub fn nonzero_captured(self) -> Result<NamedOutput, CmdError> {
        crate::nonzero_captured(self.name, self.output)
    }

    /// Check status and convert into an error if nonzero (hide output in error)
    ///
    /// Because the [NamedOutput] does not contain information about whether it was originally
    /// streamed or not, use this associated function when the output has was streamed to the user.
    /// This has the effect of hiding the output in the event of [CmdError] to prevent including
    /// duplicate information twice.
    ///
    /// If the output was not streamed to the user use [NamedOutput::nonzero_captured]
    ///
    /// # Errors
    ///
    /// Returns an error if the status is not zero
    pub fn nonzero_streamed(self) -> Result<NamedOutput, CmdError> {
        crate::nonzero_streamed(self.name, self.output)
    }

    /// Return the [`ExitStatus`] of the output
    #[must_use]
    pub fn status(&self) -> &ExitStatus {
        &self.output.status
    }

    /// Return raw stdout
    #[must_use]
    pub fn stdout(&self) -> &Vec<u8> {
        &self.output.stdout
    }

    /// Return raw stderr
    #[must_use]
    pub fn stderr(&self) -> &Vec<u8> {
        &self.output.stderr
    }

    /// Return lossy stdout as a String
    #[must_use]
    pub fn stdout_lossy(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).to_string()
    }

    /// Return lossy stderr as a String
    #[must_use]
    pub fn stderr_lossy(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).to_string()
    }

    /// Return name of the command that was run
    #[must_use]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Return reference of the original [Output]
    #[must_use]
    pub fn output(&self) -> &Output {
        &self.output
    }
}

impl AsRef<Output> for NamedOutput {
    fn as_ref(&self) -> &Output {
        &self.output
    }
}

impl<'a> From<&'a NamedOutput> for &'a Output {
    fn from(value: &'a NamedOutput) -> Self {
        &value.output
    }
}

impl From<NamedOutput> for Output {
    fn from(value: NamedOutput) -> Self {
        value.output
    }
}

/// Extension trait for [`Output`] to generate [`NamedOutput`]
///
/// The primary use case is exercising a function that takes [`NamedOutput`] in its arguments in a test.
///
/// # Examples
///
/// ```
/// use fun_run::OutputWithName;
///
/// let output = std::process::Output {
///     status: std::process::ExitStatus::default(),
///     stdout: Vec::new(),
///     stderr: Vec::new()
/// };
///
/// let named: fun_run::NamedOutput = output.named("exit 0");
/// assert_eq!(String::from("exit 0"), named.name());
/// ```
///
/// For generating an [`Output`] with a non-zero status on Unix you can use [`ExitStatusFromCode::from_code`],
/// which builds the [`ExitStatus`] from a plain exit code without you having to bit-shift the raw wait status yourself:
///
/// ```
/// use fun_run::{OutputWithName, ExitStatusFromCode};
///
/// let output = std::process::Output {
///     status: std::process::ExitStatus::from_code(42),
///     stdout: Vec::new(),
///     stderr: Vec::new()
/// };
///
/// assert_eq!(42, output.status.code().unwrap());
///
/// let named: fun_run::NamedOutput = output.named("exit 42");
/// let result = named.nonzero_captured();
/// assert!(result.is_err());
/// ```
pub trait OutputWithName {
    #[must_use]
    fn named(self, s: impl AsRef<str>) -> NamedOutput;
}

impl OutputWithName for Output {
    fn named(self, s: impl AsRef<str>) -> NamedOutput {
        NamedOutput {
            name: s.as_ref().to_string(),
            output: self,
        }
    }
}
