use crate::{CmdError, CommandWithName, NamedOutput, OutputWithName};
use regex::Regex;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::process::{Command, Output};
use std::sync::LazyLock;
use std::{io, process, thread};
use std::{mem, panic};

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

pub(crate) fn output_and_write_streams<OW: Write + Send, EW: Write + Send>(
    command: &mut Command,
    stdout_write: OW,
    stderr_write: EW,
) -> io::Result<process::Output> {
    let mut stdout_buffer = Vec::new();
    let mut stderr_buffer = Vec::new();

    let mut stdout = tee(&mut stdout_buffer, stdout_write);
    let mut stderr = tee(&mut stderr_buffer, stderr_write);

    let mut child = command
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn()?;

    thread::scope(|scope| {
        let stdout_thread = mem::take(&mut child.stdout).map(|mut child_stdout| {
            scope.spawn(move || std::io::copy(&mut child_stdout, &mut stdout))
        });
        let stderr_thread = mem::take(&mut child.stderr).map(|mut child_stderr| {
            scope.spawn(move || std::io::copy(&mut child_stderr, &mut stderr))
        });

        stdout_thread
            .map_or_else(
                || Ok(0),
                |handle| match handle.join() {
                    Ok(value) => value,
                    Err(err) => panic::resume_unwind(err),
                },
            )
            .and({
                stderr_thread.map_or_else(
                    || Ok(0),
                    |handle| match handle.join() {
                        Ok(value) => value,
                        Err(err) => panic::resume_unwind(err),
                    },
                )
            })
            .and_then(|_| child.wait())
    })
    .map(|status| process::Output {
        status,
        stdout: stdout_buffer,
        stderr: stderr_buffer,
    })
}

/// Constructs a writer that writes to two other writers. Similar to the UNIX `tee` command.
pub(crate) fn tee<A: io::Write, B: io::Write>(a: A, b: B) -> TeeWrite<A, B> {
    TeeWrite {
        inner_a: a,
        inner_b: b,
    }
}

/// A tee writer that was created with the [`tee`] function.
#[derive(Debug, Clone)]
pub(crate) struct TeeWrite<A: io::Write, B: io::Write> {
    inner_a: A,
    inner_b: B,
}

impl<A: io::Write, B: io::Write> io::Write for TeeWrite<A, B> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner_a.write_all(buf)?;
        self.inner_b.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner_a.flush()?;
        self.inner_b.flush()
    }
}

// These tests exercise Unix shell behavior (`echo`/`bash`), so the whole
// module is Unix-only to avoid unused-import warnings on other platforms.
#[cfg(all(test, unix))]
mod test {
    use super::*;
    use pretty_assertions::assert_str_eq;
    use std::process::Command;

    #[test]
    fn test_output_and_write_streams_stdout() {
        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        let mut cmd = Command::new("echo");
        cmd.args(["-n", "Hello World!"]);

        let output = output_and_write_streams(&mut cmd, &mut stdout_buf, &mut stderr_buf).unwrap();

        assert_eq!(stdout_buf, "Hello World!".as_bytes());
        assert_eq!(stderr_buf, Vec::<u8>::new());

        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, "Hello World!".as_bytes());
        assert_eq!(output.stderr, Vec::<u8>::new());
    }

    #[test]
    fn test_output_and_write_streams_stderr() {
        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        let mut cmd = Command::new("bash");
        cmd.args(["-c", "echo -n Hello World! >&2"]);

        let _ = output_and_write_streams(&mut cmd, &mut stdout_buf, &mut stderr_buf).unwrap();

        assert_str_eq!(&String::from_utf8_lossy(&stderr_buf), "Hello World!");
    }
}
