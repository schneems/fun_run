## Unreleased

- Update documentation (https://github.com/schneems/fun_run/pull/16)
- Add Windows support: the crate now compiles on `*-pc-windows-*` targets, verified in CI via a cross-compile clippy job. The optional `which_problem` feature remains Unix-only.
- Fix `CmdError::status()` and the `impl From<CmdError> for NamedOutput` conversion so a failed-to-launch command (the `CmdError::SystemError` variant) returns a decodable, shell-conventional exit code (127 not-found, 126 not-executable, 1 otherwise) instead of a raw errno that could decode as a signal. The status is still synthetic (the command never ran) and only guaranteed to be non-zero, so prefer inspecting the underlying `std::io::Error` and its `ErrorKind` directly rather than relying on the exact code. (https://github.com/schneems/fun_run/pull/25)
- Set the minimum supported Rust version (MSRV) to 1.87. This is required by the `std::io::ErrorKind` variants used to map launch failures to exit codes, and by the optional `which_problem` dependency which relies on `OsStr::display` (stabilized in 1.87). (https://github.com/schneems/fun_run/pull/25)

## 0.6.0

- Add `impl<'a> From<&'a mut Command> for NamedCommand<'a>` to construct a `NamedCommand` from a regular command reference without renaming it. This is useful when "shortening" names of commands (https://github.com/schneems/fun_run/pull/15)

## 0.5.0

- Add `impl CommandWithName for &mut NamedCommand` in addition to `NamedCommand` (https://github.com/schneems/fun_run/pull/14)

## 0.4.0

- Add `impl CommandWithName for &mut Command` in addition to `Command` (https://github.com/schneems/fun_run/pull/12)

## 0.3.0

- Add `NamedOutput::output()` which returns `&Output`. (https://github.com/schneems/fun_run/pull/10)
- Add `impl From<&NamedOutput> for &Output` in addition to the existing `impl From<NamedOutput> for Output`. (https://github.com/schneems/fun_run/pull/10)
- Add `NamedOutput::stdout()` and `stderr()` to return references to the original `Vec<u8>`. This is in addition to `stdout_lossy` and `stderr_lossy` functions that return `String`. (https://github.com/schneems/fun_run/pull/10)
- Add `CmdError::status()` to return an `ExitStatus` (https://github.com/schneems/fun_run/pull/10)

## 0.2.0

- Add `std::error::Error` trait to `CmdError` (https://github.com/schneems/fun_run/pull/8)

## 0.1.3

- Update docs on crates.io (https://github.com/schneems/fun_run/pull/7)

## 0.1.2

- Add metadata for crates.io (https://github.com/schneems/fun_run/pull/5)

## 0.1.1

- Fix stderr copying to stdout bug (https://github.com/schneems/fun_run/pull/3)

## 0.1.0

- First release
