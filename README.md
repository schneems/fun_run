<!-- cargo-rdme start -->

# Fun Run

What does the "Zombie Zoom 5K", the "Wibbly wobbly log jog", and the "Turkey Trot" have in common?
They're runs with a fun name! That's exactly what `fun_run` does. It makes running your Rust `Command`s
more fun, by naming them.

## What is Fun Run?

Fun run is designed for the use case where not only do you want to run a `Command` you want to
output what you're running and what happened. Building a CLI tool is a great use case. Another is
creating [a buildpack](https://github.com/heroku/buildpacks-ruby/tree/4f514f6046568ada523eefd41b3024f86f1c67ce).

Here's some things you can do with fun_run:

- Advertise the command being run before execution
- Customize how commands are displayed
- Return error messages with the command name.
- Turn non-zero status results into an error
- Embed stdout and stderr into errors (when not streamed)
- Store stdout and stderr for debug and diagnosis without displaying them (when streamed)

Just like you don't need to dress up in a giant turkey costume to run a 5K you also don't **need**
`fun_run` to do these things. Though, unlike the turkey costume, using `fun_run` will also make the
experience easier.

## Install

```shell
$ cargo add fun_run
```

## Ready to Roll

For a quick and easy fun run you can use the `fun_run::CommandWithName` trait extension to stream
output:

```rust
use fun_run::CommandWithName;
use std::process::Command;

let mut cmd = Command::new("bundle");
cmd.args(["install"]);

// Advertise the command being run before execution
println!("Running `{name}`", name = cmd.name());

// Stream output to the end user
// Turn non-zero status results into an error
let result = cmd
    .stream_output(std::io::stdout(), std::io::stderr());

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

## Pretty (good) errors

Fun run comes with nice errors by default:

```rust
use fun_run::CommandWithName;
use std::process::Command;

let mut cmd = Command::new("becho");
cmd.args(["hello", "world"]);

let expected = r#"Could not run command `becho hello world`. No such file or directory"#;
match cmd.stream_output(std::io::stdout(), std::io::stderr()) {
    Ok(_) => todo!(),
    Err(cmd_error) => {
        let actual = cmd_error.to_string();
        assert!(actual.contains(expected), "Expected {actual:?} to contain {expected:?}, but it did not")
    }
}
```

And commands that don't return an exit code 0 return an Err so you don't accidentally ignore a
failure, and the output of the command is captured:

```rust
use fun_run::CommandWithName;
use std::process::Command;

let mut cmd = Command::new("bash");
cmd.arg("-c");
cmd.arg("echo -n 'hello world' && exit 1");

// Quietly gets output
match cmd.named_output() {
    Ok(_) => todo!(),
    Err(cmd_error) => {
        let expected = r#"
Command failed `bash -c "echo -n 'hello world' && exit 1"`
exit status: 1
stdout: hello world
stderr: <empty>
        "#;

        let actual = cmd_error.to_string();
        assert!(
            actual.trim().contains(expected.trim()),
            "Expected {:?} to contain {:?}, but it did not", actual.trim(), expected.trim()
        )
    }
}
```

By default, streamed output won't duplicated in error messages (but is still there if you want
to inspect it in your program):

```rust
use fun_run::CommandWithName;
use std::process::Command;

let mut cmd = Command::new("bash");
cmd.arg("-c");
cmd.arg("echo -n 'hello world' && exit 1");


let expected = r#"
Command failed `bash -c "echo -n 'hello world' && exit 1"`
exit status: 1
stdout: <see above>
stderr: <see above>
"#;

// Quietly gets output
match cmd.stream_output(std::io::stdout(), std::io::stderr()) {
    Ok(_) => todo!(),
    Err(cmd_error) => {
        let actual = cmd_error.to_string();
        assert!(
            actual.trim().contains(expected.trim()),
            "Expected {:?} to contain {:?}, but it did not", actual.trim(), expected.trim()
        );

        let named_output: fun_run::NamedOutput = cmd_error.into();

        assert_eq!(
            "hello world",
            named_output.stdout_lossy().trim()
        );

        assert_eq!(
            "bash -c \"echo -n 'hello world' && exit 1\"",
            named_output.name()
        );
    }
}
```

## Renaming

If you need to provide an alternate display for your command you can rename it, this is useful
for omitting implementation details.

```rust
use fun_run::CommandWithName;
use std::process::Command;

let mut cmd = Command::new("bash");
cmd.arg("-c");
cmd.arg("echo -n 'hello world' && exit 1");

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

## Debugging system failures with `which_problem`

When a command execution returns an Err due to a system error (and not because the program it
executed launched but returned non-zero status), it's usually because the executable couldn't be
found, or if it was found, it couldn't be launched, for example due to a permissions error. The
[which_problem](https://github.com/schneems/which_problem) crate is designed to add debuggin errors
to help you identify why the command couldn't be launched.

The name `which_problem` works like `which` to but helps you identify common mistakes such as typos:

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

## What won't it do?

The `fun_run` library doesn't support executing a `Command` in ways that do not produce an
`Output`, for example calling `Command::spawn` returns a `Result<std::process::Child, std::io::Error>`
(Which doesn't contain an `Output`). If you want to run-for-fun in the background, spawn a thread
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

## FUN(ctional)

If you don't want to use the trait, you can still use `fun_run` by functionally mapping the
features you want:

```rust
let mut cmd = std::process::Command::new("bundle");
cmd.args(["install"]);

let name = fun_run::display(&mut cmd);

cmd.output()
    .map_err(|error| fun_run::on_system_error(name.clone(), error))
    .and_then(|output| fun_run::nonzero_captured(name.clone(), output))
    .unwrap();
```

Here's some fun functions you can use to help you run:

- [`on_system_error`] - Convert `std::io::Error` into `CmdError`
- [`nonzero_streamed`] - Produces a `NamedOutput` from `Output` that has already been streamd to
  the user
- [`nonzero_captured`] - Like `nonzero_streamed` but for when the user hasn't already seen the
  output
- [`display`] - Converts an `&mut Command` into a human readable string
- [`display_with_env_keys`] - Like `display` but selectively shows environment variables.

## Async

This library uses syncronous command execution. If you’re using this library in an async context,
you’ll want to use an async wrapper like [tokio::task::block_in_place](https://docs.rs/tokio/latest/tokio/task/fn.block_in_place.html).

<!-- cargo-rdme end -->

## Development

Update the readme:

```
$ cargo install cargo-rdme
$ cargo rdme
```
