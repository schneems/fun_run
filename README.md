# Fun Run

What does the "Zombie Zoom 5K", the "Wibbly wobbly log jog", and the "Turkey Trot" have in common? They're runs with a fun name! That's exactly what `fun_run` does. It makes running your Rust `Command`s more fun, by naming them.

## What is Fun Run?

Fun run is designed for the use case where not only do you want to run a `Command` you want to output what you're running and what happened. Building a CLI tool is a great use case. Another is creating [a buildpack](https://github.com/heroku/buildpacks-ruby/tree/4f514f6046568ada523eefd41b3024f86f1c67ce).

Here's some things you can do with fun_run:

- Advertise the command being run before execution
- Customize how commands are displayed
- Return error messages with the command name.
- Turn non-zero status results into an error
- Embed stdout and stderr into errors (when not streamed)
- Store stdout and stderr for debug and diagnosis without displaying them (when streamed)

Just like you don't need to dress up in a giant turkey costume to run a 5K you also don't **need** `fun_run` to do these things. Though, unlike the turkey costume, using `fun_run` will also make the experience easier.

## Ready to Roll

For a quick and easy fun run you can use the `fun_run::CommandWithName` trait extension to stream output:

```no_run
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

Or capture output without streaming:

```no_run
use fun_run::CommandWithName;
use std::process::Command;

let mut cmd = Command::new("bundle");
cmd.args(["install"]);

// Advertise the command being run before execution
println!("Quietly Running `{name}`", name = cmd.name());

// Don't stream
// Turn non-zero status results into an error
let result = cmd.named_output();

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

The `fun_run` library doesn't support executing a `Command` in ways that do not produce an `Output`, for example calling `Command::spawn` returns a `Result<std::process::Child, std::io::Error>` (Which doesn't contain an `Output`). If you want to run for fun in the background, spawn a thread and join it manually:

```no_run
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
