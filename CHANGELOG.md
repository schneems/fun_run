## Unreleased

- Add `NamedOutput::output()` which returns `&Output`.
- Add `impl From<&NamedOutput> for &Output` in addition to the existing `impl From<NamedOutput> for Output`.
- Add `NamedOutput::stdout()` and `stderr()` to return references to the original `Vec<u8>`. This is in addition to `stdout_lossy` and `stderr_lossy` functions that return `String`.
- Add `CmdError::status()` to return an `ExitStatus`

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
