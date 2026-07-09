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

use regex::Regex;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::process::Command;
use std::process::Output;
use std::sync::LazyLock;

mod cmd_error;
mod command;
mod exit_status;
mod named_command;
mod named_output;
mod which_problem;

pub use cmd_error::CmdError;
pub use exit_status::ExitStatusFromCode;
pub use named_command::{CommandWithName, NamedCommand};
pub use named_output::NamedOutput;
pub use named_output::OutputWithName;
#[cfg(feature = "which_problem")]
pub use which_problem::map_which_problem;

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

#[cfg(not(any(unix, windows)))]
compile_error!(
    "fun_run constructs `ExitStatus` values and only supports `unix` and `windows` targets"
);

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
        Ok(output.named(name))
    } else {
        Err(CmdError::NonZeroExitAlreadyStreamed(output.named(name)))
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
        Ok(output.named(name))
    } else {
        Err(CmdError::NonZeroExitNotStreamed(output.named(name)))
    }
}

#[cfg(test)]
mod tests {

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
}
