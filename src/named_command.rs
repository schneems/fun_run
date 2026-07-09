//! Logic for Command
//!
//! - [`NamedCommand`] struct for holding a command and a name
//! - [`CommandWithName`] trait extension

use crate::functions::output_and_write_streams;
use crate::{CmdError, NamedOutput, OutputWithName};
use std::io::Write;
use std::process::Command;

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
            .map(|output| output.named(name.clone()))
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
            .map(|output| output.named(name.clone()))
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
