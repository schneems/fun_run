#![doc = include_str!("../README.md")]

use command::output_and_write_streams;
use regex::Regex;
use std::ffi::OsString;
use std::fmt::Display;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Output;
use std::sync::LazyLock;
#[cfg(feature = "which_problem")]
use which_problem::Which;

mod command;

/// Rename your commands:
///
/// ```no_run
/// use fun_run::CommandWithName;
/// use std::process::Command;
///
/// let result = Command::new("gem")
///     .args(["install", "bundler", "-v", "2.4.1.7"])
///     // Overwrites default command name which would include extra arguments
///     .named("gem install")
///     .stream_output(std::io::stdout(), std::io::stderr());
///
/// match result {
///     Ok(output) => {
///         assert_eq!("bundle install", &output.name())
///     },
///     Err(varient) => {
///         assert_eq!("bundle install", &varient.name())
///     }
/// }
/// ```
///
/// Or include important env vars in the name:
///
/// ```no_run
/// use fun_run::{self, CommandWithName};
/// use std::process::Command;
/// use std::collections::HashMap;
///
/// let env = std::env::vars_os().collect::<HashMap<_, _>>();
///
///  let result = Command::new("gem")
///      .args(["install", "bundler", "-v", "2.4.1.7"])
///      .envs(&env)
///      // Overwrites default command name
///      .named_fn(|cmd| {
///          // Annotate command with GEM_HOME env var
///          fun_run::display_with_env_keys(cmd, &env, ["GEM_HOME"])
///      })
///      .stream_output(std::io::stdout(), std::io::stderr());
///
///  match result {
///      Ok(output) => {
///          assert_eq!(
///              "GEM_HOME=\"/usr/bin/local/.gems\" gem install bundler -v 2.4.1.7",
///              &output.name()
///          )
///      }
///      Err(varient) => {
///          assert_eq!(
///              "GEM_HOME=\"/usr/bin/local/.gems\" gem install bundler -v 2.4.1.7",
///              &varient.name()
///          )
///      }
///  }
/// ```
pub trait CommandWithName {
    /// Returns the desired display name of the command
    fn name(&mut self) -> String;

    /// Returns a reference to `&mut Command`
    ///
    /// This is useful for passing to other libraries.
    fn mut_cmd(&mut self) -> &mut Command;

    /// Rename a command via a given string
    ///
    /// This can be useful if a part of the command is distracting or surprising or if you
    /// desire to include additional information such as displaying environment variables.
    ///
    /// Alternatively see [CommandWithName::named_fn]
    ///
    /// Example:
    ///
    /// ```
    /// use fun_run::CommandWithName;
    ///
    /// let mut command = std::process::Command::new("bin/bundle");
    /// command.arg("install");
    /// command.arg("--no-doc");
    ///
    /// let mut cmd = command.named("bundle install");
    /// assert_eq!("bundle install", cmd.name());
    /// ```
    fn named(&mut self, s: impl AsRef<str>) -> NamedCommand<'_> {
        let name = s.as_ref().to_string();
        let command = self.mut_cmd();
        NamedCommand { name, command }
    }

    /// Rename a command via a given function
    ///
    /// This can be useful if a part of the command is distracting or surprising or if you
    /// desire to include additional information such as displaying environment variables.
    ///
    /// Alternatively see [CommandWithName::named]
    ///
    /// Example:
    ///
    /// ```
    /// use fun_run::CommandWithName;
    ///
    /// let mut command = std::process::Command::new("bundle");
    /// command.arg("install");
    ///
    /// let mut cmd = command.named_fn(|cmd| cmd.name().replace("bundle", "bin/bundle").to_string());
    /// assert_eq!("bin/bundle install", cmd.name());
    /// ```
    #[allow(clippy::needless_lifetimes)]
    fn named_fn<'a>(&'a mut self, f: impl FnOnce(&mut Command) -> String) -> NamedCommand<'a> {
        let cmd = self.mut_cmd();
        let name = f(cmd);
        self.named(name)
    }

    /// Runs the command without streaming
    ///
    /// # Errors
    ///
    /// Returns `CmdError::SystemError` if the system is unable to run the command.
    /// Returns `CmdError::NonZeroExitNotStreamed` if the exit code is not zero.
    fn named_output(&mut self) -> Result<NamedOutput, CmdError> {
        let name = self.name();
        self.mut_cmd()
            .output()
            .map_err(|io_error| CmdError::SystemError(name.clone(), io_error))
            .map(|output| NamedOutput {
                name: name.clone(),
                output,
            })
            .and_then(NamedOutput::nonzero_captured)
    }

    /// Runs the command and streams to the given writers
    ///
    /// # Errors
    ///
    /// Returns `CmdError::SystemError` if the system is unable to run the command
    /// Returns `CmdError::NonZeroExitAlreadyStreamed` if the exit code is not zero.
    fn stream_output<OW, EW>(
        &mut self,
        stdout_write: OW,
        stderr_write: EW,
    ) -> Result<NamedOutput, CmdError>
    where
        OW: Write + Send,
        EW: Write + Send,
    {
        let name = &self.name();
        let cmd = self.mut_cmd();

        output_and_write_streams(cmd, stdout_write, stderr_write)
            .map_err(|io_error| CmdError::SystemError(name.clone(), io_error))
            .map(|output| NamedOutput {
                name: name.clone(),
                output,
            })
            .and_then(NamedOutput::nonzero_streamed)
    }
}

impl CommandWithName for Command {
    fn name(&mut self) -> String {
        crate::display(self)
    }

    fn mut_cmd(&mut self) -> &mut Command {
        self
    }
}

/// It's a command, with a name
///
/// This struct allows us to re-name an existing [Command] via the [CommandWithName] trait associated
/// functions. When one of those functions such as [CommandWithName::named_fn] or [CommandWithName::named]
/// are called, Rust needs somewhere for the new name string to live, so we move it over into this struct
/// which also implements [CommandWithName]. You can gain access to the original [Command] reference
/// via `CommandWithName::mut_cmd`
pub struct NamedCommand<'a> {
    name: String,
    command: &'a mut Command,
}

impl CommandWithName for NamedCommand<'_> {
    fn name(&mut self) -> String {
        self.name.to_string()
    }

    fn mut_cmd(&mut self) -> &mut Command {
        self.command
    }
}

/// Holds a the `Output` of a command's execution along with it's "name"
///
/// When paired with `CmdError` a `Result<NamedOutput, CmdError>` will retain the
/// "name" of the command regardless of succss or failure.
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
        nonzero_captured(self.name, self.output)
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
        nonzero_streamed(self.name, self.output)
    }

    /// Return the ExitStatus of the output
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

// https://github.com/jimmycuadra/rust-shellwords/blob/d23b853a850ceec358a4137d5e520b067ddb7abc/src/lib.rs#L23
static QUOTE_ARG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([^A-Za-z0-9_\-.,:/@\n])").expect("clippy checked"));

/// Converts a command and its arguments into a user readable string
///
/// Example
///
/// ```rust
/// use std::process::Command;
/// use fun_run;
///
/// let name = fun_run::display(Command::new("bundle").arg("install"));
/// assert_eq!(String::from("bundle install"), name);
/// ```
#[must_use]
pub fn display(command: &mut Command) -> String {
    vec![command.get_program().to_string_lossy().to_string()]
        .into_iter()
        .chain(
            command
                .get_args()
                .map(std::ffi::OsStr::to_string_lossy)
                .map(|arg| {
                    if QUOTE_ARG_RE.is_match(&arg) {
                        format!("{arg:?}")
                    } else {
                        format!("{arg}")
                    }
                }),
        )
        .collect::<Vec<String>>()
        .join(" ")
}

/// Converts a command, arguments, and specified environment variables to user readable string
///
/// Example
///
/// ```rust
/// use std::process::Command;
/// use fun_run;
/// use std::collections::HashMap;
///
/// let mut env = std::env::vars().collect::<HashMap<_,_>>();
/// env.insert("RAILS_ENV".to_string(), "production".to_string());
///
/// let mut command = Command::new("bundle");
/// command.arg("install").envs(&env);
///
/// let name = fun_run::display_with_env_keys(&mut command, &env, ["RAILS_ENV"]);
/// assert_eq!(String::from(r#"RAILS_ENV="production" bundle install"#), name);
/// ```
#[must_use]
pub fn display_with_env_keys<E, K, V, I, O>(cmd: &mut Command, env: E, keys: I) -> String
where
    E: IntoIterator<Item = (K, V)>,
    K: Into<OsString>,
    V: Into<OsString>,
    I: IntoIterator<Item = O>,
    O: Into<OsString>,
{
    let env = env
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect::<std::collections::HashMap<OsString, OsString>>();

    keys.into_iter()
        .map(|key| {
            let key = key.into();
            format!(
                "{}={:?}",
                key.to_string_lossy(),
                env.get(&key).cloned().unwrap_or_else(|| OsString::from(""))
            )
        })
        .chain([display(cmd)])
        .collect::<Vec<String>>()
        .join(" ")
}

/// Who says (`Command`) errors can't be fun?
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
    SystemError(String, std::io::Error),

    NonZeroExitNotStreamed(NamedOutput),

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
                    status = named_output.output.status.code().unwrap_or(1)
                )?;
                writeln!(f, "stdout: {stdout}",)?;
                write!(f, "stderr: {stderr}",)
            }
            CmdError::NonZeroExitAlreadyStreamed(named_output) => {
                writeln!(f, "Command failed `{name}`", name = named_output.name())?;
                writeln!(
                    f,
                    "exit status: {status}",
                    status = named_output.output.status.code().unwrap_or(1)
                )?;
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
    /// Example:
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

    /// Returns the OS [ExitStatus] if one was provided
    ///
    /// If the command failed and no error can be produced a default non-zero value will be returned
    pub fn status(&self) -> ExitStatus {
        match self {
            CmdError::SystemError(_, error) => {
                ExitStatus::from_raw(error.raw_os_error().unwrap_or(-1))
            }
            CmdError::NonZeroExitNotStreamed(named_output) => named_output.status().to_owned(),
            CmdError::NonZeroExitAlreadyStreamed(named_output) => named_output.status().to_owned(),
        }
    }
}

impl From<CmdError> for NamedOutput {
    fn from(value: CmdError) -> Self {
        match value {
            CmdError::SystemError(name, error) => NamedOutput {
                name,
                output: Output {
                    status: ExitStatus::from_raw(error.raw_os_error().unwrap_or(-1)),
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

/// Converts a `std::io::Error` into a `CmdError` which includes the formatted command name
#[must_use]
pub fn on_system_error(name: String, error: std::io::Error) -> CmdError {
    CmdError::SystemError(name, error)
}

/// Converts an `Output` into an error when status is non-zero
///
/// When calling a `Command` and streaming the output to stdout/stderr
/// it can be jarring to have the contents emitted again in the error. When this
/// error is displayed those outputs will not be repeated.
///
/// Use when the `Output` comes from a source that was already streamed.
///
/// To to include the results of stdout/stderr in the display of the error
/// use `nonzero_captured` instead.
///
/// # Errors
///
/// Returns Err when the `Output` status is non-zero
pub fn nonzero_streamed(name: String, output: impl Into<Output>) -> Result<NamedOutput, CmdError> {
    let output = output.into();
    if output.status.success() {
        Ok(NamedOutput { name, output })
    } else {
        Err(CmdError::NonZeroExitAlreadyStreamed(NamedOutput {
            name,
            output,
        }))
    }
}

/// Converts an `Output` into an error when status is non-zero
///
/// Use when the `Output` comes from a source that was not streamed
/// to stdout/stderr so it will be included in the error display by default.
///
/// To avoid double printing stdout/stderr when streaming use `nonzero_streamed`
///
/// # Errors
///
/// Returns Err when the `Output` status is non-zero
pub fn nonzero_captured(name: String, output: impl Into<Output>) -> Result<NamedOutput, CmdError> {
    let output = output.into();
    if output.status.success() {
        Ok(NamedOutput { name, output })
    } else {
        Err(CmdError::NonZeroExitNotStreamed(NamedOutput {
            name,
            output,
        }))
    }
}

/// Adds diagnostic information to a `CmdError` using `which_problem` if it is a `CmdError::SystemError`
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
/// It's best used as a diagnostic for developers for why a CmdError::SytemError might have occured.
/// For example, if the programmer executed the command with an empty PATH, this debugging tool
/// would help them find and fix the (otherwise) tedius to debug problem.
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
/// ````
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
        Err(error) => format!("\nInternal error while gathering dianostic information:\n\n{error}"),
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
