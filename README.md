<!-- cargo-rdme start -->

# Fun Run

What does the "Zombie Zoom 5K", the "Wibbly wobbly log jog", and the "Turkey Trot" have in common?
They're runs with a fun name! The `fun_run` library adds display and safety features to make
running with Rust's [`Command`](https://doc.rust-lang.org/stable/std/process/struct.Command.html)-s better for you and your users.

Stream the command and raise on non-zero exit:

```rust
use fun_run::CommandWithName;
use std::process::Command;

let mut cmd = Command::new("bash");
cmd.args(["-c", "echo -n oops all berries; exit 1"]);

// Advertise the command being run before execution
println!("Running `{name}`", name = cmd.name());

// Stream output to the end user
// Turn non-zero status results into an error
let error = cmd
    .stream_output(std::io::stdout(), std::io::stderr())
    .unwrap_err();

assert_eq!(
    indoc::indoc!{r#"
        Command failed `bash -c "echo -n oops all berries; exit 1"`
        exit status: 1
        stdout: <see above>
        stderr: <see above>
    "#}.trim().to_string(),
    error.to_string()
);
```

Run the command quietly, capture stdout/stderr and raise on non-zero exit:

```rust
let error = cmd.named_output().unwrap_err();
assert_eq!(
    indoc::indoc!{r#"
        Command failed `bash -c "echo -n oops all berries; exit 1"`
        exit status: 1
        stdout: oops all berries
        stderr: <empty>
    "#}.trim().to_string(),
    error.to_string()
);
```

Output of the command is preserved in success and error cases:

```rust
// Both Ok and Err from result store the output for inspection
assert!(
    error.output().unwrap().stdout_lossy()
    .contains("oops all berries")
);
```

## Install

```shell
$ cargo add fun_run
```

## Renaming a command

If you need to provide an alternate display for your command you can rename it, this is useful
for omitting implementation details.

```rust
use fun_run::CommandWithName;
use std::process::Command;

let mut cmd = Command::new("bash");
cmd
    .args(["-eo", "pipefail", "-c"])
    .arg("echo -n 'hello world' && exit 1");

let mut renamed_cmd = cmd.named("echo 'hello world'");

assert_eq!("echo 'hello world'", &renamed_cmd.name());
```

This is also useful for adding additional information, such as environment variables:

```rust
use fun_run::CommandWithName;
use std::process::Command;

let mut cmd = Command::new("bundle");
cmd.arg("install");

let env_vars = std::env::vars();

let mut renamed_cmd = cmd.named_fn(|cmd| fun_run::display_with_env_keys(
    cmd,
    env_vars,
    ["RAILS_ENV"]
));

assert_eq!(r#"RAILS_ENV="production" bundle install"#, renamed_cmd.name())
```

## What won't it do?

The `fun_run` library doesn't support executing a [`Command`](https://doc.rust-lang.org/stable/std/process/struct.Command.html) in ways that do not produce an
[`Output`](https://doc.rust-lang.org/stable/std/process/struct.Output.html), for example calling [`Command::spawn`] returns a [`std::process::Child`](https://doc.rust-lang.org/stable/std/process/struct.Child.html)
(Which doesn't contain an [`Output`](https://doc.rust-lang.org/stable/std/process/struct.Output.html)). If you want to run-for-fun in the background, spawn a thread
and join it manually:

```rust
use fun_run::CommandWithName;
use std::process::Command;
use std::thread;

let mut cmd = Command::new("bundle");
cmd.args(["install"]);

// Advertise the command being run before execution
println!("Quietly Running `{name}` in the background", name = cmd.name());

let result = thread::spawn(move || {
    cmd.named_output()
}).join().unwrap();

// Command name is persisted on success or failure
match result {
    Ok(output) => {
        assert_eq!("bundle install", &output.name())
    },
    Err(cmd_error) => {
        assert_eq!("bundle install", &cmd_error.name())
    }
}
```

## Async

This library uses synchronous command execution. If you’re using this library in an async context,
you’ll want to use an async wrapper like [tokio::task::spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html).

## Clippy

To ensure all commands have their exit status checked you can add this to your `clippy.toml` to
prevent accidentally spawning an un-checked plain [`Command`](https://doc.rust-lang.org/stable/std/process/struct.Command.html):

```toml
[[disallowed-methods]]
path = "std::process::Command::output"
reason = "Use fun_run::CommandWithName::named_output"

[[disallowed-methods]]
path = "std::process::Command::status"
reason = "Use fun_run::CommandWithName::named_output and read the status from the result"

[[disallowed-methods]]
path = "std::process::Command::spawn"
reason = "Use fun_run::CommandWithName::stream_output(std::io::stdout(), std::io::stderr())"
```

## Debugging system failures with `which_problem`

When a command execution returns an Err due to a system error (and not because the program it
executed launched but returned non-zero status), it's usually because the executable couldn't be
found, or if it was found, it couldn't be launched, for example due to a permissions error. The
[which_problem](https://github.com/schneems/which_problem) crate is designed to add debugging errors
to help you identify why the command couldn't be launched.

The crate `which_problem` works like `which` but helps you identify common mistakes such as typos:

```shell
$ cargo whichp zuby
Program "zuby" not found

Info: No other executables with the same name are found on the PATH

Info: These executables have the closest spelling to "zuby" but did not match:
      "hub", "ruby", "subl"
```

Fun run supports `which_problem` integration through the `which_problem` feature. In your `Cargo.toml`:

```toml
# Cargo.toml
fun_run = { version = <version.here>, features = ["which_problem"] }
```

And annotate errors:

```rust
#[cfg(not(feature = "which_problem"))] { return; }
use fun_run::CommandWithName;
use std::process::Command;

let mut cmd = Command::new("becho");
cmd.args(["hello", "world"]);

#[cfg(feature = "which_problem")]
cmd.stream_output(std::io::stdout(), std::io::stderr())
    .map_err(|error| fun_run::map_which_problem(error, cmd.mut_cmd(), std::env::var_os("PATH"))).unwrap();
```

Now if the system cannot find a `becho` program on your system the output will give you all the
info you need to diagnose the underlying issue.

Note that `which_problem` integration is not enabled by default because it outputs information
about the contents of your disk such as layout and file permissions.

<!-- cargo-rdme end -->

## Development

Update the readme:

```
$ cargo install cargo-rdme
$ cargo rdme
```
