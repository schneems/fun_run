#![cfg_attr(command_resolved_envs, feature(command_resolved_envs))]
//! # Fun Run
//!
//! What does the "Zombie Zoom 5K", the "Wibbly wobbly log jog", and the "Turkey Trot" have in common?
//! They're runs with a fun name! The `fun_run` library adds display and safety features to make
//! running a Rust [`Command`] better for you and your users.
//!
//! Stream the command and raise on non-zero exit:
//!
//! ```rust
//! use fun_run::CommandWithName;
//! use std::process::Command;
//!
//! let mut cmd = Command::new("bash");
//! cmd.args(["-c", "echo -n oops all berries; exit 1"]);
//!
//! // Advertise the command being run before execution
//! println!("Running `{name}`", name = cmd.name());
//!
//! // Stream output to the end user
//! // Turn non-zero status results into an error
//! let error = cmd
//!     .stream_output(std::io::stdout(), std::io::stderr())
//!     .unwrap_err();
//!
//! assert_eq!(
//!     indoc::indoc!{r#"
//!         Command failed `bash -c "echo -n oops all berries; exit 1"`
//!         exit status: 1
//!         stdout: <see above>
//!         stderr: <see above>
//!     "#}.trim().to_string(),
//!     error.to_string()
//! );
//! ```
//!
//! Run the command quietly, capture stdout/stderr and raise on non-zero exit:
//!
//! ```
//! # use fun_run::CommandWithName;
//! # use std::process::Command;
//! # let mut cmd = Command::new("bash");
//! # cmd.args(["-c", "echo -n oops all berries; exit 1"]);
//! let error = cmd.named_output().unwrap_err();
//! assert_eq!(
//!     indoc::indoc!{r#"
//!         Command failed `bash -c "echo -n oops all berries; exit 1"`
//!         exit status: 1
//!         stdout: oops all berries
//!         stderr: <empty>
//!     "#}.trim().to_string(),
//!     error.to_string()
//! );
//! ```
//!
//! Output of the command is preserved in success and error cases:
//!
//! ```
//! # use fun_run::CommandWithName;
//! # use std::process::Command;
//! # let mut cmd = Command::new("bash");
//! # cmd.args(["-c", "echo -n oops all berries; exit 1"]);
//! # let error = cmd.named_output().unwrap_err();
//! // Both Ok and Err from result store the output for inspection
//! assert!(
//!     error.output().unwrap().stdout_lossy()
//!     .contains("oops all berries")
//! );
//! ```
//!
//! ## Install
//!
//! ```shell
//! $ cargo add fun_run
//! ```
//!
//! ## Renaming a command
//!
//! If you need to provide an alternate display for your command you can rename it, this is useful
//! for omitting implementation details.
//!
//! ```rust
//! use fun_run::CommandWithName;
//! use std::process::Command;
//!
//! let mut cmd = Command::new("bash");
//! cmd
//!     .args(["-eo", "pipefail", "-c"])
//!     .arg("echo -n 'hello world' && exit 1");
//!
//! let mut renamed_cmd = cmd.named("echo 'hello world'");
//!
//! assert_eq!("echo 'hello world'", &renamed_cmd.name());
//! ```
//!
//! This is also useful for adding additional information, such as environment variables:
//!
//! ```rust
//! use fun_run::CommandWithName;
//! use std::process::Command;
//!
//! let mut cmd = Command::new("bundle");
//! cmd.arg("install");
//!
//! let env_vars = std::env::vars();
//! # let mut env_vars = std::collections::HashMap::<String, String>::new();
//! # env_vars.insert("RAILS_ENV".to_string(), "production".to_string());
//!
//! let mut renamed_cmd = cmd.named_fn(|cmd| fun_run::display_with_env_keys(
//!     cmd,
//!     env_vars,
//!     ["RAILS_ENV"]
//! ));
//!
//! assert_eq!(r#"RAILS_ENV="production" bundle install"#, renamed_cmd.name())
//! ```
//!
//! ## What won't it do?
//!
//! The `fun_run` library doesn't support executing a [`Command`] in ways that do not produce an
//! [`Output`], for example calling [`Command::spawn`](https://doc.rust-lang.org/std/process/struct.Command.html#method.spawn) returns a [`std::process::Child`]
//! (Which doesn't contain an [`Output`]). If you want to run-for-fun in the background, spawn a thread
//! and join it manually:
//!
//! ```no_run
//! use fun_run::CommandWithName;
//! use std::process::Command;
//! use std::thread;
//!
//! let mut cmd = Command::new("bundle");
//! cmd.args(["install"]);
//!
//! // Advertise the command being run before execution
//! println!("Quietly Running `{name}` in the background", name = cmd.name());
//!
//! let result = thread::spawn(move || {
//!     cmd.named_output()
//! }).join().unwrap();
//!
//! // Command name is persisted on success or failure
//! match result {
//!     Ok(output) => {
//!         assert_eq!("bundle install", &output.name())
//!     },
//!     Err(cmd_error) => {
//!         assert_eq!("bundle install", &cmd_error.name())
//!     }
//! }
//! ```
//!
//! ## Async
//!
//! This library uses synchronous command execution. If you’re using this library in an async context,
//! you’ll want to use an async wrapper like [tokio::task::spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html).
//!
//! ## Clippy
//!
//! To ensure all commands have their exit status checked you can add this to your `clippy.toml` to
//! prevent accidentally spawning an un-checked plain [`Command`]:
//!
//! ```toml
//! [[disallowed-methods]]
//! path = "std::process::Command::output"
//! reason = "Use fun_run::CommandWithName::named_output"
//!
//! [[disallowed-methods]]
//! path = "std::process::Command::status"
//! reason = "Use fun_run::CommandWithName::named_output and read the status from the result"
//!
//! [[disallowed-methods]]
//! path = "std::process::Command::spawn"
//! reason = "Use fun_run::CommandWithName::stream_output(std::io::stdout(), std::io::stderr())"
//! ```
//!
//! ## Debugging system failures with `which_problem`
//!
//! When a command execution returns an Err due to a system error (and not because the program it
//! executed launched but returned non-zero status), it's usually because the executable couldn't be
//! found, or if it was found, it couldn't be launched, for example due to a permissions error. The
//! [which_problem](https://github.com/schneems/which_problem) crate is designed to add debugging errors
//! to help you identify why the command couldn't be launched.
//!
//! The crate `which_problem` works like `which` but helps you identify common mistakes such as typos:
//!
//! ```shell
//! $ cargo whichp zuby
//! Program "zuby" not found
//!
//! Info: No other executables with the same name are found on the PATH
//!
//! Info: These executables have the closest spelling to "zuby" but did not match:
//!       "hub", "ruby", "subl"
//! ```
//!
//! Fun run supports `which_problem` integration through the `which_problem` feature. In your `Cargo.toml`:
//!
//! ```toml
//! # Cargo.toml
//! fun_run = { version = <version.here>, features = ["which_problem"] }
//! ```
//!
//! And annotate errors:
//!
//! ```rust,no_run
//! #[cfg(not(feature = "which_problem"))] { return; }
//! use fun_run::CommandWithName;
//! use std::process::Command;
//!
//! let mut cmd = Command::new("becho");
//! cmd.args(["hello", "world"]);
//!
//! #[cfg(feature = "which_problem")]
//! cmd.stream_output(std::io::stdout(), std::io::stderr())
//!     .map_err(|error| fun_run::map_which_problem(error, cmd.mut_cmd(), std::env::var_os("PATH"))).unwrap();
//! ```
//!
//! Now if the system cannot find a `becho` program on your system the output will give you all the
//! info you need to diagnose the underlying issue.
//!
//! Note that `which_problem` integration is not enabled by default because it outputs information
//! about the contents of your disk such as layout and file permissions.
//!
//! ## Nightly-only items
//!
//! A few items (`display_env_vars` and `CommandWithName::named_env_vars`) require a
//! nightly toolchain. They depend on the unstable
//! [`command_resolved_envs`](https://github.com/rust-lang/rust/issues/149070)
//! feature, auto-detected at build time, and are absent on stable. Because
//! <https://docs.rs> builds on nightly, these appear in the published docs even
//! though stable users cannot use them.

use command::output_and_write_streams;
use regex::Regex;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::io::Write;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Output;
use std::sync::LazyLock;

mod command;
mod exit_status;
mod which_problem;

pub use exit_status::ExitStatusFromCode;
#[cfg(feature = "which_problem")]
pub use which_problem::map_which_problem;

/// CommandWithName trait
///
/// - [`CommandWithName::named_output`] - Runs the command and produces a Result with [`NamedOutput`] or
///   [`CmdError`]. Does NOT stream the output. Will Err on a non-zero output.
/// - [`CommandWithName::stream_output`] - Runs the command while streaming the output to the given [`Write`]
///   arguments. Returns a Result with [`NamedOutput`] or [`CmdError`]. Will Err on a non-zero output.
/// - [`CommandWithName::name`] - Returns a displayable String of the command's name.
/// - [`CommandWithName::named`] - Rename a command
/// - [`CommandWithName::named_fn`] - Rename a command with a function
///
pub trait CommandWithName {
    /// Returns the desired display name of the command
    ///
    /// ```
    /// use fun_run::CommandWithName;
    /// use std::process::Command;
    ///
    /// let mut command = Command::new("cargo");
    /// command.arg("test");
    ///
    /// assert_eq!(command.name(), "cargo test".to_string());
    /// ```
    fn name(&mut self) -> String;

    /// Returns a reference to `&mut Command`
    ///
    /// This is useful for passing to other libraries. This allows [`NamedCommand`]
    /// and [`Command`] to have a shared interface to the raw command.
    fn mut_cmd(&mut self) -> &mut Command;

    /// Rename a command via a given string
    ///
    /// This can be useful if a part of the command is distracting or surprising or if you
    /// desire to include additional information such as displaying environment variables.
    ///
    /// Alternatively see [CommandWithName::named_fn]
    ///
    /// # Examples
    ///
    /// ```
    /// use fun_run::CommandWithName;
    ///
    /// let mut command = std::process::Command::new("bin/bundle");
    /// command.args(["install", "--no-doc"]);
    ///
    /// assert_eq!("bin/bundle install --no-doc", command.name());
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
    /// Alternatively see [`CommandWithName::named`]
    ///
    /// # Examples
    ///
    /// ```
    /// use fun_run::{CommandWithName, display_with_env_keys};
    /// use std::env::vars_os;
    ///
    /// let mut command = std::process::Command::new("cargo");
    /// command.arg("test");
    /// # unsafe { std::env::set_var("RUST_BACKTRACE", "1")};
    ///
    /// let mut cmd = command.named_fn(|cmd| {
    ///     display_with_env_keys(cmd, vars_os(), ["RUST_BACKTRACE"])
    /// });
    /// assert_eq!(r#"RUST_BACKTRACE="1" cargo test"#, cmd.name());
    /// ```
    #[allow(clippy::needless_lifetimes)]
    fn named_fn<'a>(&'a mut self, f: impl FnOnce(&mut Command) -> String) -> NamedCommand<'a> {
        let cmd = self.mut_cmd();
        let name = f(cmd);
        self.named(name)
    }

    /// Adds given environment variables to the command's name
    ///
    /// Takes an environment variable **key** and if it will be used when the command runs
    /// prepends the `<key>=<value>` pair to the front of the command.
    ///
    /// **Warning:** By default a [`Command`] will inherit environment variables from the parent process.
    /// Limit environment variables to non-sensitive keys or use [`Command::env_clear`] and explicitly
    /// [`Command::envs`] to set only what you need.
    ///
    /// **Note:** Requires a nightly toolchain. This method relies on the unstable
    /// [`command_resolved_envs`](https://github.com/rust-lang/rust/issues/149070)
    /// feature, which is auto-detected at build time. On a stable toolchain it does
    /// not exist, so calling it (or referring to it) will not compile.
    ///
    /// # Examples
    ///
    /// ```
    /// use fun_run::CommandWithName;
    ///
    /// let mut command = std::process::Command::new("bundle");
    /// command
    ///     .arg("install")
    ///     .env("BUNDLE_WITHOUT", "development:test");
    ///
    /// let mut cmd = command.named_env_vars(["BUNDLE_WITHOUT"]);
    /// assert_eq!(
    ///     r#"BUNDLE_WITHOUT="development:test" bundle install"#,
    ///     cmd.name()
    /// );
    /// ```
    ///
    /// Preserves prior re-naming:
    ///
    /// ```
    /// use fun_run::CommandWithName;
    ///
    /// let mut command = std::process::Command::new("bundle");
    /// command
    ///     .arg("install")
    ///     .envs([
    ///         ("BUNDLE_WITHOUT", "development:test"),
    ///         ("BUNDLE_PATH", "vendor/bundle")
    ///     ]);
    ///
    /// let mut cmd = command.named("./bin/bundle install");
    /// let mut cmd = cmd.named_env_vars(["BUNDLE_WITHOUT"]);
    ///
    /// assert_eq!(
    ///     r#"BUNDLE_WITHOUT="development:test" ./bin/bundle install"#,
    ///     cmd.name()
    /// );
    ///
    /// let mut cmd = cmd.named_env_vars(["BUNDLE_PATH"]);
    /// assert_eq!(
    ///     r#"BUNDLE_PATH="vendor/bundle" BUNDLE_WITHOUT="development:test" ./bin/bundle install"#,
    ///     cmd.name()
    /// );
    /// ```
    ///
    /// Re-naming a command that previously had an environment variable prepended will NOT
    /// preserve the environment variables.
    ///
    /// This function is NOT (currently) idempotent. Calling it twice will prepend
    /// the same environment variable twice. This behavior might change in the future (such that
    /// under some conditions it becomes idempotent). Therefore you shouldn't consider this warning
    /// a stability guarantee.
    #[cfg(command_resolved_envs)]
    #[allow(clippy::needless_lifetimes)]
    #[must_use]
    fn named_env_vars<'a, T, K>(&'a mut self, keys: T) -> NamedCommand<'a>
    where
        T: IntoIterator<Item = K>,
        K: Into<OsString>,
    {
        let old = self.name();
        let cmd = self.mut_cmd();
        let name = display_name_with_env_keys(
            old,
            cmd.get_resolved_envs()
                .collect::<HashMap<OsString, OsString>>(),
            keys,
        );
        self.named(name)
    }

    /// Runs the command without streaming
    ///
    /// It's like [`Command::output`] but all the outputs carry the name of the original
    /// command. Will Err on non-zero exit.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fun_run::CommandWithName;
    /// use std::process::Command;
    ///
    /// let mut cmd = Command::new("echo");
    /// cmd.args(["-n", "hello world"]);
    ///
    /// // Do NOT stream output to user
    /// // Turn non-zero status results into an error
    /// let result = cmd.named_output();
    ///
    /// assert_eq!(
    ///     result.unwrap().stdout_lossy(),
    ///     "hello world".to_string()
    /// );
    /// ```
    ///
    /// # Errors
    ///
    /// - Returns [`CmdError::SystemError`] if the system is unable to run the command.
    /// - Returns [`CmdError::NonZeroExitNotStreamed`] if the exit code is not zero. Since the output
    ///   is not already streamed, displaying this error will include stdout and stderr.
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
    /// Similar to calling [`Command::spawn`], but the output of the command is preserved, and
    /// all outputs of the Result carry the name of the original command. Will Err on non-zero exit.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fun_run::CommandWithName;
    /// use std::process::Command;
    ///
    /// let mut cmd = Command::new("echo");
    /// cmd.args(["-n", "hello world"]);
    ///
    /// // Stream output to the end user
    /// // Turn non-zero status results into an error
    /// let result = cmd
    ///     .stream_output(std::io::stdout(), std::io::stderr());
    ///
    /// assert_eq!(
    ///     result.unwrap().stdout_lossy(),
    ///     "hello world".to_string()
    /// );
    /// ```
    ///
    /// # Errors
    ///
    /// - Returns [`CmdError::SystemError`] if the system is unable to run the command
    /// - Returns [`CmdError::NonZeroExitAlreadyStreamed`] if the exit code is not zero.
    ///   Since the stdout and stderr are already streamed this error will not re-display the
    ///   stdout and stderr.
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

impl CommandWithName for &mut Command {
    fn name(&mut self) -> String {
        crate::display(self)
    }

    fn mut_cmd(&mut self) -> &mut Command {
        self
    }
}

/// It's a command, with a name
///
/// This struct allows us to re-name an existing [`Command`] via the [`CommandWithName`] trait associated
/// functions. When one of those functions such as [CommandWithName::named_fn] or [CommandWithName::named]
/// are called, Rust needs somewhere for the new name string to live, so we move it over into this struct
/// which also implements [`CommandWithName`]. You can gain access to the original [`Command`] reference
/// via [`CommandWithName::mut_cmd`].
pub struct NamedCommand<'a> {
    name: String,
    command: &'a mut Command,
}

impl<'a> From<&'a mut Command> for NamedCommand<'a> {
    /// Convert a [Command] reference into a [NamedCommand]
    ///
    /// Useful to "shorten" a command (to hide additional/unexpected flags).
    ///
    /// ```
    /// use fun_run::{NamedCommand, CommandWithName};
    ///
    /// let mut command = std::process::Command::new("go");
    /// let mut short: NamedCommand = command
    ///     .args(["list", "-tags", "heroku"])
    ///     .into();
    ///
    /// short
    ///     .mut_cmd()
    ///     .args([
    ///         "-f",
    ///         "{{ if eq .Name \"main\" }}{{ .ImportPath }}{{ end }}",
    ///         "./...",
    ///     ]);
    ///
    /// // Short name
    /// assert_eq!("go list -tags heroku", &short.name());
    /// // Full args
    /// assert_eq!("go", short.mut_cmd().get_program().to_str().unwrap());
    /// assert_eq!(
    ///     "list -tags heroku -f {{ if eq .Name \"main\" }}{{ .ImportPath }}{{ end }} ./...",
    ///     short
    ///         .mut_cmd()
    ///         .get_args()
    ///         .map(|arg| arg.to_str().unwrap())
    ///         .collect::<Vec<&str>>()
    ///         .join(" ")
    /// );
    /// ```
    fn from(command: &'a mut Command) -> Self {
        // Eventually we can deprecate `CommandWithName::named(String)` and change it
        // to `CommandWithName::rename(String)` and then have `CommandWithName::named()`
        // return a NamedCommand for better ergonomics
        NamedCommand {
            name: command.name(),
            command,
        }
    }
}

impl CommandWithName for NamedCommand<'_> {
    fn name(&mut self) -> String {
        self.name.to_string()
    }

    fn mut_cmd(&mut self) -> &mut Command {
        self.command
    }
}

impl CommandWithName for &mut NamedCommand<'_> {
    fn name(&mut self) -> String {
        self.name.to_string()
    }

    fn mut_cmd(&mut self) -> &mut Command {
        self.command
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

// https://github.com/jimmycuadra/rust-shellwords/blob/d23b853a850ceec358a4137d5e520b067ddb7abc/src/lib.rs#L23
static QUOTE_ARG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([^A-Za-z0-9_\-.,:/@\n])").expect("clippy checked"));

/// Converts a command and its arguments into a user readable string
///
/// # Examples
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
        .chain(command.get_args().map(OsStr::to_string_lossy).map(|arg| {
            if QUOTE_ARG_RE.is_match(&arg) {
                format!("{arg:?}")
            } else {
                format!("{arg}")
            }
        }))
        .collect::<Vec<String>>()
        .join(" ")
}

/// Converts a command, and specified environment variables to user readable string
///
/// Takes an environment variable **key** and if it will be used when the command runs
/// prepends the `<key>=<value>` pair to the front of the command.
///
/// **Warning:** By default a [`Command`] will inherit environment variables from the parent process.
/// Limit environment variables to non-sensitive keys or use [`Command::env_clear`] and explicitly
/// [`Command::envs`] to set only what you need.
///
/// This safer alternative to [`display_with_env_keys`] resolves environment variables from
/// [`Command::get_resolved_envs`]. That function will automatically account for
/// inherited environment variables and any env modifications such as [`Command::env_clear`]
/// or [`Command::env_remove`].
///
/// **Note:** Requires a nightly toolchain. This function relies on the unstable
/// [`command_resolved_envs`](https://github.com/rust-lang/rust/issues/149070)
/// feature, which is auto-detected at build time. On a stable toolchain it does
/// not exist, so calling it (or referring to it) will not compile.
///
/// # Examples
///
/// ```rust
/// use std::process::Command;
/// use fun_run;
///
/// let mut command = Command::new("bundle");
/// command.arg("install").envs([("RAILS_ENV", "production")]);
///
/// let name = fun_run::display_env_vars(&mut command, ["RAILS_ENV"]);
/// assert_eq!(String::from(r#"RAILS_ENV="production" bundle install"#), name);
/// ```
#[cfg(command_resolved_envs)]
#[must_use]
pub fn display_env_vars<T, K>(cmd: &mut Command, keys: T) -> String
where
    T: IntoIterator<Item = K>,
    K: Into<OsString>,
{
    let env: HashMap<OsString, OsString> = cmd.get_resolved_envs().collect();
    display_with_env_keys(cmd, env, keys)
}

/// Converts a command, arguments, and specified environment variables to user readable string
///
/// Useful for showing usage of a command that uses environment variables for configuration.
///
/// # Examples
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
///
/// There's no guarantee that the env provided was passed to construct the Command.
/// A [`Command`] can also inherit environment variables from the parent.
///
/// Note that [`Command::env_clear`] and [`Command::env_remove`] both change the Command's
/// env var inheritance behavior.
#[must_use]
pub fn display_with_env_keys<E, K, V, I, O>(cmd: &mut Command, env: E, keys: I) -> String
where
    E: IntoIterator<Item = (K, V)>,
    K: Into<OsString>,
    V: Into<OsString>,
    I: IntoIterator<Item = O>,
    O: Into<OsString>,
{
    display_name_with_env_keys(cmd.name(), env, keys)
}

fn display_name_with_env_keys<E, K, V, I, O>(name: String, env: E, keys: I) -> String
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
        .collect::<HashMap<OsString, OsString>>();

    keys.into_iter()
        .map(|key| {
            let key = key.into();
            format!(
                "{}={:?}",
                key.to_string_lossy(),
                env.get(&key).cloned().unwrap_or_else(|| OsString::from(""))
            )
        })
        .chain([name])
        .collect::<Vec<String>>()
        .join(" ")
}

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

#[cfg(not(any(unix, windows)))]
compile_error!(
    "fun_run constructs `ExitStatus` values and only supports `unix` and `windows` targets"
);

fn display_out_or_empty(contents: &[u8]) -> String {
    let contents = String::from_utf8_lossy(contents);
    if contents.trim().is_empty() {
        "<empty>".to_string()
    } else {
        contents.to_string()
    }
}

/// Converts a [`std::io::Error`] into a [`CmdError`] which includes the formatted command name
#[must_use]
pub fn on_system_error(name: String, error: std::io::Error) -> CmdError {
    CmdError::SystemError(name, error)
}

/// Converts an [`Output`] into an error when status is non-zero
///
/// When calling a [`Command`] and streaming the output to stdout/stderr
/// it can be jarring to have the contents emitted again in the error. When this
/// error is displayed those outputs will not be repeated.
///
/// Use when the [`Output`] comes from a source that was already streamed.
///
/// To to include the results of stdout/stderr in the display of the error
/// use [`nonzero_captured`] instead.
///
/// # Errors
///
/// Returns Err when the [`Output`] status is non-zero
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

/// Converts an [`Output`] into an error when status is non-zero
///
/// Use when the [`Output`] comes from a source that was not streamed
/// to stdout/stderr so it will be included in the error display by default.
///
/// To avoid double printing stdout/stderr when streaming use [`nonzero_streamed`]
///
/// # Errors
///
/// Returns Err when the [`Output`] status is non-zero
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

#[cfg(test)]
mod tests {
    use super::*;

    // Nightly CI greps for this test name; update ci.yml if you rename it.
    #[test]
    #[cfg(command_resolved_envs)]
    fn get_resolved_envs_detects_nightly_feature() {
        let mut cmd = Command::new("does-not-run");
        cmd.env("FUN_RUN_TEST_VAR", "1");
        let resolved: HashMap<OsString, OsString> = cmd.get_resolved_envs().collect();
        assert_eq!(
            resolved.get(OsStr::new("FUN_RUN_TEST_VAR")),
            Some(&OsString::from("1"))
        );
    }

    #[test]
    fn test_status_from_code() {
        for code in 0..=255 {
            let status = ExitStatus::from_code(code);
            assert_eq!(Some(code as i32), status.code());
        }
    }
}
